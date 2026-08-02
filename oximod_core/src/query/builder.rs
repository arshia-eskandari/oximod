//! Typed query construction and execution.
//!
//! This module contains [`Query`], the builder produced by
//! [`Queryable::query`]. It stores typed filters and query modifiers until an
//! execution method reads from, updates, or deletes documents in the model's
//! MongoDB collection.
//!
//! Filters and text searches apply to every execution method. Sorting applies
//! to reads and single-document writes. Skipping, limiting, and pagination are
//! applied by [`Query::all`]; bulk writes reject them rather than silently
//! ignoring them. Array filters are used only by typed update operations.

use std::marker::PhantomData;

use mongodb::{
    bson::Document,
    options::ReturnDocument,
    results::{DeleteResult, UpdateResult},
};

use crate::{
    error::{
        oximod_error::OxiModError,
        query_error::{BulkWriteOperation, QueryError, QueryModifier},
    },
    feature::model::Model,
    query::{
        expression::Expression, queryable::Queryable, sort::SortExpression,
        text_search::TextSearch, update_expression::UpdateExpression,
    },
};

/// A type-safe MongoDB query for model `M`.
///
/// Queries are normally created through [`Queryable::query`] rather than
/// constructed directly.
///
/// The builder stores query configuration until an execution method such as
/// [`Query::all`], [`Query::first`], [`Query::update_one`], or
/// [`Query::delete_all`] is called.
///
/// # Example
///
/// ```ignore
/// let users = User::query()
///     .filter(|user| {
///         user.active.eq(true)
///             & user.age.gte(18)
///     })
///     .sort_by(|user| user.age.desc())
///     .limit(10)
///     .all()
///     .await?;
/// ```
///
/// Repeated calls to [`Query::filter`] are combined using logical AND.
/// Primary sorting methods replace the existing sort, while
/// [`Query::then_sort_by`] appends an additional sort field.
///
/// Some validation errors, such as invalid pagination, are recorded while the
/// query is being built and returned when the query is executed. When several
/// construction errors occur, the first error is preserved.
///
/// # Modifier scope
///
/// - filters and text searches apply to every execution method;
/// - sorting applies to [`Query::all`], [`Query::first`],
///   [`Query::update_one`], and [`Query::delete_one`];
/// - skipping, limiting, and pagination are applied by [`Query::all`];
/// - [`Query::count`] ignores sorting, skipping, limiting, and pagination;
/// - [`Query::update_all`] and [`Query::delete_all`] reject sorting, skipping,
///   limiting, and pagination;
/// - array filters are applied by [`Query::update_one`] and
///   [`Query::update_all`].
#[must_use = "queries do nothing unless an execution method is called"]
#[derive(Debug, Clone)]
pub struct Query<M> {
    filter: Option<Expression>,
    text: Option<TextSearch>,
    sort: Option<SortExpression>,
    limit: Option<u64>,
    skip: Option<u64>,
    error: Option<QueryError>,
    marker: PhantomData<fn() -> M>,
    pagination: bool,
    array_filters: Vec<Document>,
}

