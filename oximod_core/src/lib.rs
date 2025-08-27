/// Macro for attaching `Printable` info to an error with an optional suggestion message.
///
/// # Example
/// ```ignore
/// return Err(attach!(MyError::Something, "Check your DB connection"));
/// ```
#[macro_export]
macro_rules! attach_printables {
    // No suggestion — just attach backtrace if enabled.
    ($err:expr) => {{
        let __e = $crate::error::printable::Printable::attach_printables($err);
        __e
    }};

    // Static suggestion: &'static str or concat!(...) — zero alloc.
    ($err:expr, @static $msg:expr) => {{
        let __e = $crate::error::printable::Printable::attach_printables($err);
        if $crate::error::printable::diagnostics_enabled() {
            $crate::error::printable::print_suggestion_cold($msg);
        }
        __e
    }};

    // Formatted suggestion using `format_args!` — zero alloc.
    ($err:expr, @fmt $($arg:tt)+) => {{
        let __e = $crate::error::printable::Printable::attach_printables($err);
        if $crate::error::printable::diagnostics_enabled() {
            $crate::error::printable::print_suggestion_args_cold(::std::format_args!($($arg)+));
        }
        __e
    }};

    // Generic Display-able expr (String, &str, etc.) — zero alloc via `format_args!`.
    ($err:expr, $msg:expr) => {{
        let __e = $crate::error::printable::Printable::attach_printables($err);
        if $crate::error::printable::diagnostics_enabled() {
            $crate::error::printable::print_suggestion_args_cold(::std::format_args!("{}", $msg));
        }
        __e
    }};
}

pub mod error;
pub mod feature;
pub mod helpers;
pub use error::printable::Printable;
