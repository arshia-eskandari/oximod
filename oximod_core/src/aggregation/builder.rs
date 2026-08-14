//! Aggregation pipeline construction and execution.
//!
//! This module contains [`Aggregation`], the builder produced by
//! [`Queryable::aggregate`]. It stores an ordered MongoDB aggregation
//! pipeline until an execution method runs it against the model's collection
//! and deserializes the output.
//!
//! Unlike [`Query`](crate::query::Query), whose results are always the model
//! type, an aggregation pipeline may reshape its stream: stages such as
//! `$group` and `$project` can produce documents that no longer look like the
//! source model. The builder therefore carries two type parameters: the
//! source model `M`, which provides typed fields for stage construction, and
//! the output type `R`, which controls how final pipeline output is
//! deserialized. The default `R = M` keeps shape-preserving pipelines
//! ergonomic; [`Aggregation::with_type`] selects a different output type.

use std::marker::PhantomData;

use mongodb::{
    Client, ClientSession,
    bson::{Bson, Document, doc},
    options::AggregateOptions,
};

use crate::{
    error::{
        classify::{OperationDomain, classify_driver_error},
        oximod_error::OxiModError,
    },
    feature::model::Model,
    query::{Expression, Queryable, SortExpression, TextSearch},
};

/// A type-safe MongoDB aggregation pipeline for source model `M` producing
/// output values of type `R`.
///
/// Aggregations are normally created through [`Queryable::aggregate`] rather
/// than constructed directly.
///
/// The builder stores an ordered pipeline of MongoDB stages until an
/// execution method such as [`Aggregation::all`] or [`Aggregation::first`]
/// is called. Every stage method appends exactly at its call position;
/// OxiMod never reorders, merges, or rewrites the pipeline, because MongoDB
/// evaluates aggregation stages strictly in order.
///
/// # Typed stages and raw stages
///
/// Typed stage methods ([`Aggregation::match_`], [`Aggregation::sort_by`],
/// [`Aggregation::skip`], [`Aggregation::limit`], [`Aggregation::text`])
/// reuse OxiMod's generated fields, so source field paths — including Serde
/// renames — stay compiler-linked. The rest of MongoDB's aggregation
/// language is reached through [`Aggregation::raw_stage`],
/// [`Aggregation::raw_stage_with`], and [`Aggregation::raw_pipeline`], whose
/// BSON content OxiMod passes to MongoDB unchanged and does not type-check.
///
/// Typed source-field stages always refer to `M`'s serialized source paths.
/// After a shape-changing raw stage such as `$group` or `$project`, use
/// typed source-field stages only if those source fields are still present
/// with the same meaning; otherwise continue with raw stages and select the
/// final output type explicitly with [`Aggregation::with_type`].
///
/// # Example
///
/// ```ignore
/// #[derive(Debug, Deserialize)]
/// struct RoleSummary {
///     #[serde(rename = "_id")]
///     role: String,
///     count: i64,
/// }
///
/// let summaries = User::aggregate()
///     .match_(|user| user.active.eq(true))
///     .raw_stage_with(|user| {
///         doc! {
///             "$group": {
///                 "_id": format!("${}", user.role.name()),
///                 "count": { "$sum": 1 },
///             },
///         }
///     })
///     .with_type::<RoleSummary>()
///     .all()
///     .await?;
/// ```
///
/// # Server semantics
///
/// MongoDB itself validates stage placement and advanced pipeline rules —
/// for example `$geoNear` must be the first stage, `$match` with `$text` has
/// position restrictions, and the write stages `$out` and `$merge` are
/// restricted inside transactions. OxiMod does not duplicate that
/// validation; a server rejection surfaces as
/// [`OxiModError::Aggregation`].
#[must_use = "aggregations do nothing unless an execution method is called"]
#[derive(Debug, Clone)]
pub struct Aggregation<M, R = M> {
    pipeline: Vec<Document>,
    options: Option<AggregateOptions>,
    marker: PhantomData<fn() -> (M, R)>,
}

