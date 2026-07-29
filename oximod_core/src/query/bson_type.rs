use mongodb::bson::Bson;

/// A BSON type accepted by MongoDB's `$type` query operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BsonType {
    Double,
    String,
    Object,
    Array,
    Binary,
    ObjectId,
    Boolean,
    Date,
    Null,
    RegularExpression,
    JavaScript,
    JavaScriptWithScope,
    Int32,
    Timestamp,
    Int64,
    Decimal128,
    MinKey,
    MaxKey,
}

impl BsonType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Double => "double",
            Self::String => "string",
            Self::Object => "object",
            Self::Array => "array",
            Self::Binary => "binData",
            Self::ObjectId => "objectId",
            Self::Boolean => "bool",
            Self::Date => "date",
            Self::Null => "null",
            Self::RegularExpression => "regex",
            Self::JavaScript => "javascript",
            Self::JavaScriptWithScope => "javascriptWithScope",
            Self::Int32 => "int",
            Self::Timestamp => "timestamp",
            Self::Int64 => "long",
            Self::Decimal128 => "decimal",
            Self::MinKey => "minKey",
            Self::MaxKey => "maxKey",
        }
    }
}

impl From<BsonType> for Bson {
    fn from(value: BsonType) -> Self {
        Self::String(value.as_str().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Bson;

    use super::BsonType;

    #[test]
    fn bson_type_converts_to_mongodb_alias() {
        assert_eq!(
            Bson::from(BsonType::String),
            Bson::String("string".to_owned()),
        );

        assert_eq!(Bson::from(BsonType::Int64), Bson::String("long".to_owned()),);
    }
}
