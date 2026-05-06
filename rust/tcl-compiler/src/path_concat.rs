//! W201 — manual path concatenation heuristic.
//!
//! Detects `set x "/path/$var"` style assignments where the rendered
//! value carries both a literal path separator (`/` or `\`) and an
//! interpolation hole (`$var` or `[cmd]`). Ported from
//! `core/compiler/taint/_path_concat.py` (C29 follow-up).
//!
//! Detection consumes two already-computed per-function maps:
//!
//! * `rendered_props` — provides
//!   `HAS_FORWARD_SLASH` / `HAS_BACKSLASH` (path-separator evidence)
//!   and `HAS_INTERPOLATION` (substitution evidence) on each SSA def.
//! * `taints` — `PATH_NORMALISED` on the defined value suppresses the
//!   warning (the value has already been through `file normalize` or
//!   equivalent). *Latent today* — the Rust taint engine does not yet
//!   set `PATH_NORMALISED` on `[file normalize]` results; this arm
//!   will light up once that propagation lands. The equivalent Python
//!   check also suppresses on `PATH_JOINED`, which has no Rust
//!   counterpart yet and is tracked as a follow-up.
//!
//! A forward-scan within the same block also suppresses the warning
//! when the very next assignment to the same variable is
//! `[file normalize $var]` — this is the only suppression path that
//! currently fires end-to-end.

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;

use crate::cfg::Function as CfgFunction;
use crate::ir::Statement;
use crate::rendered_properties::{RenderedProperties, RenderedValueProps};
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
use crate::taint::{TaintColour, TaintLattice};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};

/// A W201 diagnostic emitted by [`find_path_concat_warnings`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathConcatWarning {
    /// Source span of the offending assignment (value token when
    /// available, whole statement otherwise).
    pub span: Span,
    /// Name of the variable receiving the concatenated path.
    pub variable: String,
    /// Always `"W201"`.
    pub code: String,
    /// Formatted message.
    pub message: String,
    /// Optional `[file join …]` replacement text when the RHS
    /// decomposes into simple segments.
    pub replacement: Option<String>,
}

/// True when `value` is exactly `[file normalize $var_name]` (bare or
/// braced variable reference, possibly quoted).
fn is_file_normalize_of(value: &str, var_name: &str) -> bool {
    let Some((cmd, args)) = parse_command_substitution(value) else {
        return false;
    };
    if cmd != "file" || args.is_empty() || args[0] != "normalize" {
        return false;
    }
    for arg in &args[1..] {
        let stripped = arg.trim().trim_matches('"');
        if stripped == format!("${var_name}") || stripped == format!("${{{var_name}}}") {
            return true;
        }
    }
    false
}

