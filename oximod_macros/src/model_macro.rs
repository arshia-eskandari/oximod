//! Token generation used by the `Model` derive macro.
//!
//! Generation is divided by responsibility:
//!
//! - `model_token` generates validation support;
//! - `collection_model_token` generates collection persistence methods;
//! - `hook_tokens` generates optional lifecycle-hook invocations.

mod collection_model_token;
mod hook_tokens;
mod model_token;

pub use collection_model_token::generate_collection_model_token;
pub use model_token::generate_model_token;
