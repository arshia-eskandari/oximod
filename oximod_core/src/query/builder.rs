use crate::error::oximod_error::OxiModError;
use crate::feature::model::Model;
use crate::query::expression::Expression;
use crate::query::queryable::Queryable;
use crate::query::sort::SortExpression;
use mongodb::bson::Document;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Query<M> {
    filter: Option<Expression>,
    sort: Option<SortExpression>,
    limit: Option<u64>,
    skip: Option<u64>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Query<M> {
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self {
            filter: None,
            sort: None,
            limit: None,
            skip: None,
            marker: PhantomData,
        }
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

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn skip(mut self, skip: u64) -> Self {
        self.skip = Some(skip);
        self
    }

    pub fn page(mut self, page: u64, page_size: u64) -> Self {
        assert!(page > 0, "page must be at least 1");
        assert!(page_size > 0, "page size must be at least 1");

        let skip = (page - 1)
            .checked_mul(page_size)
            .expect("pagination offset exceeds the supported range");

        self.skip = Some(skip);
        self.limit = Some(page_size);

        self
    }

    pub fn sort_by<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> SortExpression,
    {
        let fields = M::fields();

        self.sort = Some(build(&fields));
        self
    }

    pub fn then_sort_by<F>(mut self, build: F) -> Self
    where
        F: FnOnce(&M::Fields) -> SortExpression,
    {
        let fields = M::fields();
        let next_sort = build(&fields);

        match &mut self.sort {
            Some(existing) => existing.extend(next_sort),
            None => self.sort = Some(next_sort),
        }

        self
    }

    pub(crate) fn into_filter_document(self) -> Document {
        self.filter
            .map(Expression::into_document)
            .unwrap_or_default()
    }
}

impl<M> Query<M>
where
    M: Queryable + Model,
{
    pub async fn first(mut self) -> Result<Option<M>, OxiModError> {
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

    pub async fn count(self) -> Result<u64, OxiModError> {
        let filter = self.into_filter_document();
        let collection = M::get_collection()?;

        collection
            .count_documents(filter)
            .await
            .map_err(|error| OxiModError::database("Failed to count typed query results", error))
    }

    pub async fn all(mut self) -> Result<Vec<M>, OxiModError> {
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
            let limit = i64::try_from(limit).map_err(|error| {
                OxiModError::custom_with_source(
                    "Query limit exceeds MongoDB's supported range",
                    error,
                )
            })?;

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
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::Query;
    use crate::query::field::Field;
    use crate::query::queryable::Queryable;

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
    #[should_panic(expected = "page must be at least 1")]
    fn page_rejects_zero_page_number() {
        let _query = User::query().page(0, 10);
    }

    #[test]
    #[should_panic(expected = "page size must be at least 1")]
    fn page_rejects_zero_page_size() {
        let _query = User::query().page(1, 0);
    }
}
