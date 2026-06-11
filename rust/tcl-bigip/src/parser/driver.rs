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
                module: module.clone(),
                object_type: object_type.clone(),
                identifier: identifier.clone(),
                header: block.header.clone(),
                range: Some(range),
            },
        ));

        // Typed dispatch — mirrors the strict `_parse_header` path.
        if let Some(placed) = dispatch_block(
            &block.header,
            &module,
            &object_type,
            &identifier,
            &block.body,
            range,
            &partition_prefix,
        ) {
            config.objects.push(placed);
        }
    }

    config
}

/// Route one stanza to its typed object, mirroring the strict-header
/// dispatch in `_driver.py`. `generic_*` come from the generic header
/// (already partition-prefixed) for the bare-singleton fallback.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::wildcard_imports
)]
fn dispatch_block(
    header: &str,
    generic_module: &str,
    generic_type: &str,
    generic_identifier: &str,
    body: &str,
    range: Range,
    partition_prefix: &str,
) -> Option<Placed> {
    use crate::model::gen::dispatch::{
        dispatch_ltm_tables, dispatch_minimal, dispatch_named, dispatch_singleton,
        parse_header_strict,
    };
    use crate::model::gen::parsers::*;
    use crate::model::ModelObject;

    let parsed = parse_header_strict(header).map(|(m, o, fp)| {
        let fp = if !fp.is_empty()
            && !fp.starts_with('/')
            && !NO_PARTITION_PREFIX.contains(&(m.as_str(), o.as_str()))
        {
            format!("{partition_prefix}{fp}")
        } else {
            fp
        };
        (m, o, fp)
    });

    let Some((module, object_type, full_path)) = parsed else {
        // Bare singleton: route the empty-identifier generic via the
        // singleton / minimal tables.
        if !generic_identifier.is_empty() {
            return None;
        }
        if let Some((table, object)) =
            dispatch_singleton(generic_module, generic_type, "", body, range)
        {
            return placed(table, "", object);
        }
        if let Some((table, object)) =
            dispatch_minimal(generic_module, generic_type, "", body, range)
        {
            return placed(table, "", object);
        }
        return None;
    };

    let fp = full_path.as_str();

    // Minimal pre-pass + named + singleton + rich ltm tables (generated).
    if let Some((table, object)) = dispatch_minimal(&module, &object_type, fp, body, range) {
        return placed(table, fp, object);
    }
    if let Some((table, object)) = dispatch_named(&module, &object_type, fp, body, range) {
        return placed(table, fp, object);
    }
    if full_path.is_empty() {
        if let Some((table, object)) = dispatch_singleton(&module, &object_type, fp, body, range) {
            return placed(table, fp, object);
        }
    }
    if let Some((table, object)) = dispatch_ltm_tables(&module, &object_type, fp, body, range) {
        return placed(table, fp, object);
    }

    // Family parsers with a sub-type argument + the ltm/gtm match block.
    if module == "apm" && object_type.starts_with("policy agent ") {
        return placed(
            "apm_policy_agents",
            fp,
            ModelObject::ApmPolicyAgent(parse_bigip_apm_policy_agent(fp, body, range)),
        );
    }
    if module == "gtm" && object_type.starts_with("pool ") {
        return placed(
            "gtm_pools",
            fp,
            ModelObject::GtmPool(parse_bigip_gtm_pool(fp, body, range)),
        );
    }
    if module == "gtm" && object_type.starts_with("wideip ") {
        return placed(
            "gtm_wideips",
            fp,
            ModelObject::GtmWideip(parse_bigip_gtm_wideip(fp, body, range)),
        );
    }
    if module == "pem"
        && matches!(
            object_type.as_str(),
            "profile diameter-endpoint"
                | "profile radius-aaa"
                | "profile spm"
                | "profile subscriber-mgmt"
        )
    {
        return placed(
            "pem_profiles",
            fp,
            ModelObject::PemProfile(parse_bigip_pem_profile(fp, body, range)),
        );
    }
    if module != "ltm" && module != "gtm" {
        return None;
    }

    let object = match object_type.as_str() {
        "data-group internal" | "data-group external" => {
            return placed(
                "data_groups",
                fp,
                ModelObject::DataGroup(parse_bigip_data_group(fp, body, range)),
            )
        }
        "pool" if module == "ltm" => ModelObject::Pool(parse_bigip_pool(fp, body, range)),
        "virtual" => ModelObject::VirtualServer(parse_bigip_virtual_server(fp, body, range)),
        "virtual-address" if module == "ltm" => {
            ModelObject::VirtualAddress(parse_bigip_virtual_address(fp, body, range))
        }
        "node" => ModelObject::Node(parse_bigip_node(fp, body, range)),
        "snatpool" => ModelObject::SnatPool(parse_bigip_snat_pool(fp, body, range)),
        "rule" => ModelObject::Rule(parse_bigip_rule(fp, body, range)),
        "policy" if module == "ltm" => ModelObject::Policy(parse_bigip_policy(fp, body, range)),
        ot if ot.starts_with("dns cache records ") && module == "ltm" => {
            return placed(
                "ltm_dns_cache_records",
                fp,
                ModelObject::LtmDnsCacheRecord(parse_bigip_ltm_dns_cache_record(fp, body, range)),
            )
        }
        ot if ot.starts_with("profile ") => {
            return placed(
                "profiles",
                fp,
                ModelObject::Profile(parse_bigip_profile(fp, body, range)),
            )
        }
        ot if ot.starts_with("persistence ") => {
            return placed(
                "persistence",
                fp,
                ModelObject::Persistence(parse_bigip_persistence(fp, body, range)),
            )
        }
        ot if ot.starts_with("monitor ") => {
            let table = if module == "gtm" {
                "gtm_monitors"
            } else {
                "monitors"
            };
            return placed(
                table,
                fp,
                ModelObject::Monitor(parse_bigip_monitor(fp, body, range)),
            );
        }
        _ => return None,
    };
    let table = match object_type.as_str() {
        "pool" => "pools",
        "virtual" => "virtual_servers",
        "virtual-address" => "virtual_addresses",
        "node" => "nodes",
        "snatpool" => "snat_pools",
        "rule" => "rules",
        "policy" => "policies",
        _ => return None,
    };
    placed(table, fp, object)
}

/// Build a [`Placed`] from a table name + object.
#[allow(clippy::unnecessary_wraps)]
fn placed(table: &'static str, full_path: &str, object: ModelObject) -> Option<Placed> {
    Some(Placed {
        table_name: table,
        full_path: full_path.to_owned(),
        object,
    })
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

    #[test]
    fn typed_object_inventory_matches_python_on_corpus() {
        let src = include_str!("../../../../samples/bigip/bigip.conf");
        let config = parse_bigip_conf(src, "Common");
        // Per-table counts captured from the live Python parser.
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for p in &config.objects {
            *counts.entry(p.table_name).or_default() += 1;
        }
        let expected = [
            ("data_groups", 4),
            ("pools", 3),
            ("virtual_servers", 4),
            ("nodes", 3),
            ("profiles", 5),
            ("monitors", 1),
            ("snat_pools", 1),
            ("persistence", 2),
            ("rules", 5),
        ];
        for (table, n) in expected {
            assert_eq!(counts.get(table).copied().unwrap_or(0), n, "table {table}");
        }
        assert_eq!(config.objects.len(), 28, "total typed objects");
    }
}