impl<M> Aggregation<M> {
    /// Creates an empty aggregation.
    ///
    /// This constructor is used by [`Queryable::aggregate`]. Applications
    /// should normally call `User::aggregate()` on a derived model instead.
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self {
            pipeline: Vec::new(),
            options: None,
            marker: PhantomData,
        }
    }
}

impl<M> Default for Aggregation<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M, R> Aggregation<M, R> {
    /// Returns the pipeline stages built so far, in execution order.
    ///
    /// The returned stages are exactly what execution sends to MongoDB. This
    /// accessor is useful for inspecting or logging a pipeline before running
    /// it.
    #[must_use]
    pub fn pipeline(&self) -> &[Document] {
        &self.pipeline
    }

    /// Appends a raw MongoDB aggregation stage.
    ///
    /// The document is passed to MongoDB unchanged, in the pipeline position
    /// where this method is called. This is the escape hatch for the parts of
    /// MongoDB's aggregation language that OxiMod does not type: `$group`,
    /// `$project`, `$lookup`, `$geoNear`, `$unwind`, `$facet`, and so on.
    ///
    /// ```ignore
    /// let aggregation = User::aggregate()
    ///     .raw_stage(doc! {
    ///         "$group": {
    ///             "_id": "$role",
    ///             "count": { "$sum": 1 },
    ///         },
    ///     });
    /// ```
    ///
    /// OxiMod does not verify raw stage names, operators, or field paths; use
    /// [`Aggregation::raw_stage_with`] to keep source field names
    /// compiler-linked. Raw stages remain subject to MongoDB's server rules,
    /// including stage-position requirements and the transaction restrictions
    /// on the write stages `$out` and `$merge`. A server rejection surfaces
    /// as [`OxiModError::Aggregation`].
    ///
    /// After a stage that changes the document shape, the source model's
    /// fields may no longer exist in the stream. Select the matching output
    /// type with [`Aggregation::with_type`].
    ///
    /// # Parameters
    ///
    /// - `stage`: One complete aggregation stage document, such as
    ///   `doc! { "$group": { ... } }`.
    pub fn raw_stage(mut self, stage: Document) -> Self {
        self.pipeline.push(stage);
        self
    }

    /// Appends raw MongoDB aggregation stages in order.
    ///
    /// Every document is passed to MongoDB unchanged, preserving the
    /// iterator's order at the pipeline position where this method is called.
    ///
    /// ```ignore
    /// let aggregation = User::aggregate()
    ///     .raw_pipeline([
    ///         doc! { "$unwind": "$tags" },
    ///         doc! {
    ///             "$group": {
    ///                 "_id": "$tags",
    ///                 "count": { "$sum": 1 },
    ///             },
    ///         },
    ///     ]);
    /// ```
    ///
    /// The same guarantees and boundaries apply as for
    /// [`Aggregation::raw_stage`].
    ///
    /// # Parameters
    ///
    /// - `stages`: The aggregation stage documents to append.
    pub fn raw_pipeline<I>(mut self, stages: I) -> Self
    where
        I: IntoIterator<Item = Document>,
    {
        self.pipeline.extend(stages);
        self
    }

    /// Selects the type that pipeline output deserializes into.
    ///
    /// This method changes only deserialization; it does not add a MongoDB
    /// stage or alter the pipeline. Use it when the pipeline's final document
    /// shape is no longer the source model — typically after `$group`,
    /// `$project`, or a stage that computes new fields:
    ///
    /// ```ignore
    /// #[derive(Debug, Deserialize)]
    /// struct RoleSummary {
    ///     #[serde(rename = "_id")]
    ///     role: String,
    ///     count: i64,
    /// }
    ///
    /// let summaries = User::aggregate()
    ///     .raw_stage(doc! {
    ///         "$group": {
    ///             "_id": "$role",
    ///             "count": { "$sum": 1 },
    ///         },
    ///     })
    ///     .with_type::<RoleSummary>()
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// OxiMod cannot verify at compile time that `R2` matches what the
    /// pipeline actually produces — that synchronization is the caller's
    /// responsibility. If an output document does not deserialize as `R2`,
    /// execution fails with [`OxiModError::Serialization`] rather than
    /// silently dropping or defaulting values.
    ///
    /// Typed source-field stage methods remain available after
    /// `with_type()`; they still refer to `M`'s serialized field paths and
    /// are only meaningful while those fields still exist in the pipeline
    /// stream.
    pub fn with_type<R2>(self) -> Aggregation<M, R2> {
        Aggregation {
            pipeline: self.pipeline,
            options: self.options,
            marker: PhantomData,
        }
    }

