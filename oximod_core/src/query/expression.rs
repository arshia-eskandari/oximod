use mongodb::bson::{Bson, Document, doc};
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
    Not(Box<Expression>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComparisonOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    Nin,
    Exists,
    All,
    Size,
    ElemMatch,
    Type,
    Mod,
    BitsAllSet,
    BitsAnySet,
    BitsAllClear,
    BitsAnyClear,
    Near,
    GeoWithin,
    GeoIntersects,
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct ElementExpression {
    kind: ElementExpressionKind,
}

#[derive(Debug, Clone, PartialEq)]
enum ElementExpressionKind {
    Comparison {
        operator: ComparisonOperator,
        value: Bson,
    },
    And(Vec<ElementExpression>),
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
            Self::In => Some("$in"),
            Self::Nin => Some("$nin"),
            Self::Exists => Some("$exists"),
            Self::All => Some("$all"),
            Self::Size => Some("$size"),
            Self::ElemMatch => Some("$elemMatch"),
            Self::Type => Some("$type"),
            Self::Mod => Some("$mod"),
            Self::BitsAllSet => Some("$bitsAllSet"),
            Self::BitsAnySet => Some("$bitsAnySet"),
            Self::BitsAllClear => Some("$bitsAllClear"),
            Self::BitsAnyClear => Some("$bitsAnyClear"),
            Self::Near => Some("$near"),
            Self::GeoWithin => Some("$geoWithin"),
            Self::GeoIntersects => Some("$geoIntersects"),
        }
    }
}

impl ElementExpression {
    pub(crate) fn comparison<V>(operator: ComparisonOperator, value: V) -> Self
    where
        V: Into<Bson>,
    {
        Self {
            kind: ElementExpressionKind::Comparison {
                operator,
                value: value.into(),
            },
        }
    }

    pub(crate) fn into_document(self) -> Document {
        match self.kind {
            ElementExpressionKind::Comparison { operator, value } => {
                element_comparison_into_document(operator, value)
            }

            ElementExpressionKind::And(expressions) => element_and_into_document(expressions),
        }
    }

    fn into_and_children(self) -> Vec<Self> {
        match self.kind {
            ElementExpressionKind::And(expressions) => expressions,

            kind => vec![Self { kind }],
        }
    }
}

impl BitAnd for ElementExpression {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let mut expressions = self.into_and_children();
        expressions.extend(rhs.into_and_children());

        Self {
            kind: ElementExpressionKind::And(expressions),
        }
    }
}

fn element_comparison_into_document(operator: ComparisonOperator, value: Bson) -> Document {
    let operator_name = operator.mongo_name().unwrap_or("$eq");

    let mut document = Document::new();
    document.insert(operator_name, value);

    document
}

fn element_and_into_document(expressions: Vec<ElementExpression>) -> Document {
    let documents = expressions
        .into_iter()
        .map(ElementExpression::into_document)
        .collect::<Vec<_>>();

    let mut operator_names = std::collections::HashSet::new();

    let has_duplicate_operator = documents
        .iter()
        .flat_map(Document::keys)
        .any(|operator| !operator_names.insert(operator.to_owned()));

    if has_duplicate_operator {
        let clauses = documents.into_iter().map(Bson::Document).collect();

        let mut document = Document::new();

        document.insert("$and", Bson::Array(clauses));

        return document;
    }

    let mut result = Document::new();

    for document in documents {
        result.extend(document);
    }

    result
}

fn not_into_document(expression: Expression) -> Document {
    match expression.kind {
        ExpressionKind::Comparison {
            field,
            operator,
            value,
        } => {
            let not_value = match (operator, value) {
                (ComparisonOperator::Eq, value @ Bson::RegularExpression(_)) => value,

                (ComparisonOperator::Eq, value) => Bson::Document(doc! {
                    "$eq": value,
                }),

                (operator, value) => {
                    let operator_name = operator.mongo_name().expect(
                        "non-equality comparison must have \
                             a MongoDB operator",
                    );

                    let mut operator_document = Document::new();

                    operator_document.insert(operator_name, value);

                    Bson::Document(operator_document)
                }
            };

            let mut not_document = Document::new();
            not_document.insert("$not", not_value);

            let mut document = Document::new();
            document.insert(field, not_document);

            document
        }

        kind => {
            let expression = Expression { kind };

            doc! {
                "$nor": [
                    expression.into_document(),
                ],
            }
        }
    }
}