impl<M> Query<M> {
    /// Creates an empty query.
    ///
    /// This constructor is used by [`Queryable::query`] and generated model
    /// code. Applications should normally call `User::query()` on a derived
    /// model instead.
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self {
            filter: None,
            text: None,
            sort: None,
            limit: None,
            skip: None,
            error: None,
            marker: PhantomData,
            pagination: false,
            array_filters: Vec::new(),
        }
    }

    fn set_error(&mut self, error: QueryError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn take_error(&mut self) -> Result<(), OxiModError> {
        match self.error.take() {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }

    fn validate_bulk_write_modifiers(
        operation: BulkWriteOperation,
        pagination: bool,
        has_sort: bool,
        has_skip: bool,
        has_limit: bool,
    ) -> Result<(), QueryError> {
        if pagination {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Pagination,
            });
        }

        if has_sort {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Sort,
            });
        }

        if has_skip {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Skip,
            });
        }

        if has_limit {
            return Err(QueryError::UnsupportedBulkWriteModifier {
                operation,
                modifier: QueryModifier::Limit,
            });
        }

        Ok(())
    }

    /// Adds a MongoDB `$text` search.
    ///
    /// The model's collection must have a text index covering the searchable
    /// fields.
    ///
    /// A string creates a basic search:
    ///
    /// ```ignore
    /// let articles = Article::query()
    ///     .text("rust mongodb")
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Use [`TextSearch`] to configure language, case sensitivity, or
    /// diacritic sensitivity:
    ///
    /// ```ignore
    /// let articles = Article::query()
    ///     .text(
    ///         TextSearch::new("Café")
    ///             .language("none")
    ///             .case_sensitive(true)
    ///             .diacritic_sensitive(true),
    ///     )
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Phrase searches and excluded terms can be expressed using MongoDB's
    /// text-search syntax:
    ///
    /// ```ignore
    /// let articles = Article::query()
    ///     .text("\"rust mongodb\" -beginner")
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Calling this method again replaces the previously configured text
    /// search.
    ///
    /// # Parameters
    ///
    /// - `search`: A search string or configured [`TextSearch`].
    pub fn text<S>(mut self, search: S) -> Self
    where
        S: Into<TextSearch>,
    {
        self.text = Some(search.into());
        self
    }

    /// Sorts text-search results by MongoDB relevance score.
    ///
    /// This method replaces the existing primary sort. Use
    /// [`Query::then_sort_by`] afterward to add a deterministic secondary
    /// ordering.
    ///
    /// ```ignore
    /// let articles = Article::query()
    ///     .text("rust mongodb")
    ///     .sort_by_text_score()
    ///     .then_sort_by(|article| article.title.asc())
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// This method is intended for queries containing [`Query::text`].
    /// MongoDB determines relevance from the collection's text index.
    pub fn sort_by_text_score(mut self) -> Self {
        self.sort = Some(SortExpression::text_score("_textScore"));

        self
    }

    fn build_filter_document(filter: Option<Expression>, text: Option<TextSearch>) -> Document {
        let mut document = filter.map(Expression::into_document).unwrap_or_default();

        if let Some(text) = text {
            document.extend(text.into_document());
        }

        document
    }
}

