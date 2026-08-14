//! Typed MongoDB bulk-write batches.
//!
//! This module contains the runtime building blocks for OxiMod's bulk-write
//! API. Applications normally begin with
//! [`Model::bulk_write`](crate::feature::model::Model::bulk_write) and queue
//! insert, update, replace, and delete intentions against one model's
//! collection before executing them as a single MongoDB
//! [`bulkWrite`](mongodb::Client::bulk_write) command.
//!
//! Public bulk-write types are re-exported from the main `oximod` crate, so
//! applications should normally import them directly from `oximod`.
//!
//! # Example
//!
//! ```ignore
//! let result = Job::bulk_write()
//!     .insert(job1)
//!     .insert(job2)
//!     .update_one(
//!         Job::query().filter(|job| job.dedupe_key.eq("job-3")),
//!         |job| job.status.set("ready"),
//!     )
//!     .delete_many(Job::query().filter(|job| job.status.eq("expired")))
//!     .ordered(false)
//!     .execute()
//!     .await?;
//! ```
//!
//! Bulk writes require MongoDB Server 8.0 or newer.

mod builder;

pub use builder::BulkWrite;