/// Return `true` when `s` is a simple path segment (alphanumerics +
/// `_`/`.`/`-`), matching Python `_SIMPLE_PATH_SEGMENT_RE`.
fn is_simple_path_segment(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Return `true` when `s` is a pure `$name` / `${name}` reference with
/// a conservative identifier body (first char alpha/`_`, then
/// alphanumerics / `_` / `:`), matching Python `_SIMPLE_PATH_VAR_RE`.
fn is_simple_path_var(s: &str) -> bool {
    let inner = if let Some(rest) = s.strip_prefix("${") {
        match rest.strip_suffix('}') {
            Some(i) => i,
            None => return false,
        }
    } else if let Some(rest) = s.strip_prefix('$') {
        rest
    } else {
        return false;
    };
    let mut chars = inner.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// Conservative `[file join …]` replacement text. Returns `None` when
/// the RHS has characters the trivial split can't handle safely.
///
/// Mirrors `_build_file_join_fix` in Python: strip one layer of
/// surrounding quotes/braces, bail on `[`/`]`/`;`/whitespace, split on
/// `/` and `\`, and emit the rewrite only when every non-empty segment
/// is either a simple path token or a simple `$var` reference.
#[must_use]
pub fn build_file_join_fix(path_expr: &str) -> Option<String> {
    let mut text = path_expr.trim();
    if (text.starts_with('"') && text.ends_with('"'))
        || (text.starts_with('{') && text.ends_with('}'))
    {
        text = &text[1..text.len() - 1];
    }
    if text.is_empty() {
        return None;
    }
    if text.chars().any(|c| matches!(c, '[' | ']' | ';')) {
        return None;
    }
    if text.chars().any(char::is_whitespace) {
        return None;
    }

    let parts: Vec<&str> = text.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    for part in &parts {
        if !is_simple_path_var(part) && !is_simple_path_segment(part) {
            return None;
        }
    }
    Some(format!("[file join {}]", parts.join(" ")))
}

/// Find W201 warnings in `cfg` / `ssa`.
///
/// An `AssignValue` triggers W201 when its SSA def's rendered
/// properties carry both a path-separator (`HAS_FORWARD_SLASH` /
/// `HAS_BACKSLASH`) and an interpolation hole (`HAS_INTERPOLATION`),
/// unless suppressed by `PATH_NORMALISED` on the taint lattice or by
/// a following `[file normalize $var]` assignment in the same block.
///
/// Blocks are traversed in `cfg_order` for deterministic output and
/// skipped when not in `executable_blocks`.
#[must_use]
pub(crate) fn find_path_concat_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    rendered_props: &HashMap<ValueKey, RenderedValueProps>,
    taints: &HashMap<ValueKey, TaintLattice>,
    executable_blocks: &HashSet<String>,
) -> Vec<PathConcatWarning> {
    let mut out: Vec<PathConcatWarning> = Vec::new();
    let path_sep_bits = RenderedProperties::HAS_FORWARD_SLASH | RenderedProperties::HAS_BACKSLASH;
    // Only `PATH_NORMALISED` — Python also uses `PATH_JOINED`, but the
    // Rust lattice has no counterpart yet. Neither colour is actually
    // assigned by the current taint engine, so this arm is latent
    // until `[file normalize]` / `[file join]` propagation lands; the
    // forward-scan below is the only active suppression.
    let suppress_colours = TaintColour::PATH_NORMALISED;

    for bn in cfg_order(cfg) {
        if !executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = cfg.blocks.get(&bn) else {
            continue;
        };
        let Some(ssa_block) = ssa.blocks.get(&bn) else {
            continue;
        };

        for (idx, stmt) in block.statements.iter().enumerate() {
            let Statement::AssignValue {
                name,
                value,
                span,
                tokens,
                ..
            } = stmt
            else {
                continue;
            };

            // Skip pure `$var` aliases and pure `[cmd …]` subs — these are
            // structural forms, not manual path concatenation.
            let trimmed = value.trim();
            if is_pure_var_ref(trimmed) {
                continue;
            }
            if parse_command_substitution(trimmed).is_some() {
                continue;
            }

            let Some(ssa_stmt) = ssa_block.statements.get(idx) else {
                continue;
            };

            let mut has_path_sep = false;
            let mut has_interp = false;
            let mut suppressed_by_colour = false;
            for (def_name, &def_ver) in &ssa_stmt.defs {
                let key: ValueKey = (def_name.clone(), def_ver);
                if let Some(rp) = rendered_props.get(&key) {
                    if rp.may.intersects(path_sep_bits) {
                        has_path_sep = true;
                    }
                    if rp.may.contains(RenderedProperties::HAS_INTERPOLATION) {
                        has_interp = true;
                    }
                }
                if let Some(t) = taints.get(&key) {
                    if t.colours.intersects(suppress_colours) {
                        suppressed_by_colour = true;
                    }
                }
            }

            if !has_path_sep || !has_interp || suppressed_by_colour {
                continue;
            }

            // Forward-scan: the next assignment to the same variable in
            // this block is `[file normalize $var]` → clean.
            let mut suppressed = false;
            for later in &block.statements[idx + 1..] {
                if let Statement::AssignValue {
                    name: later_name,
                    value: later_val,
                    ..
                } = later
                {
                    if later_name == name {
                        if is_file_normalize_of(later_val, name) {
                            suppressed = true;
                        }
                        break;
                    }
                }
            }
            if suppressed {
                continue;
            }

            // Prefer the value-token span (argv[2] for `set name value`)
            // so editors highlight the offending word, not the whole
            // statement. Fall back to the command span when tokens aren't
            // available (e.g. synthesised assignments).
            let value_span = tokens
                .as_ref()
                .and_then(|t| t.argv.get(2).copied())
                .unwrap_or(*span);

            let replacement = build_file_join_fix(value);
            out.push(PathConcatWarning {
                span: value_span,
                variable: name.clone(),
                code: "W201".to_owned(),
                message: "Possible manual path concatenation. Use [file join] for portable path \
                          construction."
                    .to_owned(),
                replacement,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use tcl_registry::CommandRegistry;

    use crate::compilation_unit::CompilationUnit;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn warnings_for(source: &str) -> Vec<PathConcatWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let fu = cu.function("::top").unwrap();
        find_path_concat_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.rendered_props,
            &fu.taints,
            &fu.sccp.executable_blocks,
        )
    }

    // ----- build_file_join_fix unit tests ------------------------------------

    #[test]
    fn file_join_fix_simple_two_segments() {
        assert_eq!(
            build_file_join_fix("/etc/hosts").as_deref(),
            Some("[file join etc hosts]"),
        );
    }

    #[test]
    fn file_join_fix_with_var_segment() {
        assert_eq!(
            build_file_join_fix("/var/log/$name").as_deref(),
            Some("[file join var log $name]"),
        );
    }

    /// Mixed segments (`$name.log` — neither a pure var nor a pure
    /// literal) force the conservative `None` branch, matching Python.
    #[test]
    fn file_join_fix_rejects_mixed_segment() {
        assert!(build_file_join_fix("/var/log/$name.log").is_none());
    }

    #[test]
    fn file_join_fix_strips_quotes() {
        assert_eq!(
            build_file_join_fix("\"/tmp/$x\"").as_deref(),
            Some("[file join tmp $x]"),
        );
    }

    #[test]
    fn file_join_fix_rejects_bracketed_subst() {
        assert!(build_file_join_fix("/tmp/[cmd]").is_none());
    }

    #[test]
    fn file_join_fix_rejects_whitespace() {
        assert!(build_file_join_fix("/tmp/foo bar").is_none());
    }

    #[test]
    fn file_join_fix_rejects_single_segment() {
        assert!(build_file_join_fix("foo").is_none());
    }

    #[test]
    fn file_join_fix_rejects_semicolon() {
        assert!(build_file_join_fix("/a/b;rm").is_none());
    }

    // ----- is_file_normalize_of unit tests -----------------------------------

    #[test]
    fn file_normalize_matches_bare_var() {
        assert!(is_file_normalize_of("[file normalize $p]", "p"));
    }

    #[test]
    fn file_normalize_matches_braced_var() {
        assert!(is_file_normalize_of("[file normalize ${p}]", "p"));
    }

    #[test]
    fn file_normalize_rejects_other_cmd() {
        assert!(!is_file_normalize_of("[file join $p]", "p"));
    }

    #[test]
    fn file_normalize_rejects_different_var() {
        assert!(!is_file_normalize_of("[file normalize $q]", "p"));
    }

    // ----- end-to-end detection tests ----------------------------------------

    /// Baseline: `set p "/tmp/$x"` flags W201.
    #[test]
    fn flags_manual_path_with_var_interp() {
        let ws = warnings_for("set x 42\nset p \"/tmp/$x\"");
        assert!(
            ws.iter().any(|w| w.variable == "p" && w.code == "W201"),
            "expected W201 on /tmp/$x concatenation: {ws:?}",
        );
    }

    /// Pure `$var` RHS is structural aliasing, not concatenation.
    #[test]
    fn pure_var_ref_rhs_does_not_flag() {
        let ws = warnings_for("set src /etc/hosts\nset dst $src");
        assert!(
            ws.iter().all(|w| w.variable != "dst"),
            "pure var alias should not be W201: {ws:?}",
        );
    }

    /// A literal path without interpolation does not flag W201.
    #[test]
    fn literal_path_without_interp_does_not_flag() {
        let ws = warnings_for("set p /etc/hosts");
        assert!(ws.is_empty(), "literal path should not flag W201: {ws:?}");
    }

    /// Interpolation without a path separator does not flag W201.
    #[test]
    fn interp_without_path_sep_does_not_flag() {
        let ws = warnings_for("set x 1\nset greeting \"hello $x\"");
        assert!(
            ws.is_empty(),
            "plain interpolation should not flag W201: {ws:?}",
        );
    }

    /// A later `[file normalize $p]` in the same block suppresses the
    /// warning for the concatenated assignment.
    #[test]
    fn file_normalize_forward_scan_suppresses() {
        let ws = warnings_for("set x 42\nset p \"/tmp/$x\"\nset p [file normalize $p]");
        assert!(
            ws.iter().all(|w| w.variable != "p"),
            "subsequent [file normalize $p] should suppress W201: {ws:?}",
        );
    }

    /// The emitted warning carries a buildable `[file join …]`
    /// replacement when the RHS decomposes cleanly. The lowering
    /// pipeline may brace the variable reference (`$x` → `${x}`), so
    /// both forms are accepted.
    #[test]
    fn warning_includes_replacement_when_buildable() {
        let ws = warnings_for("set x 42\nset p \"/tmp/$x\"");
        let w = ws
            .iter()
            .find(|w| w.variable == "p")
            .expect("expected W201 on p");
        let replacement = w.replacement.as_deref().unwrap_or("");
        assert!(
            replacement == "[file join tmp $x]" || replacement == "[file join tmp ${x}]",
            "unexpected replacement: {replacement:?}",
        );
    }
}
