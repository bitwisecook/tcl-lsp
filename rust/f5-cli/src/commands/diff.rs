//! The `diff` verb — object-aware diff between two BIG-IP configs.
//!
//! Port of `compute_diff` / `_run_diff` (`dialects/f5/bigip/diff.py`,
//! `tooling/f5/verbs/diff.py`). Parses both configs with `tcl-bigip`, groups
//! objects by kind, and classifies each full-path as added / removed /
//! modified, comparing a fixed set of "interesting" fields.
//!
//! Field values are read from each object's canonical JSON (`canon_fields`),
//! which carries the same field names as the Python dataclasses. Scalar and
//! string-list fields reach byte-parity; object-list fields (pool `members`,
//! data-group `records`) display differently from Python's dataclass `repr`
//! (change *detection* is still correct since the formatting is deterministic).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use tcl_bigip::canonical::Canon;
use tcl_bigip::parser::{parse_bigip_conf, BigipConfig};

/// Fields compared inside a modified object (mirrors `_INTERESTING_FIELDS`).
const INTERESTING_FIELDS: &[&str] = &[
    "destination",
    "pool",
    "rules",
    "profiles",
    "persist",
    "snatpool",
    "source_address_translation",
    "members",
    "monitor",
    "load_balancing_mode",
    "address",
    "kind",
    "value_type",
    "records",
    "monitor_type",
    "persistence_type",
    "profile_type",
    "source",
];

/// `(module, object_type)` pairs with a specialised typed inventory, excluded
/// from the `generic` kind (mirrors `_SPECIALISED_GENERIC_TYPES`).
const SPECIALISED: &[(&str, &str)] = &[
    ("ltm", "virtual"),
    ("ltm", "pool"),
    ("ltm", "node"),
    ("ltm", "monitor"),
    ("ltm", "snatpool"),
    ("ltm", "persistence"),
    ("ltm", "rule"),
    ("ltm", "data-group"),
    ("ltm", "profile"),
];

/// Map a `Placed.table_name` to the diff "kind".
fn table_to_kind(table: &str) -> Option<&'static str> {
    match table {
        "virtual_servers" => Some("virtual"),
        "pools" => Some("pool"),
        "nodes" => Some("node"),
        "profiles" => Some("profile"),
        "monitors" => Some("monitor"),
        "data_groups" => Some("data-group"),
        "snat_pools" => Some("snatpool"),
        "persistence" => Some("persistence"),
        "rules" => Some("rule"),
        _ => None,
    }
}

/// `kind -> {full_path: canonical-fields}` (mirrors `_kind_inventories`).
fn kind_inventories(cfg: &BigipConfig) -> BTreeMap<String, BTreeMap<String, Value>> {
    let mut inv: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    // All typed kinds are always present (possibly empty), matching Python.
    for kind in [
        "virtual",
        "pool",
        "node",
        "profile",
        "monitor",
        "data-group",
        "snatpool",
        "persistence",
        "rule",
        "generic",
    ] {
        inv.insert(kind.to_owned(), BTreeMap::new());
    }

    for placed in &cfg.objects {
        if let Some(kind) = table_to_kind(placed.table_name) {
            inv.get_mut(kind)
                .unwrap()
                .insert(placed.full_path.clone(), placed.object.canon_fields());
        }
    }

    let specialised: HashSet<(&str, &str)> = SPECIALISED.iter().copied().collect();
    let generic = inv.get_mut("generic").unwrap();
    for (_key, obj) in &cfg.generic_objects {
        if specialised.contains(&(obj.module.as_str(), obj.object_type.as_str())) {
            continue;
        }
        generic.insert(obj.identifier.clone(), obj.canon_fields());
    }
    inv
}

#[derive(Serialize)]
struct FieldChange {
    field: &'static str,
    before: String,
    after: String,
}

#[derive(Serialize)]
struct ObjectChange {
    #[serde(rename = "fullPath")]
    full_path: String,
    kind: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<FieldChange>,
}

#[derive(Serialize)]
struct Summary {
    added: usize,
    removed: usize,
    modified: usize,
}

#[derive(Serialize)]
struct DiffReport {
    added: Vec<ObjectChange>,
    removed: Vec<ObjectChange>,
    modified: Vec<ObjectChange>,
    summary: Summary,
}

