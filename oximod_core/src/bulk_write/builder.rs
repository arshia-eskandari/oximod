//! Typed bulk-write batch construction and execution.
//!
//! This module contains [`BulkWrite`], the builder produced by
//! [`Model::bulk_write`](crate::feature::model::Model::bulk_write). It stores
//! typed write intentions in call order until an execution method sends them
//! to MongoDB as one [`bulkWrite`](mongodb::Client::bulk_write) command.

use std::marker::PhantomData;

use mongodb::{
    Client, ClientSession, Collection as MongoCollection,
    bson::{Array, Bson, Document},
    options::{
        BulkWriteOptions, DeleteManyModel, DeleteOneModel, UpdateManyModel, UpdateModifications,
        UpdateOneModel, WriteModel,
    },
    results::{SummaryBulkWriteResult, VerboseBulkWriteResult},
};

use crate::{
    error::{
        classify::{OperationDomain, classify_driver_error},
        oximod_error::OxiModError,
        query_error::{BulkWriteOperation, QueryError, QueryModifier},
    },
    feature::{conn::client::OxiClient, model::Model},
    query::{Query, QueryParts, Queryable, UpdateExpression},
};

/// A typed batch of MongoDB write operations for model `M`.
///
/// Bulk writes are normally created through
/// [`Model::bulk_write`](crate::feature::model::Model::bulk_write) rather
/// than constructed directly.
///
/// The builder queues insert, update, replace, and delete intentions against
/// one model's collection and executes them as a **single** MongoDB
/// [`bulkWrite`](mongodb::Client::bulk_write) command — never as one network
/// call per queued operation. Operations execute in exactly the order they
/// were queued (subject to MongoDB's unordered semantics when
/// [`BulkWrite::ordered`] is set to `false`).
///
/// # Example
///
/// ```ignore
/// let result = Job::bulk_write()
///     .insert(job1)
///     .insert(job2)
///     .update_one(
///         Job::query().filter(|job| job.dedupe_key.eq("job-3")),
///         |job| job.status.set("ready"),
///     )
///     .delete_many(Job::query().filter(|job| job.status.eq("expired")))
///     .ordered(false)
///     .execute()
///     .await?;
/// ```
///
/// # Server requirement
///
/// The underlying `bulkWrite` command requires **MongoDB Server 8.0 or
/// newer**. OxiMod does not emulate it on older servers: a server that
/// cannot execute the command fails through the ordinary bulk-write error
/// path ([`OxiModError::BulkWrite`]) with the driver's incompatibility error
/// preserved as the source. OxiMod never silently expands a bulk write into
/// individual write commands.
///
/// # Validation
///
/// Whole model values queued through [`BulkWrite::insert`],
/// [`BulkWrite::insert_many`], and [`BulkWrite::replace_one`] run OxiMod's
/// `#[validate]` rules for **every** queued value before any network
/// communication. If any queued model fails validation, the batch returns
/// [`OxiModError::Validation`] and **zero** writes are sent.
///
/// Typed update expressions queued through [`BulkWrite::update_one`] and
/// [`BulkWrite::update_many`] do not run whole-model validation, exactly
/// like [`Query::update_one`] and [`Query::update_all`].
///
/// # Lifecycle hooks
///
/// Bulk-write operations are a separate execution surface. They preserve
/// typed query/update construction and whole-model validation where
/// applicable, but they do **not** invoke OxiMod lifecycle hooks.
///
/// # Indexes
///
/// Executing a bulk write never establishes declared `#[index(...)]`
/// specifications. Initialize indexes explicitly before relying on them —
/// especially unique constraints used as idempotency guards — with
/// `ModelType::init_indexes()` or `ModelType::init_indexes_from(&client)`.
///
/// # Ordered and unordered execution
///
/// MongoDB executes bulk writes in order by default and stops after the
/// first failing operation. With [`BulkWrite::ordered`] set to `false`, the
/// server continues attempting the remaining operations after a failure and
/// may reorder execution for performance; applications must not depend on
/// execution order when unordered.
///
/// # Failed and partially failed batches
///
/// When individual writes fail, the returned [`OxiModError::BulkWrite`]
/// preserves the driver's [`mongodb::error::BulkWriteError`], whose
/// `write_errors` map is keyed by the **original queued operation index**
/// and whose `partial_result` describes the writes that succeeded. See
/// [`OxiModError::bulk_write_error`] for direct access.
///
/// # Retries
///
/// Retryable-write behavior belongs to the MongoDB driver. OxiMod adds no
/// retry loop of its own.
#[must_use = "bulk writes do nothing unless an execution method is called"]
#[derive(Debug, Clone)]
pub struct BulkWrite<M> {
    operations: Vec<PendingOperation<M>>,
    options: Option<BulkWriteOptions>,
    error: Option<QueryError>,
    marker: PhantomData<fn() -> M>,
}

