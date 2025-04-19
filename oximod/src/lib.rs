// --- public API ---
pub use oximod_core::feature::model::Model as ModelTrait;
pub use oximod_core::feature::conn::client::set_global_client;
pub use oximod_macros::Model;

#[doc(hidden)]
pub use oximod_core::feature;
#[doc(hidden)]
pub mod error {
    pub use oximod_core::error::conn_error;
    pub use oximod_core::error::printable;
}
#[doc(hidden)]
pub use oximod_core::attach_printables;
#[doc(hidden)]
pub extern crate async_trait;
#[doc(hidden)]
pub extern crate futures_util;
#[doc(hidden)]
pub use oximod_core::feature::model::Model; // removes the need of importing the trait
