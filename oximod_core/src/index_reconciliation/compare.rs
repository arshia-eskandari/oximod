//! Pure declared-vs-actual index comparison.
//!
//! MongoDB's `listIndexes` output is not byte-shaped like the request that
//! created an index: numeric key values may use a different BSON numeric
//! type, server-defaulted options are materialized, boolean options are
//! omitted when false, text indexes are listed with internal `_fts`/`_ftsx`
//! keys, and collations come back with defaulted fields filled in. This
//! module therefore compares semantics, not serialized `IndexModel`s.
//!
//! Everything here is deterministic and side-effect free so the entire
//! comparison contract can be unit-tested without a server.

use std::collections::BTreeMap;
use std::time::Duration;

use mongodb::{
    IndexModel,
    bson::{Bson, Document},
    options::{Collation, IndexVersion, Sphere2DIndexVersion, TextIndexVersion},
};

use super::{DeclaredIndexStatus, IndexDriftReport, IndexMismatchCandidate};

/// MongoDB's default text-index field weight.
const DEFAULT_TEXT_WEIGHT: f64 = 1.0;

/// MongoDB's default text-index language.
const DEFAULT_TEXT_LANGUAGE: &str = "english";

/// MongoDB's default text-index language-override field name.
const DEFAULT_TEXT_LANGUAGE_OVERRIDE: &str = "language";

/// Builds the drift report for one model.
///
/// `actual` is `None` when the collection does not exist: every declared
/// index is then [`Missing`](DeclaredIndexStatus::Missing) and `unmanaged`
/// is empty, so a model with zero declarations is in sync.
pub(super) fn build_drift_report(
    declared: &[IndexModel],
    actual: Option<&[IndexModel]>,
    collection_default_collation: Option<&Collation>,
) -> IndexDriftReport {
    let actual_indexes = actual.unwrap_or(&[]);
    let mut referenced = vec![false; actual_indexes.len()];

    let declared_statuses = declared
        .iter()
        .map(|expected| {
            classify_declaration(
                expected,
                actual_indexes,
                collection_default_collation,
                &mut referenced,
            )
        })
        .collect();

    let mut unmanaged: Vec<IndexModel> = actual_indexes
        .iter()
        .enumerate()
        .filter(|(position, index)| !referenced[*position] && actual_name(index) != Some("_id_"))
        .map(|(_, index)| index.clone())
        .collect();

    unmanaged.sort_by_key(deterministic_sort_key);

    IndexDriftReport {
        declared: declared_statuses,
        unmanaged,
    }
}

/// Classifies one declaration against the server's indexes, marking every
/// server index it references so unmanaged filtering can exclude them.
fn classify_declaration(
    expected: &IndexModel,
    actual_indexes: &[IndexModel],
    collection_default_collation: Option<&Collation>,
    referenced: &mut [bool],
) -> DeclaredIndexStatus {
    let expected_name = effective_expected_name(expected);

    // Step 1: an actual index under the declaration's effective name is the
    // primary candidate even when its keys or options differ, so name
    // collisions surface as Mismatched rather than Missing.
    let mut candidate_positions: Vec<usize> = actual_indexes
        .iter()
        .enumerate()
        .filter(|(_, index)| actual_name(index) == Some(expected_name.as_str()))
        .map(|(position, _)| position)
        .collect();

    // Step 2: without a name candidate, fall back to the same logical key
    // family. Any actual text index conflicts with a text declaration
    // because MongoDB allows at most one text index per collection.
    if candidate_positions.is_empty() {
        candidate_positions = actual_indexes
            .iter()
            .enumerate()
            .filter(|(_, index)| {
                if is_text_declaration(expected) {
                    is_text_listing(index)
                } else {
                    !is_text_listing(index) && keys_equivalent(&expected.keys, &index.keys)
                }
            })
            .map(|(position, _)| position)
            .collect();
    }

    if candidate_positions.is_empty() {
        return DeclaredIndexStatus::Missing {
            expected: expected.clone(),
        };
    }

    for &position in &candidate_positions {
        if compute_differences(
            expected,
            &actual_indexes[position],
            collection_default_collation,
        )
        .is_empty()
        {
            referenced[position] = true;

            return DeclaredIndexStatus::InSync {
                expected: expected.clone(),
                actual: actual_indexes[position].clone(),
            };
        }
    }

    let mut candidates: Vec<IndexMismatchCandidate> = candidate_positions
        .into_iter()
        .map(|position| {
            referenced[position] = true;

            IndexMismatchCandidate {
                actual: actual_indexes[position].clone(),
                differences: compute_differences(
                    expected,
                    &actual_indexes[position],
                    collection_default_collation,
                ),
            }
        })
        .collect();

    candidates.sort_by_key(|candidate| deterministic_sort_key(&candidate.actual));

    DeclaredIndexStatus::Mismatched {
        expected: expected.clone(),
        candidates,
    }
}

