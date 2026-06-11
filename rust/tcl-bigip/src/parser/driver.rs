//! Top-level driver — `parse_bigip_conf`, a faithful port of
//! `dialects/f5/bigip/parser/_driver.py`.
//!
//! The container mirrors the Python `BigipConfig`: a `default_partition`,
//! the `generic_objects` index (every stanza lands one row), and the
//! typed objects (each carrying the `BigipConfig` field name it belongs
//! to so the `PyO3` layer can place it). Typed dispatch is layered on in
//! subsequent stages; this stage establishes the block iteration,
//! header resolution, partition-prefixing, and `generic_objects` exactly.

use tcl_lexer::LineIndex;
use tcl_registry::bigip::BigipRegistry;

use crate::model::{BigipGenericObject, ModelObject};
use crate::range::Range;

use super::header::{parse_generic_header, ObjectTypeIndex};
use super::helpers::extract_blocks;

/// One typed object placed in a `BigipConfig` collection. `table_name`
/// is the Python `BigipConfig` attribute it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The `BigipConfig` attribute name (e.g. `"pools"`).
    pub table_name: &'static str,
    /// Object full path (dict key; `""` for singletons).
    pub full_path: String,
    /// The parsed object.
    pub object: ModelObject,
}

/// Parsed BIG-IP configuration. Mirrors the Python `BigipConfig`
/// contract (a `default_partition`, the `generic_objects` index, and the
/// per-kind typed objects).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipConfig {
    /// The partition short-name this source belongs to.
    pub default_partition: String,
    /// `generic_objects`: `(key, object)` for every stanza, in source
    /// order. The key is `"<module>::<object_type>::<identifier or
    /// '<singleton>'>"`.
    pub generic_objects: Vec<(String, BigipGenericObject)>,
    /// Typed objects, each tagged with its destination `BigipConfig`
    /// field. In source order.
    pub objects: Vec<Placed>,
}

/// `(module, object_type)` pairs whose identifier is itself a partition
/// / cluster-wide reference and must not be partition-prefixed. Mirrors
/// the Python `_no_partition_prefix`.
const NO_PARTITION_PREFIX: &[(&str, &str)] = &[("auth", "partition")];

/// Parse a BIG-IP configuration file into a [`BigipConfig`]. Mirrors
/// `parse_bigip_conf`.
#[must_use]
pub fn parse_bigip_conf(source: &str, default_partition: &str) -> BigipConfig {
    let part = if default_partition.is_empty() {
        "Common"
    } else {
        default_partition
    };
    let partition_prefix = format!("/{}/", part.trim_matches('/'));

    let registry = BigipRegistry::build();
    let index = ObjectTypeIndex::build(&registry);
    let line_index = LineIndex::new(source);

    let mut config = BigipConfig {
        default_partition: default_partition.to_owned(),
        ..BigipConfig::default()
    };

    for block in extract_blocks(source) {
        // ``gtm topology`` carries a multi-token condition rather than a
        // full path; the Python driver handles it before generic_objects
        // and `continue`s, so it never lands a generic_objects row.
        if let Some(_topo_id) = block.header.strip_prefix("gtm topology ") {
            // Typed gtm-topology parsing lands with the bespoke parsers.
            continue;
        }

        let Some((module, object_type, mut identifier)) =
            parse_generic_header(&block.header, &index)
        else {
            continue;
        };

        if !identifier.is_empty()
            && !identifier.starts_with('/')
            && !NO_PARTITION_PREFIX.contains(&(module.as_str(), object_type.as_str()))
        {
            identifier = format!("{partition_prefix}{identifier}");
        }

        let id_part = if identifier.is_empty() {
            "<singleton>"
        } else {
            identifier.as_str()
        };
        let key = format!("{module}::{object_type}::{id_part}");
        let range = Range::from_offsets(source, &line_index, block.start_offset, block.end_offset);
        config.generic_objects.push((
            key,
            BigipGenericObject {
                module,
                object_type,
                identifier,
                header: block.header.clone(),
                range: Some(range),
            },
        ));
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_objects_match_python_on_corpus() {
        let src = include_str!("../../../../samples/bigip/bigip.conf");
        let config = parse_bigip_conf(src, "Common");
        assert_eq!(config.default_partition, "Common");
        assert_eq!(config.generic_objects.len(), 28);

        let (key, obj) = &config.generic_objects[0];
        assert_eq!(key, "ltm::data-group internal::/Common/allowed_hosts");
        assert_eq!(obj.module, "ltm");
        assert_eq!(obj.object_type, "data-group internal");
        assert_eq!(obj.identifier, "/Common/allowed_hosts");
        assert_eq!(obj.header, "ltm data-group internal /Common/allowed_hosts");
        // Range captured from the live Python parser.
        let r = obj.range.unwrap();
        assert_eq!(
            (r.start.line, r.start.character, r.start.offset),
            (3, 46, 162)
        );
        assert_eq!((r.end.line, r.end.character, r.end.offset), (10, 1, 287));
    }
}
