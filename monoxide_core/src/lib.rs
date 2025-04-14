/// Macro for attaching `Printable` info to an error with an optional suggestion message.
///
/// # Example
/// ```ignore
/// return Err(attach!(MyError::Something, "Check your DB connection"));
/// ```
#[macro_export]
macro_rules! attach_printables {
    ($err:expr, $msg:expr) => {
        $err.attach_printables(std::backtrace::Backtrace::capture(), Some($msg))
    };
    ($err:expr) => {
        $err.attach_printables(std::backtrace::Backtrace::capture(), None)
    };
}

pub mod error;
pub mod feature;
pub use error::printable::Printable;