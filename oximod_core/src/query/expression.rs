use mongodb::bson::{Bson, Document};
use std::ops::{BitAnd, BitOr};

#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    pub(crate) kind: ExpressionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExpressionKind {
    Comparison {
        field: String,
        operator: ComparisonOperator,
        value: Bson,
    },
    And(Vec<Expression>),
    Or(Vec<Expression>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComparisonOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl ComparisonOperator {
    fn mongo_name(self) -> Option<&'static str> {
        match self {
            Self::Eq => None,
            Self::Ne => Some("$ne"),
            Self::Gt => Some("$gt"),
            Self::Gte => Some("$gte"),
            Self::Lt => Some("$lt"),
            Self::Lte => Some("$lte"),
        }
    }
}

impl Expression {
    pub(crate) fn comparison(
        field: impl Into<String>,
        operator: ComparisonOperator,
        value: impl Into<Bson>,
    ) -> Self {
        Self {
            kind: ExpressionKind::Comparison {
                field: field.into(),
                operator,
                value: value.into(),
            },
        }
    }

    pub(crate) fn into_document(self) -> Document {
        match self.kind {
            ExpressionKind::Comparison {
                field,
                operator,
                value,
            } => comparison_into_document(field, operator, value),

            ExpressionKind::And(expressions) => logical_into_document("$and", expressions),

            ExpressionKind::Or(expressions) => logical_into_document("$or", expressions),
        }
    }
}

impl BitAnd for Expression {
    type Output = Expression;

    fn bitand(self, rhs: Expression) -> Self::Output {
        Expression {
            kind: ExpressionKind::And(vec![self, rhs]),
        }
    }
}

impl BitOr for Expression {
    type Output = Expression;

    fn bitor(self, rhs: Expression) -> Self::Output {
        Expression {
            kind: ExpressionKind::Or(vec![self, rhs]),
        }
    }
}

fn comparison_into_document(field: String, operator: ComparisonOperator, value: Bson) -> Document {
    let mut document = Document::new();

    match operator.mongo_name() {
        None => {
            document.insert(field, value);
        }

        Some(operator_name) => {
            let mut operator_document = Document::new();
            operator_document.insert(operator_name, value);

            document.insert(field, operator_document);
        }
    }

    document
}