/// Computes the semantic differences between a declaration and one server
/// index, in a fixed property order.
///
/// An empty result means the server index satisfies the declaration.
fn compute_differences(
    expected: &IndexModel,
    actual: &IndexModel,
    collection_default_collation: Option<&Collation>,
) -> Vec<String> {
    let mut differences = Vec::new();

    let expected_text = is_text_declaration(expected);
    let actual_text = is_text_listing(actual);
    let expected_options = expected.options.as_ref();
    let actual_options = actual.options.as_ref();

    // Keys. Text listings use internal `_fts`/`_ftsx` keys, so text-vs-text
    // compares the logical indexed fields reconstructed from `weights`.
    if expected_text && actual_text {
        let expected_weights = expected_text_weights(expected);
        let actual_weights = actual_text_weights(actual);

        if expected_weights != actual_weights {
            differences.push(format!(
                "text fields: expected {}, actual {}",
                weights_repr(&expected_weights),
                weights_repr(&actual_weights)
            ));
        }
    } else if expected_text != actual_text || !keys_equivalent(&expected.keys, &actual.keys) {
        differences.push(format!(
            "keys: expected {}, actual {}",
            keys_repr(&expected.keys),
            keys_repr(&actual.keys)
        ));
    }

    // Name. OxiMod's create lifecycle would conflict on a different name
    // (IndexOptionsConflict), so a same-spec index under another name is
    // still a difference.
    let expected_name = effective_expected_name(expected);

    if actual_name(actual) != Some(expected_name.as_str()) {
        let actual_display = match actual_name(actual) {
            Some(name) => format!("`{name}`"),
            None => "unnamed".to_string(),
        };

        differences.push(format!(
            "name: expected `{expected_name}`, actual {actual_display}"
        ));
    }

    // Boolean options normalize absence to false.
    let expected_unique = expected_options
        .and_then(|options| options.unique)
        .unwrap_or(false);
    let actual_unique = actual_options
        .and_then(|options| options.unique)
        .unwrap_or(false);

    if expected_unique != actual_unique {
        differences.push(format!(
            "unique: expected {expected_unique}, actual {actual_unique}"
        ));
    }

    // Sparse is not a drift dimension for intrinsically sparse index types:
    // the flag's presence or absence never changes their semantics.
    if !is_intrinsically_sparse(expected) && !is_intrinsically_sparse(actual) {
        let expected_sparse = expected_options
            .and_then(|options| options.sparse)
            .unwrap_or(false);
        let actual_sparse = actual_options
            .and_then(|options| options.sparse)
            .unwrap_or(false);

        if expected_sparse != actual_sparse {
            differences.push(format!(
                "sparse: expected {expected_sparse}, actual {actual_sparse}"
            ));
        }
    }

    let expected_hidden = expected_options
        .and_then(|options| options.hidden)
        .unwrap_or(false);
    let actual_hidden = actual_options
        .and_then(|options| options.hidden)
        .unwrap_or(false);

    if expected_hidden != actual_hidden {
        differences.push(format!(
            "hidden: expected {expected_hidden}, actual {actual_hidden}"
        ));
    }

    // TTL: absence means non-TTL; presence must match to the second.
    let expected_ttl = expected_options.and_then(|options| options.expire_after);
    let actual_ttl = actual_options.and_then(|options| options.expire_after);

    if expected_ttl != actual_ttl {
        differences.push(format!(
            "expire_after_secs: expected {}, actual {}",
            ttl_repr(expected_ttl),
            ttl_repr(actual_ttl)
        ));
    }

    // Collation compares effective values: a declaration without an
    // index-level collation inherits the collection default (except for
    // text/2d indexes, which only support simple binary comparison), and
    // server-defaulted collation fields are normalized.
    let expected_collation = effective_expected_collation(expected, collection_default_collation);
    let actual_collation = actual_options
        .and_then(|options| options.collation.as_ref())
        .and_then(normalize_collation);

    if expected_collation != actual_collation {
        differences.push(format!(
            "collation: expected {}, actual {}",
            collation_repr(expected_collation.as_ref()),
            collation_repr(actual_collation.as_ref())
        ));
    }

    // Explicitly declared versions and 2d parameters compare semantically;
    // omission delegates to the server default and is never drift.
    if let Some(expected_version) = expected_options.and_then(|options| options.version.as_ref()) {
        let expected_value = index_version_value(expected_version);
        let actual_value = actual_options
            .and_then(|options| options.version.as_ref())
            .map(index_version_value);

        if actual_value != Some(expected_value) {
            differences.push(format!(
                "version: expected {expected_value}, actual {}",
                optional_number_repr(actual_value)
            ));
        }
    }

    if expected_text
        && actual_text
        && let Some(expected_version) =
            expected_options.and_then(|options| options.text_index_version.as_ref())
    {
        let expected_value = text_index_version_value(expected_version);
        let actual_value = actual_options
            .and_then(|options| options.text_index_version.as_ref())
            .map(text_index_version_value);

        if actual_value != Some(expected_value) {
            differences.push(format!(
                "text_index_version: expected {expected_value}, actual {}",
                optional_number_repr(actual_value)
            ));
        }
    }

    if let Some(expected_version) =
        expected_options.and_then(|options| options.sphere_2d_index_version.as_ref())
    {
        let expected_value = sphere_2d_index_version_value(expected_version);
        let actual_value = actual_options
            .and_then(|options| options.sphere_2d_index_version.as_ref())
            .map(sphere_2d_index_version_value);

        if actual_value != Some(expected_value) {
            differences.push(format!(
                "sphere_2d_index_version: expected {expected_value}, actual {}",
                optional_number_repr(actual_value)
            ));
        }
    }

    if let Some(expected_bits) = expected_options.and_then(|options| options.bits) {
        let actual_bits = actual_options.and_then(|options| options.bits);

        if actual_bits != Some(expected_bits) {
            differences.push(format!(
                "bits: expected {expected_bits}, actual {}",
                optional_number_repr(actual_bits)
            ));
        }
    }

    if let Some(expected_min) = expected_options.and_then(|options| options.min) {
        let actual_min = actual_options.and_then(|options| options.min);

        if actual_min != Some(expected_min) {
            differences.push(format!(
                "min: expected {expected_min}, actual {}",
                optional_number_repr(actual_min)
            ));
        }
    }

    if let Some(expected_max) = expected_options.and_then(|options| options.max) {
        let actual_max = actual_options.and_then(|options| options.max);

        if actual_max != Some(expected_max) {
            differences.push(format!(
                "max: expected {expected_max}, actual {}",
                optional_number_repr(actual_max)
            ));
        }
    }

    // Text language settings compare effective values against MongoDB's
    // documented defaults.
    if expected_text && actual_text {
        let expected_language = expected_options
            .and_then(|options| options.default_language.as_deref())
            .unwrap_or(DEFAULT_TEXT_LANGUAGE);
        let actual_language = actual_options
            .and_then(|options| options.default_language.as_deref())
            .unwrap_or(DEFAULT_TEXT_LANGUAGE);

        if expected_language != actual_language {
            differences.push(format!(
                "default_language: expected {expected_language}, actual {actual_language}"
            ));
        }

        let expected_override = expected_options
            .and_then(|options| options.language_override.as_deref())
            .unwrap_or(DEFAULT_TEXT_LANGUAGE_OVERRIDE);
        let actual_override = actual_options
            .and_then(|options| options.language_override.as_deref())
            .unwrap_or(DEFAULT_TEXT_LANGUAGE_OVERRIDE);

        if expected_override != actual_override {
            differences.push(format!(
                "language_override: expected {expected_override}, actual {actual_override}"
            ));
        }
    }

    // Semantic options OxiMod cannot declare: their presence on the server
    // index changes what the index means, so the candidate cannot count as
    // in sync. They are reported, never repaired.
    if let Some(filter) =
        actual_options.and_then(|options| options.partial_filter_expression.as_ref())
    {
        differences.push(format!(
            "partial_filter_expression: expected none, actual {filter}"
        ));
    }

    if let Some(projection) =
        actual_options.and_then(|options| options.wildcard_projection.as_ref())
    {
        differences.push(format!(
            "wildcard_projection: expected none, actual {projection}"
        ));
    }

    if let Some(storage_engine) = actual_options.and_then(|options| options.storage_engine.as_ref())
    {
        differences.push(format!(
            "storage_engine: expected none, actual {storage_engine}"
        ));
    }

    if let Some(bucket_size) = actual_options.and_then(|options| options.bucket_size) {
        differences.push(format!("bucket_size: expected none, actual {bucket_size}"));
    }

    // `background` is deliberately not compared: MongoDB 4.2+ ignores the
    // option during creation, so it is not a durable semantic property.

    differences
}

/// Returns the name a declaration is expected to have on the server: the
/// explicit name, or MongoDB's default name derived from the requested key
/// pattern (`score_1`, `content_text`, `location_2dsphere`, ...).
pub(super) fn effective_expected_name(expected: &IndexModel) -> String {
    expected
        .options
        .as_ref()
        .and_then(|options| options.name.clone())
        .unwrap_or_else(|| default_index_name(&expected.keys))
}

