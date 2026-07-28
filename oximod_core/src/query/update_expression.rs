use mongodb::bson::{Bson, Document};

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
}
