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

//! Predicates + allow-lists used by var-escape transfer functions:
//!
//! * [`is_literal_name`] / [`is_dynamic_token`] / [`is_dynamic_name`]
//!   — string-shape predicates for argument tokens and lowering's
//!   preserved name fields.
//! * [`is_frameless_runtime_command`] — audited allow-list of
//!   commands the codegen dispatches via dedicated runtime helpers
//!   (no eval-fallback), sourced from the registry's
//!   `FRAMELESS_RUNTIME` trait.
//! * [`is_name_first_command`] — the commands whose first arg is the
//!   variable name (registry `FIRST_ARG_VARNAME` trait), used by the
//!   dynamic-name-first detector.
//! * [`scan_value_for_info_hazards`] — find embedded
//!   `[info <subcmd> ...]` substitutions inside a value string.

use std::collections::HashSet;
use std::sync::OnceLock;

use tcl_registry::CommandRegistry;
use tcl_registry::InvocationFacts;
use tcl_registry::prelude::Traits;

use crate::ir::{CommandTokens, Statement};
use crate::registry_invocation::{RegistryInvocationResolution, resolve_command_tokens};
use crate::var_escape::info_subcommands::{
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand,
};

/// Memoised set of commands carrying the registry's
/// [`Traits::FIRST_ARG_VARNAME`] trait — the commands whose first arg is
/// the variable *name* (`set` / `incr` / `append` / `lappend` / `lset` /
/// `unset`). Used to detect dynamic-name forms like `set $n value`. Sourced from
/// the registry (the single source of truth); cached once because the set
/// is dialect-agnostic (all entries are core Tcl commands).
fn name_first_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .commands_with_trait(Traits::FIRST_ARG_VARNAME)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Memoised set of commands carrying the registry's
/// [`Traits::FRAMELESS_RUNTIME`] trait — the audited allow-list of
/// commands whose codegen always uses a dedicated runtime helper (or
/// a structured IR node) and never falls back to the interpreter, so
/// a call to one needs no runtime frame in the callee.
///
/// Sourced from the registry (the single source of truth) rather than
/// a hardcoded copy; cached once because the set is dialect-agnostic
/// (all entries are core Tcl commands present in `build_default`).
fn frameless_runtime_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .commands_with_trait(Traits::FRAMELESS_RUNTIME)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Compatibility registry for dialect-blind var-escape entry points.
///
/// Tcl 8.6 is the compiler's documented plain-Tcl baseline. Unlike an owned
/// `build_default()` registry, this cached registry carries an explicit
/// profile, so structured subcommand/form resolution is never surface-blind.
/// Production callers should pass their selected registry via
/// the `*_with_registry` APIs instead.
pub(crate) fn default_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
}

/// Resolve a compiler statement's source-safe word snapshot to the registry's
/// target-neutral invocation facts.
///
/// This deliberately has no fallback through `Statement::{Call,Barrier}`'s
/// lossy `command`/`args` strings: substituted and expanded words must not be
/// reinterpreted as literal Tcl values by an analysis pass.
pub(crate) fn invocation_facts(
    stmt: &Statement,
    registry: &CommandRegistry,
) -> Option<Box<InvocationFacts>> {
    let (Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. }) = stmt else {
        return None;
    };
    let Some(tokens) = tokens else {
        return None;
    };
    invocation_facts_from_tokens(tokens, registry)
}

/// Resolve an ordered source-word snapshot to target-neutral registry facts.
pub(crate) fn invocation_facts_from_tokens(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<Box<InvocationFacts>> {
    // The store's own profile *is* an already-resolved environment id, so
    // this is the id-keyed context for the registry the caller handed us
    // (ledger C1 re-key).
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    match resolve_command_tokens(registry, context, tokens).ok()? {
        RegistryInvocationResolution::Resolved(facts) => Some(facts),
        RegistryInvocationResolution::Unresolved(_) => None,
    }
}

/// True if *arg* is a plain identifier, not a substituted ref.
/// Applies the same "starts with `$` or `[`" filter used throughout
/// the memory-SSA alias detectors.
#[must_use]
pub fn is_literal_name(arg: &str) -> bool {
    if arg.is_empty() {
        return false;
    }
    !arg.starts_with('$') && !arg.starts_with('[')
}

/// True if *arg* contains a runtime substitution.
#[must_use]
pub fn is_dynamic_token(arg: &str) -> bool {
    arg.starts_with('$') || arg.starts_with('[') || arg.contains('$') || arg.contains('[')
}

/// True if *head* is the level argument of `upvar` and is
/// potentially dynamic at runtime.
///
/// Any non-literal head shape —
/// `$var`, `[cmd]`, double-quoted `"..."` containing `$` or `[`, or
/// `#`-absolute forms with embedded substitution like `"#${n}"` /
/// `"#[expr ...]"` — must trigger the pessimistic-spill path.
/// Recognising only `$var` let `upvar [expr {$n}] ...` /
/// `upvar "#${n}" ...` / `upvar $::ns::level ...` callsites bypass
/// the pessimism, leaving caller-side mirrors stale after an alias
/// write through the dynamic level.
///
/// The caller is expected to invoke this only when the registry's
/// `FrameLevelWord::ArityParity` layout says a level word is present; this
/// helper only classifies the dynamic shape.
#[must_use]
pub fn is_dynamic_upvar_level(head: &str) -> bool {
    head.starts_with('$')
        || head.starts_with('[')
        || (head.starts_with('"') && (head.contains('$') || head.contains('[')))
        || (head.starts_with('#') && (head.contains('$') || head.contains('[')))
}

/// True if *name* is an empty or interpolated variable name.
///
/// Lowering preserves dynamic names as e.g. `${user_var}` literal
/// text in the `name` field of `Statement::AssignValue` / `Incr`.
/// Treat both the empty-name form and any substituted form as
/// dynamic.
#[must_use]
pub fn is_dynamic_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    is_dynamic_token(name)
}

