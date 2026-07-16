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

//! Canonical JSON snapshot of the command registry per dialect.
//!
//! Drives the `tcl registry-dump` verb. The output is deterministic
//! JSON serialised with two-space indentation and sorted keys.
//!
//! **iRules note:** the per-command `info.validEvents*` fields embed the
//! event-validity cross-product, which only applies to the `f5-irules`
//! dialect (every Tcl command resolves to an empty valid-event set). The
//! Tcl path therefore emits the constant empty-list count/digest.

use std::collections::BTreeMap;

use tcl_dialect::DialectProfile;

use crate::arity::Arity;
use crate::body_kind::BodyKind;
use crate::dialects::DialectSet;
use crate::hover::FormKind;
use crate::profile_queries::ProfileQueries;
use crate::registry::CommandRegistry;
use crate::side_effects::StorageType;
use crate::snapshot::Json;
use crate::spec::{CommandSpec, SubCommand};
use crate::traits::Traits;

/// `sha256` of the empty string — the digest of an empty list. Every Tcl
/// command's `valid_events` set is empty, so this constant is the only
/// digest the Tcl path emits.
const EMPTY_LIST_DIGEST: &str =
    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// The boolean `CommandSpec` trait fields backed by a [`Traits`] bit, paired
/// with their field name (the snapshot's `traits` keys). The spec folds
/// these boolean fields into the [`Traits`] bitflags.
const TRAIT_FLAGS: &[(&str, Traits)] = &[
    ("byte_compiled", Traits::BYTE_COMPILED),
    ("configures_channel", Traits::CONFIGURES_CHANNEL),
    ("creates_dynamic_barrier", Traits::CREATES_DYNAMIC_BARRIER),
    ("creates_scope_alias", Traits::CREATES_SCOPE_ALIAS),
    ("cse_candidate", Traits::CSE_CANDIDATE),
    ("defines_procedure", Traits::DEFINES_PROCEDURE),
    ("destroys_variable", Traits::DESTROYS_VARIABLE),
    ("diagram_action", Traits::DIAGRAM_ACTION),
    ("evaluates_code", Traits::EVALUATES_CODE),
    ("frameless_runtime", Traits::FRAMELESS_RUNTIME),
    ("has_boolean_condition", Traits::HAS_BOOLEAN_COND),
    ("has_destructive_ops", Traits::HAS_DESTRUCTIVE_OPS),
    ("has_interp_eval", Traits::HAS_INTERP_EVAL),
    ("has_loop_body", Traits::HAS_LOOP_BODY),
    (
        "has_string_list_confusion_risk",
        Traits::STRING_LIST_CONFUSION,
    ),
    ("has_switch_body", Traits::HAS_SWITCH_BODY),
    ("irules_top_level_only", Traits::IRULES_TOP_LEVEL_ONLY),
    ("is_control_flow", Traits::CONTROL_FLOW),
    ("is_irules_event_handler", Traits::IS_EVENT_HANDLER),
    ("is_language_keyword", Traits::LANGUAGE_KEYWORD),
    ("is_oo_metaclass", Traits::IS_OO_METACLASS),
    ("is_side_switch", Traits::IS_SIDE_SWITCH),
    ("is_unescape_command", Traits::IS_UNESCAPE),
    (
        "is_unnormalized_http_getter",
        Traits::UNNORMALISED_HTTP_GETTER,
    ),
    ("loop_list_header", Traits::LOOP_LIST_HEADER),
    ("needs_start_cmd", Traits::NEEDS_START_CMD),
    ("never_inline_body", Traits::NEVER_INLINE_BODY),
    ("not_proc_factory", Traits::NOT_PROC_FACTORY),
    ("opens_channel", Traits::OPENS_CHANNEL),
    ("password_option_command", Traits::PASSWORD_OPTION),
    ("performs_substitution", Traits::PERFORMS_SUBSTITUTION),
    ("produces_canonical_list", Traits::PRODUCES_CANONICAL_LIST),
    ("pure", Traits::PURE),
    ("pure_evaluation", Traits::PURE_EVALUATION),
    ("reads_variable_before_write", Traits::READS_BEFORE_WRITE),
    ("returns_path", Traits::RETURNS_PATH),
    ("sources_file", Traits::SOURCES_FILE),
    ("taint_sink", Traits::TAINT_SINK),
    ("terminates_block", Traits::TERMINATES_BLOCK),
    ("unsafe", Traits::UNSAFE),
    ("wasm_emits_nothing", Traits::WASM_EMITS_NOTHING),
];