/// One queued logical write intention.
///
/// Whole model values stay unserialized until execution so the complete
/// validation preflight can run before any BSON conversion or network
/// communication.
#[derive(Debug, Clone)]
enum PendingOperation<M> {
    Insert {
        model: M,
    },
    UpdateOne {
        filter: Document,
        update: Document,
        array_filters: Vec<Document>,
        sort: Option<Document>,
    },
    UpdateMany {
        filter: Document,
        update: Document,
        array_filters: Vec<Document>,
    },
    ReplaceOne {
        filter: Document,
        replacement: M,
        sort: Option<Document>,
    },
    DeleteOne {
        filter: Document,
    },
    DeleteMany {
        filter: Document,
    },
}

impl<M> BulkWrite<M> {
    /// Creates an empty bulk-write batch.
    ///
    /// This constructor is used by
    /// [`Model::bulk_write`](crate::feature::model::Model::bulk_write).
    /// Applications should normally call `User::bulk_write()` on a derived
    /// model instead.
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self {
            operations: Vec::new(),
            options: None,
            error: None,
            marker: PhantomData,
        }
    }

    fn set_error(&mut self, error: QueryError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    /// Checks a consumed query against what the driver write model for
    /// `operation` can represent.
    ///
    /// Unsupported modifiers fail here — deterministically and before any
    /// network communication — rather than being silently dropped.
    fn preflight(
        operation: BulkWriteOperation,
        parts: &QueryParts,
        sort_supported: bool,
        array_filters_supported: bool,
    ) -> Result<(), QueryError> {
        if parts.pagination {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Pagination,
            });
        }

        if !sort_supported && parts.sort.is_some() {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Sort,
            });
        }

        if parts.has_skip {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Skip,
            });
        }

        if parts.has_limit {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Limit,
            });
        }

        if !array_filters_supported && !parts.array_filters.is_empty() {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::ArrayFilters,
            });
        }

        Ok(())
    }

    /// Consumes a query for one queued operation, adopting its deferred
    /// construction error and running the operation's modifier preflight.
    ///
    /// Returns `None` — after recording the first error — when the operation
    /// must not be queued.
    fn admit_query(
        &mut self,
        operation: BulkWriteOperation,
        parts: QueryParts,
        sort_supported: bool,
        array_filters_supported: bool,
    ) -> Option<QueryParts> {
        if let Some(error) = &parts.error {
            self.set_error(error.clone());
            return None;
        }

        if let Err(error) =
            Self::preflight(operation, &parts, sort_supported, array_filters_supported)
        {
            self.set_error(error);
            return None;
        }

        Some(parts)
    }

    /// Queues one document insert.
    ///
    /// The model value is validated during execution, before any network
    /// communication, together with every other queued insert and
    /// replacement. It is serialized with the same Serde representation used
    /// by the rest of OxiMod.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write()
    ///     .insert(Job::new().dedupe_key("job-1"))
    ///     .insert(Job::new().dedupe_key("job-2"));
    /// ```
    ///
    /// # Parameters
    ///
    /// - `model`: The model value to insert.
    pub fn insert(mut self, model: M) -> Self {
        self.operations.push(PendingOperation::Insert { model });
        self
    }

    /// Queues one insert per model value, preserving iteration order.
    ///
    /// Equivalent to calling [`BulkWrite::insert`] for each value.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write().insert_many(jobs);
    /// ```
    ///
    /// # Parameters
    ///
    /// - `models`: The model values to insert.
    pub fn insert_many(mut self, models: impl IntoIterator<Item = M>) -> Self {
        self.operations.extend(
            models
                .into_iter()
                .map(|model| PendingOperation::Insert { model }),
        );
        self
    }

    /// Sets the driver options for the whole batch.
    ///
    /// This **replaces** the current options state, including any ordering
    /// configured through [`BulkWrite::ordered`]; the last configuring call
    /// wins. Passing `None` restores the driver defaults.
    ///
    /// The driver's [`BulkWriteOptions`] cover ordered execution,
    /// server-side document-validation bypass, a comment, `let` variables,
    /// and the write concern.
    ///
    /// `bypass_document_validation` is MongoDB's **server-side** document
    /// validation option. It does not disable OxiMod's client-side
    /// `#[validate]` preflight for whole models queued through
    /// [`BulkWrite::insert`] and [`BulkWrite::replace_one`]; intentionally
    /// bypassing OxiMod validation requires writing through the MongoDB
    /// driver directly.
    ///
    /// ```ignore
    /// use oximod::BulkWriteOptions;
    ///
    /// let options = BulkWriteOptions::builder()
    ///     .ordered(false)
    ///     .build();
    ///
    /// let batch = Job::bulk_write()
    ///     .insert_many(jobs)
    ///     .with_options(options);
    /// ```
    ///
    /// # Parameters
    ///
    /// - `options`: The driver options to apply, or `None` to clear them.
    pub fn with_options(mut self, options: impl Into<Option<BulkWriteOptions>>) -> Self {
        self.options = options.into();
        self
    }

    /// Configures ordered or unordered execution.
    ///
    /// MongoDB defaults to ordered execution: operations run in queue order
    /// and the server stops after the first failing operation. With
    /// `ordered(false)`, the server continues attempting the remaining
    /// operations after a failure and may reorder execution for
    /// performance; do not depend on execution order when unordered.
    ///
    /// This convenience updates the ordering field of the same options
    /// state that [`BulkWrite::with_options`] sets, preserving the other
    /// configured options; a later `with_options` call replaces the whole
    /// state again. The last configuring call wins.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write()
    ///     .insert_many(jobs)
    ///     .ordered(false);
    /// ```
    ///
    /// # Parameters
    ///
    /// - `ordered`: Whether the server should stop at the first failure.
    pub fn ordered(mut self, ordered: bool) -> Self {
        self.options
            .get_or_insert_with(BulkWriteOptions::default)
            .ordered = Some(ordered);
        self
    }
}