impl<M> Default for Query<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Query<M>
where
    M: Queryable,
{
    /// Adds a typed filter expression.
    ///
    /// The closure receives the field structure generated for `M`. Field
    /// methods create type-safe MongoDB expressions, preventing operators from
    /// being applied to incompatible field types.
    ///
    /// ```ignore
    /// let query = User::query().filter(|user| {
    ///     user.active.eq(true)
    ///         & user.age.gte(18)
    ///         & (
    ///             user.role.eq("admin")
    ///                 | user.role.eq("member")
    ///         )
    /// });
    /// ```
    ///
    /// Repeated calls are combined using logical AND:
    ///
    /// ```ignore
    /// let query = User::query()
    ///     .filter(|user| user.active.eq(true))
    ///     .filter(|user| user.age.gte(18));
    /// ```
    ///
    /// # Parameters
    ///
    /// - `build`: A closure that creates an [`Expression`] from the generated
    ///   fields for `M`.
    pub fn filter<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> Expression,
    {
        let fields = M::fields();
        let expression = build(&fields);

        self.filter = Some(match self.filter.take() {
            Some(existing) => existing & expression,
            None => expression,
        });

        self
    }

    /// Limits the number of documents returned by [`Query::all`].
    ///
    /// A later call replaces the previously configured limit. The limit is
    /// applied by [`Query::all`]. Bulk writes reject configured limits.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .limit(20)
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `limit`: The maximum number of documents to return.
    ///
    /// # Errors
    ///
    /// Execution returns [`OxiModError`] if `limit` cannot be represented by
    /// the signed integer required by the MongoDB driver.
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skips matching documents before collecting results with
    /// [`Query::all`].
    ///
    /// A later call replaces the previously configured offset. The offset is
    /// applied by [`Query::all`]. Bulk writes reject configured offsets.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .sort_by(|user| user.name.asc())
    ///     .skip(20)
    ///     .limit(10)
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `skip`: The number of matching documents to skip.
    pub fn skip(mut self, skip: u64) -> Self {
        self.skip = Some(skip);
        self
    }

    /// Applies one-based pagination.
    ///
    /// Page `1` represents the first page. This method calculates:
    ///
    /// ```text
    /// skip = (page - 1) * page_size
    /// limit = page_size
    /// ```
    ///
    /// ```ignore
    /// let second_page = User::query()
    ///     .sort_by(|user| user.name.asc())
    ///     .page(2, 25)
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `page`: The one-based page number.
    /// - `page_size`: The number of documents per page.
    ///
    /// # Errors
    ///
    /// Invalid values are recorded on the query and returned when an execution
    /// method is called:
    ///
    /// - `page == 0`
    /// - `page_size == 0`
    /// - multiplication overflow while calculating the offset
    ///
    /// Pagination is applied by [`Query::all`] and ignored by
    /// [`Query::count`]. It is not supported by [`Query::update_all`] or
    /// [`Query::delete_all`]. Single-document operations select by filter and
    /// sorting rather than by page.
    pub fn page(mut self, page: u64, page_size: u64) -> Self {
        self.pagination = true;

        if page == 0 {
            self.set_error(QueryError::InvalidPageNumber { page });
            return self;
        }

        if page_size == 0 {
            self.set_error(QueryError::InvalidPageSize { page_size });
            return self;
        }

        let Some(skip) = (page - 1).checked_mul(page_size) else {
            self.set_error(QueryError::PaginationOverflow { page, page_size });

            return self;
        };

        self.skip = Some(skip);
        self.limit = Some(page_size);

        self
    }

    /// Sets the primary sort expression.
    ///
    /// Calling this method again replaces the existing sort. Use
    /// [`Query::then_sort_by`] to append additional fields.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .sort_by(|user| user.age.desc())
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Sorting is used by [`Query::all`], [`Query::first`],
    /// [`Query::update_one`], and [`Query::delete_one`].
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

        self.sort = Some(build(&fields));
        self
    }

    /// Appends a secondary sort expression.
    ///
    /// Sort fields are evaluated in insertion order. When no primary sort has
    /// been configured, this method creates one.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .sort_by(|user| user.role.asc())
    ///     .then_sort_by(|user| user.age.desc())
    ///     .then_sort_by(|user| user.name.asc())
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `build`: A closure that creates the next [`SortExpression`].
    pub fn then_sort_by<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> SortExpression,
    {
        let fields = M::fields();
        let next_sort = build(&fields);

        match &mut self.sort {
            Some(existing) => {
                existing.extend(next_sort);
            }
            None => {
                self.sort = Some(next_sort);
            }
        }

        self
    }

    /// Adds a MongoDB array filter for a filtered positional update.
    ///
    /// The identifier used here must match the identifier supplied to
    /// `.filtered()` in the update expression.
    ///
    /// ```ignore
    /// User::query()
    ///     .filter(|user| user.name.eq("User1"))
    ///     .array_filter(|user| {
    ///         user.addresses.array_filter(
    ///             "address",
    ///             |address| address.active.eq(false),
    ///         )
    ///     })
    ///     .update_one(|user| {
    ///         user.addresses.filtered(
    ///             "address",
    ///             |address| address.active.set(true),
    ///         )
    ///     })
    ///     .await?;
    /// ```
    ///
    /// Multiple calls add multiple array-filter documents. Array filters are
    /// applied by [`Query::update_one`] and [`Query::update_all`].
    ///
    /// # Parameters
    ///
    /// - `build`: A closure producing the array-filter expression.
    pub fn array_filter<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> Expression,
    {
        let fields = M::fields();
        let expression = build(&fields);

        self.array_filters.push(expression.into_document());

        self
    }

    pub(crate) fn into_filter_document(self) -> Document {
        Self::build_filter_document(self.filter, self.text)
    }
}