/// Derives MongoDB's default index name from a requested key pattern.
///
/// Matches the server's construction (and the driver's own `update_name`):
/// key names and values joined with `_`, string index types verbatim, other
/// values in their canonical BSON display form.
fn default_index_name(keys: &Document) -> String {
    keys.iter()
        .map(|(field, value)| match value {
            Bson::String(kind) => format!("{field}_{kind}"),
            other => format!("{field}_{other}"),
        })
        .collect::<Vec<_>>()
        .join("_")
}

/// Returns the server-reported name of an actual index, if present.
fn actual_name(index: &IndexModel) -> Option<&str> {
    index
        .options
        .as_ref()
        .and_then(|options| options.name.as_deref())
}

/// Returns whether a declaration requests a text index.
fn is_text_declaration(expected: &IndexModel) -> bool {
    expected
        .keys
        .iter()
        .any(|(_, value)| matches!(value, Bson::String(kind) if kind == "text"))
}

/// Returns whether a server index listing is a text index, recognized by
/// MongoDB's internal `_fts: "text"` key shape.
fn is_text_listing(actual: &IndexModel) -> bool {
    matches!(actual.keys.get("_fts"), Some(Bson::String(kind)) if kind == "text")
}

/// Returns whether an index type is intrinsically sparse, making the
/// `sparse` option meaningless as a drift dimension.
///
/// MongoDB documents text, 2d, and 2dsphere (version 2 and later) indexes
/// as always sparse. Verified against MongoDB 8.0: the server persists an
/// explicitly requested `sparse: true` verbatim in `listIndexes` for these
/// types while ignoring it behaviorally, so the flag never distinguishes
/// two semantically different indexes of these types. The key shape
/// identifies the type for declarations and listings alike; text listings
/// carry the internal `_fts: "text"` value.
///
/// Wildcard indexes are deliberately NOT in this set even though they are
/// also inherently sparse: MongoDB 8.0 rejects the `sparse` option for them
/// outright (code 67, CannotCreateIndex), so a `sparse` wildcard
/// declaration cannot actually be created. Treating an existing flagless
/// wildcard index as satisfying such a declaration would hide that OxiMod
/// could never recreate the declared desired state; the sparse difference
/// therefore stays visible and the declaration reports `Mismatched`.
fn is_intrinsically_sparse(index: &IndexModel) -> bool {
    index.keys.iter().any(|(_, value)| {
        matches!(
            value,
            Bson::String(kind) if kind == "text" || kind == "2d" || kind == "2dsphere"
        )
    })
}

/// Reconstructs the effective indexed fields and weights of a text
/// declaration: every requested text key field defaults to weight 1, and a
/// declared weights document overrides or extends that set, matching
/// MongoDB's semantics.
fn expected_text_weights(expected: &IndexModel) -> BTreeMap<String, f64> {
    let mut weights = BTreeMap::new();

    for (field, value) in expected.keys.iter() {
        if matches!(value, Bson::String(kind) if kind == "text") {
            weights.insert(field.clone(), DEFAULT_TEXT_WEIGHT);
        }
    }

    if let Some(declared) = expected
        .options
        .as_ref()
        .and_then(|options| options.weights.as_ref())
    {
        for (field, value) in declared.iter() {
            if let Some(weight) = numeric_value(value) {
                weights.insert(field.clone(), weight);
            }
        }
    }

    weights
}

