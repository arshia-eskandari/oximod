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

    pub(crate) fn into_document(self) -> Document {
        self.document
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
}