fn logical_into_document(operator: &'static str, expressions: Vec<Expression>) -> Document {
    let children = expressions
        .into_iter()
        .map(|expression| Bson::Document(expression.into_document()))
        .collect::<Vec<_>>();

    let mut document = Document::new();
    document.insert(operator, Bson::Array(children));
    document
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, Document, doc, oid::ObjectId};

    use super::{ComparisonOperator, Expression};

    fn comparison(field: &str, operator: ComparisonOperator, value: impl Into<Bson>) -> Expression {
        Expression::comparison(field, operator, value)
    }

    #[test]
    fn equality_expression_converts_to_direct_field_value() {
        let expression = comparison("active", ComparisonOperator::Eq, true);

        assert_eq!(
            expression.into_document(),
            doc! {
                "active": true,
            }
        );
    }

    #[test]
    fn inequality_expression_converts_to_ne_operator() {
        let expression = comparison("status", ComparisonOperator::Ne, "deleted");

        assert_eq!(
            expression.into_document(),
            doc! {
                "status": {
                    "$ne": "deleted",
                },
            }
        );
    }

    #[test]
    fn greater_than_expression_converts_to_gt_operator() {
        let expression = comparison("age", ComparisonOperator::Gt, 18);

        assert_eq!(
            expression.into_document(),
            doc! {
                "age": {
                    "$gt": 18,
                },
            }
        );
    }

    #[test]
    fn greater_than_or_equal_expression_converts_to_gte_operator() {
        let expression = comparison("age", ComparisonOperator::Gte, 18);

        assert_eq!(
            expression.into_document(),
            doc! {
                "age": {
                    "$gte": 18,
                },
            }
        );
    }

    #[test]
    fn less_than_expression_converts_to_lt_operator() {
        let expression = comparison("price", ComparisonOperator::Lt, 100);

        assert_eq!(
            expression.into_document(),
            doc! {
                "price": {
                    "$lt": 100,
                },
            }
        );
    }

    #[test]
    fn less_than_or_equal_expression_converts_to_lte_operator() {
        let expression = comparison("price", ComparisonOperator::Lte, 100);

        assert_eq!(
            expression.into_document(),
            doc! {
                "price": {
                    "$lte": 100,
                },
            }
        );
    }

    #[test]
    fn equality_preserves_string_values() {
        let expression = comparison("name", ComparisonOperator::Eq, "Arshia");

        assert_eq!(
            expression.into_document(),
            doc! {
                "name": "Arshia",
            }
        );
    }

    #[test]
    fn equality_preserves_null_values() {
        let expression = comparison("deleted_at", ComparisonOperator::Eq, Bson::Null);

        assert_eq!(
            expression.into_document(),
            doc! {
                "deleted_at": null,
            }
        );
    }

    #[test]
    fn equality_preserves_object_id_values() {
        let id = ObjectId::new();

        let expression = comparison("_id", ComparisonOperator::Eq, id);

        assert_eq!(
            expression.into_document(),
            doc! {
                "_id": id,
            }
        );
    }

    #[test]
    fn equality_preserves_array_values() {
        let expression = comparison(
            "roles",
            ComparisonOperator::Eq,
            Bson::Array(vec![
                Bson::String("admin".to_owned()),
                Bson::String("editor".to_owned()),
            ]),
        );

        assert_eq!(
            expression.into_document(),
            doc! {
                "roles": ["admin", "editor"],
            }
        );
    }

    #[test]
    fn expression_supports_nested_field_paths() {
        let expression = comparison("address.city", ComparisonOperator::Eq, "Toronto");

        assert_eq!(
            expression.into_document(),
            doc! {
                "address.city": "Toronto",
            }
        );
    }

    #[test]
    fn two_expressions_can_be_combined_with_and() {
        let active = comparison("active", ComparisonOperator::Eq, true);

        let adult = comparison("age", ComparisonOperator::Gte, 18);

        let expression = active & adult;

        assert_eq!(
            expression.into_document(),
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
    fn two_expressions_can_be_combined_with_or() {
        let admin = comparison("role", ComparisonOperator::Eq, "admin");

        let verified = comparison("verified", ComparisonOperator::Eq, true);

        let expression = admin | verified;

        assert_eq!(
            expression.into_document(),
            doc! {
                "$or": [
                    {
                        "role": "admin",
                    },
                    {
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn or_expression_can_be_nested_inside_and_expression() {
        let active = comparison("active", ComparisonOperator::Eq, true);

        let adult = comparison("age", ComparisonOperator::Gte, 18);

        let admin = comparison("role", ComparisonOperator::Eq, "admin");

        let expression = active & (adult | admin);

        assert_eq!(
            expression.into_document(),
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
    fn and_expression_can_be_nested_inside_or_expression() {
        let active = comparison("active", ComparisonOperator::Eq, true);

        let adult = comparison("age", ComparisonOperator::Gte, 18);

        let admin = comparison("role", ComparisonOperator::Eq, "admin");

        let expression = active | (adult & admin);

        assert_eq!(
            expression.into_document(),
            doc! {
                "$or": [
                    {
                        "active": true,
                    },
                    {
                        "$and": [
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
    fn logical_expressions_can_be_nested_multiple_levels_deep() {
        let active = comparison("active", ComparisonOperator::Eq, true);

        let adult = comparison("age", ComparisonOperator::Gte, 18);

        let admin = comparison("role", ComparisonOperator::Eq, "admin");

        let verified = comparison("verified", ComparisonOperator::Eq, true);

        let banned = comparison("banned", ComparisonOperator::Eq, false);

        let expression = active & (adult | ((admin & verified) | banned));

        assert_eq!(
            expression.into_document(),
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
                                "$or": [
                                    {
                                        "$and": [
                                            {
                                                "role": "admin",
                                            },
                                            {
                                                "verified": true,
                                            },
                                        ],
                                    },
                                    {
                                        "banned": false,
                                    },
                                ],
                            },
                        ],
                    },
                ],
            }
        );
    }

    #[test]
    fn chained_and_operations_are_left_associative() {
        let first = comparison("active", ComparisonOperator::Eq, true);

        let second = comparison("age", ComparisonOperator::Gte, 18);

        let third = comparison("verified", ComparisonOperator::Eq, true);

        // Rust interprets this as:
        //
        // (first & second) & third
        let expression = first & second & third;

        assert_eq!(
            expression.into_document(),
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
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn chained_or_operations_are_left_associative() {
        let first = comparison("role", ComparisonOperator::Eq, "admin");

        let second = comparison("role", ComparisonOperator::Eq, "moderator");

        let third = comparison("verified", ComparisonOperator::Eq, true);

        // Rust interprets this as:
        //
        // (first | second) | third
        let expression = first | second | third;

        assert_eq!(
            expression.into_document(),
            doc! {
                "$or": [
                    {
                        "$or": [
                            {
                                "role": "admin",
                            },
                            {
                                "role": "moderator",
                            },
                        ],
                    },
                    {
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn bitwise_and_has_higher_precedence_than_bitwise_or() {
        let active = comparison("active", ComparisonOperator::Eq, true);

        let adult = comparison("age", ComparisonOperator::Gte, 18);

        let verified = comparison("verified", ComparisonOperator::Eq, true);

        // Rust interprets this as:
        //
        // active | (adult & verified)
        let expression = active | adult & verified;

        assert_eq!(
            expression.into_document(),
            doc! {
                "$or": [
                    {
                        "active": true,
                    },
                    {
                        "$and": [
                            {
                                "age": {
                                    "$gte": 18,
                                },
                            },
                            {
                                "verified": true,
                            },
                        ],
                    },
                ],
            }
        );
    }

    #[test]
    fn parentheses_override_default_operator_precedence() {
        let active = comparison("active", ComparisonOperator::Eq, true);

        let adult = comparison("age", ComparisonOperator::Gte, 18);

        let verified = comparison("verified", ComparisonOperator::Eq, true);

        let expression = (active | adult) & verified;

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "$or": [
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
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn converting_an_expression_does_not_add_unnecessary_operators() {
        let expression = comparison("email", ComparisonOperator::Eq, "arshia@example.com");

        let document = expression.into_document();

        assert_eq!(
            document,
            doc! {
                "email": "arshia@example.com",
            }
        );

        assert!(!document.contains_key("$and"));
        assert!(!document.contains_key("$or"));

        let email = document.get("email").expect("email field should exist");

        assert!(!matches!(email, Bson::Document(_)));
    }

    #[test]
    fn logical_operator_values_are_stored_as_bson_arrays() {
        let left = comparison("active", ComparisonOperator::Eq, true);

        let right = comparison("verified", ComparisonOperator::Eq, true);

        let document = (left & right).into_document();

        let conditions = document
            .get_array("$and")
            .expect("$and should contain a BSON array");

        assert_eq!(conditions.len(), 2);

        assert!(matches!(conditions.first(), Some(Bson::Document(_))));

        assert!(matches!(conditions.get(1), Some(Bson::Document(_))));
    }

    #[test]
    fn expression_conversion_produces_valid_mongodb_document_structure() {
        let expression = comparison("active", ComparisonOperator::Eq, true)
            & (comparison("age", ComparisonOperator::Gte, 18)
                | comparison("role", ComparisonOperator::Eq, "admin"));

        let document: Document = expression.into_document();

        let root_conditions = document
            .get_array("$and")
            .expect("root expression should be an $and array");

        assert_eq!(root_conditions.len(), 2);

        let active_condition = root_conditions[0]
            .as_document()
            .expect("first condition should be a document");

        assert_eq!(active_condition.get_bool("active"), Ok(true),);

        let nested_or = root_conditions[1]
            .as_document()
            .expect("second condition should be a document")
            .get_array("$or")
            .expect("second condition should contain an $or array");

        assert_eq!(nested_or.len(), 2);

        assert_eq!(
            nested_or[0]
                .as_document()
                .expect("age condition should be a document"),
            &doc! {
                "age": {
                    "$gte": 18,
                },
            }
        );

        assert_eq!(
            nested_or[1]
                .as_document()
                .expect("role condition should be a document"),
            &doc! {
                "role": "admin",
            }
        );
    }
}
