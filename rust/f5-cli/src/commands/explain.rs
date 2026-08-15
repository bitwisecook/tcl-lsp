// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `explain` verb — resolve and describe the plan for a virtual or pool.
//!
//! Model-based: it parses with `tcl-bigip`, resolves short names via
//! `resolve_name`, and walks each object's canonical JSON (`canon_fields`) to
//! render profiles / iRules / persistence / SNAT / pool sections.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use tcl_bigip::parser::BigipConfig;
use tcl_cli_support::{OutputTarget, write_text_output};

/// Map a `Placed.table_name` to the kinds explain resolves against.
fn table_to_kind(table: &str) -> Option<&'static str> {
    match table {
        "virtual_servers" => Some("virtual"),
        "pools" => Some("pool"),
        "profiles" => Some("profile"),
        "persistence" => Some("persistence"),
        "rules" => Some("rule"),
        "snat_pools" => Some("snatpool"),
        _ => None,
    }
}

/// Per-config ordered inventories (source order), so `resolve_name`'s suffix
/// fallback picks a deterministic key.
struct Model {
    default_partition: String,
    by_kind: HashMap<&'static str, Vec<(String, Value)>>,
}

impl Model {
    fn build(cfg: &BigipConfig) -> Self {
        let mut by_kind: HashMap<&'static str, Vec<(String, Value)>> = HashMap::new();
        for placed in &cfg.objects {
            if let Some(kind) = table_to_kind(placed.table_name) {
                by_kind
                    .entry(kind)
                    .or_default()
                    .push((placed.full_path.clone(), placed.object.canon_fields()));
            }
        }
        Self {
            default_partition: cfg.default_partition.clone(),
            by_kind,
        }
    }

    fn inv(&self, kind: &str) -> &[(String, Value)] {
        self.by_kind.get(kind).map_or(&[][..], Vec::as_slice)
    }

    fn get(&self, kind: &str, path: &str) -> Option<&Value> {
        self.inv(kind)
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, v)| v)
    }

    fn resolve(&self, kind: &str, name: &str) -> Option<String> {
        resolve_name(name, self.inv(kind), &self.default_partition)
    }
}