    /// Sets the driver options used when the aggregation executes.
    ///
    /// The options are passed to the MongoDB driver unchanged and apply
    /// identically to global, explicit-client, and session execution. This is
    /// the pass-through for driver behavior such as `allow_disk_use`, batch
    /// size, max time, collation, comment, hint, and `let` variables:
    ///
    /// ```ignore
    /// use mongodb::options::AggregateOptions;
    ///
    /// let options = AggregateOptions::builder()
    ///     .allow_disk_use(true)
    ///     .build();
    ///
    /// let users = User::aggregate()
    ///     .match_(|user| user.active.eq(true))
    ///     .with_options(options)
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Calling this method again replaces the previously configured options.
    /// Options never change the pipeline stages themselves.
    ///
    /// # Parameters
    ///
    /// - `options`: The driver [`AggregateOptions`] to apply, or `None` to
    ///   clear previously configured options.
    pub fn with_options(mut self, options: impl Into<Option<AggregateOptions>>) -> Self {
        self.options = options.into();
        self
    }
}

impl<M, R> Aggregation<M, R>
where
    M: Queryable,
{
    /// Appends a typed `$match` stage.
    ///
    /// The closure receives the field structure generated for `M` and builds
    /// the same type-safe [`Expression`] used by
    /// [`Query::filter`](crate::query::Query::filter), so serialized field
    /// paths — including Serde renames — stay compiler-linked:
    ///
    /// ```ignore
    /// let adults = User::aggregate()
    ///     .match_(|user| {
    ///         user.active.eq(true)
    ///             & user.age.gte(18)
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Each call appends exactly one `$match` stage at its call position.
    /// Repeated calls append repeated stages in call order; they are never
    /// merged, because stages between them may change what the later match
    /// sees.
    ///
    /// MongoDB forbids some query operators inside an aggregation `$match`,
    /// notably `$near` and `$nearSphere`; geospatial distance pipelines use
    /// the `$geoNear` stage through [`Aggregation::raw_stage`] as the first
    /// pipeline stage instead. OxiMod does not duplicate MongoDB's operator
    /// validation; a server rejection surfaces as
    /// [`OxiModError::Aggregation`].
    ///
    /// # Parameters
    ///
    /// - `build`: A closure that creates an [`Expression`] from the generated
    ///   fields for `M`.
    pub fn match_<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> Expression,
    {
        let fields = M::fields();
        let expression = build(&fields);

        self.pipeline.push(doc! {
            "$match": expression.into_document(),
        });

        self
    }

    /// Appends a `$match` stage containing a MongoDB `$text` search.
    ///
    /// The model's collection must have a text index covering the searchable
    /// fields. A string creates a basic search; use [`TextSearch`] to
    /// configure language, case sensitivity, or diacritic sensitivity:
    ///
    /// ```ignore
    /// let articles = Article::aggregate()
    ///     .text("rust mongodb")
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// MongoDB requires a `$match` stage using `$text` to be the first stage
    /// of the pipeline. OxiMod appends the stage exactly where this method is
    /// called and never repositions it; calling `text()` anywhere but first
    /// is rejected by the server as [`OxiModError::Aggregation`].
    ///
    /// # Parameters
    ///
    /// - `search`: A search string or configured [`TextSearch`].
    pub fn text<S>(mut self, search: S) -> Self
    where
        S: Into<TextSearch>,
    {
        let search = search.into();

        self.pipeline.push(doc! {
            "$match": search.into_document(),
        });

        self
    }

    /// Appends a typed `$sort` stage.
    ///
    /// The closure receives the field structure generated for `M` and builds
    /// a [`SortExpression`] with the same field methods used by
    /// [`Query::sort_by`](crate::query::Query::sort_by). One stage can carry
    /// multiple sort keys by chaining [`SortExpression::then`]:
    ///
    /// ```ignore
    /// let users = User::aggregate()
    ///     .match_(|user| user.active.eq(true))
    ///     .sort_by(|user| user.role.asc().then(user.age.desc()))
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Each call appends one `$sort` stage at its call position. Unlike
    /// [`Query::sort_by`](crate::query::Query::sort_by), a later call does
    /// not replace an earlier one — it appends another stage, because in a
    /// pipeline the position of each sort is semantically meaningful.
    ///
    /// The sort keys refer to `M`'s serialized source paths. To sort by a
    /// pipeline-computed field, use [`Aggregation::raw_stage`].
    ///
    /// # Parameters
    ///
    /// - `build`: A closure that creates a [`SortExpression`] from the
    ///   generated fields for `M`.
    pub fn sort_by<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> SortExpression,
    {
        let fields = M::fields();
        let sort = build(&fields);

        self.pipeline.push(doc! {
            "$sort": sort.into_document(),
        });

        self
    }

    /// Appends a `$skip` stage.
    ///
    /// The stage skips the given number of documents at its position in the
    /// pipeline. Unlike [`Query::skip`](crate::query::Query::skip), this is
    /// an ordered stage, not a query-wide modifier: a `$skip` before a
    /// `$group` behaves differently from a `$skip` after it, and repeated
    /// calls append repeated stages.
    ///
    /// ```ignore
    /// let page = User::aggregate()
    ///     .sort_by(|user| user.name.asc())
    ///     .skip(20)
    ///     .limit(10)
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `skip`: The number of documents to skip. The value converts exactly
    ///   into the 64-bit integer MongoDB expects; no saturation or truncation
    ///   occurs.
    pub fn skip(mut self, skip: u32) -> Self {
        self.pipeline.push(doc! {
            "$skip": Bson::Int64(i64::from(skip)),
        });

        self
    }

    /// Appends a `$limit` stage.
    ///
    /// The stage limits the stream to the given number of documents at its
    /// position in the pipeline. Unlike
    /// [`Query::limit`](crate::query::Query::limit), this is an ordered
    /// stage, not a query-wide modifier, and repeated calls append repeated
    /// stages.
    ///
    /// ```ignore
    /// let top_three = User::aggregate()
    ///     .sort_by(|user| user.age.desc())
    ///     .limit(3)
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `limit`: The maximum number of documents to pass through. The value
    ///   converts exactly into the 64-bit integer MongoDB expects; no
    ///   saturation or truncation occurs.
    pub fn limit(mut self, limit: u32) -> Self {
        self.pipeline.push(doc! {
            "$limit": Bson::Int64(i64::from(limit)),
        });

        self
    }

    /// Appends a raw MongoDB aggregation stage built from `M`'s generated
    /// fields.
    ///
    /// This is [`Aggregation::raw_stage`] with access to the typed field
    /// structure, so the source field names used inside the raw BSON stay
    /// compiler-linked — a renamed or removed model field becomes a compile
    /// error instead of a silently broken pipeline:
    ///
    /// ```ignore
    /// let aggregation = User::aggregate()
    ///     .raw_stage_with(|user| {
    ///         doc! {
    ///             "$group": {
    ///                 "_id": format!("${}", user.role.name()),
    ///                 "average_age": {
    ///                     "$avg": format!("${}", user.age.name()),
    ///                 },
    ///             },
    ///         }
    ///     });
    /// ```
    ///
    /// [`Field::name`](crate::query::Field::name) returns the serialized
    /// MongoDB field path, honoring Serde renames; prefix it with `$` to
    /// form an aggregation field reference.
    ///
    /// Only the field paths taken from the closure's argument are
    /// compiler-linked. MongoDB operator names, stage structure, and any
    /// hand-written paths inside the returned document are passed to the
    /// server unchanged and unchecked, exactly as with
    /// [`Aggregation::raw_stage`].
    ///
    /// # Parameters
    ///
    /// - `build`: A closure that creates one complete aggregation stage
    ///   document from the generated fields for `M`.
    pub fn raw_stage_with<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> Document,
    {
        let fields = M::fields();
        let stage = build(&fields);

        self.pipeline.push(stage);

        self
    }
}

