use mongodb::bson::{Bson, Document};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortExpression {
    fields: Vec<SortField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SortField {
    name: String,
    direction: SortValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortValue {
    Ascending,
    Descending,
    TextScore,
}

impl SortExpression {
    pub(crate) fn ascending(field: impl Into<String>) -> Self {
        Self {
            fields: vec![SortField {
                name: field.into(),
                direction: SortValue::Ascending,
            }],
        }
    }

    pub(crate) fn descending(field: impl Into<String>) -> Self {
        Self {
            fields: vec![SortField {
                name: field.into(),
                direction: SortValue::Descending,
            }],
        }
    }

    pub(crate) fn text_score(field: impl Into<String>) -> Self {
        Self {
            fields: vec![SortField {
                name: field.into(),
                direction: SortValue::TextScore,
            }],
        }
    }

    pub(crate) fn extend(&mut self, other: Self) {
        self.fields.extend(other.fields);
    }

    pub(crate) fn into_document(self) -> Document {
        let mut document = Document::new();

        for field in self.fields {
            document.insert(field.name, field.direction.mongo_value());
        }

        document
    }
}

impl SortValue {
    fn mongo_value(self) -> Bson {
        match self {
            Self::Ascending => Bson::Int32(1),
            Self::Descending => Bson::Int32(-1),
            Self::TextScore => Bson::Document(mongodb::bson::doc! {
                "$meta": "textScore",
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::SortExpression;

    #[test]
    fn ascending_sort_converts_to_mongodb_document() {
        let sort = SortExpression::ascending("age");

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": 1,
            }
        );
    }

    #[test]
    fn descending_sort_converts_to_mongodb_document() {
        let sort = SortExpression::descending("age");

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": -1,
            }
        );
    }

    #[test]
    fn nested_field_sort_preserves_field_path() {
        let sort = SortExpression::ascending("address.city");

        assert_eq!(
            sort.into_document(),
            doc! {
                "address.city": 1,
            }
        );
    }

    #[test]
    fn sort_expressions_can_be_combined() {
        let mut sort = SortExpression::ascending("age");

        sort.extend(SortExpression::descending("name"));

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": 1,
                "name": -1,
            }
        );
    }

    #[test]
    fn combined_sort_preserves_insertion_order() {
        let mut sort = SortExpression::ascending("role");

        sort.extend(SortExpression::descending("age"));

        sort.extend(SortExpression::ascending("name"));

        assert_eq!(
            sort.into_document(),
            doc! {
                "role": 1,
                "age": -1,
                "name": 1,
            }
        );
    }

    #[test]
    fn text_score_sort_converts_to_mongodb_document() {
        let sort = SortExpression::text_score("_textScore");

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
    fn text_score_sort_can_be_combined_with_field_sort() {
        let mut sort = SortExpression::text_score("_textScore");

        sort.extend(SortExpression::ascending("name"));

        assert_eq!(
            sort.into_document(),
            doc! {
                "_textScore": {
                    "$meta": "textScore",
                },
                "name": 1,
            }
        );
    }
}