/// Strip a leading `::` from a substitution head for allow-list
/// lookup. The frameless allow-list keys commands by their bare
/// identifier (`set`, `list`, …); a substitution writing
/// `[::set foo 1]` is the same command, just qualified.
#[must_use]
pub fn normalise_cmd_subst_head(head: &str) -> &str {
    head.strip_prefix("::").unwrap_or(head)
}

/// True if *cmd* is in the audited frameless-runtime allow-list
/// (the registry's [`Traits::FRAMELESS_RUNTIME`] commands).
#[must_use]
pub fn is_frameless_runtime_command(cmd: &str) -> bool {
    frameless_runtime_set().contains(cmd)
}

/// True if *cmd* is one of the five `set` / `incr` / `append` /
/// `lappend` / `unset` commands whose first arg is a variable
/// name.
#[must_use]
pub fn is_name_first_command(cmd: &str) -> bool {
    name_first_set().contains(cmd)
}

/// Scan a value string for embedded `[info <subcmd> ...]`
/// substitutions. Returns `(pessimistic, escape_names)`:
///
/// * *`pessimistic`* — true when any match triggers the
///   frame-inspecting rule (`info level` / `frame` / `vars` /
///   `locals` or `info exists $dynamic`).
/// * *`escape_names`* — names referenced by `info exists
///   <literal>` (the caller should escape each).
///
/// Implemented as a hand-rolled scanner to avoid pulling in the
/// `regex` crate for a single pattern.
#[must_use]
pub fn scan_value_for_info_hazards(value: &str) -> (bool, Vec<String>) {
    let mut escape_names: Vec<String> = Vec::new();
    if !value.contains('[') || !value.contains("info") {
        return (false, escape_names);
    }
    let bytes = value.as_bytes();
    let mut i = 0usize;
    let n = bytes.len();
    while i < n {
        // Find next ``[``.
        let Some(open_off) = value[i..].find('[') else {
            break;
        };
        let open = i + open_off;
        // Skip whitespace after ``[``.
        let mut j = open + 1;
        while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        // Match literal ``info``.
        if !value[j..].starts_with("info") {
            i = open + 1;
            continue;
        }
        j += "info".len();
        // Require whitespace after ``info``.
        if j >= n || (bytes[j] != b' ' && bytes[j] != b'\t') {
            i = open + 1;
            continue;
        }
        while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        // Capture the subcommand word (alphanumeric / underscore).
        let sub_start = j;
        while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
            j += 1;
        }
        if j == sub_start {
            i = open + 1;
            continue;
        }
        let sub = &value[sub_start..j];
        // Capture the rest until the matching ``]``.
        let mut rest_start = j;
        while rest_start < n && (bytes[rest_start] == b' ' || bytes[rest_start] == b'\t') {
            rest_start += 1;
        }
        let close_off = value[rest_start..].find(']');
        let close = match close_off {
            Some(off) => rest_start + off,
            None => break,
        };
        let rest = value[rest_start..close].trim();

        if is_frame_inspecting_info_subcommand(sub) {
            return (true, escape_names);
        }
        if sub == "exists" {
            // ``info exists name`` — escape *name* if literal,
            // else pessimistic.
            if rest.is_empty() {
                return (true, escape_names);
            }
            let target = rest;
            if is_dynamic_token(target) {
                return (true, escape_names);
            }
            escape_names.push(target.to_string());
        } else if !is_safe_info_subcommand(sub) {
            // Unknown subcommand — be conservative.
            return (true, escape_names);
        }
        i = close + 1;
    }
    (false, escape_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_name_rejects_substituted_tokens() {
        assert!(is_literal_name("foo"));
        assert!(is_literal_name("foo_bar"));
        assert!(!is_literal_name("$foo"));
        assert!(!is_literal_name("[cmd]"));
        assert!(!is_literal_name(""));
    }

    #[test]
    fn dynamic_token_detects_dollar_or_bracket() {
        assert!(is_dynamic_token("$x"));
        assert!(is_dynamic_token("[expr 1+1]"));
        assert!(is_dynamic_token("foo$bar"));
        assert!(is_dynamic_token("foo[cmd]"));
        assert!(!is_dynamic_token("foo"));
        assert!(!is_dynamic_token(""));
    }

    #[test]
    fn dynamic_upvar_level_recognises_all_substitution_shapes() {
        // ``$var`` and ``${var}``.
        assert!(is_dynamic_upvar_level("$lvl"));
        assert!(is_dynamic_upvar_level("${lvl}"));
        assert!(is_dynamic_upvar_level("$::ns::level"));
        // ``[cmd subst]``.
        assert!(is_dynamic_upvar_level("[expr {$n}]"));
        assert!(is_dynamic_upvar_level("[foo]"));
        // Double-quoted strings carrying ``$`` / ``[``.
        assert!(is_dynamic_upvar_level("\"#${n}\""));
        assert!(is_dynamic_upvar_level("\"[expr {$n}]\""));
        // ``#``-absolute forms with embedded substitution.
        assert!(is_dynamic_upvar_level("#$n"));
        assert!(is_dynamic_upvar_level("#${n}"));
        assert!(is_dynamic_upvar_level("#[foo]"));
        // Non-substituted shapes — not flagged here. Callers gate
        // literal integers / ``#N`` separately via
        // ``is_level_literal``.
        assert!(!is_dynamic_upvar_level("1"));
        assert!(!is_dynamic_upvar_level("#0"));
        assert!(!is_dynamic_upvar_level("\"plain\""));
        assert!(!is_dynamic_upvar_level(""));
    }

    #[test]
    fn dynamic_name_includes_empty() {
        assert!(is_dynamic_name(""));
        assert!(is_dynamic_name("$x"));
        assert!(!is_dynamic_name("plain_name"));
    }

    #[test]
    fn normalise_cmd_subst_head_strips_leading_colons() {
        assert_eq!(normalise_cmd_subst_head("::set"), "set");
        assert_eq!(normalise_cmd_subst_head("set"), "set");
        assert_eq!(normalise_cmd_subst_head("::ns::cmd"), "ns::cmd");
    }

    #[test]
    fn frameless_runtime_command_membership() {
        assert!(is_frameless_runtime_command("list"));
        assert!(is_frameless_runtime_command("string"));
        assert!(is_frameless_runtime_command("expr"));
        assert!(is_frameless_runtime_command("set"));
        assert!(!is_frameless_runtime_command("foreach"));
        assert!(!is_frameless_runtime_command("proc"));
    }

    #[test]
    fn name_first_command_is_the_documented_six() {
        for cmd in ["set", "incr", "append", "lappend", "lset", "unset"] {
            assert!(is_name_first_command(cmd), "{cmd}");
        }
        assert!(!is_name_first_command("string"));
        assert!(!is_name_first_command("list"));
    }

    #[test]
    fn scan_value_finds_info_exists_literal() {
        let (pess, names) = scan_value_for_info_hazards("foo [info exists bar] baz");
        assert!(!pess);
        assert_eq!(names, vec!["bar".to_string()]);
    }

    #[test]
    fn scan_value_dynamic_info_exists_is_pessimistic() {
        let (pess, names) = scan_value_for_info_hazards("[info exists $dyn]");
        assert!(pess);
        assert!(names.is_empty());
    }

    #[test]
    fn scan_value_frame_inspecting_is_pessimistic() {
        let (pess, _) = scan_value_for_info_hazards("[info level]");
        assert!(pess);
        let (pess, _) = scan_value_for_info_hazards("[info frame]");
        assert!(pess);
        let (pess, _) = scan_value_for_info_hazards("[info vars]");
        assert!(pess);
    }

    #[test]
    fn scan_value_safe_info_does_not_escape() {
        let (pess, names) = scan_value_for_info_hazards("[info patchlevel]");
        assert!(!pess);
        assert!(names.is_empty());
    }

    #[test]
    fn scan_value_no_info_means_no_hazard() {
        let (pess, names) = scan_value_for_info_hazards("plain text");
        assert!(!pess);
        assert!(names.is_empty());
        let (pess, names) = scan_value_for_info_hazards("[other_cmd arg]");
        assert!(!pess);
        assert!(names.is_empty());
    }

    #[test]
    fn scan_value_unknown_info_subcommand_is_pessimistic() {
        // Unknown ``info`` subcommands are conservative — pessimistic.
        let (pess, _) = scan_value_for_info_hazards("[info totally_made_up]");
        assert!(pess);
    }
}
