use mongodb::bson::{Bson, Document};
use std::ops::BitAnd;

/// A type-safe MongoDB update expression.
///
/// Update expressions are normally created through methods on typed
/// fields, such as [`Field::set`](super::Field::set).
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateExpression {
    document: Document,
}

impl UpdateExpression {
    pub(crate) fn set(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        let mut fields = Document::new();
        fields.insert(field.into(), value.into());

        let mut document = Document::new();
        document.insert("$set", fields);

        Self { document }
    }

    pub(crate) fn unset(field: impl Into<String>) -> Self {
        let mut fields = Document::new();

        // MongoDB ignores the value associated with a field under
        // `$unset`. An empty string is the conventional representation.
        fields.insert(field.into(), "");

        let mut document = Document::new();
        document.insert("$unset", fields);

        Self { document }
    }

    pub(crate) fn into_document(self) -> Document {
        self.document
    }

    pub(crate) fn inc(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        let mut fields = Document::new();
        fields.insert(field.into(), value.into());

        let mut document = Document::new();
        document.insert("$inc", fields);

        Self { document }
    }

    pub(crate) fn push(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        let mut fields = Document::new();
        fields.insert(field.into(), value.into());

        let mut document = Document::new();
        document.insert("$push", fields);

        Self { document }
    }

    pub(crate) fn add_to_set(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        let mut fields = Document::new();
        fields.insert(field.into(), value.into());

        let mut document = Document::new();
        document.insert("$addToSet", fields);

        Self { document }
    }

    pub(crate) fn pull(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        let mut fields = Document::new();
        fields.insert(field.into(), value.into());

        let mut document = Document::new();
        document.insert("$pull", fields);

        Self { document }
    }

    pub(crate) fn pop(field: impl Into<String>, position: i32) -> Self {
        let mut fields = Document::new();
        fields.insert(field.into(), position);

        let mut document = Document::new();
        document.insert("$pop", fields);

        Self { document }
    }

    pub(crate) fn push_each<I>(field: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Bson>,
    {
        let values = values.into_iter().map(Into::into).collect::<Vec<_>>();

        let mut each = Document::new();
        each.insert("$each", values);

        let mut fields = Document::new();
        fields.insert(field.into(), each);

        let mut document = Document::new();
        document.insert("$push", fields);

        Self { document }
    }
}

impl BitAnd for UpdateExpression {
    type Output = Self;

    /// Combines two update expressions into one MongoDB update document.
    ///
    /// Fields using the same update operator are merged into the same
    /// operator document.
    fn bitand(mut self, rhs: Self) -> Self::Output {
        for (operator, value) in rhs.document {
            match value {
                Bson::Document(rhs_fields) => {
                    if let Some(Bson::Document(lhs_fields)) = self.document.get_mut(&operator) {
                        for (field, value) in rhs_fields {
                            // When the same field is supplied more than once,
                            // the expression on the right takes precedence.
                            lhs_fields.insert(field, value);
                        }
                    } else {
                        self.document.insert(operator, Bson::Document(rhs_fields));
                    }
                }

                value => {
                    self.document.insert(operator, value);
                }
            }
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use crate::query::Field;
    use mongodb::bson::doc;

    use super::UpdateExpression;

    #[test]
    fn set_builds_set_update_document() {
        let update = UpdateExpression::set("active", true);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
            }
        );
    }

    #[test]
    fn combines_multiple_set_update_expressions() {
        let update = UpdateExpression::set("name", "UpdatedUser")
            & UpdateExpression::set("active", true)
            & UpdateExpression::set("age", 21);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "name": "UpdatedUser",
                    "active": true,
                    "age": 21,
                },
            }
        );
    }

    #[test]
    fn rightmost_set_value_takes_precedence() {
        let update = UpdateExpression::set("age", 20) & UpdateExpression::set("age", 21);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "age": 21,
                },
            }
        );
    }

    #[test]
    fn unset_builds_unset_update_document() {
        let update = UpdateExpression::unset("nickname");

        assert_eq!(
            update.into_document(),
            doc! {
                "$unset": {
                    "nickname": "",
                },
            }
        );
    }

    #[test]
    fn optional_field_builds_unset_update_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.unset().into_document(),
            doc! {
                "$unset": {
                    "nickname": "",
                },
            }
        );
    }

    #[test]
    fn combines_set_and_unset_update_expressions() {
        let update = UpdateExpression::set("active", false) & UpdateExpression::unset("nickname");

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": false,
                },
                "$unset": {
                    "nickname": "",
                },
            }
        );
    }

    #[test]
    fn inc_builds_inc_update_document() {
        let update = UpdateExpression::inc("login_count", 2);

        assert_eq!(
            update.into_document(),
            doc! {
                "$inc": {
                    "login_count": 2,
                },
            }
        );
    }

    #[test]
    fn combines_set_unset_and_inc_update_expressions() {
        let update = UpdateExpression::set("active", true)
            & UpdateExpression::unset("nickname")
            & UpdateExpression::inc("login_count", 1);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
                "$unset": {
                    "nickname": "",
                },
                "$inc": {
                    "login_count": 1,
                },
            }
        );
    }

    #[test]
    fn push_builds_push_update_document() {
        let update = UpdateExpression::push("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$push": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn add_to_set_builds_add_to_set_update_document() {
        let update = UpdateExpression::add_to_set("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$addToSet": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn pull_builds_pull_update_document() {
        let update = UpdateExpression::pull("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$pull": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn pop_first_builds_pop_update_document() {
        let update = UpdateExpression::pop("tags", -1);

        assert_eq!(
            update.into_document(),
            doc! {
                "$pop": {
                    "tags": -1,
                },
            }
        );
    }

    #[test]
    fn pop_last_builds_pop_update_document() {
        let update = UpdateExpression::pop("tags", 1);

        assert_eq!(
            update.into_document(),
            doc! {
                "$pop": {
                    "tags": 1,
                },
            }
        );
    }
}
