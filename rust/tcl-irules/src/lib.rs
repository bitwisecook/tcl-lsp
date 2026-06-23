//! F5 iRules BIG-IP object-reference extractor.
//!
//! Two layers:
//!
//! - [`resolve_object_ref_args`] — the declarative command→`(arg_index, kinds)`
//!   table (`_BASE_SPECS` + `_POOL_REF_TEMPLATES` + the `class`/`persist` custom
//!   resolvers). The hand-written specs are embedded from `data/`; the 1 MB
//!   generated dependency graph backs *completion/coverage* elsewhere, not edge
//!   resolution, so it is intentionally not included here.
//! - the walker (`extract_irules_object_references`) that
//!   segments an iRule body, asks the table for each command's reference args,
//!   and resolves them — including names that flow through `set <var> <literal>`.
//!
//! Shared by the BIG-IP reference graph (`tcl-bigip`) and the LSP semantic-token
//! layer (`tcl-lsp-core`).

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::Deserialize;

mod walker;
pub use walker::{IrulesObjectReference, extract_irules_object_references, object_ref_spans};

mod specs {
    use super::Deserialize;

    #[derive(Deserialize)]
    pub struct ObjectRefArg {
        pub index: Option<i64>,
        pub after_keyword: Option<String>,
        pub last: bool,
    }

    #[derive(Deserialize)]
    pub struct ObjectRefSpec {
        pub command: String,
        pub position: ObjectRefArg,
        pub kinds: Vec<String>,
        pub sub_command: Option<String>,
        pub when_module: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct PoolTemplate {
        pub command: String,
        pub position: ObjectRefArg,
    }

    #[derive(Deserialize)]
    pub struct RawData {
        pub base_specs: Vec<ObjectRefSpec>,
        pub pool_templates: Vec<PoolTemplate>,
        pub data_group_kinds: Vec<String>,
        pub persistence_kinds: Vec<String>,
        pub ssl_profile_kinds: Vec<String>,
        pub falsey_ref_tokens: Vec<String>,
        pub ltm_pool_kinds: Vec<String>,
        pub gtm_pool_kinds: Vec<String>,
    }
}

use specs::{ObjectRefArg, ObjectRefSpec, PoolTemplate, RawData};

/// The parsed, deduplicated reference tables.
struct Tables {
    base_specs: Vec<ObjectRefSpec>,
    pool_templates: Vec<PoolTemplate>,
    falsey: HashSet<String>,
    data_group_kinds: Vec<String>,
    persistence_kinds: Vec<String>,
    ltm_pool_kinds: Vec<String>,
    gtm_pool_kinds: Vec<String>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

fn tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        let raw: RawData = serde_json::from_str(include_str!("../data/irules_ref_specs.json"))
            .expect("embedded irules_ref_specs.json is valid");
        let _ = &raw.ssl_profile_kinds; // reserved for the SSL::profile completion path
        Tables {
            base_specs: raw.base_specs,
            pool_templates: raw.pool_templates,
            falsey: raw.falsey_ref_tokens.into_iter().collect(),
            data_group_kinds: raw.data_group_kinds,
            persistence_kinds: raw.persistence_kinds,
            ltm_pool_kinds: raw.ltm_pool_kinds,
            gtm_pool_kinds: raw.gtm_pool_kinds,
        }
    })
}

/// Delimiters stripped from a positional token before the falsey-keyword check
/// (strips the leading/trailing characters `{}"'[](),`).
const STRIP: &[char] = &['{', '}', '"', '\'', '[', ']', '(', ')', ','];

/// `class match`/`search` comparison operators.
const CLASS_OPERATORS: &[&str] = &[
    "equals",
    "starts_with",
    "ends_with",
    "contains",
    "matches_glob",
    "matches_regex",
];

/// `persist` heads that are type keywords, not a profile-name reference.
const PERSIST_NON_REFERENCE_KEYWORDS: &[&str] = &[
    "add",
    "delete",
    "lookup",
    "replace",
    "none",
    "simple",
    "sticky",
    "source_addr",
    "dest_addr",
    "hash",
    "cookie",
    "ssl",
    "msrdp",
    "sip",
    "uie",
    "carp",
    "universal",
];

/// Pool kinds for a rule module — GTM rules use the per-record-type GTM pool
/// kinds (mirrors `pool_kinds_for_module`).
fn pool_kinds_for_module(rule_module: Option<&str>) -> Vec<&'static str> {
    let t = tables();
    let src = if rule_module == Some("gtm") {
        &t.gtm_pool_kinds
    } else {
        &t.ltm_pool_kinds
    };
    src.iter().map(String::as_str).collect()
}

/// The argument index named by a position spec (mirrors
/// `_arg_index_for_position`): `last`, an explicit `index`, or the token after a
/// keyword.
fn arg_index_for_position(position: &ObjectRefArg, args: &[&str]) -> Option<usize> {
    if position.last {
        return (!args.is_empty()).then(|| args.len() - 1);
    }
    if let Some(index) = position.index {
        let index = usize::try_from(index).ok()?;
        return (index < args.len()).then_some(index);
    }
    if let Some(keyword) = &position.after_keyword {
        for (i, arg) in args.iter().enumerate() {
            if arg.eq_ignore_ascii_case(keyword) && i + 1 < args.len() {
                return Some(i + 1);
            }
        }
        return None;
    }
    None
}

