//! Retry-aware asynchronous one-time initialization.
//!
//! [`OnceAsync`](crate::helpers::once_async::OnceAsync) ensures that at most
//! one initialization future runs at a
//! time. Concurrent callers wait for the active attempt to finish. A
//! successful attempt permanently completes the initializer, while a failed
//! or cancelled attempt returns it to the uninitialized state so a later call
//! can try again.
//!
//! Retry and duration options are currently diagnostic metadata. They can be
//! inspected with
//! [`OnceAsync::has_exceeded_retries`](crate::helpers::once_async::OnceAsync::has_exceeded_retries)
//! and [`OnceAsync::is_stuck`](crate::helpers::once_async::OnceAsync::is_stuck),
//! but [`OnceAsync::run_once`](crate::helpers::once_async::OnceAsync::run_once)
//! does not independently
//! stop an attempt or manufacture an error when a configured limit is reached.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Mutex, MutexGuard,
        atomic::{
            AtomicU8,
            Ordering::{AcqRel, Acquire, Release},
        },
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

/// Coordinates an asynchronous initialization that should complete once.
///
/// Only one caller executes the initialization closure at a time. Other
/// callers wait for the active attempt:
///
/// - when the attempt succeeds, all waiting callers return `Ok(())`;
/// - when the attempt fails or its future is dropped, the state is reset and
///   waiting callers may start another attempt.
///
/// The type uses standard-library synchronization primitives and does not
/// depend on a specific asynchronous runtime.
pub struct OnceAsync {
    state: AtomicU8,
    waiters: Mutex<Vec<Waker>>,
    meta: Mutex<Meta>,
}

struct Meta {
    failed_attempts: u32,
    started_at: Option<Instant>,
    max_retries: Option<u32>,
    max_init_duration: Option<Duration>,
}

impl Default for OnceAsync {
    fn default() -> Self {
        Self::new()
    }
}

impl OnceAsync {
    const UNINITIALIZED: u8 = 0;
    const IN_PROGRESS: u8 = 1;
    const COMPLETED: u8 = 2;