impl<M> Default for BulkWrite<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> BulkWrite<M>
where
    M: Queryable,
{
    /// Queues an update of the first document matching `query`.
    ///
    /// The update closure receives the generated field structure for `M` and
    /// must produce an [`UpdateExpression`], exactly like
    /// [`Query::update_one`]. Typed updates do not run whole-model
    /// validation.
    ///
    /// The query's typed filter, text search, array filters, and sorting are
    /// reused. Sorting selects which matching document is updated, exactly
    /// as in the driver's `UpdateOneModel`. Skipping, limiting, and
    /// pagination cannot be represented by a bulk update and cause the
    /// batch to fail locally before any network communication.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write().update_one(
    ///     Job::query().filter(|job| job.dedupe_key.eq("job-3")),
    ///     |job| job.status.set("ready") & job.attempts.inc(1),
    /// );
    /// ```
    ///
    /// # Parameters
    ///
    /// - `query`: The typed query selecting the document to update.
    /// - `build`: A closure producing the typed update expression.
    pub fn update_one<F>(mut self, query: Query<M>, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> UpdateExpression,
    {
        let Some(parts) = self.admit_query(
            BulkWriteOperation::UpdateOne,
            query.into_parts(),
            true,
            true,
        ) else {
            return self;
        };

        let fields = M::fields();
        let update = build(&fields).into_document();

        self.operations.push(PendingOperation::UpdateOne {
            filter: parts.filter,
            update,
            array_filters: parts.array_filters,
            sort: parts.sort,
        });

        self
    }

    /// Queues an update of every document matching `query`.
    ///
    /// The update closure receives the generated field structure for `M` and
    /// must produce an [`UpdateExpression`], exactly like
    /// [`Query::update_all`]. Typed updates do not run whole-model
    /// validation.
    ///
    /// The query's typed filter, text search, and array filters are reused.
    /// Sorting, skipping, limiting, and pagination cannot be represented by
    /// a bulk multi-document update and cause the batch to fail locally
    /// before any network communication.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write().update_many(
    ///     Job::query().filter(|job| job.status.eq("stale")),
    ///     |job| job.status.set("expired"),
    /// );
    /// ```
    ///
    /// # Warning
    ///
    /// An empty query updates every document in the model's collection.
    ///
    /// # Parameters
    ///
    /// - `query`: The typed query selecting the documents to update.
    /// - `build`: A closure producing the typed update expression.
    pub fn update_many<F>(mut self, query: Query<M>, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> UpdateExpression,
    {
        let Some(parts) = self.admit_query(
            BulkWriteOperation::UpdateMany,
            query.into_parts(),
            false,
            true,
        ) else {
            return self;
        };

        let fields = M::fields();
        let update = build(&fields).into_document();

        self.operations.push(PendingOperation::UpdateMany {
            filter: parts.filter,
            update,
            array_filters: parts.array_filters,
        });

        self
    }

    /// Queues a replacement of the first document matching `query`.
    ///
    /// The replacement value is validated during execution, before any
    /// network communication, together with every other queued insert and
    /// replacement, and is serialized with the same Serde representation
    /// used by the rest of OxiMod.
    ///
    /// The query's typed filter, text search, and sorting are reused.
    /// Sorting selects which matching document is replaced, exactly as in
    /// the driver's `ReplaceOneModel`. Skipping, limiting, pagination, and
    /// array filters cannot be represented by a bulk replacement and cause
    /// the batch to fail locally before any network communication.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write().replace_one(
    ///     Job::query().filter(|job| job.dedupe_key.eq("job-3")),
    ///     replacement_job,
    /// );
    /// ```
    ///
    /// # Warning
    ///
    /// MongoDB replaces the **whole stored document** with the serialized
    /// model. Fields present in the stored document but not represented by
    /// the Rust model — for example fields written by a newer application
    /// version — are removed by the replacement. When such documents may
    /// exist, prefer targeted typed updates over whole-document
    /// replacement.
    ///
    /// # Parameters
    ///
    /// - `query`: The typed query selecting the document to replace.
    /// - `replacement`: The model value that replaces the stored document.
    pub fn replace_one(mut self, query: Query<M>, replacement: M) -> Self {
        let Some(parts) = self.admit_query(
            BulkWriteOperation::ReplaceOne,
            query.into_parts(),
            true,
            false,
        ) else {
            return self;
        };

        self.operations.push(PendingOperation::ReplaceOne {
            filter: parts.filter,
            replacement,
            sort: parts.sort,
        });

        self
    }

    /// Queues a deletion of the first document matching `query`.
    ///
    /// The query's typed filter and text search are reused. Unlike
    /// [`Query::delete_one`], the driver's bulk `DeleteOneModel` cannot
    /// carry a sort, so a configured sort — as well as skipping, limiting,
    /// pagination, and array filters — causes the batch to fail locally
    /// before any network communication rather than being silently
    /// ignored.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write().delete_one(
    ///     Job::query().filter(|job| job.dedupe_key.eq("job-3")),
    /// );
    /// ```
    ///
    /// # Parameters
    ///
    /// - `query`: The typed query selecting the document to delete.
    pub fn delete_one(mut self, query: Query<M>) -> Self {
        let Some(parts) = self.admit_query(
            BulkWriteOperation::DeleteOne,
            query.into_parts(),
            false,
            false,
        ) else {
            return self;
        };

        self.operations.push(PendingOperation::DeleteOne {
            filter: parts.filter,
        });

        self
    }

    /// Queues a deletion of every document matching `query`.
    ///
    /// The query's typed filter and text search are reused. Sorting,
    /// skipping, limiting, pagination, and array filters cannot be
    /// represented by a bulk multi-document deletion and cause the batch to
    /// fail locally before any network communication.
    ///
    /// ```ignore
    /// let batch = Job::bulk_write().delete_many(
    ///     Job::query().filter(|job| job.status.eq("expired")),
    /// );
    /// ```
    ///
    /// # Warning
    ///
    /// An empty query deletes every document in the model's collection.
    ///
    /// # Parameters
    ///
    /// - `query`: The typed query selecting the documents to delete.
    pub fn delete_many(mut self, query: Query<M>) -> Self {
        let Some(parts) = self.admit_query(
            BulkWriteOperation::DeleteMany,
            query.into_parts(),
            false,
            false,
        ) else {
            return self;
        };

        self.operations.push(PendingOperation::DeleteMany {
            filter: parts.filter,
        });

        self
    }
}

impl<M> BulkWrite<M>
where
    M: Queryable + Model,
{
    /// Runs the complete local preflight and converts the queued operations
    /// into driver write models in queue order.
    ///
    /// The preflight returns the first deferred builder error, then
    /// validates **every** queued insert and replacement before any BSON
    /// serialization. A failure therefore always means zero writes were
    /// sent.
    fn materialize(
        self,
        collection: &MongoCollection<M>,
    ) -> Result<(Vec<WriteModel>, Option<BulkWriteOptions>), OxiModError> {
        let Self {
            operations,
            options,
            error,
            marker: _,
        } = self;

        if let Some(error) = error {
            return Err(error.into());
        }

        for operation in &operations {
            match operation {
                PendingOperation::Insert { model } => model.validate()?,
                PendingOperation::ReplaceOne { replacement, .. } => replacement.validate()?,
                _ => {}
            }
        }

        let namespace = collection.namespace();
        let mut models = Vec::with_capacity(operations.len());

        for operation in operations {
            let model = match operation {
                PendingOperation::Insert { model } => {
                    WriteModel::InsertOne(collection.insert_one_model(&model).map_err(|error| {
                        classify_driver_error(
                            "Failed to serialize a queued bulk-write insert",
                            OperationDomain::BulkWrite,
                            error,
                        )
                    })?)
                }
                PendingOperation::UpdateOne {
                    filter,
                    update,
                    array_filters,
                    sort,
                } => {
                    let mut model = UpdateOneModel::builder()
                        .namespace(namespace.clone())
                        .filter(filter)
                        .update(UpdateModifications::Document(update))
                        .build();

                    model.array_filters = into_array_filters(array_filters);
                    model.sort = sort;

                    WriteModel::UpdateOne(model)
                }
                PendingOperation::UpdateMany {
                    filter,
                    update,
                    array_filters,
                } => {
                    let mut model = UpdateManyModel::builder()
                        .namespace(namespace.clone())
                        .filter(filter)
                        .update(UpdateModifications::Document(update))
                        .build();

                    model.array_filters = into_array_filters(array_filters);

                    WriteModel::UpdateMany(model)
                }
                PendingOperation::ReplaceOne {
                    filter,
                    replacement,
                    sort,
                } => {
                    let mut model =
                        collection
                            .replace_one_model(filter, &replacement)
                            .map_err(|error| {
                                classify_driver_error(
                                    "Failed to serialize a queued bulk-write replacement",
                                    OperationDomain::BulkWrite,
                                    error,
                                )
                            })?;

                    model.sort = sort;

                    WriteModel::ReplaceOne(model)
                }
                PendingOperation::DeleteOne { filter } => WriteModel::DeleteOne(
                    DeleteOneModel::builder()
                        .namespace(namespace.clone())
                        .filter(filter)
                        .build(),
                ),
                PendingOperation::DeleteMany { filter } => WriteModel::DeleteMany(
                    DeleteManyModel::builder()
                        .namespace(namespace.clone())
                        .filter(filter)
                        .build(),
                ),
            };

            models.push(model);
        }

        Ok((models, options))
    }

    /// Executes the batch as one MongoDB `bulkWrite` command using the
    /// global [`OxiClient`], returning summary counts.
    ///
    /// See the [type-level documentation](BulkWrite) for validation, hook,
    /// index, ordering, and failure semantics. Bulk writes require MongoDB
    /// Server 8.0+.
    ///
    /// An empty batch is rejected by the MongoDB driver without any network
    /// communication and returns [`OxiModError::BulkWrite`].
    ///
    /// # Returns
    ///
    /// The driver's [`SummaryBulkWriteResult`] with inserted, matched,
    /// modified, upserted, and deleted counts.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the batch contains a deferred construction error
    /// - a queued insert or replacement fails validation (zero writes sent)
    /// - a queued insert or replacement cannot be serialized (zero writes
    ///   sent)
    /// - the global client has not been initialized
    /// - the driver reports a connectivity failure
    /// - the server rejects the command or individual writes fail
    ///   ([`OxiModError::BulkWrite`], with per-operation indexes and any
    ///   partial result preserved through the source)
    pub async fn execute(self) -> Result<SummaryBulkWriteResult, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        self.execute_from(client).await
    }

    /// Executes the batch through an explicit client, returning summary
    /// counts.
    ///
    /// This is the explicit-client counterpart to [`BulkWrite::execute`]:
    /// the model collection is resolved from `client` and the single
    /// `bulkWrite` command is issued through that same client, without any
    /// global-client dependency.
    ///
    /// # Parameters
    ///
    /// - `client`: The MongoDB client to execute through.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`BulkWrite::execute`], except that no global client is involved.
    pub async fn execute_from(
        self,
        client: &Client,
    ) -> Result<SummaryBulkWriteResult, OxiModError> {
        let collection = M::get_collection_from(client)?;
        let (models, options) = self.materialize(&collection)?;

        client
            .bulk_write(models)
            .with_options(options)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute MongoDB bulk write",
                    OperationDomain::BulkWrite,
                    error,
                )
            })
    }

    /// Executes the batch through an explicit [`ClientSession`], returning
    /// summary counts.
    ///
    /// This is the session-aware counterpart to [`BulkWrite::execute`]: the
    /// model collection is resolved from the session's own client and the
    /// single `bulkWrite` command executes with the session attached, so it
    /// participates in any transaction active on that session and is rolled
    /// back if that transaction aborts.
    ///
    /// Validation, hook, and index semantics are unchanged: queued inserts
    /// and replacements are validated before the command is sent, lifecycle
    /// hooks do not run, and declared indexes are never established —
    /// initialize indexes before starting transactional work.
    ///
    /// # Parameters
    ///
    /// - `session`: The session (and therefore transaction) to run under.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`BulkWrite::execute`], except that no global client is involved.
    pub async fn execute_with_session(
        self,
        session: &mut ClientSession,
    ) -> Result<SummaryBulkWriteResult, OxiModError> {
        let client = session.client();
        let collection = M::get_collection_from(&client)?;
        let (models, options) = self.materialize(&collection)?;

        client
            .bulk_write(models)
            .with_options(options)
            .session(&mut *session)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute MongoDB bulk write",
                    OperationDomain::BulkWrite,
                    error,
                )
            })
    }

    /// Executes the batch using the global [`OxiClient`], returning verbose
    /// per-operation results.
    ///
    /// This is the verbose counterpart to [`BulkWrite::execute`]. The
    /// returned [`VerboseBulkWriteResult`] contains the summary counts plus
    /// per-operation insert, update, and delete results keyed by each
    /// operation's **original queued index**.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`BulkWrite::execute`].
    pub async fn execute_verbose(self) -> Result<VerboseBulkWriteResult, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        self.execute_verbose_from(client).await
    }

    /// Executes the batch through an explicit client, returning verbose
    /// per-operation results.
    ///
    /// This is the verbose counterpart to [`BulkWrite::execute_from`]; see
    /// [`BulkWrite::execute_verbose`] for the result semantics.
    ///
    /// # Parameters
    ///
    /// - `client`: The MongoDB client to execute through.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`BulkWrite::execute_from`].
    pub async fn execute_verbose_from(
        self,
        client: &Client,
    ) -> Result<VerboseBulkWriteResult, OxiModError> {
        let collection = M::get_collection_from(client)?;
        let (models, options) = self.materialize(&collection)?;

        client
            .bulk_write(models)
            .with_options(options)
            .verbose_results()
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute MongoDB bulk write",
                    OperationDomain::BulkWrite,
                    error,
                )
            })
    }

    /// Executes the batch through an explicit [`ClientSession`], returning
    /// verbose per-operation results.
    ///
    /// This is the verbose counterpart to
    /// [`BulkWrite::execute_with_session`]; see
    /// [`BulkWrite::execute_verbose`] for the result semantics.
    ///
    /// # Parameters
    ///
    /// - `session`: The session (and therefore transaction) to run under.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`BulkWrite::execute_with_session`].
    pub async fn execute_verbose_with_session(
        self,
        session: &mut ClientSession,
    ) -> Result<VerboseBulkWriteResult, OxiModError> {
        let client = session.client();
        let collection = M::get_collection_from(&client)?;
        let (models, options) = self.materialize(&collection)?;

        client
            .bulk_write(models)
            .with_options(options)
            .verbose_results()
            .session(&mut *session)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute MongoDB bulk write",
                    OperationDomain::BulkWrite,
                    error,
                )
            })
    }
}