impl<M, R> Aggregation<M, R>
where
    M: Queryable + Model,
    R: serde::de::DeserializeOwned,
{
    /// Runs the pipeline and returns every output value.
    ///
    /// The pipeline executes exactly as built against the model's collection
    /// resolved from the globally initialized client, and every result is
    /// deserialized as `R`.
    ///
    /// ```ignore
    /// let adults: Vec<User> = User::aggregate()
    ///     .match_(|user| user.age.gte(18))
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Returns
    ///
    /// A vector containing every document produced by the pipeline, in the
    /// order MongoDB returned them.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the model collection cannot be resolved
    /// - MongoDB rejects the pipeline or fails during execution
    ///   ([`OxiModError::Aggregation`], with the driver error available
    ///   through `source()`)
    /// - connectivity fails ([`OxiModError::Connection`])
    /// - an output document cannot be deserialized as `R`
    ///   ([`OxiModError::Serialization`])
    ///
    /// Deserialization failures are never skipped: if any output document
    /// cannot be deserialized as `R`, `all()` returns `Err` rather than
    /// silently dropping it.
    pub async fn all(self) -> Result<Vec<R>, OxiModError> {
        let collection = M::get_collection()?;

        let Self {
            pipeline, options, ..
        } = self;

        let cursor = collection
            .aggregate(pipeline)
            .with_type::<R>()
            .with_options(options)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute aggregation pipeline",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

        Self::collect_all(cursor).await
    }

    /// Runs the pipeline and returns at most the first output value.
    ///
    /// The pipeline executes exactly as built — OxiMod does not append a
    /// server-side `$limit: 1`, because rewriting the pipeline could change
    /// the behavior of raw stages. The cursor is simply read at most once.
    /// Callers who want the server to stop producing results early should
    /// append an explicit [`Aggregation::limit`] stage:
    ///
    /// ```ignore
    /// let oldest = User::aggregate()
    ///     .sort_by(|user| user.age.desc())
    ///     .limit(1)
    ///     .first()
    ///     .await?;
    /// ```
    ///
    /// # Returns
    ///
    /// - `Ok(Some(R))` containing the first output value.
    /// - `Ok(None)` when the pipeline produces no output.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`Aggregation::all`]; the first output document failing to
    /// deserialize as `R` is an error, not `None`.
    pub async fn first(self) -> Result<Option<R>, OxiModError> {
        let collection = M::get_collection()?;

        let Self {
            pipeline, options, ..
        } = self;

        let cursor = collection
            .aggregate(pipeline)
            .with_type::<R>()
            .with_options(options)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute aggregation pipeline",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

        Self::take_first(cursor).await
    }

    /// Runs the pipeline through an explicit [`Client`] and returns every
    /// output value.
    ///
    /// This is the explicit-client counterpart to [`Aggregation::all`]: the
    /// model's collection is resolved from `client` instead of the global
    /// client, so aggregation does not require global initialization.
    ///
    /// ```ignore
    /// let adults = User::aggregate()
    ///     .match_(|user| user.age.gte(18))
    ///     .all_from(&client)
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `client`: The MongoDB client to resolve the model collection from.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`Aggregation::all`].
    pub async fn all_from(self, client: &Client) -> Result<Vec<R>, OxiModError> {
        let collection = M::get_collection_from(client)?;

        let Self {
            pipeline, options, ..
        } = self;

        let cursor = collection
            .aggregate(pipeline)
            .with_type::<R>()
            .with_options(options)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute aggregation pipeline",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

        Self::collect_all(cursor).await
    }

    /// Runs the pipeline through an explicit [`Client`] and returns at most
    /// the first output value.
    ///
    /// This is the explicit-client counterpart to [`Aggregation::first`],
    /// with the same semantics: the pipeline executes exactly as built and
    /// the cursor is read at most once, without appending a server-side
    /// `$limit`.
    ///
    /// # Parameters
    ///
    /// - `client`: The MongoDB client to resolve the model collection from.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`Aggregation::first`].
    pub async fn first_from(self, client: &Client) -> Result<Option<R>, OxiModError> {
        let collection = M::get_collection_from(client)?;

        let Self {
            pipeline, options, ..
        } = self;

        let cursor = collection
            .aggregate(pipeline)
            .with_type::<R>()
            .with_options(options)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute aggregation pipeline",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

        Self::take_first(cursor).await
    }

    /// Runs the pipeline through an explicit [`ClientSession`] and returns
    /// every output value.
    ///
    /// This is the session-aware counterpart to [`Aggregation::all`]: the
    /// model collection is resolved from the session's own client, the
    /// aggregation runs on the session, and the session cursor is advanced
    /// with the same session, so inside a transaction the pipeline reads the
    /// transaction's own uncommitted writes.
    ///
    /// ```ignore
    /// let totals = Order::aggregate()
    ///     .match_(|order| order.status.eq("paid"))
    ///     .all_with_session(&mut session)
    ///     .await?;
    /// ```
    ///
    /// Session participation is always explicit: aggregations executed
    /// without a session never join an open transaction. MongoDB restricts
    /// the write stages `$out` and `$merge` inside transactions; a raw stage
    /// violating those rules is rejected by the server as
    /// [`OxiModError::Aggregation`].
    ///
    /// # Parameters
    ///
    /// - `session`: The session (and therefore transaction) to run under.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`Aggregation::all`].
    pub async fn all_with_session(
        self,
        session: &mut ClientSession,
    ) -> Result<Vec<R>, OxiModError> {
        let client = session.client();
        let collection = M::get_collection_from(&client)?;

        let Self {
            pipeline, options, ..
        } = self;

        let mut cursor = collection
            .aggregate(pipeline)
            .with_type::<R>()
            .with_options(options)
            .session(&mut *session)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute aggregation pipeline",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

        let mut results = Vec::new();

        while cursor.advance(&mut *session).await.map_err(|error| {
            classify_driver_error(
                "Failed to advance aggregation cursor",
                OperationDomain::Aggregation,
                error,
            )
        })? {
            let value = cursor.deserialize_current().map_err(|error| {
                classify_driver_error(
                    "Failed to deserialize aggregation result",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

            results.push(value);
        }

        Ok(results)
    }

    /// Runs the pipeline through an explicit [`ClientSession`] and returns
    /// at most the first output value.
    ///
    /// This is the session-aware counterpart to [`Aggregation::first`], with
    /// the same semantics: the pipeline executes exactly as built and the
    /// session cursor is read at most once, without appending a server-side
    /// `$limit`.
    ///
    /// # Parameters
    ///
    /// - `session`: The session (and therefore transaction) to run under.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] under the same conditions as
    /// [`Aggregation::first`].
    pub async fn first_with_session(
        self,
        session: &mut ClientSession,
    ) -> Result<Option<R>, OxiModError> {
        let client = session.client();
        let collection = M::get_collection_from(&client)?;

        let Self {
            pipeline, options, ..
        } = self;

        let mut cursor = collection
            .aggregate(pipeline)
            .with_type::<R>()
            .with_options(options)
            .session(&mut *session)
            .await
            .map_err(|error| {
                classify_driver_error(
                    "Failed to execute aggregation pipeline",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

        if !cursor.advance(&mut *session).await.map_err(|error| {
            classify_driver_error(
                "Failed to advance aggregation cursor",
                OperationDomain::Aggregation,
                error,
            )
        })? {
            return Ok(None);
        }

        let value = cursor.deserialize_current().map_err(|error| {
            classify_driver_error(
                "Failed to deserialize aggregation result",
                OperationDomain::Aggregation,
                error,
            )
        })?;

        Ok(Some(value))
    }

    /// Drains an ordinary cursor into a vector of `R`.
    async fn collect_all(mut cursor: mongodb::Cursor<R>) -> Result<Vec<R>, OxiModError> {
        let mut results = Vec::new();

        while cursor.advance().await.map_err(|error| {
            classify_driver_error(
                "Failed to advance aggregation cursor",
                OperationDomain::Aggregation,
                error,
            )
        })? {
            let value = cursor.deserialize_current().map_err(|error| {
                classify_driver_error(
                    "Failed to deserialize aggregation result",
                    OperationDomain::Aggregation,
                    error,
                )
            })?;

            results.push(value);
        }

        Ok(results)
    }

    /// Reads at most one value from an ordinary cursor.
    async fn take_first(mut cursor: mongodb::Cursor<R>) -> Result<Option<R>, OxiModError> {
        if !cursor.advance().await.map_err(|error| {
            classify_driver_error(
                "Failed to advance aggregation cursor",
                OperationDomain::Aggregation,
                error,
            )
        })? {
            return Ok(None);
        }

        let value = cursor.deserialize_current().map_err(|error| {
            classify_driver_error(
                "Failed to deserialize aggregation result",
                OperationDomain::Aggregation,
                error,
            )
        })?;

        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};
    use mongodb::options::AggregateOptions;

    use super::Aggregation;
    use crate::query::{Field, Queryable};

    struct User;

    struct UserFields {
        active: Field<bool>,
        age: Field<i32>,
        role: Field<String>,
        /// Simulates `#[serde(rename = "userName")]` on a `name` field.
        name: Field<String>,
        /// Simulates a nested embedded-model path.
        city: Field<String>,
    }

    impl Queryable for User {
        type Fields = UserFields;

        fn fields() -> Self::Fields {
            UserFields {
                active: Field::new("active"),
                age: Field::new("age"),
                role: Field::new("role"),
                name: Field::new("userName"),
                city: Field::new("address.city"),
            }
        }
    }

    #[test]
    fn empty_aggregation_has_no_stages() {
        let aggregation = User::aggregate();

        assert!(aggregation.pipeline().is_empty());
    }

    #[test]
    fn match_appends_a_match_stage() {
        let aggregation = User::aggregate().match_(|user| user.active.eq(true) & user.age.gte(18));

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$match": {
                    "$and": [
                        {
                            "active": true,
                        },
                        {
                            "age": {
                                "$gte": 18,
                            },
                        },
                    ],
                },
            }],
        );
    }

    #[test]
    fn repeated_match_calls_append_stages_in_call_order() {
        let aggregation = User::aggregate()
            .match_(|user| user.active.eq(true))
            .match_(|user| user.age.gte(18));

        assert_eq!(
            aggregation.pipeline(),
            &[
                doc! {
                    "$match": {
                        "active": true,
                    },
                },
                doc! {
                    "$match": {
                        "age": {
                            "$gte": 18,
                        },
                    },
                },
            ],
        );
    }

    #[test]
    fn match_uses_serialized_field_names() {
        let aggregation = User::aggregate().match_(|user| user.name.eq("User1"));

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$match": {
                    "userName": "User1",
                },
            }],
        );
    }

    #[test]
    fn match_supports_nested_source_paths() {
        let aggregation = User::aggregate().match_(|user| user.city.eq("Toronto"));

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$match": {
                    "address.city": "Toronto",
                },
            }],
        );
    }

    #[test]
    fn sort_by_appends_a_sort_stage() {
        let aggregation = User::aggregate().sort_by(|user| user.age.desc());

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$sort": {
                    "age": -1,
                },
            }],
        );
    }

    #[test]
    fn sort_by_carries_multiple_keys_in_one_stage() {
        let aggregation = User::aggregate().sort_by(|user| user.role.asc().then(user.age.desc()));

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$sort": {
                    "role": 1,
                    "age": -1,
                },
            }],
        );
    }

    #[test]
    fn repeated_sort_by_calls_append_separate_stages() {
        let aggregation = User::aggregate()
            .sort_by(|user| user.role.asc())
            .sort_by(|user| user.age.desc());

        assert_eq!(
            aggregation.pipeline(),
            &[
                doc! {
                    "$sort": {
                        "role": 1,
                    },
                },
                doc! {
                    "$sort": {
                        "age": -1,
                    },
                },
            ],
        );
    }

    #[test]
    fn skip_and_limit_append_int64_stages() {
        let aggregation = User::aggregate().skip(20).limit(10);

        assert_eq!(
            aggregation.pipeline(),
            &[
                doc! {
                    "$skip": Bson::Int64(20),
                },
                doc! {
                    "$limit": Bson::Int64(10),
                },
            ],
        );
    }

    #[test]
    fn text_appends_a_text_match_stage() {
        let aggregation = User::aggregate().text("rust mongodb");

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$match": {
                    "$text": {
                        "$search": "rust mongodb",
                    },
                },
            }],
        );
    }

    #[test]
    fn raw_stage_appends_the_document_unchanged() {
        let aggregation = User::aggregate().raw_stage(doc! {
            "$group": {
                "_id": "$role",
                "count": { "$sum": 1 },
            },
        });

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$group": {
                    "_id": "$role",
                    "count": { "$sum": 1 },
                },
            }],
        );
    }

    #[test]
    fn raw_stage_with_links_serialized_field_names() {
        let aggregation = User::aggregate().raw_stage_with(|user| {
            doc! {
                "$group": {
                    "_id": format!("${}", user.name.name()),
                    "average_age": {
                        "$avg": format!("${}", user.age.name()),
                    },
                },
            }
        });

        assert_eq!(
            aggregation.pipeline(),
            &[doc! {
                "$group": {
                    "_id": "$userName",
                    "average_age": {
                        "$avg": "$age",
                    },
                },
            }],
        );
    }

    #[test]
    fn raw_pipeline_appends_stages_in_order() {
        let aggregation = User::aggregate()
            .match_(|user| user.active.eq(true))
            .raw_pipeline([
                doc! { "$unwind": "$tags" },
                doc! {
                    "$group": {
                        "_id": "$tags",
                        "count": { "$sum": 1 },
                    },
                },
            ]);

        assert_eq!(
            aggregation.pipeline(),
            &[
                doc! {
                    "$match": {
                        "active": true,
                    },
                },
                doc! { "$unwind": "$tags" },
                doc! {
                    "$group": {
                        "_id": "$tags",
                        "count": { "$sum": 1 },
                    },
                },
            ],
        );
    }

    #[test]
    fn stages_preserve_exact_call_order() {
        let aggregation = User::aggregate()
            .match_(|user| user.active.eq(true))
            .sort_by(|user| user.age.desc())
            .skip(5)
            .limit(10)
            .raw_stage(doc! { "$count": "total" });

        let stage_names: Vec<&str> = aggregation
            .pipeline()
            .iter()
            .map(|stage| {
                stage
                    .keys()
                    .next()
                    .expect("stage should have an operator")
                    .as_str()
            })
            .collect();

        assert_eq!(
            stage_names,
            ["$match", "$sort", "$skip", "$limit", "$count"],
        );
    }

    #[test]
    fn with_type_changes_output_type_without_adding_a_stage() {
        #[derive(Debug, serde::Deserialize)]
        struct RoleSummary {
            #[serde(rename = "_id")]
            #[allow(dead_code)]
            role: String,
        }

        let before = User::aggregate().raw_stage(doc! {
            "$group": {
                "_id": "$role",
            },
        });

        let before_pipeline = before.pipeline().to_vec();

        let after: Aggregation<User, RoleSummary> = before.with_type::<RoleSummary>();

        assert_eq!(after.pipeline(), before_pipeline.as_slice());
    }

    #[test]
    fn with_options_does_not_change_pipeline_stages() {
        let options = AggregateOptions::builder().allow_disk_use(true).build();

        let without_options = User::aggregate().match_(|user| user.active.eq(true));
        let expected = without_options.pipeline().to_vec();

        let with_options = without_options.with_options(options);

        assert_eq!(with_options.pipeline(), expected.as_slice());
    }
}