impl Expression {
    pub(crate) fn comparison<V>(
        field: impl Into<String>,
        operator: ComparisonOperator,
        value: V,
    ) -> Self
    where
        V: Into<Bson>,
    {
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

            ExpressionKind::Not(expression) => not_into_document(*expression),
        }
    }

    pub(crate) fn not(expression: Expression) -> Self {
        Self {
            kind: ExpressionKind::Not(Box::new(expression)),
        }
    }

    fn into_and_children(self) -> Vec<Expression> {
        match self.kind {
            ExpressionKind::And(expressions) => expressions,
            kind => vec![Expression { kind }],
        }
    }

    fn into_or_children(self) -> Vec<Expression> {
        match self.kind {
            ExpressionKind::Or(expressions) => expressions,
            kind => vec![Expression { kind }],
        }
    }
}

impl BitAnd for Expression {
    type Output = Expression;

    fn bitand(self, rhs: Expression) -> Self::Output {
        let mut expressions = self.into_and_children();

        expressions.extend(rhs.into_and_children());

        Expression {
            kind: ExpressionKind::And(expressions),
        }
    }
}

impl BitOr for Expression {
    type Output = Expression;

    fn bitor(self, rhs: Expression) -> Self::Output {
        let mut expressions = self.into_or_children();

        expressions.extend(rhs.into_or_children());

        Expression {
            kind: ExpressionKind::Or(expressions),
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
    use crate::query::ElementExpression;
    use crate::query::UpdateExpression;
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
        let expression = comparison("name", ComparisonOperator::Eq, "User1");

        assert_eq!(
            expression.into_document(),
            doc! {
                "name": "User1",
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
                "roles": [
                    "admin",
                    "editor",
                ],
            }
        );
    }

    #[test]
    fn expression_supports_nested_field_paths() {
        let expression = comparison("address.city", ComparisonOperator::Eq, "City1");

        assert_eq!(
            expression.into_document(),
            doc! {
                "address.city": "City1",
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
            }
        );
    }

    #[test]
    fn chained_and_operations_are_flattened() {
        let first = comparison("active", ComparisonOperator::Eq, true);

        let second = comparison("age", ComparisonOperator::Gte, 18);

        let third = comparison("verified", ComparisonOperator::Eq, true);

        let expression = first & second & third;

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
                    {
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn chained_or_operations_are_flattened() {
        let first = comparison("role", ComparisonOperator::Eq, "admin");

        let second = comparison("role", ComparisonOperator::Eq, "moderator");

        let third = comparison("verified", ComparisonOperator::Eq, true);

        let expression = first | second | third;

        assert_eq!(
            expression.into_document(),
            doc! {
                "$or": [
                    {
                        "role": "admin",
                    },
                    {
                        "role": "moderator",
                    },
                    {
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn and_flattens_expression_from_right_hand_side() {
        let first = comparison("active", ComparisonOperator::Eq, true);

        let second = comparison("age", ComparisonOperator::Gte, 18);

        let third = comparison("verified", ComparisonOperator::Eq, true);

        let expression = first & (second & third);

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
                    {
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn or_flattens_expression_from_right_hand_side() {
        let first = comparison("role", ComparisonOperator::Eq, "admin");

        let second = comparison("role", ComparisonOperator::Eq, "moderator");

        let third = comparison("verified", ComparisonOperator::Eq, true);

        let expression = first | (second | third);

        assert_eq!(
            expression.into_document(),
            doc! {
                "$or": [
                    {
                        "role": "admin",
                    },
                    {
                        "role": "moderator",
                    },
                    {
                        "verified": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn same_operator_groups_are_flattened_from_both_sides() {
        let first = comparison("first", ComparisonOperator::Eq, true);

        let second = comparison("second", ComparisonOperator::Eq, true);

        let third = comparison("third", ComparisonOperator::Eq, true);

        let fourth = comparison("fourth", ComparisonOperator::Eq, true);

        let expression = (first & second) & (third & fourth);

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "first": true,
                    },
                    {
                        "second": true,
                    },
                    {
                        "third": true,
                    },
                    {
                        "fourth": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn mixed_operator_groups_are_not_flattened() {
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
        let expression = comparison("email", ComparisonOperator::Eq, "user@example.com");

        let document = expression.into_document();

        assert_eq!(
            document,
            doc! {
                "email": "user@example.com",
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

        assert!(matches!(conditions.first(), Some(Bson::Document(_)),));

        assert!(matches!(conditions.get(1), Some(Bson::Document(_)),));
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

    #[test]
    fn in_expression_converts_to_in_operator() {
        let expression = comparison(
            "role",
            ComparisonOperator::In,
            Bson::Array(vec![
                Bson::String("admin".to_owned()),
                Bson::String("moderator".to_owned()),
            ]),
        );

        assert_eq!(
            expression.into_document(),
            doc! {
                "role": {
                    "$in": [
                        "admin",
                        "moderator",
                    ],
                },
            }
        );
    }

    #[test]
    fn not_in_expression_converts_to_nin_operator() {
        let expression = comparison(
            "role",
            ComparisonOperator::Nin,
            Bson::Array(vec![
                Bson::String("admin".to_owned()),
                Bson::String("moderator".to_owned()),
            ]),
        );

        assert_eq!(
            expression.into_document(),
            doc! {
                "role": {
                    "$nin": [
                        "admin",
                        "moderator",
                    ],
                },
            }
        );
    }

    #[test]
    fn exists_expression_converts_to_exists_operator() {
        let expression = comparison("nickname", ComparisonOperator::Exists, true);

        assert_eq!(
            expression.into_document(),
            doc! {
                "nickname": {
                    "$exists": true,
                },
            }
        );
    }

    #[test]
    fn not_exists_expression_converts_to_exists_false() {
        let expression = comparison("nickname", ComparisonOperator::Exists, false);

        assert_eq!(
            expression.into_document(),
            doc! {
                "nickname": {
                    "$exists": false,
                },
            }
        );
    }

    #[test]
    fn all_expression_converts_to_all_operator() {
        let expression = Expression::comparison(
            "tags",
            ComparisonOperator::All,
            Bson::Array(vec![
                Bson::String("rust".to_owned()),
                Bson::String("mongodb".to_owned()),
            ]),
        );

        assert_eq!(
            expression.into_document(),
            doc! {
                "tags": {
                    "$all": [
                        "rust",
                        "mongodb",
                    ],
                },
            }
        );
    }

    #[test]
    fn size_expression_converts_to_size_operator() {
        let expression = Expression::comparison("tags", ComparisonOperator::Size, Bson::Int64(2));

        assert_eq!(
            expression.into_document(),
            doc! {
                "tags": {
                    "$size": Bson::Int64(2),
                },
            }
        );
    }

    #[test]
    fn not_expression_negates_field_comparison() {
        let expression =
            Expression::not(Expression::comparison("age", ComparisonOperator::Gte, 18));

        assert_eq!(
            expression.into_document(),
            doc! {
                "age": {
                    "$not": {
                        "$gte": 18,
                    },
                },
            }
        );
    }

    #[test]
    fn element_and_expression_combines_operators() {
        let expression = ElementExpression::comparison(ComparisonOperator::Gte, 80)
            & ElementExpression::comparison(ComparisonOperator::Lt, 90);

        assert_eq!(
            expression.into_document(),
            doc! {
                "$gte": 80,
                "$lt": 90,
            }
        );
    }

    #[test]
    fn push_each_builds_push_each_update_document() {
        let update = UpdateExpression::push_each("tags", ["mongodb", "backend"]);

        assert_eq!(
            update.into_document(),
            doc! {
                "$push": {
                    "tags": {
                        "$each": [
                            "mongodb",
                            "backend",
                        ],
                    },
                },
            }
        );
    }

    #[test]
    fn add_each_to_set_builds_add_to_set_each_update_document() {
        let update = UpdateExpression::add_each_to_set("tags", ["mongodb", "backend", "systems"]);

        assert_eq!(
            update.into_document(),
            doc! {
                "$addToSet": {
                    "tags": {
                        "$each": [
                            "mongodb",
                            "backend",
                            "systems",
                        ],
                    },
                },
            }
        );
    }
}