    /// Creates an unconfigured asynchronous initializer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(Self::UNINITIALIZED),
            waiters: Mutex::new(Vec::new()),
            meta: Mutex::new(Meta {
                failed_attempts: 0,
                started_at: None,
                max_retries: None,
                max_init_duration: None,
            }),
        }
    }

    /// Creates an initializer with optional retry and duration metadata.
    ///
    /// These values are exposed through [`OnceAsync::has_exceeded_retries`]
    /// and [`OnceAsync::is_stuck`]. They are not automatically enforced by
    /// [`OnceAsync::run_once`].
    #[must_use]
    pub const fn new_with_options(
        max_retries: Option<u32>,
        max_init_duration: Option<Duration>,
    ) -> Self {
        Self {
            state: AtomicU8::new(Self::UNINITIALIZED),
            waiters: Mutex::new(Vec::new()),
            meta: Mutex::new(Meta {
                failed_attempts: 0,
                started_at: None,
                max_retries,
                max_init_duration,
            }),
        }
    }

    /// Sets the retry threshold using builder-style syntax.
    ///
    /// This is an alias for [`OnceAsync::with_max_retries`].
    #[must_use]
    pub fn max_retries(self, max_retries: u32) -> Self {
        self.with_max_retries(max_retries)
    }

    /// Sets the maximum initialization duration in seconds using builder-style
    /// syntax.
    ///
    /// This is an alias for [`OnceAsync::with_max_init_secs`].
    #[must_use]
    pub fn max_init(self, seconds: u64) -> Self {
        self.with_max_init_secs(seconds)
    }

    /// Sets the retry threshold using builder-style syntax.
    ///
    /// The threshold is diagnostic and can be checked with
    /// [`OnceAsync::has_exceeded_retries`].
    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.meta_mut().max_retries = Some(max_retries);
        self
    }

    /// Sets the maximum initialization duration in seconds using builder-style
    /// syntax.
    ///
    /// The duration is diagnostic and can be checked with
    /// [`OnceAsync::is_stuck`].
    #[must_use]
    pub fn with_max_init_secs(self, seconds: u64) -> Self {
        self.with_max_init(Duration::from_secs(seconds))
    }

    /// Sets the maximum initialization duration using builder-style syntax.
    ///
    /// The duration is diagnostic and can be checked with
    /// [`OnceAsync::is_stuck`].
    #[must_use]
    pub fn with_max_init(mut self, duration: Duration) -> Self {
        self.meta_mut().max_init_duration = Some(duration);
        self
    }

    /// Replaces the retry threshold.
    ///
    /// The threshold is diagnostic and can be checked with
    /// [`OnceAsync::has_exceeded_retries`].
    pub fn set_max_retries(&self, max_retries: u32) -> &Self {
        self.lock_meta().max_retries = Some(max_retries);
        self
    }

    /// Replaces the maximum initialization duration using seconds.
    ///
    /// The duration is diagnostic and can be checked with
    /// [`OnceAsync::is_stuck`].
    pub fn set_max_init_secs(&self, seconds: u64) -> &Self {
        self.set_max_init(Duration::from_secs(seconds))
    }

    /// Replaces the maximum initialization duration.
    ///
    /// The duration is diagnostic and can be checked with
    /// [`OnceAsync::is_stuck`].
    pub fn set_max_init(&self, duration: Duration) -> &Self {
        self.lock_meta().max_init_duration = Some(duration);
        self
    }

    /// Returns whether initialization has completed successfully.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.state.load(Acquire) == Self::COMPLETED
    }

    /// Returns the number of failed initialization attempts.
    ///
    /// Successful attempts are not included. The counter saturates at
    /// [`u32::MAX`].
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.lock_meta().failed_attempts
    }

    /// Returns whether the failed-attempt count has reached the configured
    /// retry threshold.
    ///
    /// Returns `false` when no threshold is configured. This method does not
    /// prevent a later call to [`OnceAsync::run_once`] from trying again.
    #[must_use]
    pub fn has_exceeded_retries(&self) -> bool {
        let meta = self.lock_meta();

        match meta.max_retries {
            Some(limit) => meta.failed_attempts >= limit,
            None => false,
        }
    }

    /// Returns whether the active attempt has run longer than its configured
    /// maximum duration.
    ///
    /// Returns `false` when initialization is not active or no duration is
    /// configured. This method observes the active attempt but does not cancel
    /// it.
    #[must_use]
    pub fn is_stuck(&self) -> bool {
        if self.state.load(Acquire) != Self::IN_PROGRESS {
            return false;
        }

        let meta = self.lock_meta();

        match (meta.started_at, meta.max_init_duration) {
            (Some(started_at), Some(limit)) => started_at.elapsed() >= limit,
            _ => false,
        }
    }

    /// Runs an asynchronous initialization at most once successfully.
    ///
    /// The closure is called only by the caller that transitions the
    /// initializer from uninitialized to in progress. Concurrent callers wait
    /// until the active attempt succeeds, fails, or is cancelled.
    ///
    /// A failed attempt increments [`OnceAsync::attempts`] and returns its
    /// original error. A later call may retry. A successful attempt marks the
    /// initializer as complete, and all future calls return `Ok(())` without
    /// invoking their closures.
    ///
    /// # Errors
    ///
    /// Returns the error produced by the initialization closure.
    pub async fn run_once<F, Fut, E>(&self, mut initialize: F) -> Result<(), E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<(), E>>,
    {
        loop {
            if self.is_completed() {
                return Ok(());
            }

            if self
                .state
                .compare_exchange(Self::UNINITIALIZED, Self::IN_PROGRESS, AcqRel, Acquire)
                .is_ok()
            {
                let mut guard = InProgressGuard {
                    once: self,
                    completed: false,
                };

                self.lock_meta().started_at = Some(Instant::now());

                let result = initialize().await;

                self.lock_meta().started_at = None;

                match result {
                    Ok(()) => {
                        self.state.store(Self::COMPLETED, Release);
                        guard.completed = true;
                        self.notify_all();

                        return Ok(());
                    }
                    Err(error) => {
                        let mut meta = self.lock_meta();
                        meta.failed_attempts = meta.failed_attempts.saturating_add(1);

                        return Err(error);
                    }
                }
            } else {
                Wait { once: self }.await;
            }
        }
    }

    fn meta_mut(&mut self) -> &mut Meta {
        match self.meta.get_mut() {
            Ok(meta) => meta,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn lock_meta(&self) -> MutexGuard<'_, Meta> {
        match self.meta.lock() {
            Ok(meta) => meta,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn lock_waiters(&self) -> MutexGuard<'_, Vec<Waker>> {
        match self.waiters.lock() {
            Ok(waiters) => waiters,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn notify_all(&self) {
        let waiters = std::mem::take(&mut *self.lock_waiters());

        for waiter in waiters {
            waiter.wake();
        }
    }
}

struct InProgressGuard<'a> {
    once: &'a OnceAsync,
    completed: bool,
}

impl Drop for InProgressGuard<'_> {
    fn drop(&mut self) {
        if self.completed {
            return;
        }

        self.once.state.store(OnceAsync::UNINITIALIZED, Release);
        self.once.lock_meta().started_at = None;
        self.once.notify_all();
    }
}

struct Wait<'a> {
    once: &'a OnceAsync,
}