// `CommandSpec.dialects` serialisation derives from
// `DialectSet::member_names` — the same canonical-name table `parse`
// inverts — rather than a parallel hand-list. A hand-list here missed
// `tcl9.1`/`bpf` once (RUST_ISSUE_082: a `TCL90_PLUS` spec dropped its 9.1
// membership and a BPF-only spec serialised `[]`, indistinguishable from
// "available nowhere") and the Milestone 6 `TMSH`/`BIGIP` bits a second
// time, so new primitive bits can never be forgotten again.

/// Serialise an [`Arity`] as `{"min", "max"}` (`max` null = unbounded).
fn arity_json(arity: Arity) -> Json {
    let mut m = BTreeMap::new();
    m.insert("min".to_owned(), Json::Int(i64::from(arity.min)));
    m.insert(
        "max".to_owned(),
        if arity.is_unlimited() {
            Json::Null
        } else {
            Json::Int(i64::from(arity.max))
        },
    );
    Json::Object(m)
}

/// The spec's dialect set: `null` for "all dialects", else the sorted
/// list of canonical dialect-name strings (every primitive bit the set
/// carries, via `DialectSet::member_names`).
fn dialects_json(dialects: Option<DialectSet>) -> Json {
    match dialects {
        None => Json::Null,
        Some(set) => {
            let mut names = set.member_names();
            names.sort_unstable();
            Json::Array(names.into_iter().map(Json::s).collect())
        }
    }
}

/// Whether the subcommand is available under `profile` (own gate wins;
/// else inherit the parent `CommandSpec.dialects`; else available) — the
/// same §5.1 intersects membership every other availability consumer uses
/// (the old `contains` rule hid a vendor profile's embedded-core
/// subcommands from the dump).
fn sub_available(profile: &DialectProfile, sub: &SubCommand, parent: Option<DialectSet>) -> bool {
    sub.dialects
        .or(parent)
        .is_none_or(|gate| gate.intersects(profile.availability_mask))
}

/// Sorted union of every option name declared on `spec` (no dialect
/// filter) — the sorted `switch_names()`, or for a single form the
/// sorted option names.
fn all_option_names(spec: &CommandSpec) -> Vec<&'static str> {
    let mut names = spec.switch_names(None);
    names.sort_unstable();
    names
}

/// The `forms` block. Per-form `arity` is always `null` and
/// `pure`/`mutator` always `false` for the Tcl dialects; the only
/// non-trivial per-form field is `options`, which (for the single-form
/// common case) is the command's full option set.
fn forms_json(spec: &CommandSpec) -> Json {
    let single = spec.forms.len() == 1;
    let opts = all_option_names(spec);
    let forms = spec
        .forms
        .iter()
        .map(|form| {
            let mut m = BTreeMap::new();
            m.insert("kind".to_owned(), Json::s(form_kind_value(form.kind)));
            m.insert("synopsis".to_owned(), Json::s(form.synopsis));
            m.insert("arity".to_owned(), Json::Null);
            // Single-form commands carry all their options on that form.
            let form_opts = if single {
                opts.iter().map(|o| Json::s(*o)).collect()
            } else {
                Vec::new()
            };
            m.insert("options".to_owned(), Json::Array(form_opts));
            m.insert("pure".to_owned(), Json::Bool(false));
            m.insert("mutator".to_owned(), Json::Bool(false));
            Json::Object(m)
        })
        .collect();
    Json::Array(forms)
}

/// `FormKind.value` — the lowercase wire form.
fn form_kind_value(kind: FormKind) -> &'static str {
    match kind {
        FormKind::Default => "default",
        FormKind::Getter => "getter",
        FormKind::Setter => "setter",
    }
}

/// `BodyKind` → enum `.name` (`INLINE` / `STRUCTURAL`).
fn body_kind_name(kind: BodyKind) -> &'static str {
    match kind {
        BodyKind::Plain => "INLINE",
        BodyKind::Structural => "STRUCTURAL",
    }
}

/// `StorageType` → enum `.name`.
fn storage_type_name(t: StorageType) -> &'static str {
    match t {
        StorageType::Dict => "DICT",
        StorageType::List => "LIST",
        StorageType::Array => "ARRAY",
    }
}

