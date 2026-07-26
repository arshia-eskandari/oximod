use mongodb::bson::Document;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortExpression {
    field: &'static str,
    direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    Ascending,
    Descending,
}

impl SortExpression {
    pub(crate) const fn ascending(field: &'static str) -> Self {
        Self {
            field,
            direction: SortDirection::Ascending,
        }
    }

    pub(crate) const fn descending(field: &'static str) -> Self {
        Self {
            field,
            direction: SortDirection::Descending,
        }
    }

    pub(crate) fn into_document(self) -> Document {
        let mut document = Document::new();

        document.insert(self.field, self.direction.mongo_value());

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
}