impl Future for Wait<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.once.state.load(Acquire) != OnceAsync::IN_PROGRESS {
            return Poll::Ready(());
        }

        let mut waiters = self.once.lock_waiters();

        if self.once.state.load(Acquire) != OnceAsync::IN_PROGRESS {
            return Poll::Ready(());
        }

        if let Some(position) = waiters
            .iter()
            .position(|waiter| waiter.will_wake(context.waker()))
        {
            waiters[position] = context.waker().clone();
        } else {
            waiters.push(context.waker().clone());
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::OnceAsync;
    use std::{
        cell::Cell,
        future::{Future, ready},
        task::{Context, Poll, Waker},
        time::Duration,
    };

    fn block_on_ready<F>(future: F) -> F::Output
    where
        F: Future,
    {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = std::pin::pin!(future);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => {
                panic!("test future unexpectedly returned Pending")
            }
        }
    }

    #[test]
    fn new_initializer_is_uninitialized() {
        let once = OnceAsync::new();

        assert!(!once.is_completed());
        assert_eq!(once.attempts(), 0);
        assert!(!once.has_exceeded_retries());
        assert!(!once.is_stuck());
    }

    #[test]
    fn successful_initialization_runs_only_once() {
        let once = OnceAsync::new();
        let calls = Cell::new(0);

        block_on_ready(once.run_once(|| {
            calls.set(calls.get() + 1);
            ready(Ok::<(), ()>(()))
        }))
        .expect("first initialization should succeed");

        block_on_ready(once.run_once(|| {
            calls.set(calls.get() + 1);
            ready(Ok::<(), ()>(()))
        }))
        .expect("completed initialization should remain successful");

        assert!(once.is_completed());
        assert_eq!(once.attempts(), 0);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn failed_initialization_can_be_retried() {
        let once = OnceAsync::new();

        let error =
            block_on_ready(once.run_once(|| ready::<Result<(), &'static str>>(Err("failed"))))
                .expect_err("first initialization should fail");

        assert_eq!(error, "failed");
        assert!(!once.is_completed());
        assert_eq!(once.attempts(), 1);

        block_on_ready(once.run_once(|| ready(Ok::<(), &'static str>(()))))
            .expect("second initialization should succeed");

        assert!(once.is_completed());
        assert_eq!(once.attempts(), 1);
    }

    #[test]
    fn retry_threshold_is_observable() {
        let once = OnceAsync::new().with_max_retries(2);

        assert!(!once.has_exceeded_retries());

        for _ in 0..2 {
            let _ = block_on_ready(once.run_once(|| ready::<Result<(), ()>>(Err(()))));
        }

        assert!(once.has_exceeded_retries());
    }

    #[test]
    fn builder_and_setter_options_are_recorded() {
        let once = OnceAsync::new().max_retries(3).max_init(5);

        assert!(!once.has_exceeded_retries());
        assert!(!once.is_stuck());

        once.set_max_retries(1).set_max_init(Duration::from_secs(1));

        let _ = block_on_ready(once.run_once(|| ready::<Result<(), ()>>(Err(()))));

        assert!(once.has_exceeded_retries());
    }
}
