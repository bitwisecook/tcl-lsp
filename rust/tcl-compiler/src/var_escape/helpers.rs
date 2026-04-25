//! Predicates + allow-lists used by var-escape transfer functions
//! (C33b1).
//!
//! Mirrors the helper section of
//! `core/compiler/var_escape/_propagation.py`:
//!
//! * [`is_literal_name`] / [`is_dynamic_token`] / [`is_dynamic_name`]
//!   — string-shape predicates for argument tokens and lowering's
//!   preserved name fields.
//! * [`FRAMELESS_RUNTIME_COMMANDS`] — audited allow-list of commands
//!   the codegen dispatches via dedicated runtime helpers (no
//!   eval-fallback).
//! * [`NAME_FIRST_COMMANDS`] — the five commands whose first arg is
//!   the variable name (used by the dynamic-name-first detector).
//! * [`scan_value_for_info_hazards`] — find embedded
//!   `[info <subcmd> ...]` substitutions inside a value string.

use crate::var_escape::info_subcommands::{
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand,
};

/// Commands whose first arg is the variable name (read / write /
/// modify). Used to detect dynamic-name forms like `set $n value`.
pub const NAME_FIRST_COMMANDS: &[&str] = &["set", "incr", "append", "lappend", "unset"];

/// Commands whose codegen always uses a dedicated runtime helper
/// and never falls back to the interpreter. Calls to these
/// commands do **not** require a runtime frame on the WASM side —
/// they're straight-line primitives with no `$var` resolution
/// needed at the callee.
///
/// Keep this list audited: adding a command that secretly falls
/// back will break eval-inside-proc semantics in escape-free
/// procs. Mirrors Python's `_FRAMELESS_RUNTIME_COMMANDS`.
pub const FRAMELESS_RUNTIME_COMMANDS: &[&str] = &[
    // List primitives.
    "list",
    "lindex",
    "lrange",
    "linsert",
    "llength",
    "lsort",
    "lsearch",
    "lappend",
    "lreverse",
    "lreplace",
    "lrepeat",
    "lassign",
    "concat",
    // String primitives.
    "split",
    "join",
    "string",
    // Arithmetic wrappers.
    "expr",
    // Locally-handled scope declarations — no runtime frame needed.
    "global",
    "variable",
    "upvar",
    "namespace",
    // Self-assignment shapes already covered by IRAssign* / IRIncr.
    "set",
    "incr",
    "append",
    "unset",
    // I/O and diagnostics.
    "puts",
    "return",
    "error",
    "continue",
    "break",
];

/// True if *arg* is a plain identifier, not a substituted ref.
/// Mirrors the "starts with `$` or `[`" filter used throughout the
/// memory-SSA alias detectors.
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

/// True if *cmd* is in the audited frameless-runtime allow-list.
#[must_use]
pub fn is_frameless_runtime_command(cmd: &str) -> bool {
    FRAMELESS_RUNTIME_COMMANDS.contains(&cmd)
}

/// True if *cmd* is one of the five `set` / `incr` / `append` /
/// `lappend` / `unset` commands whose first arg is a variable
/// name.
#[must_use]
pub fn is_name_first_command(cmd: &str) -> bool {
    NAME_FIRST_COMMANDS.contains(&cmd)
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
/// Mirrors Python's `_scan_value_for_info_hazards`. Implemented
/// as a hand-rolled scanner to avoid pulling in the `regex` crate
/// for a single pattern.
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
    fn name_first_command_is_the_documented_five() {
        for cmd in ["set", "incr", "append", "lappend", "unset"] {
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