/// The per-command `scalars` block (non-boolean trait fields).
fn scalars_json(spec: &CommandSpec) -> Json {
    let mut m = BTreeMap::new();
    m.insert(
        "allow_unknown_subcommands".to_owned(),
        Json::Bool(spec.allow_unknown_subcommands),
    );
    m.insert(
        "assigns_variable_at".to_owned(),
        spec.assigns_variable_at
            .map_or(Json::Null, |v| Json::Int(i64::from(v))),
    );
    m.insert(
        "body_arg_implicit_args".to_owned(),
        Json::Int(i64::from(spec.body_arg_implicit_args)),
    );
    m.insert(
        "body_kind".to_owned(),
        Json::s(body_kind_name(spec.body_kind)),
    );
    m.insert(
        "deprecated_replacement".to_owned(),
        spec.deprecated_replacement.map_or(Json::Null, Json::s),
    );
    m.insert(
        "format_string_type".to_owned(),
        spec.format_string_type
            .map_or(Json::Null, |t| Json::s(t.as_str())),
    );
    m.insert(
        "inferred_storage_type".to_owned(),
        spec.inferred_storage_type
            .map_or(Json::Null, |t| Json::s(storage_type_name(t))),
    );
    m.insert(
        "pattern_type".to_owned(),
        spec.pattern_type
            .map_or(Json::Null, |t| Json::s(t.as_str())),
    );
    m.insert(
        "required_package".to_owned(),
        spec.required_package.map_or(Json::Null, Json::s),
    );
    m.insert(
        "tcllib_package".to_owned(),
        spec.tcllib_package.map_or(Json::Null, Json::s),
    );
    Json::Object(m)
}

/// The per-command `traits` block: every boolean `CommandSpec` field with
/// its name. `xc_translatable` is an optional boolean, so it is
/// emitted only when set.
fn traits_json(spec: &CommandSpec) -> Json {
    let mut m = BTreeMap::new();
    for (name, flag) in TRAIT_FLAGS {
        m.insert((*name).to_owned(), Json::Bool(spec.traits.contains(*flag)));
    }
    m.insert(
        "allow_unknown_subcommands".to_owned(),
        Json::Bool(spec.allow_unknown_subcommands),
    );
    m.insert(
        "is_namespace_exported".to_owned(),
        Json::Bool(spec.is_namespace_exported),
    );
    m.insert(
        "warn_missing_import".to_owned(),
        Json::Bool(spec.warn_missing_import),
    );
    if let Some(v) = spec.xc_translatable {
        m.insert("xc_translatable".to_owned(), Json::Bool(v));
    }
    Json::Object(m)
}

/// The `subcommands` block: subcommands available under `profile`, sorted
/// by name.
fn subcommands_json(spec: &CommandSpec, profile: &DialectProfile) -> Json {
    let mut subs: Vec<&SubCommand> = spec
        .subcommands
        .iter()
        .filter(|sub| sub_available(profile, sub, spec.dialects))
        .collect();
    subs.sort_by(|a, b| a.name.cmp(b.name));
    let out = subs
        .into_iter()
        .map(|sub| {
            let mut opts: Vec<&str> = sub.options.iter().map(|o| o.name).collect();
            opts.sort_unstable();
            let mut m = BTreeMap::new();
            m.insert("name".to_owned(), Json::s(sub.name));
            m.insert("arity".to_owned(), arity_json(sub.arity));
            m.insert("pure".to_owned(), Json::Bool(sub.pure));
            m.insert("mutator".to_owned(), Json::Bool(sub.mutator));
            m.insert("destructive".to_owned(), Json::Bool(sub.destructive));
            // The Rust `SubCommand` carries no deprecation field (no Tcl
            // subcommand declares one); always `false` / `null`.
            m.insert("deprecated".to_owned(), Json::Bool(false));
            m.insert("deprecatedReplacement".to_owned(), Json::Null);
            m.insert(
                "options".to_owned(),
                Json::Array(opts.into_iter().map(Json::s).collect()),
            );
            m.insert("returnsPath".to_owned(), Json::Bool(sub.returns_path));
            Json::Object(m)
        })
        .collect();
    Json::Array(out)
}

/// `info` block — the shape the `command-info --json` front-end emits.
fn info_json(spec: &CommandSpec) -> Json {
    let summary = spec.hover.map_or("", |h| h.summary);
    let synopsis: Vec<Json> = spec
        .hover
        .map(|h| h.synopsis.iter().map(|s| Json::s(*s)).collect())
        .unwrap_or_default();
    let switches: Vec<Json> = all_option_names(spec).into_iter().map(Json::s).collect();
    let mut m = BTreeMap::new();
    m.insert("summary".to_owned(), Json::s(summary));
    m.insert("synopsis".to_owned(), Json::Array(synopsis));
    m.insert("switches".to_owned(), Json::Array(switches));
    // Every Tcl command's valid-event set is empty (the cross-product is an
    // f5-irules-only fact), so the count/digest are constant.
    m.insert("validEventCount".to_owned(), Json::Int(0));
    m.insert("validEventsDigest".to_owned(), Json::s(EMPTY_LIST_DIGEST));
    m.insert("validInAnyEvent".to_owned(), Json::Bool(false));
    Json::Object(m)
}