/// Reconstructs the indexed fields and weights of an actual text index from
/// its `weights` document, the authoritative source: the listed keys only
/// contain the internal `_fts`/`_ftsx` markers.
fn actual_text_weights(actual: &IndexModel) -> BTreeMap<String, f64> {
    actual
        .options
        .as_ref()
        .and_then(|options| options.weights.as_ref())
        .map(|weights| {
            weights
                .iter()
                .filter_map(|(field, value)| {
                    numeric_value(value).map(|weight| (field.clone(), weight))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Returns whether two requested/listed key patterns are logically
/// equivalent: same fields in the same order with numerically equivalent
/// directions or identical index-type strings.
fn keys_equivalent(expected: &Document, actual: &Document) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual.iter()).all(
            |((expected_field, expected_value), (actual_field, actual_value))| {
                expected_field == actual_field
                    && index_key_values_equivalent(expected_value, actual_value)
            },
        )
}

/// Compares two index key values semantically: numeric values by numeric
/// value regardless of BSON storage type, everything else by BSON equality.
fn index_key_values_equivalent(expected: &Bson, actual: &Bson) -> bool {
    match (numeric_value(expected), numeric_value(actual)) {
        (Some(expected_number), Some(actual_number)) => expected_number == actual_number,
        _ => expected == actual,
    }
}

/// Returns a BSON value's numeric value, when it is numeric.
fn numeric_value(value: &Bson) -> Option<f64> {
    match value {
        Bson::Int32(number) => Some(f64::from(*number)),
        Bson::Int64(number) => Some(*number as f64),
        Bson::Double(number) => Some(*number),
        _ => None,
    }
}

/// A collation normalized to its effective comparison semantics.
///
/// `None` at the usage sites means simple binary collation. MongoDB omits
/// creation-time defaults or materializes them depending on context, so
/// comparing `Collation` structs by field presence would report false
/// drift; this fills MongoDB's documented defaults instead.
#[derive(Debug, Clone, PartialEq)]
struct EffectiveCollation {
    locale: String,
    strength: u32,
    case_level: bool,
    case_first: String,
    numeric_ordering: bool,
    alternate: String,
    max_variable: String,
    normalization: bool,
    backwards: bool,
}

/// Normalizes a collation, mapping the explicit `simple` locale to `None`.
fn normalize_collation(collation: &Collation) -> Option<EffectiveCollation> {
    if collation.locale == "simple" {
        return None;
    }

    Some(EffectiveCollation {
        locale: collation.locale.clone(),
        strength: collation.strength.map(u32::from).unwrap_or(3),
        case_level: collation.case_level.unwrap_or(false),
        case_first: collation
            .case_first
            .map(|case_first| case_first.as_str().to_string())
            .unwrap_or_else(|| "off".to_string()),
        numeric_ordering: collation.numeric_ordering.unwrap_or(false),
        alternate: collation
            .alternate
            .map(|alternate| alternate.as_str().to_string())
            .unwrap_or_else(|| "non-ignorable".to_string()),
        max_variable: collation
            .max_variable
            .map(|max_variable| max_variable.as_str().to_string())
            .unwrap_or_else(|| "punct".to_string()),
        normalization: collation.normalization.unwrap_or(false),
        backwards: collation.backwards.unwrap_or(false),
    })
}

/// Computes the collation a declaration is expected to have on the server.
///
/// An index-level collation wins. Otherwise the collection's default
/// collation applies — except for text and 2d indexes, which only support
/// simple binary comparison. Without either, the effective collation is
/// simple.
fn effective_expected_collation(
    expected: &IndexModel,
    collection_default: Option<&Collation>,
) -> Option<EffectiveCollation> {
    if let Some(collation) = expected
        .options
        .as_ref()
        .and_then(|options| options.collation.as_ref())
    {
        return normalize_collation(collation);
    }

    if declaration_supports_only_simple_collation(expected) {
        return None;
    }

    collection_default.and_then(normalize_collation)
}

/// Returns whether the declared index type only supports simple binary
/// comparison (text and 2d indexes).
fn declaration_supports_only_simple_collation(expected: &IndexModel) -> bool {
    expected
        .keys
        .iter()
        .any(|(_, value)| matches!(value, Bson::String(kind) if kind == "text" || kind == "2d"))
}

/// Maps an [`IndexVersion`] to its numeric wire value.
fn index_version_value(version: &IndexVersion) -> u32 {
    match version {
        #[allow(deprecated)]
        IndexVersion::V0 => 0,
        IndexVersion::V1 => 1,
        IndexVersion::V2 => 2,
        IndexVersion::Custom(value) => *value,
        // The driver enum is non-exhaustive; conservatively map unknown
        // future variants to a value no OxiMod declaration produces so they
        // compare as a mismatch rather than a false match.
        _ => u32::MAX,
    }
}

/// Maps a [`TextIndexVersion`] to its numeric wire value.
fn text_index_version_value(version: &TextIndexVersion) -> u32 {
    match version {
        TextIndexVersion::V1 => 1,
        TextIndexVersion::V2 => 2,
        TextIndexVersion::V3 => 3,
        TextIndexVersion::Custom(value) => *value,
    }
}

/// Maps a [`Sphere2DIndexVersion`] to its numeric wire value.
fn sphere_2d_index_version_value(version: &Sphere2DIndexVersion) -> u32 {
    match version {
        Sphere2DIndexVersion::V2 => 2,
        Sphere2DIndexVersion::V3 => 3,
        Sphere2DIndexVersion::Custom(value) => *value,
    }
}

/// Deterministic ordering key for candidate and unmanaged listings: index
/// name first, then the key representation.
fn deterministic_sort_key(index: &IndexModel) -> (String, String) {
    (
        actual_name(index).unwrap_or_default().to_string(),
        keys_repr(&index.keys),
    )
}

/// Renders a key pattern deterministically, preserving key order.
fn keys_repr(keys: &Document) -> String {
    let entries = keys
        .iter()
        .map(|(field, value)| format!("{field}: {value}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("{{ {entries} }}")
}

/// Renders a logical text field/weight set deterministically.
fn weights_repr(weights: &BTreeMap<String, f64>) -> String {
    let entries = weights
        .iter()
        .map(|(field, weight)| format!("{field}: {weight}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("{{ {entries} }}")
}

/// Renders a TTL duration for difference output.
fn ttl_repr(ttl: Option<Duration>) -> String {
    match ttl {
        Some(duration) => duration.as_secs().to_string(),
        None => "none".to_string(),
    }
}

/// Renders an optional numeric option for difference output.
fn optional_number_repr<T: std::fmt::Display>(value: Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "server default".to_string(),
    }
}

/// Renders an effective collation for difference output.
fn collation_repr(collation: Option<&EffectiveCollation>) -> String {
    match collation {
        Some(collation) => format!(
            "{{ locale: {}, strength: {}, case_level: {}, case_first: {}, \
             numeric_ordering: {}, alternate: {}, max_variable: {}, \
             normalization: {}, backwards: {} }}",
            collation.locale,
            collation.strength,
            collation.case_level,
            collation.case_first,
            collation.numeric_ordering,
            collation.alternate,
            collation.max_variable,
            collation.normalization,
            collation.backwards
        ),
        None => "simple".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::doc;
    use mongodb::options::{
        CollationAlternate, CollationCaseFirst, CollationMaxVariable, CollationStrength,
        IndexOptions,
    };

    /// Builds a declaration the way OxiMod's generated code does: a key
    /// pattern plus a fully-defaulted options struct with overrides.
    fn declaration(keys: Document, configure: impl FnOnce(&mut IndexOptions)) -> IndexModel {
        let mut options = IndexOptions::default();
        configure(&mut options);

        IndexModel::builder().keys(keys).options(options).build()
    }

    /// Builds a server-side index listing: name and `v: 2` are always
    /// materialized the way `listIndexes` reports them.
    fn listing(
        name: &str,
        keys: Document,
        configure: impl FnOnce(&mut IndexOptions),
    ) -> IndexModel {
        let mut options = IndexOptions::default();
        options.name = Some(name.to_string());
        options.version = Some(IndexVersion::V2);
        configure(&mut options);

        IndexModel::builder().keys(keys).options(options).build()
    }

    /// The `_id_` index as MongoDB reports it.
    fn id_listing() -> IndexModel {
        listing("_id_", doc! { "_id": 1 }, |_| {})
    }

    /// A server-shaped text listing with internal `_fts`/`_ftsx` keys.
    fn text_listing(
        name: &str,
        weights: Document,
        configure: impl FnOnce(&mut IndexOptions),
    ) -> IndexModel {
        listing(name, doc! { "_fts": "text", "_ftsx": 1 }, |options| {
            options.weights = Some(weights);
            options.default_language = Some("english".to_string());
            options.language_override = Some("language".to_string());
            options.text_index_version = Some(TextIndexVersion::V3);
            configure(options);
        })
    }

    /// A fully server-materialized `en` collation as MongoDB returns it.
    fn server_en_collation(strength: CollationStrength) -> Collation {
        let mut collation = Collation::builder().locale("en".to_string()).build();
        collation.strength = Some(strength);
        collation.case_level = Some(false);
        collation.case_first = Some(CollationCaseFirst::Off);
        collation.numeric_ordering = Some(false);
        collation.alternate = Some(CollationAlternate::NonIgnorable);
        collation.max_variable = Some(CollationMaxVariable::Punct);
        collation.normalization = Some(false);
        collation.backwards = Some(false);
        collation
    }

    fn report(
        declared: &[IndexModel],
        actual: Option<&[IndexModel]>,
        default_collation: Option<&Collation>,
    ) -> IndexDriftReport {
        build_drift_report(declared, actual, default_collation)
    }

    fn single_status(report: &IndexDriftReport) -> &DeclaredIndexStatus {
        assert_eq!(report.declared().len(), 1, "expected one declared status");
        &report.declared()[0]
    }

    fn assert_in_sync(report: &IndexDriftReport) {
        let status = single_status(report);
        assert!(
            matches!(status, DeclaredIndexStatus::InSync { .. }),
            "expected InSync, got {status:?}"
        );
    }

    fn assert_missing(report: &IndexDriftReport) {
        let status = single_status(report);
        assert!(
            matches!(status, DeclaredIndexStatus::Missing { .. }),
            "expected Missing, got {status:?}"
        );
    }

    fn mismatch_candidates(report: &IndexDriftReport) -> &[IndexMismatchCandidate] {
        match single_status(report) {
            DeclaredIndexStatus::Mismatched { candidates, .. } => candidates,
            other => panic!("expected Mismatched, got {other:?}"),
        }
    }

    // ----- numeric key equivalence ---------------------------------------

    #[test]
    fn numeric_key_values_compare_by_semantic_value() {
        let equivalent = [
            (Bson::Int32(1), Bson::Int64(1)),
            (Bson::Int32(1), Bson::Double(1.0)),
            (Bson::Int64(-1), Bson::Double(-1.0)),
            (
                Bson::String("hashed".to_string()),
                Bson::String("hashed".to_string()),
            ),
            (
                Bson::String("text".to_string()),
                Bson::String("text".to_string()),
            ),
            (
                Bson::String("2dsphere".to_string()),
                Bson::String("2dsphere".to_string()),
            ),
        ];

        for (expected, actual) in equivalent {
            assert!(
                index_key_values_equivalent(&expected, &actual),
                "{expected:?} should equal {actual:?}"
            );
        }

        let different = [
            (Bson::Int32(1), Bson::Int32(-1)),
            (Bson::Int32(1), Bson::Double(1.5)),
            (Bson::Int32(1), Bson::String("hashed".to_string())),
            (
                Bson::String("2d".to_string()),
                Bson::String("2dsphere".to_string()),
            ),
        ];

        for (expected, actual) in different {
            assert!(
                !index_key_values_equivalent(&expected, &actual),
                "{expected:?} should differ from {actual:?}"
            );
        }
    }

    #[test]
    fn key_patterns_preserve_field_order_and_length() {
        assert!(keys_equivalent(
            &doc! { "a": 1, "b": -1 },
            &doc! { "a": Bson::Double(1.0), "b": Bson::Int64(-1) },
        ));
        assert!(!keys_equivalent(
            &doc! { "a": 1, "b": -1 },
            &doc! { "b": -1, "a": 1 },
        ));
        assert!(!keys_equivalent(&doc! { "a": 1 }, &doc! { "a": 1, "b": 1 }));
    }

    // ----- effective default names ---------------------------------------

    #[test]
    fn default_names_match_mongodb_construction_for_every_key_family() {
        let cases = [
            (doc! { "score": 1 }, "score_1"),
            (doc! { "score": -1 }, "score_-1"),
            (doc! { "content": "text" }, "content_text"),
            (doc! { "user_id": "hashed" }, "user_id_hashed"),
            (doc! { "attributes.$**": 1 }, "attributes.$**_1"),
            (doc! { "location": "2dsphere" }, "location_2dsphere"),
            (doc! { "position": "2d" }, "position_2d"),
            (doc! { "a": 1, "b": -1 }, "a_1_b_-1"),
        ];

        for (keys, expected_name) in cases {
            let expected = declaration(keys, |_| {});
            assert_eq!(effective_expected_name(&expected), expected_name);
        }
    }

    #[test]
    fn explicit_names_override_the_default_name() {
        let expected = declaration(doc! { "score": 1 }, |options| {
            options.name = Some("score_idx".to_string());
        });

        assert_eq!(effective_expected_name(&expected), "score_idx");
    }

    // ----- basic classification ------------------------------------------

    #[test]
    fn matching_scalar_index_is_in_sync() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [
            id_listing(),
            listing("score_1", doc! { "score": 1 }, |_| {}),
        ];

        let drift = report(&[expected], Some(&actual), None);

        assert_in_sync(&drift);
        assert!(drift.is_in_sync());
        assert!(!drift.has_drift());
        assert!(drift.unmanaged().is_empty());
    }

    #[test]
    fn absent_collection_reports_every_declaration_missing() {
        let expected = declaration(doc! { "score": 1 }, |_| {});

        let drift = report(&[expected], None, None);

        assert_missing(&drift);
        assert_eq!(drift.missing_count(), 1);
        assert!(drift.unmanaged().is_empty());
        assert!(drift.has_drift());
    }

    #[test]
    fn absent_collection_with_no_declarations_is_in_sync() {
        let drift = report(&[], None, None);

        assert!(drift.is_in_sync());
        assert!(!drift.has_drift());
        assert_eq!(drift.missing_count(), 0);
        assert_eq!(drift.mismatched_count(), 0);
    }

    #[test]
    fn dropped_declared_index_reports_missing() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [id_listing()];

        let drift = report(&[expected], Some(&actual), None);

        assert_missing(&drift);
    }

    // ----- adversarial name/key candidate selection ----------------------

    #[test]
    fn expected_name_with_wrong_key_is_mismatched_not_missing() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [listing("score_1", doc! { "level": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        let candidates = mismatch_candidates(&drift);
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0]
                .differences()
                .iter()
                .any(|difference| difference.starts_with("keys:")),
            "expected a keys difference: {:?}",
            candidates[0].differences()
        );
        assert_eq!(drift.mismatched_count(), 1);
    }

    #[test]
    fn same_key_under_a_different_name_is_mismatched_not_in_sync() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [listing("custom_score", doc! { "score": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        let candidates = mismatch_candidates(&drift);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].differences(),
            &["name: expected `score_1`, actual `custom_score`".to_string()]
        );
    }

    #[test]
    fn mismatch_candidates_are_not_duplicated_in_unmanaged() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [
            listing("custom_score", doc! { "score": 1 }, |_| {}),
            listing("extra_1", doc! { "extra": 1 }, |_| {}),
        ];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(mismatch_candidates(&drift).len(), 1);
        assert_eq!(drift.unmanaged().len(), 1);
        assert_eq!(
            drift.unmanaged()[0]
                .options
                .as_ref()
                .and_then(|options| options.name.as_deref()),
            Some("extra_1")
        );
    }

    #[test]
    fn unmanaged_indexes_do_not_break_sync_and_id_is_ignored() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [
            id_listing(),
            listing("score_1", doc! { "score": 1 }, |_| {}),
            listing("compound", doc! { "a": 1, "b": -1 }, |_| {}),
            listing("partial", doc! { "rating": 1 }, |options| {
                options.partial_filter_expression = Some(doc! { "rating": { "$gt": 5 } });
            }),
        ];

        let drift = report(&[expected], Some(&actual), None);

        assert!(drift.is_in_sync());
        let unmanaged_names: Vec<_> = drift
            .unmanaged()
            .iter()
            .map(|index| {
                index
                    .options
                    .as_ref()
                    .and_then(|options| options.name.as_deref())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        assert_eq!(unmanaged_names, ["compound", "partial"]);
    }

    #[test]
    fn unmanaged_ordering_is_deterministic_by_name() {
        let actual = [
            listing("zzz", doc! { "z": 1 }, |_| {}),
            listing("aaa", doc! { "a": 1 }, |_| {}),
            listing("mmm", doc! { "m": 1 }, |_| {}),
        ];

        let drift = report(&[], Some(&actual), None);

        let names: Vec<_> = drift
            .unmanaged()
            .iter()
            .map(|index| {
                index
                    .options
                    .as_ref()
                    .and_then(|options| options.name.as_deref())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        assert_eq!(names, ["aaa", "mmm", "zzz"]);
    }

    #[test]
    fn candidate_ordering_is_deterministic_by_name() {
        // Two same-key indexes under other names: both are candidates,
        // sorted by name regardless of server cursor order.
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [
            listing("z_score", doc! { "score": 1 }, |_| {}),
            listing("a_score", doc! { "score": 1 }, |_| {}),
        ];

        let drift = report(&[expected], Some(&actual), None);

        let candidates = mismatch_candidates(&drift);
        let names: Vec<_> = candidates
            .iter()
            .map(|candidate| {
                candidate
                    .actual()
                    .options
                    .as_ref()
                    .and_then(|options| options.name.as_deref())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        assert_eq!(names, ["a_score", "z_score"]);
    }

    // ----- option normalization ------------------------------------------

    #[test]
    fn boolean_options_normalize_absence_to_false() {
        let expected = declaration(doc! { "score": 1 }, |options| {
            options.unique = Some(false);
            options.sparse = Some(false);
            options.hidden = Some(false);
        });
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        assert_in_sync(&drift);
    }

    #[test]
    fn unique_ttl_and_hidden_mismatches_are_reported() {
        let expected = declaration(doc! { "score": 1 }, |options| {
            options.unique = Some(true);
            options.expire_after = Some(Duration::from_secs(3600));
        });
        let actual = [listing("score_1", doc! { "score": 1 }, |options| {
            options.hidden = Some(true);
            options.expire_after = Some(Duration::from_secs(60));
        })];

        let drift = report(&[expected], Some(&actual), None);

        let differences = mismatch_candidates(&drift)[0].differences().to_vec();
        assert_eq!(
            differences,
            [
                "unique: expected true, actual false",
                "hidden: expected false, actual true",
                "expire_after_secs: expected 3600, actual 60",
            ]
        );
    }

    #[test]
    fn ttl_expected_but_absent_is_reported() {
        let expected = declaration(doc! { "created_at": 1 }, |options| {
            options.expire_after = Some(Duration::from_secs(120));
        });
        let actual = [listing("created_at_1", doc! { "created_at": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["expire_after_secs: expected 120, actual none".to_string()]
        );
    }

    #[test]
    fn omitted_version_accepts_any_server_default() {
        // The listing helper always materializes v: 2; a declaration that
        // omitted the version must not report drift.
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn declared_version_mismatch_is_reported() {
        let expected = declaration(doc! { "score": 1 }, |options| {
            options.version = Some(IndexVersion::V1);
        });
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["version: expected 1, actual 2".to_string()]
        );
    }

    #[test]
    fn declared_matching_version_is_in_sync() {
        let expected = declaration(doc! { "score": 1 }, |options| {
            options.version = Some(IndexVersion::V2);
        });
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn background_is_never_a_drift_dimension() {
        let expected = declaration(doc! { "score": 1 }, |options| {
            options.background = Some(true);
        });
        // Modern MongoDB ignores and does not report the option.
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn declared_2d_parameters_compare_when_present() {
        let expected = declaration(doc! { "position": "2d" }, |options| {
            options.bits = Some(26);
            options.min = Some(-180.0);
            options.max = Some(180.0);
        });

        let matching = [listing(
            "position_2d",
            doc! { "position": "2d" },
            |options| {
                options.bits = Some(26);
                options.min = Some(-180.0);
                options.max = Some(180.0);
            },
        )];
        assert_in_sync(&report(
            std::slice::from_ref(&expected),
            Some(&matching),
            None,
        ));

        let differing = [listing(
            "position_2d",
            doc! { "position": "2d" },
            |options| {
                options.bits = Some(24);
                options.min = Some(-90.0);
                options.max = Some(180.0);
            },
        )];
        let drift = report(&[expected], Some(&differing), None);
        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &[
                "bits: expected 26, actual 24".to_string(),
                "min: expected -180, actual -90".to_string(),
            ]
        );
    }

    #[test]
    fn wildcard_keys_compare_exactly_and_never_match_whole_document() {
        let expected = declaration(doc! { "attributes.$**": 1 }, |_| {});

        let matching = [listing(
            "attributes.$**_1",
            doc! { "attributes.$**": 1 },
            |_| {},
        )];
        assert_in_sync(&report(
            std::slice::from_ref(&expected),
            Some(&matching),
            None,
        ));

        // A whole-document wildcard index is a different index entirely.
        let whole_document = [listing("$**_1", doc! { "$**": 1 }, |_| {})];
        assert_missing(&report(&[expected], Some(&whole_document), None));
    }

    #[test]
    fn wildcard_projection_on_candidate_is_a_mismatch() {
        let expected = declaration(doc! { "attributes.$**": 1 }, |_| {});
        let actual = [listing(
            "attributes.$**_1",
            doc! { "attributes.$**": 1 },
            |options| {
                options.wildcard_projection = Some(doc! { "attributes.secret": 0 });
            },
        )];

        let drift = report(&[expected], Some(&actual), None);

        assert!(
            mismatch_candidates(&drift)[0]
                .differences()
                .iter()
                .any(|difference| difference.starts_with("wildcard_projection:")),
        );
    }

    #[test]
    fn partial_filter_on_same_key_candidate_is_a_mismatch() {
        let expected = declaration(doc! { "rating": 1 }, |_| {});
        let actual = [listing("rating_1", doc! { "rating": 1 }, |options| {
            options.partial_filter_expression = Some(doc! { "rating": { "$gt": 5 } });
        })];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &[
                r#"partial_filter_expression: expected none, actual { "rating": { "$gt": 5 } }"#
                    .to_string()
            ]
        );
    }

    #[test]
    fn storage_engine_and_bucket_size_on_candidate_are_mismatches() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [listing("score_1", doc! { "score": 1 }, |options| {
            options.storage_engine = Some(doc! { "wiredTiger": {} });
            options.bucket_size = Some(5);
        })];

        let drift = report(&[expected], Some(&actual), None);

        let differences = mismatch_candidates(&drift)[0].differences().to_vec();
        assert!(differences.iter().any(|d| d.starts_with("storage_engine:")));
        assert!(differences.iter().any(|d| d.starts_with("bucket_size:")));
    }

    // ----- text normalization --------------------------------------------

    #[test]
    fn text_declaration_matches_fts_ftsx_listing() {
        // Adversarial case C: correct text index must be InSync even though
        // listIndexes reports internal keys.
        let expected = declaration(doc! { "content": "text" }, |_| {});
        let actual = [text_listing("content_text", doc! { "content": 1 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn text_declaration_with_weight_and_languages_matches_materialized_listing() {
        let expected = declaration(doc! { "title": "text" }, |options| {
            options.weights = Some(doc! { "title": 5 });
            options.default_language = Some("english".to_string());
            options.language_override = Some("language".to_string());
            options.name = Some("title_text_idx".to_string());
        });
        let actual = [text_listing("title_text_idx", doc! { "title": 5 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn text_weight_mismatch_is_reported() {
        let expected = declaration(doc! { "title": "text" }, |options| {
            options.weights = Some(doc! { "title": 5 });
        });
        let actual = [text_listing("title_text", doc! { "title": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["text fields: expected { title: 5 }, actual { title: 1 }".to_string()]
        );
    }

    #[test]
    fn text_index_on_different_field_is_a_conflict_candidate() {
        // Adversarial case I: MongoDB allows one text index per collection,
        // so an existing text index on another field blocks creation.
        let expected = declaration(doc! { "content": "text" }, |_| {});
        let actual = [text_listing("summary_text", doc! { "summary": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        let differences = mismatch_candidates(&drift)[0].differences().to_vec();
        assert!(
            differences
                .iter()
                .any(|difference| difference.starts_with("text fields:")),
            "expected a text fields difference: {differences:?}"
        );
        assert_eq!(drift.missing_count(), 0);
    }

    #[test]
    fn text_language_defaults_compare_effectively() {
        // Declaration omits languages; server materializes the documented
        // defaults. No drift.
        let expected = declaration(doc! { "content": "text" }, |_| {});
        let actual = [text_listing("content_text", doc! { "content": 1 }, |_| {})];
        assert_in_sync(&report(&[expected], Some(&actual), None));

        // A genuinely different language is drift.
        let expected = declaration(doc! { "content": "text" }, |_| {});
        let actual = [text_listing(
            "content_text",
            doc! { "content": 1 },
            |options| {
                options.default_language = Some("spanish".to_string());
            },
        )];
        let drift = report(&[expected], Some(&actual), None);
        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["default_language: expected english, actual spanish".to_string()]
        );
    }

    #[test]
    fn omitted_text_index_version_accepts_server_default() {
        let expected = declaration(doc! { "content": "text" }, |_| {});
        let actual = [text_listing("content_text", doc! { "content": 1 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn declared_text_index_version_mismatch_is_reported() {
        let expected = declaration(doc! { "content": "text" }, |options| {
            options.text_index_version = Some(TextIndexVersion::V2);
        });
        let actual = [text_listing("content_text", doc! { "content": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["text_index_version: expected 2, actual 3".to_string()]
        );
    }

    #[test]
    fn text_sparse_is_not_a_drift_dimension() {
        // Text indexes are always sparse; MongoDB ignores the option.
        let expected = declaration(doc! { "content": "text" }, |options| {
            options.sparse = Some(true);
        });
        let actual = [text_listing("content_text", doc! { "content": 1 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn geo_sparse_is_not_a_drift_dimension() {
        // 2dsphere (v2+) and 2d indexes are always sparse; the option has no
        // semantic effect, so a declared flag must not drift against an
        // equivalent index created without it.
        let expected = declaration(doc! { "location": "2dsphere" }, |options| {
            options.sparse = Some(true);
        });
        let actual = [listing(
            "location_2dsphere",
            doc! { "location": "2dsphere" },
            |_| {},
        )];
        assert_in_sync(&report(&[expected], Some(&actual), None));

        let expected = declaration(doc! { "position": "2d" }, |options| {
            options.sparse = Some(true);
        });
        let actual = [listing("position_2d", doc! { "position": "2d" }, |_| {})];
        assert_in_sync(&report(&[expected], Some(&actual), None));

        // The inverse also holds: MongoDB persists an explicitly requested
        // `sparse: true` in listIndexes for these types, and that persisted
        // flag must not drift against a declaration that omitted it.
        let expected = declaration(doc! { "location": "2dsphere" }, |_| {});
        let actual = [listing(
            "location_2dsphere",
            doc! { "location": "2dsphere" },
            |options| {
                options.sparse = Some(true);
            },
        )];
        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn uncreatable_wildcard_sparse_declaration_is_mismatched_not_in_sync() {
        // MongoDB 8.0 rejects the `sparse` option on wildcard indexes
        // outright (code 67, CannotCreateIndex), so this declaration cannot
        // actually be created. An existing flagless wildcard index must not
        // count as InSync — after it is dropped, OxiMod could not recreate
        // the declared desired state — but it remains a logical-key
        // candidate, so the declaration is Mismatched, never Missing.
        let expected = declaration(doc! { "attributes.$**": 1 }, |options| {
            options.sparse = Some(true);
        });
        let actual = [listing(
            "attributes.$**_1",
            doc! { "attributes.$**": 1 },
            |_| {},
        )];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["sparse: expected true, actual false".to_string()]
        );
        assert_eq!(drift.missing_count(), 0);
    }

    #[test]
    fn ordinary_sparse_remains_a_drift_dimension() {
        // For ordinary indexes the option is semantic: declared but absent
        // on the server is real drift.
        let expected = declaration(doc! { "rank": 1 }, |options| {
            options.sparse = Some(true);
        });
        let actual = [listing("rank_1", doc! { "rank": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["sparse: expected true, actual false".to_string()]
        );
    }

    #[test]
    fn non_text_index_under_expected_text_name_is_mismatched() {
        let expected = declaration(doc! { "content": "text" }, |_| {});
        let actual = [listing("content_text", doc! { "content": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), None);

        let differences = mismatch_candidates(&drift)[0].differences().to_vec();
        assert!(
            differences
                .iter()
                .any(|difference| difference.starts_with("keys:")),
            "expected a keys difference: {differences:?}"
        );
    }

    // ----- collation normalization ---------------------------------------

    #[test]
    fn case_insensitive_preset_matches_server_materialized_collation() {
        // OxiMod declares only locale + strength; the server returns the
        // full defaulted document.
        let expected = declaration(doc! { "email": 1 }, |options| {
            options.collation = Some(
                Collation::builder()
                    .locale("en".to_string())
                    .strength(CollationStrength::Secondary)
                    .build(),
            );
        });
        let actual = [listing("email_1", doc! { "email": 1 }, |options| {
            options.collation = Some(server_en_collation(CollationStrength::Secondary));
        })];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn collation_strength_difference_is_reported() {
        let expected = declaration(doc! { "email": 1 }, |options| {
            options.collation = Some(
                Collation::builder()
                    .locale("en".to_string())
                    .strength(CollationStrength::Secondary)
                    .build(),
            );
        });
        let actual = [listing("email_1", doc! { "email": 1 }, |options| {
            options.collation = Some(server_en_collation(CollationStrength::Tertiary));
        })];

        let drift = report(&[expected], Some(&actual), None);

        let differences = mismatch_candidates(&drift)[0].differences().to_vec();
        assert_eq!(differences.len(), 1);
        assert!(
            differences[0].starts_with("collation: expected { locale: en, strength: 2"),
            "unexpected difference: {differences:?}"
        );
    }

    #[test]
    fn missing_collation_against_simple_actual_is_in_sync() {
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        assert_in_sync(&report(&[expected], Some(&actual), None));
    }

    #[test]
    fn explicit_simple_locale_normalizes_to_simple() {
        let simple = Collation::builder().locale("simple".to_string()).build();
        assert_eq!(normalize_collation(&simple), None);
    }

    #[test]
    fn collection_default_collation_is_inherited_by_ordinary_declarations() {
        let default_collation = server_en_collation(CollationStrength::Secondary);

        // The server creates the index with the inherited default; the
        // declaration without a collation therefore expects it.
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [listing("score_1", doc! { "score": 1 }, |options| {
            options.collation = Some(server_en_collation(CollationStrength::Secondary));
        })];

        assert_in_sync(&report(
            &[expected],
            Some(&actual),
            Some(&default_collation),
        ));
    }

    #[test]
    fn collation_drift_against_collection_default_is_reported() {
        let default_collation = server_en_collation(CollationStrength::Secondary);

        // Actual index has simple collation although the collection default
        // says otherwise: something replaced it.
        let expected = declaration(doc! { "score": 1 }, |_| {});
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        let drift = report(&[expected], Some(&actual), Some(&default_collation));

        let differences = mismatch_candidates(&drift)[0].differences().to_vec();
        assert!(
            differences[0].starts_with("collation: expected { locale: en"),
            "unexpected difference: {differences:?}"
        );
    }

    #[test]
    fn text_and_2d_declarations_ignore_collection_default_collation() {
        // Text and 2d indexes only support simple binary comparison; the
        // collection default must not create false drift.
        let default_collation = server_en_collation(CollationStrength::Secondary);

        let text_expected = declaration(doc! { "content": "text" }, |_| {});
        let text_actual = [text_listing("content_text", doc! { "content": 1 }, |_| {})];
        assert_in_sync(&report(
            &[text_expected],
            Some(&text_actual),
            Some(&default_collation),
        ));

        let flat_expected = declaration(doc! { "position": "2d" }, |_| {});
        let flat_actual = [listing("position_2d", doc! { "position": "2d" }, |_| {})];
        assert_in_sync(&report(
            &[flat_expected],
            Some(&flat_actual),
            Some(&default_collation),
        ));
    }

    #[test]
    fn index_level_collation_overrides_collection_default() {
        let default_collation = server_en_collation(CollationStrength::Tertiary);

        let expected = declaration(doc! { "email": 1 }, |options| {
            options.collation = Some(
                Collation::builder()
                    .locale("en".to_string())
                    .strength(CollationStrength::Secondary)
                    .build(),
            );
        });
        let actual = [listing("email_1", doc! { "email": 1 }, |options| {
            options.collation = Some(server_en_collation(CollationStrength::Secondary));
        })];

        assert_in_sync(&report(
            &[expected],
            Some(&actual),
            Some(&default_collation),
        ));
    }

    // ----- multi-declaration behavior ------------------------------------

    #[test]
    fn statuses_follow_declaration_order_and_mix_correctly() {
        let declared = [
            declaration(doc! { "a": 1 }, |_| {}),
            declaration(doc! { "b": 1 }, |_| {}),
            declaration(doc! { "c": 1 }, |options| {
                options.unique = Some(true);
            }),
        ];
        let actual = [
            id_listing(),
            listing("a_1", doc! { "a": 1 }, |_| {}),
            listing("c_1", doc! { "c": 1 }, |_| {}),
        ];

        let drift = report(&declared, Some(&actual), None);

        assert_eq!(drift.declared().len(), 3);
        assert!(matches!(
            drift.declared()[0],
            DeclaredIndexStatus::InSync { .. }
        ));
        assert!(matches!(
            drift.declared()[1],
            DeclaredIndexStatus::Missing { .. }
        ));
        assert!(matches!(
            drift.declared()[2],
            DeclaredIndexStatus::Mismatched { .. }
        ));
        assert_eq!(drift.missing_count(), 1);
        assert_eq!(drift.mismatched_count(), 1);
        assert!(drift.has_drift());
        assert_eq!(drift.declared()[1].expected().keys, doc! { "b": 1 });
    }

    #[test]
    fn one_actual_can_be_a_candidate_for_multiple_conflicting_declarations() {
        // Two declarations over the same field sharing the implicit default
        // name remain individually diagnosed against the same actual index.
        let declared = [
            declaration(doc! { "score": 1 }, |options| {
                options.unique = Some(true);
            }),
            declaration(doc! { "score": 1 }, |_| {}),
        ];
        let actual = [listing("score_1", doc! { "score": 1 }, |_| {})];

        let drift = report(&declared, Some(&actual), None);

        assert!(matches!(
            drift.declared()[0],
            DeclaredIndexStatus::Mismatched { .. }
        ));
        assert!(matches!(
            drift.declared()[1],
            DeclaredIndexStatus::InSync { .. }
        ));
        assert!(drift.unmanaged().is_empty());
    }

    #[test]
    fn difference_property_order_is_fixed() {
        // A candidate differing in many dimensions reports them in the
        // fixed order: keys, name, unique, sparse, hidden, TTL, collation.
        let expected = declaration(doc! { "score": 1 }, |options| {
            options.name = Some("score_idx".to_string());
            options.unique = Some(true);
        });
        let actual = [listing("score_idx", doc! { "score": -1 }, |options| {
            options.sparse = Some(true);
            options.hidden = Some(true);
            options.expire_after = Some(Duration::from_secs(60));
            options.collation = Some(server_en_collation(CollationStrength::Secondary));
        })];

        let drift = report(&[expected], Some(&actual), None);

        let differences = mismatch_candidates(&drift)[0].differences().to_vec();
        let prefixes: Vec<_> = differences
            .iter()
            .map(|difference| difference.split(':').next().unwrap_or_default().to_string())
            .collect();
        assert_eq!(
            prefixes,
            [
                "keys",
                "unique",
                "sparse",
                "hidden",
                "expire_after_secs",
                "collation"
            ]
        );
    }

    #[test]
    fn hashed_and_geo_declarations_match_their_listings() {
        let declared = [
            declaration(doc! { "user_id": "hashed" }, |_| {}),
            declaration(doc! { "location": "2dsphere" }, |_| {}),
        ];
        let actual = [
            listing("user_id_hashed", doc! { "user_id": "hashed" }, |_| {}),
            listing(
                "location_2dsphere",
                doc! { "location": "2dsphere" },
                |options| {
                    options.sphere_2d_index_version = Some(Sphere2DIndexVersion::V3);
                },
            ),
        ];

        let drift = report(&declared, Some(&actual), None);

        assert!(drift.is_in_sync());
    }

    #[test]
    fn declared_sphere_version_mismatch_is_reported() {
        let expected = declaration(doc! { "location": "2dsphere" }, |options| {
            options.sphere_2d_index_version = Some(Sphere2DIndexVersion::V2);
        });
        let actual = [listing(
            "location_2dsphere",
            doc! { "location": "2dsphere" },
            |options| {
                options.sphere_2d_index_version = Some(Sphere2DIndexVersion::V3);
            },
        )];

        let drift = report(&[expected], Some(&actual), None);

        assert_eq!(
            mismatch_candidates(&drift)[0].differences(),
            &["sphere_2d_index_version: expected 2, actual 3".to_string()]
        );
    }
}
