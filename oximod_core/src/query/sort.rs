use mongodb::bson::Document;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortExpression {
    fields: Vec<SortField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SortField {
    name: &'static str,
    direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    Ascending,
    Descending,
}

impl SortExpression {
    pub(crate) fn ascending(field: &'static str) -> Self {
        Self {
            fields: vec![SortField {
                name: field,
                direction: SortDirection::Ascending,
            }],
        }
    }

    pub(crate) fn descending(field: &'static str) -> Self {
        Self {
            fields: vec![SortField {
                name: field,
                direction: SortDirection::Descending,
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

impl SortDirection {
    const fn mongo_value(self) -> i32 {
        match self {
            Self::Ascending => 1,
            Self::Descending => -1,
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
}