/// Full structured snapshot of a single command under `profile`.
fn command_entry(spec: &CommandSpec, profile: &DialectProfile) -> Json {
    let mut switches: Vec<&str> = profile.available_option_names(spec);
    // Top-level `switches` preserves declaration order — no sort.
    let switches_json: Vec<Json> = switches.drain(..).map(Json::s).collect();

    let mut excluded: Vec<&str> = spec.excluded_events.to_vec();
    excluded.sort_unstable();

    let mut m = BTreeMap::new();
    m.insert("name".to_owned(), Json::s(spec.name));
    m.insert("dialects".to_owned(), dialects_json(spec.dialects));
    m.insert("arity".to_owned(), arity_json(spec.arity));
    m.insert("switches".to_owned(), Json::Array(switches_json));
    m.insert("subcommands".to_owned(), subcommands_json(spec, profile));
    m.insert("forms".to_owned(), forms_json(spec));
    m.insert(
        "excludedEvents".to_owned(),
        Json::Array(excluded.into_iter().map(Json::s).collect()),
    );
    m.insert("traits".to_owned(), traits_json(spec));
    m.insert("scalars".to_owned(), scalars_json(spec));
    m.insert("info".to_owned(), info_json(spec));
    Json::Object(m)
}

/// Sorted command names available under `profile` (no package filtering:
/// with no active-package set, every package-gated command is visible).
fn command_names(registry: &CommandRegistry, profile: &DialectProfile) -> Vec<String> {
    let mut names: Vec<String> = registry
        .command_names()
        .filter(|name| profile.resolve_command(registry, name).is_some())
        .map(str::to_owned)
        .collect();
    names.sort_unstable();
    names
}

/// The full snapshot entry for a single `name` in `dialect`, or `None`
/// when the command is unavailable. Used by the golden test to compare
/// individual command entries.
#[must_use]
pub fn command_entry_json(registry: &CommandRegistry, dialect: &str, name: &str) -> Option<Json> {
    let profile = DialectProfile::by_name(dialect);
    profile
        .resolve_command(registry, name)
        .map(|spec| command_entry(spec, profile))
}

/// Snapshot of every command available in `dialect`.
///
/// Dialect resolution goes through the profile catalog: an unknown dialect
/// string dumps the permissive `PLAIN_TCL` (`ALL_TCL`) view — the one
/// unified fallback (design doc §8) — rather than the old ad-hoc `TCL86`.
#[must_use]
pub fn command_registry_snapshot(registry: &CommandRegistry, dialect: &str) -> Json {
    let profile = DialectProfile::by_name(dialect);
    let names = command_names(registry, profile);
    let commands: Vec<Json> = names
        .iter()
        .filter_map(|name| {
            profile
                .resolve_command(registry, name)
                .map(|spec| command_entry(spec, profile))
        })
        .collect();
    let mut m = BTreeMap::new();
    m.insert("dialect".to_owned(), Json::s(dialect));
    m.insert(
        "commandCount".to_owned(),
        Json::Int(i64::try_from(commands.len()).unwrap_or(i64::MAX)),
    );
    m.insert("commands".to_owned(), Json::Array(commands));
    Json::Object(m)
}

/// Multi-dialect snapshot.
#[must_use]
pub fn command_registry_snapshots(registry: &CommandRegistry, dialects: &[&str]) -> Json {
    let mut by_dialect = BTreeMap::new();
    for dialect in dialects {
        by_dialect.insert(
            (*dialect).to_owned(),
            command_registry_snapshot(registry, dialect),
        );
    }
    let mut m = BTreeMap::new();
    m.insert("schema".to_owned(), Json::s("tcl-lsp/registry/commands/v1"));
    m.insert("dialects".to_owned(), Json::Object(by_dialect));
    Json::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rendered dialect-name strings for a set.
    fn names(set: DialectSet) -> Vec<String> {
        match dialects_json(Some(set)) {
            Json::Array(items) => items
                .into_iter()
                .filter_map(|j| match j {
                    Json::Str(s) => Some(s),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn tcl90_plus_includes_tcl91() {
        // RUST_ISSUE_082: a TCL90_PLUS spec must serialise BOTH tcl9.0 and
        // tcl9.1 — 9.1 was silently dropped.
        let n = names(DialectSet::TCL90_PLUS);
        assert!(n.contains(&"tcl9.0".to_owned()), "{n:?}");
        assert!(n.contains(&"tcl9.1".to_owned()), "{n:?}");
    }

    #[test]
    fn bpf_only_spec_is_not_empty() {
        // RUST_ISSUE_082: a BPF-only spec serialised `[]` (looks like
        // "available nowhere"); it must render `["bpf"]`.
        assert_eq!(names(DialectSet::BPF), vec!["bpf".to_owned()]);
    }
}
