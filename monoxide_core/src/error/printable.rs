use std::backtrace::Backtrace;
use std::error::Error;

/// A trait for attaching helpful debugging output to errors.
pub trait Printable {
    fn backtrace(&self, capture: Backtrace) {
        eprintln!("\nBacktrace: {}\n", capture);
    }

    fn suggest(&self, suggest_msg: &str) {
        eprintln!("\nSuggestion: {}\n", suggest_msg);
    }

    fn attach_printables(self, capture: Backtrace, suggest_msg: Option<&str>) -> Self
    where
        Self: Sized,
    {
        self.backtrace(capture);
        if let Some(msg) = suggest_msg {
            self.suggest(msg);
        }
        self
    }
}

// Blanket implementation for all types that implement `Error`
impl<T: Error> Printable for T {}