/// `f5 diff` — object-aware diff between two configs.
pub fn run_diff(
    before_path: &Path,
    after_path: &Path,
    json: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let before = read_config(before_path)?;
    let after = read_config(after_path)?;

    let a_inv = kind_inventories(&before);
    let b_inv = kind_inventories(&after);

    let mut added: Vec<ObjectChange> = Vec::new();
    let mut removed: Vec<ObjectChange> = Vec::new();
    let mut modified: Vec<ObjectChange> = Vec::new();

    let mut kinds: BTreeSet<&String> = BTreeSet::new();
    kinds.extend(a_inv.keys());
    kinds.extend(b_inv.keys());

    let empty = BTreeMap::new();
    for kind in kinds {
        let a_kind = a_inv.get(kind).unwrap_or(&empty);
        let b_kind = b_inv.get(kind).unwrap_or(&empty);

        let mut b_only: Vec<&String> = b_kind.keys().filter(|p| !a_kind.contains_key(*p)).collect();
        b_only.sort();
        for path in b_only {
            added.push(ObjectChange {
                full_path: path.clone(),
                kind: kind.clone(),
                status: "added",
                fields: Vec::new(),
            });
        }

        let mut a_only: Vec<&String> = a_kind.keys().filter(|p| !b_kind.contains_key(*p)).collect();
        a_only.sort();
        for path in a_only {
            removed.push(ObjectChange {
                full_path: path.clone(),
                kind: kind.clone(),
                status: "removed",
                fields: Vec::new(),
            });
        }

        let mut common: Vec<&String> = a_kind.keys().filter(|p| b_kind.contains_key(*p)).collect();
        common.sort();
        for path in common {
            let fields = diff_object(&a_kind[path], &b_kind[path]);
            if !fields.is_empty() {
                modified.push(ObjectChange {
                    full_path: path.clone(),
                    kind: kind.clone(),
                    status: "modified",
                    fields,
                });
            }
        }
    }

    let report = DiffReport {
        summary: Summary {
            added: added.len(),
            removed: removed.len(),
            modified: modified.len(),
        },
        added,
        removed,
        modified,
    };

    let has_changes =
        !report.added.is_empty() || !report.removed.is_empty() || !report.modified.is_empty();

    let mut rendered = if json {
        tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&report)?)
    } else {
        format_text(&report)
    };
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    let target = tcl_cli_support::OutputTarget::from_arg(output);
    tcl_cli_support::write_text_output(&target, &rendered)?;
    Ok(u8::from(has_changes))
}

fn read_config(path: &Path) -> anyhow::Result<BigipConfig> {
    // Read via the UCS-aware resolver (mirrors `read_path` in the Python diff
    // handler) so a `.ucs` — plain or encrypted — is transparently extracted to
    // SCF, exactly like a `.conf`/`.scf`.
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let (_uri, source) = tcl_bigip_io::read_path(&path.to_string_lossy(), false, &opts)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    // Accept tmsh-script input as well as SCF (mirrors `_to_scf` in the Python
    // diff handler) so `tmsh create/modify` headers compare against the same
    // objects, not `tmsh`-module generics.
    Ok(parse_bigip_conf(
        &crate::commands::scf::to_scf(&source),
        "Common",
    ))
}

/// Compare the interesting fields of two objects (mirrors `_diff_object`).
fn diff_object(before: &Value, after: &Value) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    for &field in INTERESTING_FIELDS {
        let b = field_value(before, field);
        let a = field_value(after, field);
        if b != a {
            changes.push(FieldChange {
                field,
                before: b,
                after: a,
            });
        }
    }
    changes
}

/// Extract a field as a comparable string (mirrors `_field_value`).
fn field_value(obj: &Value, field: &str) -> String {
    let Some(val) = obj.get(field) else {
        return String::new();
    };
    if val.is_null() {
        return String::new();
    }
    if field == "source" {
        return normalise_irule_source(val.as_str().unwrap_or(""));
    }
    if let Some(arr) = val.as_array() {
        let mut items: Vec<String> = arr.iter().map(py_str).collect();
        items.sort();
        return items.join(",");
    }
    py_str(val)
}

/// Python `str(value)` for a JSON scalar/collection (best-effort; object
/// elements diverge from dataclass `repr`).
fn py_str(val: &Value) -> String {
    match val {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Strip comments and collapse whitespace (mirrors `_normalise_irule_source`).
fn normalise_irule_source(source: &str) -> String {
    let mut no_comments = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let body = line.find('#').map_or(line, |idx| &line[..idx]);
        no_comments.push_str(body);
        if body.len() < line.len() && line.ends_with('\n') {
            no_comments.push('\n');
        }
    }
    no_comments.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text report (mirrors `_format_text`).
fn format_text(report: &DiffReport) -> String {
    let mut lines: Vec<String> = vec![format!(
        "diff: +{} added, -{} removed, ~{} modified",
        report.added.len(),
        report.removed.len(),
        report.modified.len()
    )];
    for c in &report.added {
        lines.push(format!("+ [{}] {}", c.kind, c.full_path));
    }
    for c in &report.removed {
        lines.push(format!("- [{}] {}", c.kind, c.full_path));
    }
    for c in &report.modified {
        lines.push(format!("~ [{}] {}", c.kind, c.full_path));
        for fc in &c.fields {
            lines.push(format!(
                "    {}: {} -> {}",
                fc.field,
                py_repr(&fc.before),
                py_repr(&fc.after)
            ));
        }
    }
    lines.join("\n")
}

/// Python `repr()` of a string (mirrors `{value!r}`).
fn py_repr(s: &str) -> String {
    use std::fmt::Write as _;
    let use_double = s.contains('\'') && !s.contains('"');
    let quote = if use_double { '"' } else { '\'' };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}