/// Resolve a possibly-short name to a full path.
fn resolve_name(name: &str, inv: &[(String, Value)], default_partition: &str) -> Option<String> {
    if inv.iter().any(|(p, _)| p == name) {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let trimmed = default_partition.trim_matches('/');
        let partition = if trimmed.is_empty() {
            "Common"
        } else {
            trimmed
        };
        let candidate = format!("/{partition}/{name}");
        if inv.iter().any(|(p, _)| *p == candidate) {
            return Some(candidate);
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if inv.iter().any(|(p, _)| *p == candidate) {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    inv.iter()
        .find(|(p, _)| p.ends_with(&suffix))
        .map(|(p, _)| p.clone())
}

/// The `"v"` payloads of a `BigipList`-shaped canonical field.
fn list_items(field: Option<&Value>) -> Vec<&Value> {
    field
        .and_then(|f| f.get("L"))
        .and_then(|l| l.get("items"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|it| it.get("v")).collect())
        .unwrap_or_default()
}

fn non_empty_str(field: Option<&Value>) -> Option<&str> {
    field.and_then(Value::as_str).filter(|s| !s.is_empty())
}

type Section = (&'static str, Vec<String>);

// One straight-line builder appending each config section in report order;
// splitting it would scatter the virtual-server layout.
#[allow(clippy::too_many_lines)]
fn explain_virtual(model: &Model, vs: &Value) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();

    let dest = match vs.get("destination") {
        None | Some(Value::Null) => "(none)".to_owned(),
        Some(d) => d
            .get("s")
            .and_then(Value::as_str)
            .map_or_else(|| "(none)".to_owned(), str::to_owned),
    };
    sections.push(("destination", vec![dest]));

    let mut profiles = Vec::new();
    for v in list_items(vs.get("profiles")) {
        let Some(pref) = v
            .get("d")
            .and_then(|d| d.get("path"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let resolved = model
            .resolve("profile", pref)
            .unwrap_or_else(|| pref.to_owned());
        if let Some(profile) = model.get("profile", &resolved) {
            let ptype = profile
                .get("profile_type")
                .and_then(|t| t.get("e"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            profiles.push(format!("{resolved} ({ptype})"));
        } else {
            profiles.push(format!("{pref} (unresolved)"));
        }
    }
    sections.push(("profiles", or_none(profiles)));

    let mut rules = Vec::new();
    for v in list_items(vs.get("rules")) {
        let Some(rref) = v.as_str() else { continue };
        let resolved = model
            .resolve("rule", rref)
            .unwrap_or_else(|| rref.to_owned());
        if let Some(rule) = model.get("rule", &resolved) {
            let body = rule.get("source").and_then(Value::as_str).unwrap_or("");
            let events: BTreeSet<String> = tcl_irules::when_blocks(body)
                .into_iter()
                .map(|block| block.event)
                .collect();
            let loc = count_irule_lines(body);
            let event_str = if events.is_empty() {
                "(no when blocks)".to_owned()
            } else {
                events.into_iter().collect::<Vec<_>>().join(", ")
            };
            rules.push(format!("{resolved} — {loc} loc; events: {event_str}"));
        } else {
            rules.push(format!("{rref} (unresolved)"));
        }
    }
    sections.push(("iRules", or_none(rules)));

    let mut persist = Vec::new();
    for v in list_items(vs.get("persist")) {
        let Some(pref) = v
            .get("d")
            .and_then(|d| d.get("path"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let resolved = model
            .resolve("persistence", pref)
            .unwrap_or_else(|| pref.to_owned());
        if let Some(prof) = model.get("persistence", &resolved) {
            let ptype = prof
                .get("persistence_type")
                .and_then(Value::as_str)
                .unwrap_or("");
            persist.push(format!("{resolved} ({ptype})"));
        } else {
            persist.push(format!("{pref} (unresolved)"));
        }
    }
    sections.push(("persistence", or_none(persist)));

    let mut snat = Vec::new();
    if let Some(snatpool) = non_empty_str(vs.get("snatpool")) {
        let resolved = model
            .resolve("snatpool", snatpool)
            .unwrap_or_else(|| snatpool.to_owned());
        if let Some(sp) = model.get("snatpool", &resolved) {
            let members = list_items(sp.get("members")).len();
            snat.push(format!("snatpool {resolved} ({members} member(s))"));
        } else {
            snat.push(format!("snatpool {snatpool} (unresolved)"));
        }
    }
    if let Some(sat) = non_empty_str(vs.get("source_address_translation")) {
        snat.push(format!("translation: {sat}"));
    }
    sections.push(("SNAT", or_none(snat)));

    let mut pool_lines = Vec::new();
    if let Some(pool) = non_empty_str(vs.get("pool")) {
        if let Some(resolved) = model.resolve("pool", pool) {
            if let Some(p) = model.get("pool", &resolved) {
                pool_lines = explain_pool_lines(p);
            }
        } else {
            pool_lines = vec![format!("{pool} (unresolved)")];
        }
    }
    sections.push(("default pool", or_none(pool_lines)));

    sections
}

fn explain_pool_lines(pool: &Value) -> Vec<String> {
    let full_path = pool.get("full_path").and_then(Value::as_str).unwrap_or("");
    let lbm = non_empty_str(pool.get("load_balancing_mode")).unwrap_or("round-robin");
    let mut lines = vec![format!("{full_path} ({lbm})")];
    if let Some(mon) = non_empty_str(pool.get("monitor")) {
        lines.push(format!("  pool monitor: {mon}"));
    }
    for v in list_items(pool.get("members")) {
        use std::fmt::Write as _;
        let d = v.get("d");
        let name = d
            .and_then(|d| d.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let mut line = format!("  member {name}");
        if let Some(addr) = d
            .and_then(|d| d.get("address"))
            .and_then(|a| a.get("s"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            let _ = write!(line, " addr={addr}");
        }
        if let Some(port) = d.and_then(|d| d.get("port")).and_then(Value::as_i64)
            && port != 0
        {
            let _ = write!(line, " port={port}");
        }
        if let Some(mon) = non_empty_str(d.and_then(|d| d.get("monitor"))) {
            let _ = write!(line, " monitor={mon}");
        }
        lines.push(line);
    }
    lines
}

fn or_none(items: Vec<String>) -> Vec<String> {
    if items.is_empty() {
        vec!["(none)".to_owned()]
    } else {
        items
    }
}

/// Count non-blank, non-comment lines.
fn count_irule_lines(body: &str) -> usize {
    body.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .count()
}

#[derive(Serialize)]
struct SectionJson {
    heading: &'static str,
    lines: Vec<String>,
}

#[derive(Serialize)]
struct ExplainJson {
    target: String,
    kind: String,
    found: bool,
    sections: Vec<SectionJson>,
}

pub(crate) struct ExplainReport {
    target: String,
    kind: String,
    found: bool,
    sections: Vec<Section>,
}

pub(crate) fn compute_explain(
    cfg: &BigipConfig,
    target: &str,
    kind_hint: Option<&str>,
) -> ExplainReport {
    let model = Model::build(cfg);

    let resolved = if kind_hint.is_none() || kind_hint == Some("virtual") {
        model
            .resolve("virtual", target)
            .and_then(|p| model.get("virtual", &p).cloned())
            .map(|vs| ("virtual", vs))
    } else {
        None
    }
    .or_else(|| {
        if kind_hint.is_none() || kind_hint == Some("pool") {
            model
                .resolve("pool", target)
                .and_then(|p| model.get("pool", &p).cloned())
                .map(|pool| ("pool", pool))
        } else {
            None
        }
    });

    let Some((actual_kind, obj)) = resolved else {
        return ExplainReport {
            target: target.to_owned(),
            kind: kind_hint.unwrap_or("?").to_owned(),
            found: false,
            sections: Vec::new(),
        };
    };

    let sections = if actual_kind == "virtual" {
        explain_virtual(&model, &obj)
    } else {
        vec![("pool", explain_pool_lines(&obj))]
    };

    ExplainReport {
        target: target.to_owned(),
        kind: actual_kind.to_owned(),
        found: true,
        sections,
    }
}

pub(crate) fn full_path_of(
    report: &ExplainReport,
    cfg: &BigipConfig,
    kind_hint: Option<&str>,
) -> String {
    // Re-resolve to recover the matched object's full_path for the header.
    let model = Model::build(cfg);
    let target = &report.target;
    if (kind_hint.is_none() || kind_hint == Some("virtual"))
        && report.kind == "virtual"
        && let Some(p) = model.resolve("virtual", target)
    {
        return p;
    }
    if report.kind == "pool"
        && let Some(p) = model.resolve("pool", target)
    {
        return p;
    }
    target.clone()
}

pub(crate) fn format_text(report: &ExplainReport, full_path: &str) -> String {
    if !report.found {
        return format!("no virtual or pool found for {}", py_repr(&report.target));
    }
    let mut lines: Vec<String> = vec![
        format!("explain {}: {full_path}", report.kind),
        String::new(),
    ];
    for (heading, items) in &report.sections {
        lines.push(format!("  {heading}:"));
        for item in items {
            lines.push(format!("    {item}"));
        }
        lines.push(String::new());
    }
    lines.join("\n").trim_end().to_owned()
}

/// `f5 explain`.
pub fn run_explain(
    kind: &str,
    target: &str,
    inputs: &[std::path::PathBuf],
    json: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let kind_hint = if kind == "auto" { None } else { Some(kind) };

    // Resolve inputs via the UCS-aware loader, so a `.ucs` — plain or encrypted
    // — is transparently extracted to SCF and parsed just like a `.conf`/`.scf`.
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let paths: Vec<String> = inputs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let configs: Vec<BigipConfig> = tcl_bigip_io::load_paths(&paths, &opts)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .into_iter()
        .map(|loaded| loaded.config)
        .collect();

    // Use the first config that resolves the target; otherwise the first.
    let mut chosen = 0usize;
    let mut report = compute_explain(&configs[0], target, kind_hint);
    for (i, cfg) in configs.iter().enumerate() {
        let r = compute_explain(cfg, target, kind_hint);
        if r.found {
            chosen = i;
            report = r;
            break;
        }
    }

    let rendered = if json {
        let payload = ExplainJson {
            target: report.target.clone(),
            kind: report.kind.clone(),
            found: report.found,
            sections: report
                .sections
                .iter()
                .map(|(h, lines)| SectionJson {
                    heading: h,
                    lines: lines.clone(),
                })
                .collect(),
        };
        format!(
            "{}\n",
            tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&payload)?)
        )
    } else {
        let full_path = full_path_of(&report, &configs[chosen], kind_hint);
        let mut text = format_text(&report, &full_path);
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text
    };

    let target_out = OutputTarget::from_arg(output);
    write_text_output(&target_out, &rendered)?;
    Ok(u8::from(!report.found))
}

/// Render a string in repr form (only the simple, quote-free path
/// is needed for the not-found message).
fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    format!("{quote}{s}{quote}")
}