impl<M> Query<M>
where
    M: Queryable + Model,
{
    /// Returns the first document matching this query.
    ///
    /// Sorting can be used to control which matching document is considered
    /// first. Skipping, limiting, and pagination are not applied by this
    /// method.
    ///
    /// ```ignore
    /// let user = User::query()
    ///     .filter(|user| user.active.eq(true))
    ///     .sort_by(|user| user.created_at.asc())
    ///     .first()
    ///     .await?;
    /// ```
    ///
    /// # Returns
    ///
    /// - `Ok(Some(M))` when a matching document is found.
    /// - `Ok(None)` when no document matches.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the query contains a deferred construction error
    /// - the model collection cannot be resolved
    /// - MongoDB fails to execute the query
    /// - the matching document cannot be deserialized
    pub async fn first(mut self) -> Result<Option<M>, OxiModError> {
        self.take_error()?;

        let sort = self.sort.take();
        let filter = self.into_filter_document();
        let collection = M::get_collection()?;

        let mut find = collection.find_one(filter);

        if let Some(sort) = sort {
            find = find.sort(sort.into_document());
        }

        find.await
            .map_err(|error| OxiModError::database("Failed to execute typed query", error))
    }

    /// Counts documents matching this query's filter.
    ///
    /// Sorting, skipping, limiting, and pagination do not change the returned
    /// count.
    ///
    /// ```ignore
    /// let count = User::query()
    ///     .filter(|user| user.active.eq(true))
    ///     .count()
    ///     .await?;
    /// ```
    ///
    /// # Returns
    ///
    /// The number of matching documents.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if the query contains a deferred error,
    /// collection resolution fails, or MongoDB cannot count the documents.
    pub async fn count(mut self) -> Result<u64, OxiModError> {
        self.take_error()?;

        let filter = self.into_filter_document();
        let collection = M::get_collection()?;

        collection
            .count_documents(filter)
            .await
            .map_err(|error| OxiModError::database("Failed to count typed query results", error))
    }

    /// Returns all documents matching this query.
    ///
    /// Configured sorting, skipping, limiting, and pagination are applied
    /// before results are collected.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| user.active.eq(true))
    ///     .sort_by(|user| user.name.asc())
    ///     .page(1, 25)
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// # Returns
    ///
    /// A vector containing every document returned by MongoDB after applying
    /// the configured query options.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the query contains a deferred construction error
    /// - the limit exceeds the range supported by the driver
    /// - collection resolution or query execution fails
    /// - cursor advancement fails
    /// - a document cannot be deserialized into `M`
    pub async fn all(mut self) -> Result<Vec<M>, OxiModError> {
        self.take_error()?;

        let sort = self.sort.take();
        let limit = self.limit.take();
        let skip = self.skip.take();

        let filter = self.into_filter_document();
        let collection = M::get_collection()?;

        let mut find = collection.find(filter);

        if let Some(sort) = sort {
            find = find.sort(sort.into_document());
        }

        if let Some(skip) = skip {
            find = find.skip(skip);
        }

        if let Some(limit) = limit {
            let limit = i64::try_from(limit).map_err(|_| QueryError::LimitOutOfRange { limit })?;

            find = find.limit(limit);
        }

        let mut cursor = find
            .await
            .map_err(|error| OxiModError::database("Failed to execute typed query", error))?;

        let mut models = Vec::new();

        while cursor
            .advance()
            .await
            .map_err(|error| OxiModError::database("Failed to advance typed query cursor", error))?
        {
            let model = cursor.deserialize_current().map_err(|error| {
                OxiModError::serialization("Failed to deserialize typed query result", error)
            })?;

            models.push(model);
        }

        Ok(models)
    }

    /// Deletes and returns the first matching document.
    ///
    /// When sorting is configured, the first document according to that
    /// ordering is deleted. Skipping, limiting, and pagination are not applied
    /// by this method.
    ///
    /// ```ignore
    /// let deleted = User::query()
    ///     .filter(|user| user.active.eq(false))
    ///     .sort_by(|user| user.created_at.asc())
    ///     .delete_one()
    ///     .await?;
    /// ```
    ///
    /// # Returns
    ///
    /// - `Ok(Some(M))` containing the deleted document.
    /// - `Ok(None)` when no document matches.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if the query contains a deferred error,
    /// collection resolution fails, MongoDB rejects the query, or the deleted
    /// document cannot be deserialized.
    pub async fn delete_one(self) -> Result<Option<M>, OxiModError> {
        let Self {
            filter,
            text,
            sort,
            error,
            ..
        } = self;

        if let Some(error) = error {
            return Err(error.into());
        }

        let filter = Self::build_filter_document(filter, text);

        let collection = M::get_collection()?;

        let mut operation = collection.find_one_and_delete(filter);

        if let Some(sort) = sort {
            operation = operation.sort(sort.into_document());
        }

        operation.await.map_err(|error| {
            OxiModError::database(
                "Failed to delete the first document matching the typed query",
                error,
            )
        })
    }

    /// Deletes every document matching this query.
    ///
    /// ```ignore
    /// let result = User::query()
    ///     .filter(|user| user.active.eq(false))
    ///     .delete_all()
    ///     .await?;
    ///
    /// println!("Deleted {}", result.deleted_count);
    /// ```
    ///
    /// # Returns
    ///
    /// A [`DeleteResult`] containing the number of removed documents.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the query contains a deferred error
    /// - sorting, skipping, limiting, or pagination was configured
    /// - collection resolution or deletion fails
    ///
    /// # Warning
    ///
    /// An empty query deletes every document in the model's collection:
    ///
    /// ```ignore
    /// User::query().delete_all().await?;
    /// ```
    pub async fn delete_all(self) -> Result<DeleteResult, OxiModError> {
        let Self {
            filter,
            text,
            sort,
            limit,
            skip,
            pagination,
            error,
            ..
        } = self;

        if let Some(error) = error {
            return Err(error.into());
        }

        Self::validate_bulk_write_modifiers(
            BulkWriteOperation::DeleteAll,
            pagination,
            sort.is_some(),
            skip.is_some(),
            limit.is_some(),
        )?;

        let filter = Self::build_filter_document(filter, text);

        let collection = M::get_collection()?;

        collection.delete_many(filter).await.map_err(|error| {
            OxiModError::database("Failed to delete documents matching the typed query", error)
        })
    }

    /// Updates and returns the first matching document.
    ///
    /// The update closure receives the generated field structure for `M` and
    /// must produce an [`UpdateExpression`].
    ///
    /// When sorting is configured, the first document according to that
    /// ordering is updated. Skipping, limiting, and pagination are not applied
    /// by this method. The returned document reflects the value after the
    /// update.
    ///
    /// ```ignore
    /// let updated = User::query()
    ///     .filter(|user| user.name.eq("User1"))
    ///     .update_one(|user| {
    ///         user.active.set(true)
    ///             & user.login_count.inc(1)
    ///     })
    ///     .await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `build`: A closure producing the typed update expression.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(M))` containing the updated document.
    /// - `Ok(None)` when no document matches.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if the query contains a deferred error,
    /// collection resolution fails, MongoDB rejects the update, or the updated
    /// document cannot be deserialized.
    pub async fn update_one<F>(self, build: F) -> Result<Option<M>, OxiModError>
    where
        F: FnOnce(&M::Fields) -> UpdateExpression,
    {
        let Self {
            filter,
            text,
            sort,
            array_filters,
            error,
            ..
        } = self;

        if let Some(error) = error {
            return Err(error.into());
        }

        let fields = M::fields();
        let update = build(&fields).into_document();

        let filter = Self::build_filter_document(filter, text);

        let collection = M::get_collection()?;

        let mut operation = collection
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::After);

        if let Some(sort) = sort {
            operation = operation.sort(sort.into_document());
        }

        if !array_filters.is_empty() {
            operation = operation.array_filters(array_filters);
        }

        operation.await.map_err(|error| {
            OxiModError::database(
                "Failed to update the first document matching the typed query",
                error,
            )
        })
    }

    /// Updates every document matching this query.
    ///
    /// ```ignore
    /// let result = User::query()
    ///     .filter(|user| user.active.eq(false))
    ///     .update_all(|user| {
    ///         user.status.set("inactive")
    ///     })
    ///     .await?;
    ///
    /// println!("Modified {}", result.modified_count);
    /// ```
    ///
    /// # Parameters
    ///
    /// - `build`: A closure producing the typed update expression.
    ///
    /// # Returns
    ///
    /// An [`UpdateResult`] describing how many documents matched and were
    /// modified.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the query contains a deferred error
    /// - sorting, skipping, limiting, or pagination was configured
    /// - collection resolution or update execution fails
    ///
    /// # Warning
    ///
    /// An empty query updates every document in the model's collection.
    pub async fn update_all<F>(self, build: F) -> Result<UpdateResult, OxiModError>
    where
        F: FnOnce(&M::Fields) -> UpdateExpression,
    {
        let Self {
            filter,
            text,
            sort,
            limit,
            skip,
            array_filters,
            pagination,
            error,
            ..
        } = self;

        if let Some(error) = error {
            return Err(error.into());
        }

        Self::validate_bulk_write_modifiers(
            BulkWriteOperation::UpdateAll,
            pagination,
            sort.is_some(),
            skip.is_some(),
            limit.is_some(),
        )?;

        let fields = M::fields();
        let update = build(&fields).into_document();

        let filter = Self::build_filter_document(filter, text);

        let collection = M::get_collection()?;

        let mut operation = collection.update_many(filter, update);

        if !array_filters.is_empty() {
            operation = operation.array_filters(array_filters);
        }

        operation.await.map_err(|error| {
            OxiModError::database("Failed to update documents matching the typed query", error)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::error::query_error::{BulkWriteOperation, QueryError, QueryModifier};
    use crate::query::{Field, Query, Queryable, SortExpression, TextSearch};
    use mongodb::bson::doc;

    struct Article;

    struct ArticleFields {
        published: Field<bool>,
    }

    impl Queryable for Article {
        type Fields = ArticleFields;

        fn fields() -> Self::Fields {
            ArticleFields {
                published: Field::new("published"),
            }
        }
    }

    struct User;

    #[derive(Debug, Clone)]
    struct UserFields {
        active: Field<bool>,
        age: Field<i32>,
        role: Field<String>,
    }

    impl UserFields {
        fn new() -> Self {
            Self {
                active: Field::new("active"),
                age: Field::new("age"),
                role: Field::new("role"),
            }
        }
    }

    impl Queryable for User {
        type Fields = UserFields;

        fn fields() -> Self::Fields {
            UserFields::new()
        }
    }

    #[test]
    fn query_without_a_filter_produces_an_empty_document() {
        let query = Query::<User>::new();

        assert_eq!(query.into_filter_document(), doc! {});
    }

    #[test]
    fn queryable_model_can_create_a_query() {
        let query = User::query();

        assert_eq!(query.into_filter_document(), doc! {});
    }

    #[test]
    fn filter_builds_an_expression_from_model_fields() {
        let query = User::query().filter(|user| user.active.eq(true));

        assert_eq!(
            query.into_filter_document(),
            doc! {
                "active": true,
            }
        );
    }

    #[test]
    fn filter_supports_ordered_comparisons() {
        let query = User::query().filter(|user| user.age.gte(18));

        assert_eq!(
            query.into_filter_document(),
            doc! {
                "age": {
                    "$gte": 18,
                },
            }
        );
    }

    #[test]
    fn filter_supports_logical_and() {
        let query = User::query().filter(|user| user.active.eq(true) & user.age.gte(18));

        assert_eq!(
            query.into_filter_document(),
            doc! {
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
            }
        );
    }

    #[test]
    fn filter_supports_nested_logical_expressions() {
        let query = User::query()
            .filter(|user| user.active.eq(true) & (user.age.gte(18) | user.role.eq("admin")));

        assert_eq!(
            query.into_filter_document(),
            doc! {
                "$and": [
                    {
                        "active": true,
                    },
                    {
                        "$or": [
                            {
                                "age": {
                                    "$gte": 18,
                                },
                            },
                            {
                                "role": "admin",
                            },
                        ],
                    },
                ],
            }
        );
    }

    #[test]
    fn repeated_filter_calls_are_combined_with_and() {
        let query = User::query()
            .filter(|user| user.active.eq(true))
            .filter(|user| user.age.gte(18))
            .filter(|user| user.role.ne("banned"));

        assert_eq!(
            query.into_filter_document(),
            doc! {
                "$and": [
                    {
                        "active": true,
                    },
                    {
                        "age": {
                            "$gte": 18,
                        },
                    },
                    {
                        "role": {
                            "$ne": "banned",
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn complete_typed_query_syntax_works() {
        let document = User::query()
            .filter(|user| user.active.eq(true) & (user.age.gte(18) | user.role.eq("admin")))
            .into_filter_document();

        assert_eq!(
            document,
            doc! {
                "$and": [
                    {
                        "active": true,
                    },
                    {
                        "$or": [
                            {
                                "age": {
                                    "$gte": 18,
                                },
                            },
                            {
                                "role": "admin",
                            },
                        ],
                    },
                ],
            }
        );
    }

    #[test]
    fn then_sort_by_combines_multiple_sort_fields() {
        let query = User::query()
            .sort_by(|user| user.age.asc())
            .then_sort_by(|user| user.role.desc());

        let sort = query.sort.expect("query should contain sorting");

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": 1,
                "role": -1,
            }
        );
    }

    #[test]
    fn page_sets_skip_and_limit() {
        let query = User::query().page(3, 10);

        assert_eq!(query.skip, Some(20));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn first_page_skips_no_results() {
        let query = User::query().page(1, 10);

        assert_eq!(query.skip, Some(0));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn page_records_invalid_page_number() {
        let query = User::query().page(0, 10);

        assert_eq!(query.error, Some(QueryError::InvalidPageNumber { page: 0 }),);
    }

    #[test]
    fn page_records_invalid_page_size() {
        let query = User::query().page(1, 0);

        assert_eq!(
            query.error,
            Some(QueryError::InvalidPageSize { page_size: 0 }),
        );
    }

    #[test]
    fn page_records_pagination_overflow() {
        let query = User::query().page(u64::MAX, 2);

        assert_eq!(
            query.error,
            Some(QueryError::PaginationOverflow {
                page: u64::MAX,
                page_size: 2,
            }),
        );
    }

    #[test]
    fn later_limit_and_skip_calls_replace_previous_values() {
        let query = User::query().limit(10).limit(20).skip(5).skip(7);

        assert_eq!(query.limit, Some(20));
        assert_eq!(query.skip, Some(7));
    }

    #[test]
    fn later_sort_by_call_replaces_previous_sort() {
        let query = User::query()
            .sort_by(|user| user.age.asc())
            .sort_by(|user| user.role.desc());

        let sort = query.sort.expect("query should contain sorting");

        assert_eq!(
            sort.into_document(),
            doc! {
                "role": -1,
            },
        );
    }

    #[test]
    fn first_construction_error_is_preserved() {
        let query = User::query().page(0, 10).page(1, 0);

        assert_eq!(query.error, Some(QueryError::InvalidPageNumber { page: 0 }),);
    }

    #[test]
    fn array_filter_calls_accumulate_documents() {
        let query = User::query()
            .array_filter(|user| user.active.eq(false))
            .array_filter(|user| user.age.gte(18));

        assert_eq!(
            query.array_filters,
            vec![
                doc! {
                    "active": false,
                },
                doc! {
                    "age": {
                        "$gte": 18,
                    },
                },
            ],
        );
    }

    #[test]
    fn bulk_writes_reject_unsupported_modifiers() {
        let cases = [
            (true, false, false, false, QueryModifier::Pagination),
            (false, true, false, false, QueryModifier::Sort),
            (false, false, true, false, QueryModifier::Skip),
            (false, false, false, true, QueryModifier::Limit),
        ];

        for (pagination, has_sort, has_skip, has_limit, modifier) in cases {
            assert_eq!(
                Query::<User>::validate_bulk_write_modifiers(
                    BulkWriteOperation::UpdateAll,
                    pagination,
                    has_sort,
                    has_skip,
                    has_limit,
                ),
                Err(QueryError::UnsupportedBulkWriteModifier {
                    operation: BulkWriteOperation::UpdateAll,
                    modifier,
                }),
            );
        }
    }

    #[test]
    fn bulk_writes_accept_queries_without_unsupported_modifiers() {
        assert_eq!(
            Query::<User>::validate_bulk_write_modifiers(
                BulkWriteOperation::DeleteAll,
                false,
                false,
                false,
                false,
            ),
            Ok(()),
        );
    }

    #[test]
    fn text_search_builds_top_level_filter() {
        let filter = Query::<Article>::new().text("rust").into_filter_document();

        assert_eq!(
            filter,
            doc! {
                "$text": {
                    "$search": "rust",
                },
            }
        );
    }

    #[test]
    fn text_search_combines_with_typed_filter() {
        let filter = Query::<Article>::new()
            .text("rust")
            .filter(|article| article.published.eq(true))
            .into_filter_document();

        assert_eq!(
            filter,
            doc! {
                "published": true,
                "$text": {
                    "$search": "rust",
                },
            }
        );
    }

    #[test]
    fn configured_text_search_builds_options() {
        let filter = Query::<Article>::new()
            .text(
                TextSearch::new("Café")
                    .language("none")
                    .case_sensitive(true)
                    .diacritic_sensitive(true),
            )
            .into_filter_document();

        assert_eq!(
            filter,
            doc! {
                "$text": {
                    "$search": "Café",
                    "$language": "none",
                    "$caseSensitive": true,
                    "$diacriticSensitive": true,
                },
            }
        );
    }

    #[test]
    fn later_text_search_replaces_previous_search() {
        let filter = Query::<Article>::new()
            .text("rust")
            .text("mongodb")
            .into_filter_document();

        assert_eq!(
            filter,
            doc! {
                "$text": {
                    "$search": "mongodb",
                },
            }
        );
    }

    #[test]
    fn text_search_is_preserved_when_filter_is_added_first() {
        let filter = Query::<Article>::new()
            .filter(|article| article.published.eq(true))
            .text("rust")
            .into_filter_document();

        assert_eq!(
            filter,
            doc! {
                "published": true,
                "$text": {
                    "$search": "rust",
                },
            }
        );
    }

    #[test]
    fn sort_by_text_score_sets_text_score_sort() {
        let query = Query::<Article>::new().text("rust").sort_by_text_score();

        let sort = query.sort.expect("query should contain sorting");

        assert_eq!(
            sort.into_document(),
            doc! {
                "_textScore": {
                    "$meta": "textScore",
                },
            }
        );
    }

    #[test]
    fn text_score_sort_can_be_followed_by_typed_sort() {
        let query = Query::<Article>::new()
            .text("rust")
            .sort_by_text_score()
            .then_sort_by(|_| SortExpression::ascending("published"));

        let sort = query.sort.expect("query should contain sorting");

        assert_eq!(
            sort.into_document(),
            doc! {
                "_textScore": {
                    "$meta": "textScore",
                },
                "published": 1,
            }
        );
    }
}