/// Converts configured array-filter documents into the driver's optional
/// array representation.
fn into_array_filters(array_filters: Vec<Document>) -> Option<Array> {
    if array_filters.is_empty() {
        return None;
    }

    Some(array_filters.into_iter().map(Bson::Document).collect())
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::{BulkWrite, PendingOperation};
    use crate::error::query_error::{BulkWriteOperation, QueryError, QueryModifier};
    use crate::query::{Field, Queryable};

    /// Simulates a model whose `dedupe_key` field is Serde-renamed to
    /// `dedupeKey`, matching the paths the derive macro generates.
    #[derive(Debug, Clone, PartialEq)]
    struct Job {
        dedupe_key: String,
    }

    struct JobFields {
        dedupe_key: Field<String>,
        status: Field<String>,
        attempts: Field<i32>,
    }

    impl Queryable for Job {
        type Fields = JobFields;

        fn fields() -> Self::Fields {
            JobFields {
                dedupe_key: Field::new("dedupeKey"),
                status: Field::new("status"),
                attempts: Field::new("attempts"),
            }
        }
    }

    fn job(dedupe_key: &str) -> Job {
        Job {
            dedupe_key: dedupe_key.to_string(),
        }
    }

    fn unsupported(operation: BulkWriteOperation, modifier: QueryModifier) -> Option<QueryError> {
        Some(QueryError::UnsupportedBulkWriteModifier {
            operation,
            modifier,
        })
    }

    #[test]
    fn new_batch_is_empty_and_error_free() {
        let batch = BulkWrite::<Job>::new();

        assert!(batch.operations.is_empty());
        assert!(batch.options.is_none());
        assert!(batch.error.is_none());
    }

    #[test]
    fn insert_preserves_call_order() {
        let batch = BulkWrite::<Job>::new()
            .insert(job("job-1"))
            .insert(job("job-2"))
            .insert(job("job-3"));

        let keys: Vec<&str> = batch
            .operations
            .iter()
            .map(|operation| match operation {
                PendingOperation::Insert { model } => model.dedupe_key.as_str(),
                other => panic!("expected only inserts, got: {other:?}"),
            })
            .collect();

        assert_eq!(keys, ["job-1", "job-2", "job-3"]);
    }

    #[test]
    fn insert_many_preserves_iteration_order() {
        let batch = BulkWrite::<Job>::new().insert_many([job("job-1"), job("job-2"), job("job-3")]);

        let keys: Vec<&str> = batch
            .operations
            .iter()
            .map(|operation| match operation {
                PendingOperation::Insert { model } => model.dedupe_key.as_str(),
                other => panic!("expected only inserts, got: {other:?}"),
            })
            .collect();

        assert_eq!(keys, ["job-1", "job-2", "job-3"]);
    }

    #[test]
    fn update_one_uses_serde_renamed_filter_and_update_paths() {
        let batch = BulkWrite::<Job>::new().update_one(
            Job::query().filter(|job| job.dedupe_key.eq("job-3")),
            |job| job.dedupe_key.set("job-3-updated"),
        );

        assert!(batch.error.is_none());
        assert_eq!(batch.operations.len(), 1);

        let PendingOperation::UpdateOne {
            filter,
            update,
            array_filters,
            sort,
        } = &batch.operations[0]
        else {
            panic!("expected an update-one operation");
        };

        assert_eq!(filter, &doc! { "dedupeKey": "job-3" });
        assert_eq!(update, &doc! { "$set": { "dedupeKey": "job-3-updated" } });
        assert!(array_filters.is_empty());
        assert!(sort.is_none());
    }

    #[test]
    fn update_one_carries_sort_and_array_filters() {
        let batch = BulkWrite::<Job>::new().update_one(
            Job::query()
                .filter(|job| job.status.eq("queued"))
                .sort_by(|job| job.attempts.asc())
                .array_filter(|job| job.attempts.gte(3)),
            |job| job.status.set("ready"),
        );

        assert!(batch.error.is_none());

        let PendingOperation::UpdateOne {
            array_filters,
            sort,
            ..
        } = &batch.operations[0]
        else {
            panic!("expected an update-one operation");
        };

        assert_eq!(array_filters, &vec![doc! { "attempts": { "$gte": 3 } }]);
        assert_eq!(sort, &Some(doc! { "attempts": 1 }));
    }

    #[test]
    fn update_many_extracts_filter_update_and_array_filters() {
        let batch = BulkWrite::<Job>::new().update_many(
            Job::query()
                .filter(|job| job.status.eq("stale"))
                .array_filter(|job| job.attempts.lt(2)),
            |job| job.status.set("expired") & job.attempts.inc(1),
        );

        assert!(batch.error.is_none());

        let PendingOperation::UpdateMany {
            filter,
            update,
            array_filters,
        } = &batch.operations[0]
        else {
            panic!("expected an update-many operation");
        };

        assert_eq!(filter, &doc! { "status": "stale" });
        assert_eq!(
            update,
            &doc! {
                "$set": { "status": "expired" },
                "$inc": { "attempts": 1 },
            }
        );
        assert_eq!(array_filters, &vec![doc! { "attempts": { "$lt": 2 } }]);
    }

    #[test]
    fn replace_one_stores_filter_replacement_and_sort() {
        let batch = BulkWrite::<Job>::new().replace_one(
            Job::query()
                .filter(|job| job.dedupe_key.eq("job-3"))
                .sort_by(|job| job.attempts.desc()),
            job("job-3-replacement"),
        );

        assert!(batch.error.is_none());

        let PendingOperation::ReplaceOne {
            filter,
            replacement,
            sort,
        } = &batch.operations[0]
        else {
            panic!("expected a replace-one operation");
        };

        assert_eq!(filter, &doc! { "dedupeKey": "job-3" });
        assert_eq!(replacement, &job("job-3-replacement"));
        assert_eq!(sort, &Some(doc! { "attempts": -1 }));
    }

    #[test]
    fn delete_operations_extract_typed_filters() {
        let batch = BulkWrite::<Job>::new()
            .delete_one(Job::query().filter(|job| job.dedupe_key.eq("job-3")))
            .delete_many(Job::query().filter(|job| job.status.eq("expired")));

        assert!(batch.error.is_none());

        let PendingOperation::DeleteOne { filter } = &batch.operations[0] else {
            panic!("expected a delete-one operation");
        };
        assert_eq!(filter, &doc! { "dedupeKey": "job-3" });

        let PendingOperation::DeleteMany { filter } = &batch.operations[1] else {
            panic!("expected a delete-many operation");
        };
        assert_eq!(filter, &doc! { "status": "expired" });
    }

    #[test]
    fn mixed_operation_kinds_preserve_call_order() {
        let batch = BulkWrite::<Job>::new()
            .insert(job("job-1"))
            .update_one(Job::query(), |job| job.status.set("ready"))
            .update_many(Job::query(), |job| job.attempts.inc(1))
            .replace_one(Job::query(), job("job-2"))
            .delete_one(Job::query())
            .delete_many(Job::query());

        assert!(batch.error.is_none());

        let kinds: Vec<&str> = batch
            .operations
            .iter()
            .map(|operation| match operation {
                PendingOperation::Insert { .. } => "insert",
                PendingOperation::UpdateOne { .. } => "update_one",
                PendingOperation::UpdateMany { .. } => "update_many",
                PendingOperation::ReplaceOne { .. } => "replace_one",
                PendingOperation::DeleteOne { .. } => "delete_one",
                PendingOperation::DeleteMany { .. } => "delete_many",
            })
            .collect();

        assert_eq!(
            kinds,
            [
                "insert",
                "update_one",
                "update_many",
                "replace_one",
                "delete_one",
                "delete_many",
            ],
        );
    }

    #[test]
    fn text_search_is_merged_into_bulk_filters() {
        let batch = BulkWrite::<Job>::new().delete_many(Job::query().text("expired import"));

        let PendingOperation::DeleteMany { filter } = &batch.operations[0] else {
            panic!("expected a delete-many operation");
        };

        assert_eq!(filter, &doc! { "$text": { "$search": "expired import" } },);
    }

    #[test]
    fn unsupported_modifiers_fail_locally_and_queue_nothing() {
        let cases: Vec<(BulkWrite<Job>, Option<QueryError>)> = vec![
            (
                BulkWrite::<Job>::new()
                    .update_one(Job::query().skip(1), |job| job.status.set("ready")),
                unsupported(BulkWriteOperation::UpdateOne, QueryModifier::Skip),
            ),
            (
                BulkWrite::<Job>::new()
                    .update_one(Job::query().limit(5), |job| job.status.set("ready")),
                unsupported(BulkWriteOperation::UpdateOne, QueryModifier::Limit),
            ),
            (
                BulkWrite::<Job>::new()
                    .update_one(Job::query().page(2, 10), |job| job.status.set("ready")),
                unsupported(BulkWriteOperation::UpdateOne, QueryModifier::Pagination),
            ),
            (
                BulkWrite::<Job>::new()
                    .update_many(Job::query().sort_by(|job| job.attempts.asc()), |job| {
                        job.status.set("ready")
                    }),
                unsupported(BulkWriteOperation::UpdateMany, QueryModifier::Sort),
            ),
            (
                BulkWrite::<Job>::new().replace_one(
                    Job::query().array_filter(|job| job.attempts.gte(3)),
                    job("job-1"),
                ),
                unsupported(BulkWriteOperation::ReplaceOne, QueryModifier::ArrayFilters),
            ),
            (
                BulkWrite::<Job>::new().delete_one(Job::query().sort_by(|job| job.attempts.asc())),
                unsupported(BulkWriteOperation::DeleteOne, QueryModifier::Sort),
            ),
            (
                BulkWrite::<Job>::new().delete_one(Job::query().skip(1)),
                unsupported(BulkWriteOperation::DeleteOne, QueryModifier::Skip),
            ),
            (
                BulkWrite::<Job>::new().delete_many(Job::query().sort_by(|job| job.attempts.asc())),
                unsupported(BulkWriteOperation::DeleteMany, QueryModifier::Sort),
            ),
            (
                BulkWrite::<Job>::new().delete_many(Job::query().limit(5)),
                unsupported(BulkWriteOperation::DeleteMany, QueryModifier::Limit),
            ),
            (
                BulkWrite::<Job>::new().delete_many(Job::query().page(1, 10)),
                unsupported(BulkWriteOperation::DeleteMany, QueryModifier::Pagination),
            ),
        ];

        for (batch, expected) in cases {
            assert_eq!(batch.error, expected);
            assert!(
                batch.operations.is_empty(),
                "a rejected operation must not be queued",
            );
        }
    }

    #[test]
    fn deferred_query_construction_error_is_adopted() {
        let batch = BulkWrite::<Job>::new().delete_many(Job::query().page(0, 10));

        assert_eq!(batch.error, Some(QueryError::InvalidPageNumber { page: 0 }),);
        assert!(batch.operations.is_empty());
    }

    #[test]
    fn first_recorded_error_is_preserved() {
        let batch = BulkWrite::<Job>::new()
            .delete_one(Job::query().sort_by(|job| job.attempts.asc()))
            .update_many(Job::query().sort_by(|job| job.attempts.asc()), |job| {
                job.status.set("ready")
            });

        assert_eq!(
            batch.error,
            unsupported(BulkWriteOperation::DeleteOne, QueryModifier::Sort),
        );
    }

    #[test]
    fn operations_after_an_error_are_still_queued_when_valid() {
        let batch = BulkWrite::<Job>::new()
            .delete_one(Job::query().sort_by(|job| job.attempts.asc()))
            .insert(job("job-1"));

        assert_eq!(batch.operations.len(), 1);
        assert!(matches!(
            batch.operations[0],
            PendingOperation::Insert { .. }
        ));
    }

    #[test]
    fn ordered_configures_the_options_state() {
        let batch = BulkWrite::<Job>::new().ordered(false);

        let options = batch.options.expect("ordered should create options");
        assert_eq!(options.ordered, Some(false));
    }

    #[test]
    fn ordered_preserves_other_configured_options() {
        let mut options = mongodb::options::BulkWriteOptions::default();
        options.bypass_document_validation = Some(true);

        let batch = BulkWrite::<Job>::new().with_options(options).ordered(false);

        let options = batch.options.expect("options should be configured");
        assert_eq!(options.ordered, Some(false));
        assert_eq!(options.bypass_document_validation, Some(true));
    }

    #[test]
    fn later_with_options_replaces_the_whole_options_state() {
        let mut replacement = mongodb::options::BulkWriteOptions::default();
        replacement.bypass_document_validation = Some(true);

        let batch = BulkWrite::<Job>::new()
            .ordered(false)
            .with_options(replacement);

        let options = batch.options.expect("options should be configured");
        assert_eq!(options.ordered, None);
        assert_eq!(options.bypass_document_validation, Some(true));
    }

    #[test]
    fn with_options_none_clears_the_options_state() {
        let batch = BulkWrite::<Job>::new().ordered(false).with_options(None);

        assert!(batch.options.is_none());
    }
}
