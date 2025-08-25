// --- public API ---
pub use oximod_core::feature::conn::client::{get_global_client, set_global_client};
pub use oximod_core::feature::model::Model as ModelTrait;
pub use oximod_macros::Model;

// --- Internal API ---
#[doc(hidden)]
pub use oximod_core::feature as _feature;
#[doc(hidden)]
pub use oximod_core::helpers as _helpers;
#[doc(hidden)]
pub mod _error {
    pub use oximod_core::error::oximod_error;
}
#[doc(hidden)]
pub use async_trait as _async_trait;
#[doc(hidden)]
pub use futures_util as _futures_util;
#[doc(hidden)]
pub use mongodb as _mongodb;
#[doc(hidden)]
pub use oximod_core::attach_printables as _attach_printables;
#[doc(hidden)]
pub use oximod_core::feature::model::Model;
#[doc(hidden)]
pub use regex as _regex; // removes the need of importing the trait
