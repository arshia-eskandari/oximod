//! Type-safe MongoDB aggregation pipelines.
//!
//! This module contains the runtime building blocks for OxiMod's aggregation
//! API. Applications normally begin with
//! [`Queryable::aggregate`](crate::query::Queryable::aggregate) and build an
//! ordered pipeline from typed stages, raw stages, or both, before executing
//! it against the model's collection.
//!
//! Public aggregation types are re-exported from the main `oximod` crate, so
//! applications should normally import them directly from `oximod`.
//!
//! # Example
//!
//! ```ignore
//! let adults = User::aggregate()
//!     .match_(|user| user.active.eq(true) & user.age.gte(18))
//!     .sort_by(|user| user.age.desc())
//!     .limit(20)
//!     .all()
//!     .await?;
//! ```

mod builder;

pub use builder::Aggregation;
