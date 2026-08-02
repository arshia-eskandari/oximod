//! BSON type aliases used by typed `$type` queries.
//!
//! MongoDB's `$type` query operator accepts numeric BSON type codes or string
//! aliases. OxiMod exposes the string aliases through [`BsonType`] so typed
//! queries remain readable and avoid raw string literals.

use mongodb::bson::Bson;

/// A BSON type accepted by MongoDB's `$type` query operator.
///
/// Each variant maps to MongoDB's canonical string alias rather than its
/// numeric BSON type code.
///
/// # Example
///
/// ```ignore
/// use oximod::BsonType;
///
/// let users = User::query()
///     .filter(|user| {
///         user.nickname
///             .has_bson_type(BsonType::String)
///     })
///     .all()
///     .await?;
/// ```
///
/// This produces a filter equivalent to:
///
/// ```text
/// {
///     "nickname": {
///         "$type": "string"
///     }
/// }
/// ```
///
/// `$type` inspects the BSON representation stored in MongoDB. It does not
/// convert Rust values or deserialize documents before matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BsonType {
    /// A 64-bit IEEE 754 floating-point value.
    ///
    /// MongoDB alias: `"double"`.
    Double,

    /// A UTF-8 string.
    ///
    /// MongoDB alias: `"string"`.
    String,

    /// A BSON document.
    ///
    /// MongoDB alias: `"object"`.
    Object,

    /// A BSON array.
    ///
    /// MongoDB alias: `"array"`.
    Array,

    /// Binary data.
    ///
    /// MongoDB alias: `"binData"`.
    Binary,

    /// A MongoDB object identifier.
    ///
    /// MongoDB alias: `"objectId"`.
    ObjectId,

    /// A Boolean value.
    ///
    /// MongoDB alias: `"bool"`.
    Boolean,

    /// A BSON date-time value.
    ///
    /// MongoDB alias: `"date"`.
    Date,

    /// A BSON null value.
    ///
    /// MongoDB alias: `"null"`.
    Null,

    /// A BSON regular expression.
    ///
    /// MongoDB alias: `"regex"`.
    RegularExpression,

    /// JavaScript code without an associated scope document.
    ///
    /// MongoDB alias: `"javascript"`.
    JavaScript,

    /// JavaScript code with an associated scope document.
    ///
    /// MongoDB alias: `"javascriptWithScope"`.
    JavaScriptWithScope,

    /// A signed 32-bit integer.
    ///
    /// MongoDB alias: `"int"`.
    Int32,

    /// A BSON timestamp.
    ///
    /// MongoDB alias: `"timestamp"`.
    Timestamp,

    /// A signed 64-bit integer.
    ///
    /// MongoDB alias: `"long"`.
    Int64,

    /// A 128-bit decimal floating-point value.
    ///
    /// MongoDB alias: `"decimal"`.
    Decimal128,

    /// The BSON minimum-key sentinel value.
    ///
    /// MongoDB alias: `"minKey"`.
    MinKey,

    /// The BSON maximum-key sentinel value.
    ///
    /// MongoDB alias: `"maxKey"`.
    MaxKey,
}

impl BsonType {
    /// Returns MongoDB's canonical string alias for this BSON type.
    #[must_use]
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
    use super::BsonType;
    use mongodb::bson::Bson;

    #[test]
    fn bson_types_use_mongodb_string_aliases() {
        let cases = [
            (BsonType::Double, "double"),
            (BsonType::String, "string"),
            (BsonType::Object, "object"),
            (BsonType::Array, "array"),
            (BsonType::Binary, "binData"),
            (BsonType::ObjectId, "objectId"),
            (BsonType::Boolean, "bool"),
            (BsonType::Date, "date"),
            (BsonType::Null, "null"),
            (BsonType::RegularExpression, "regex"),
            (BsonType::JavaScript, "javascript"),
            (BsonType::JavaScriptWithScope, "javascriptWithScope"),
            (BsonType::Int32, "int"),
            (BsonType::Timestamp, "timestamp"),
            (BsonType::Int64, "long"),
            (BsonType::Decimal128, "decimal"),
            (BsonType::MinKey, "minKey"),
            (BsonType::MaxKey, "maxKey"),
        ];

        for (bson_type, expected_alias) in cases {
            assert_eq!(bson_type.as_str(), expected_alias);
            assert_eq!(
                Bson::from(bson_type),
                Bson::String(expected_alias.to_owned()),
            );
        }
    }
}
