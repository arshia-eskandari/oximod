use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{
            AtomicU8,
            Ordering::{AcqRel, Acquire, Release},
        },
        Mutex,
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

pub struct OnceAsync {
    state: AtomicU8,
    waiters: Mutex<Vec<Waker>>,
    meta: Mutex<Meta>,
}

struct Meta {
    attempts: u32,
    started_at: Option<Instant>,
    max_retries: Option<u32>,
    max_init: Option<Duration>,
}

impl OnceAsync {
    const UNINIT: u8 = 0;
    const INPROG: u8 = 1;
    const DONE: u8 = 2;

    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(Self::UNINIT),
            waiters: Mutex::new(Vec::new()),
            meta: Mutex::new(Meta {
                attempts: 0,
                started_at: None,
                max_retries: None,
                max_init: None,
            }),
        }
    }

    pub const fn new_with_options(max_retries: Option<u32>, max_init: Option<Duration>) -> Self {
        Self {
            state: AtomicU8::new(Self::UNINIT),
            waiters: Mutex::new(Vec::new()),
            meta: Mutex::new(Meta {
                attempts: 0,
                started_at: None,
                max_retries: max_retries,
                max_init: max_init,
            }),
        }
    }

    pub fn max_retries(self, n: u32) -> Self {
        self.with_max_retries(n)
    }

    pub fn max_init(self, secs: u64) -> Self {
        self.with_max_init_secs(secs)
    }

    pub fn with_max_retries(mut self, n: u32) -> Self {
        let meta = self.meta.get_mut();
        #[allow(clippy::unnecessary_mut_passed)]
        {
            let m = meta.unwrap();
            m.max_retries = Some(n);
        }
        self
    }

    pub fn with_max_init_secs(self, secs: u64) -> Self {
        self.with_max_init(Duration::from_secs(secs))
    }

    pub fn with_max_init(mut self, d: Duration) -> Self {
        let meta = self.meta.get_mut();
        #[allow(clippy::unnecessary_mut_passed)]
        {
            let m = meta.unwrap();
            m.max_init = Some(d);
        }
        self
    }

    pub fn set_max_retries(&self, n: u32) -> &Self {
        self.meta.lock().unwrap().max_retries = Some(n);
        self
    }

    pub fn set_max_init_secs(&self, secs: u64) -> &Self {
        self.set_max_init(Duration::from_secs(secs))
    }

    pub fn set_max_init(&self, d: Duration) -> &Self {
        self.meta.lock().unwrap().max_init = Some(d);
        self
    }

    pub fn is_completed(&self) -> bool {
        self.state.load(Acquire) == Self::DONE
    }

    pub fn attempts(&self) -> u32 {
        self.meta.lock().unwrap().attempts
    }

    pub fn has_exceeded_retries(&self) -> bool {
        let m = self.meta.lock().unwrap();
        match m.max_retries {
            Some(limit) => m.attempts >= limit,
            None => false,
        }
    }

    pub fn is_stuck(&self) -> bool {
        if self.state.load(Acquire) != Self::INPROG {
            return false;
        }
        let m = self.meta.lock().unwrap();
        match (m.started_at, m.max_init) {
            (Some(start), Some(limit)) => start.elapsed() > limit,
            _ => false,
        }
    }

    pub async fn run_once<F, Fut, E>(&self, mut init: F) -> Result<(), E>
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
                .compare_exchange(Self::UNINIT, Self::INPROG, AcqRel, Acquire)
                .is_ok()
            {
                let mut guard = InProgGuard {
                    once: self,
                    done: false,
                };

                {
                    let mut m = self.meta.lock().unwrap();
                    m.started_at = Some(Instant::now());
                }

                let res = init().await;

                {
                    let mut m = self.meta.lock().unwrap();
                    m.started_at = None;
                }

                match res {
                    Ok(()) => {
                        self.state.store(Self::DONE, Release);
                        guard.done = true;
                        self.notify_all();
                        return Ok(());
                    }
                    Err(e) => {
                        {
                            let mut m = self.meta.lock().unwrap();
                            m.attempts = m.attempts.saturating_add(1);
                        }
                        return Err(e);
                    }
                }
            } else {
                Wait { once: self }.await;
            }
        }
    }

    fn notify_all(&self) {
        let waiters = std::mem::take(&mut *self.waiters.lock().unwrap());
        for w in waiters {
            w.wake();
        }
    }
}

struct InProgGuard<'a> {
    once: &'a OnceAsync,
    done: bool,
}

impl<'a> Drop for InProgGuard<'a> {
    fn drop(&mut self) {
        if !self.done {
            self.once.state.store(OnceAsync::UNINIT, Release);
            let mut m = self.once.meta.lock().unwrap();
            m.started_at = None;
            drop(m);
            self.once.notify_all();
        }
    }
}

struct Wait<'a> {
    once: &'a OnceAsync,
}

impl<'a> Future for Wait<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.once.state.load(Acquire) != OnceAsync::INPROG {
            return Poll::Ready(());
        }

        let mut ws = self.once.waiters.lock().unwrap();
        if self.once.state.load(Acquire) != OnceAsync::INPROG {
            return Poll::Ready(());
        }

        if let Some(pos) = ws.iter().position(|w| w.will_wake(cx.waker())) {
            ws[pos] = cx.waker().clone();
        } else {
            ws.push(cx.waker().clone());
        }

        Poll::Pending
    }
}