/// `[(arg_index, kinds), …]` for `class <sub> …` (mirrors `_resolve_class_refs`).
fn resolve_class_refs(args: &[&str]) -> Vec<(usize, Vec<&'static str>)> {
    let dg: Vec<&str> = tables()
        .data_group_kinds
        .iter()
        .map(String::as_str)
        .collect();
    let Some(&sub0) = args.first() else {
        return Vec::new();
    };
    let sub = sub0.to_ascii_lowercase();
    match sub.as_str() {
        "match" | "search" => {
            let mut idx = 1;
            while idx < args.len() && args[idx].starts_with('-') {
                idx += 1;
            }
            if idx < args.len() && args[idx] == "--" {
                idx += 1;
            }
            idx += 1; // skip the ITEM argument
            if idx >= args.len()
                || !CLASS_OPERATORS.contains(&args[idx].to_ascii_lowercase().as_str())
            {
                return Vec::new();
            }
            idx += 1; // skip the operator
            if idx < args.len() {
                return vec![(idx, dg)];
            }
            Vec::new()
        }
        "lookup" => {
            let mut idx = 1;
            if idx < args.len() && args[idx] == "--" {
                idx += 1;
            }
            idx += 1; // skip the key
            if idx < args.len() {
                return vec![(idx, dg)];
            }
            Vec::new()
        }
        "exists" | "size" | "type" | "get" | "startsearch" | "names" | "element" => {
            if args.len() >= 2 {
                vec![(1, dg)]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// `[(arg_index, kinds), …]` for `persist …` (mirrors `_resolve_persist_refs`).
fn resolve_persist_refs(args: &[&str]) -> Vec<(usize, Vec<&'static str>)> {
    let pk: Vec<&str> = tables()
        .persistence_kinds
        .iter()
        .map(String::as_str)
        .collect();
    let Some(&head0) = args.first() else {
        return Vec::new();
    };
    let head = head0.to_ascii_lowercase();
    if !PERSIST_NON_REFERENCE_KEYWORDS.contains(&head.as_str()) || head0.starts_with('/') {
        return vec![(0, pk)];
    }
    if args.len() >= 3
        && matches!(args[1].to_ascii_lowercase().as_str(), "insert" | "lookup")
        && args[2].starts_with('/')
    {
        return vec![(2, pk)];
    }
    Vec::new()
}

/// Return the `(arg_index, kinds)` pairs naming BIG-IP objects in an iRule
/// `command args…` invocation (mirrors `resolve_object_ref_args`).
///
/// `rule_module` (`"ltm"` / `"gtm"`) selects the pool-kind family.
#[must_use]
pub fn resolve_object_ref_args(
    command: &str,
    args: &[&str],
    rule_module: Option<&str>,
) -> Vec<(usize, Vec<&'static str>)> {
    if args.is_empty() {
        return Vec::new();
    }
    let lower = command.to_ascii_lowercase();
    match lower.as_str() {
        "class" => return resolve_class_refs(args),
        "persist" => return resolve_persist_refs(args),
        _ => {}
    }

    let t = tables();
    let pool_kinds = pool_kinds_for_module(rule_module);
    let mut matches: Vec<(usize, Vec<&'static str>)> = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    // Try a single materialised spec: filter the positional token, dedup by
    // index, and record `(idx, kinds)`. Mirrors the body of the Python loop.
    let consider = |position: &ObjectRefArg,
                    kinds: Vec<&'static str>,
                    matches: &mut Vec<(usize, Vec<&'static str>)>,
                    seen: &mut HashSet<usize>| {
        let Some(idx) = arg_index_for_position(position, args) else {
            return;
        };
        if seen.contains(&idx) {
            return;
        }
        let token = args[idx].trim_matches(STRIP).to_ascii_lowercase();
        if t.falsey.contains(&token) {
            return;
        }
        if lower == "pool" && idx == 0 && matches!(token.as_str(), "member" | "members") {
            return;
        }
        seen.insert(idx);
        matches.push((idx, kinds));
    };

    // Base specs first, then the pool templates — the same order
    // `_materialise_specs` produces, so the shared `seen` dedup matches Python.
    for spec in &t.base_specs {
        if spec.command != lower {
            continue;
        }
        if let Some(wm) = &spec.when_module
            && !(rule_module.is_none() || rule_module == Some(wm.as_str()))
        {
            continue;
        }
        if let Some(sc) = &spec.sub_command
            && !args[0].eq_ignore_ascii_case(sc)
        {
            continue;
        }
        let kinds: Vec<&str> = spec.kinds.iter().map(String::as_str).collect();
        consider(&spec.position, kinds, &mut matches, &mut seen);
    }
    for tpl in &t.pool_templates {
        if tpl.command != lower {
            continue;
        }
        consider(&tpl.position, pool_kinds.clone(), &mut matches, &mut seen);
    }
    matches
}
