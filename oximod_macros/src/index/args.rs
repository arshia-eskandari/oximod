#[derive(Default, Debug)]
/// Arguments for creating an index on a field in a MongoDB collection.
///
/// This struct is populated from the `#[index(...)]` attribute
/// and specifies the behavior of the index.
///
/// The goal of these field-level index arguments is convenience:
/// they cover the most common and compact per-field index options
/// without trying to replace MongoDB's full index API.
///
/// # Fields
///
/// - `unique`: (Optional) Whether the index enforces a unique constraint.
///   - If `true`, MongoDB will reject documents that cause duplicate values for the indexed field.
///   - Default: `false`
///
/// - `sparse`: (Optional) Whether the index skips documents that are missing the field.
///   - If `true`, documents that do not have the indexed field will not be included in the index.
///   - Default: `false`
///
/// - `name`: (Optional) The custom name for the index.
///   - Useful for identifying indexes manually.
///   - If not provided, MongoDB will generate a default name.
///
/// - `background`: (Optional) Whether the index is built in the background.
///   - If `true`, index creation does not block database operations.
///   - Default: `false`
///
/// - `order`: (Optional) The order of the index.
///   - `1` for ascending order, `-1` for descending order.
///   - Default: `1`
///   - Typically used for standard scalar indexes.
///
/// - `expire_after_secs`: (Optional) The time-to-live (TTL) for the index.
///   - If set, documents will be automatically deleted after the specified number of seconds.
///   - If not provided, documents will not automatically expire.
///
/// - `version`: (Optional) The version of the index structure to use.
///   - Applies to standard indexes.
///   - Only meaningful for certain index types; may be ignored for default scalar indexes.
///
/// - `text_index_version`: (Optional) The version of the text index structure to use.
///   - Applies only to `text` indexes.
///   - Use this to explicitly control MongoDB's text indexing behavior.
///
/// - `hidden`: (Optional) Whether the index is hidden from the query planner.
///   - If `true`, the index exists but will not be used by the query planner unless explicitly hinted.
///   - Useful for testing or safely rolling out new indexes.
///   - Default: `false`
///
/// - `text`: (Optional) Whether the field should be indexed as a text index.
///   - If `true`, the field is indexed for MongoDB text search.
///   - Default: `false`
///
/// - `hashed`: (Optional) Whether the field should be indexed as a hashed index.
///   - If `true`, MongoDB stores hashed values for the field in the index.
///   - Useful for hashed lookup patterns and some sharding strategies.
///   - Default: `false`
///
/// - `wildcard`: (Optional) Whether the field should be indexed as a wildcard index.
///   - If `true`, the field is treated as a wildcard-style indexed path.
///   - Intended for dynamic or document-like fields.
///   - Default: `false`
///
/// - `case_insensitive`: (Optional) Whether the index should behave case-insensitively.
///   - Intended as a convenience flag for applying a case-insensitive collation preset internally.
///   - Best suited for string-based lookup fields such as emails or usernames.
///   - Default: `false`
///
/// - `default_language`: (Optional) The default language used by a text index.
///   - Applies only to `text` indexes.
///   - Example values include `"english"`, `"spanish"`, etc.
///
/// - `language_override`: (Optional) The document field name that overrides the default text index language.
///   - Applies only to `text` indexes.
///   - Useful for multilingual datasets.
///
/// - `weight`: (Optional) The weight assigned to this field in a text index.
///   - Applies only to `text` indexes.
///   - Higher values increase the importance of matches on this field in text search scoring.
///
/// - `geo_2dsphere`: (Optional) Whether the field should be indexed as a 2dsphere geospatial index.
///   - If `true`, the field is indexed for spherical geospatial queries.
///   - Default: `false`
///
/// - `geo_2d`: (Optional) Whether the field should be indexed as a 2d geospatial index.
///   - If `true`, the field is indexed for planar geospatial queries.
///   - Default: `false`
///
/// - `bits`: (Optional) The precision of a `geo_2d` index.
///   - Applies only to `geo_2d` indexes.
///
/// - `min`: (Optional) The lower bound for a `geo_2d` index.
///   - Applies only to `geo_2d` indexes.
///
/// - `max`: (Optional) The upper bound for a `geo_2d` index.
///   - Applies only to `geo_2d` indexes.
///
/// - `geo_2dsphere_index_version`: (Optional) The version of the 2dsphere index structure to use.
///   - Applies only to `geo_2dsphere` indexes.
///
/// # Example
///
/// ```rust
/// #[index(unique, sparse, name = "email_idx", background, order = -1)]
/// email: String,
/// ```
///
/// ```rust
/// #[index(text, weight = 10, default_language = "english")]
/// title: String,
/// ```
///
/// ```rust
/// #[index(unique, case_insensitive)]
/// email: String,
/// ```
///
/// ```rust
/// #[index(geo_2dsphere)]
/// location: GeoJsonPoint,
/// ```
///
/// # Notes
/// - These fields are intended to be short, ergonomic, field-level conveniences.
/// - Not every MongoDB index capability belongs at the field-attribute level.
/// - More complex or document-shaped index configuration is better handled elsewhere.
///
/// - `order` is generally intended for standard ascending/descending indexes.
/// - `text`, `hashed`, `wildcard`, `geo_2dsphere`, and `geo_2d` represent specialized index types.
/// - In practice, only one specialized index type should be chosen for a field.
///
/// - `text_index_version`, `default_language`, `language_override`, and `weight` are only meaningful for `text` indexes.
/// - `bits`, `min`, and `max` are only meaningful for `geo_2d` indexes.
/// - `geo_2dsphere_index_version` is only meaningful for `geo_2dsphere` indexes.
///
/// - `case_insensitive` is intended as a convenience feature and may be implemented internally
///   using a predefined collation strategy.
/// - Some combinations should be validated by the parser or macro layer to prevent invalid index definitions.
///
pub struct IndexArgs {
    pub unique: Option<bool>,
    pub sparse: Option<bool>,
    pub name: Option<String>,
    pub background: Option<bool>,
    pub order: Option<i32>,
    pub expire_after_secs: Option<i32>,
    pub version: Option<u32>,
    pub text_index_version: Option<u32>,
    pub hidden: Option<bool>,

    pub text: Option<bool>,
    pub hashed: Option<bool>,
    pub wildcard: Option<bool>,
    pub case_insensitive: Option<bool>,

    pub default_language: Option<String>,
    pub language_override: Option<String>,
    pub weight: Option<u32>,

    pub geo_2dsphere: Option<bool>,
    pub geo_2d: Option<bool>,

    pub bits: Option<u32>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub geo_2dsphere_index_version: Option<u32>,
}
