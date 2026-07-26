use std::marker::PhantomData;

use mongodb::bson::Document;

use crate::error::oximod_error::OxiModError;
use crate::feature::model::Model;
use crate::query::expression::Expression;
use crate::query::queryable::Queryable;

#[derive(Debug, Clone)]
pub struct Query<M> {
    filter: Option<Expression>,
    limit: Option<u64>,
    skip: Option<u64>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Query<M> {
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self {
            filter: None,
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
    pub async fn first(self) -> Result<Option<M>, OxiModError> {
        let filter = self.into_filter_document();
        let collection = M::get_collection()?;

        collection
            .find_one(filter)
            .await
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

    pub async fn all(self) -> Result<Vec<M>, OxiModError> {
        let limit = self.limit;
        let skip = self.skip;
        let filter = self.into_filter_document();
        let collection = M::get_collection()?;

        let mut find = collection.find(filter);

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
}
