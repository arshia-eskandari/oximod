use std::{
    backtrace::{Backtrace, BacktraceStatus},
    error::Error,
    fmt,
    io::IsTerminal,
    sync::atomic::{AtomicBool, Ordering},
};

static OXIMOD_DIAGNOSTICS: AtomicBool = AtomicBool::new(false);

pub fn enable_oximod_diagnostics() {
    OXIMOD_DIAGNOSTICS.store(true, Ordering::Relaxed);
}

#[inline(always)]
pub fn diagnostics_enabled() -> bool {
    #[cfg(any(debug_assertions, feature = "debug-print"))]
    {
        if OXIMOD_DIAGNOSTICS.load(Ordering::Relaxed) {
            return true;
        }
        if std::env::var_os("OXIMOD_DEBUG").is_some() {
            return true;
        }
    }
    false
}

#[inline(always)]
fn stderr_is_tty() -> bool {
    std::io::stderr().is_terminal()
}

#[cold]
#[inline(never)]
pub fn print_backtrace_cold(bt: &Backtrace) {
    match bt.status() {
        BacktraceStatus::Captured => {
            if stderr_is_tty() {
                eprintln!("\x1b[1;33mBacktrace:\x1b[0m\n{bt}");
            } else {
                eprintln!("Backtrace:\n{bt}");
            }
        }
        _ => {}
    }
}

#[cold]
#[inline(never)]
pub fn print_suggestion_cold(msg: &str) {
    if stderr_is_tty() {
        eprintln!("\x1b[1;36mSuggestion:\x1b[0m {msg}");
    } else {
        eprintln!("Suggestion: {msg}");
    }
}

#[cold]
#[inline(never)]
pub fn print_suggestion_args_cold(args: fmt::Arguments<'_>) {
    if stderr_is_tty() {
        eprintln!("\x1b[1;36mSuggestion:\x1b[0m {args}");
    } else {
        eprintln!("Suggestion: {args}");
    }
}

pub trait Printable {
    fn attach_printables(self) -> Self
    where
        Self: Sized,
    {
        if diagnostics_enabled() {
            let bt = Backtrace::capture();
            print_backtrace_cold(&bt);
        }
        self
    }
}

impl<T: Error> Printable for T {}
