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

//! W201 — manual path concatenation heuristic.
//!
//! Detects `set x "/path/$var"` style assignments where the rendered
//! value carries both a literal path separator (`/` or `\`) and an
//! interpolation hole (`$var` or `[cmd]`).
//!
//! Detection consumes two already-computed per-function maps:
//!
//! * `rendered_props` — provides
//!   `HAS_FORWARD_SLASH` / `HAS_BACKSLASH` (path-separator evidence)
//!   and `HAS_INTERPOLATION` (substitution evidence) on each SSA def.
//! * `taints` — `PATH_NORMALISED` on the defined value suppresses the
//!   warning (the value has already been through `file normalize` or
//!   equivalent).
//!
//! Both suppression paths read the same lattice colour. The second is a
//! forward scan over the block's own statements: when the next assignment to
//! the same variable produces a `PATH_NORMALISED` value, the concatenated
//! value never escapes unnormalised, so the warning is dropped.
//!
//! The colour is registry data — `file`'s `normalize` subcommand declares
//! `taint_transform: Some(TaintColour::PATH_NORMALISED)` and
//! `crate::taint` resolves it — so every spelling answers alike: a nested
//! `[file normalize [file join $a $b]]`, the `file nor` unique-prefix
//! abbreviation the ensemble accepts, a normalisation reaching the variable
//! through a copy, and a `::file`-qualified call. Until issue #1391 this
//! module instead text-matched `[file normalize $sameVar]` on a stale belief
//! that the taint engine never set the colour, and every other spelling
//! false-positived.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::Span;

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::rendered_properties::{RenderedProperties, RenderedValueProps};
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
use crate::taint::{TaintColour, TaintLattice};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution_with_config};

/// A W201 diagnostic emitted by [`find_path_concat_warnings`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathConcatWarning {
    /// Source span of the offending assignment (value token when
    /// available, whole statement otherwise).
    pub span: Span,
    /// Name of the variable receiving the concatenated path.
    pub variable: String,
    /// Always `"W201"`.
    pub code: DiagCode,
    /// Formatted message.
    pub message: String,
    /// Optional `[file join …]` replacement text when the RHS
    /// decomposes into simple segments.
    pub replacement: Option<String>,
}

/// Return `true` when `s` is a simple path segment (alphanumerics +
/// `_`/`.`/`-`).
fn is_simple_path_segment(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Return `true` when `s` is a pure `$name` / `${name}` reference with
/// a conservative identifier body (first char alpha/`_`, then
/// alphanumerics / `_` / `:`).
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
/// Strips one layer of surrounding double quotes, bails on a braced
/// word and on brackets, embedded quotes, semicolons, backslashes, and
/// whitespace, splits on `/`, and emits the rewrite only when every
/// segment is either a simple path token or a simple `$var` reference
/// (`${name}` included). A leading
/// `/` stays attached to the first segment — `file join /tmp $x` keeps
/// the path absolute (probed against tclsh 8.6), whereas dropping it
/// would silently relativise the result. Consecutive or trailing
/// separators bail: `file join` collapses the empty segment, so the
/// rewrite would not reproduce the original string. A backslash bails
/// too — `file join` emits forward slashes, so a `\`-separated value
/// would not round-trip textually.
#[must_use]
pub fn build_file_join_fix(path_expr: &str) -> Option<String> {
    let mut text = path_expr.trim();
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        text = &text[1..text.len() - 1];
    }
    if text.is_empty() {
        return None;
    }
    // A braced word substitutes nothing, so a live `[file join …]`
    // rewrite would change semantics.  Braces *inside* the text (the
    // `${name}` reference form) are fine — the per-segment whitelists
    // below accept exactly the balanced `${name}` shape.
    if text.starts_with('{') {
        return None;
    }
    if text
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '[' | ']' | ';' | '\\' | '"'))
    {
        return None;
    }

    let absolute = text.starts_with('/');
    let body = if absolute { &text[1..] } else { text };
    let parts: Vec<&str> = body.split('/').collect();
    if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    for part in &parts {
        if !is_simple_path_var(part) && !is_simple_path_segment(part) {
            return None;
        }
    }
    let joined = parts.join(" ");
    let prefix = if absolute { "/" } else { "" };
    Some(format!("[file join {prefix}{joined}]"))
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
pub fn find_path_concat_warnings<S1, S2, S3>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    rendered_props: &HashMap<ValueKey, RenderedValueProps, S1>,
    taints: &HashMap<ValueKey, TaintLattice, S2>,
    executable_blocks: &HashSet<BlockId, S3>,
    config: tcl_lexer::LexerConfig,
) -> Vec<PathConcatWarning>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
    S3: std::hash::BuildHasher,
{
    let mut out: Vec<PathConcatWarning> = Vec::new();
    let path_sep_bits = RenderedProperties::HAS_FORWARD_SLASH | RenderedProperties::HAS_BACKSLASH;
    // Only `PATH_NORMALISED` — the colour `file normalize` (and any other
    // spec declaring the same `taint_transform`) stamps on its result.
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
            if parse_command_substitution_with_config(trimmed, config).is_some() {
                continue;
            }
            // A URL scheme separator (`://`) marks a URL, not a filesystem
            // path — its separators are always `/` regardless of platform, so
            // `[file join]` (which emits native separators) would be wrong.
            // Likewise HTML/XML markup (`<tag>`) is not a path.
            if trimmed.contains("://") || trimmed.contains('<') || trimmed.contains('>') {
                continue;
            }

            let Some(ssa_stmt) = ssa_block.statements.get(idx) else {
                continue;
            };

            let mut has_path_sep = false;
            let mut has_interp = false;
            let mut has_literal_space = false;
            let mut suppressed_by_colour = false;
            for (&def_name, &def_ver) in &ssa_stmt.defs {
                let key: ValueKey = (def_name, def_ver);
                if let Some(rp) = rendered_props.get(&key) {
                    if rp.may.intersects(path_sep_bits) {
                        has_path_sep = true;
                    }
                    if rp.may.contains(RenderedProperties::HAS_INTERPOLATION) {
                        has_interp = true;
                    }
                    if rp.may.contains(RenderedProperties::HAS_LITERAL_SPACE) {
                        has_literal_space = true;
                    }
                }
                if let Some(t) = taints.get(&key)
                    && t.colours.intersects(suppress_colours)
                {
                    suppressed_by_colour = true;
                }
            }

            // A literal space (or tab) in the rendered value marks prose,
            // protocol, or display text — an HTTP request line
            // (`"CONNECT $host:$port HTTP/1.1"`), a usage message, an HTML
            // fragment — not a filesystem path being constructed.  Genuine
            // path concat (`set f "$dir/$name"`) carries no literal
            // whitespace.
            if !has_path_sep || !has_interp || has_literal_space || suppressed_by_colour {
                continue;
            }

            // Forward-scan: the next assignment to the same variable in this
            // block produces a `PATH_NORMALISED` value → the concatenated
            // value never leaves the block unnormalised.  The question is put
            // to the taint lattice, not to the assignment's text, so it is
            // the *sanitiser* that is recognised rather than one spelling of
            // one call to it (issue #1391).
            let mut suppressed = false;
            for (later_idx, later) in block.statements.iter().enumerate().skip(idx + 1) {
                if let Statement::AssignValue {
                    name: later_name, ..
                } = later
                    && later_name == name
                {
                    suppressed = ssa_block
                        .statements
                        .get(later_idx)
                        .is_some_and(|later_ssa| {
                            later_ssa.defs.iter().any(|(&def_name, &def_ver)| {
                                taints
                                    .get(&(def_name, def_ver))
                                    .is_some_and(|t| t.colours.intersects(suppress_colours))
                            })
                        });
                    break;
                }
            }
            if suppressed {
                continue;
            }

            // Prefer the value-token span (argv[2] for `set name value`)
            // so editors highlight the offending word, not the whole
            // statement. Fall back to the command span when tokens aren't
            // available (e.g. synthesised assignments).
            let value_span = tokens.as_ref().and_then(|t| t.argv.get(2).copied());
            // The replacement lifts into a quick fix that replaces the
            // warning span, so it is only built when that span is the
            // value word's own token range — on the whole-statement
            // fallback the rewrite would swallow `set name` too.
            let replacement = value_span.and_then(|_| build_file_join_fix(value));
            out.push(PathConcatWarning {
                span: value_span.unwrap_or(*span),
                variable: name.clone(),
                code: DiagCode::W201,
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
            tcl_lexer::LexerConfig::for_profile(registry().profile()),
        )
    }

    // build_file_join_fix unit tests

    /// A leading `/` stays attached to the first segment so the rewrite
    /// keeps the path absolute (`file join /etc hosts` → `/etc/hosts`;
    /// `file join etc hosts` would relativise it).
    #[test]
    fn file_join_fix_simple_two_segments_keeps_absolute() {
        assert_eq!(
            build_file_join_fix("/etc/hosts").as_deref(),
            Some("[file join /etc hosts]"),
        );
    }

    #[test]
    fn file_join_fix_with_var_segment() {
        assert_eq!(
            build_file_join_fix("/var/log/$name").as_deref(),
            Some("[file join /var log $name]"),
        );
    }

    #[test]
    fn file_join_fix_relative_var_segments() {
        assert_eq!(
            build_file_join_fix("$dir/$file").as_deref(),
            Some("[file join $dir $file]"),
        );
    }

    /// Mixed segments (`$name.log` — neither a pure var nor a pure
    /// literal) force the conservative `None` branch.
    #[test]
    fn file_join_fix_rejects_mixed_segment() {
        assert!(build_file_join_fix("/var/log/$name.log").is_none());
    }

    #[test]
    fn file_join_fix_strips_quotes() {
        assert_eq!(
            build_file_join_fix("\"/tmp/$x\"").as_deref(),
            Some("[file join /tmp $x]"),
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

    /// `file join` collapses empty segments, so consecutive or trailing
    /// separators would not round-trip — no fix.
    #[test]
    fn file_join_fix_rejects_consecutive_and_trailing_slashes() {
        assert!(build_file_join_fix("/tmp//$x").is_none());
        assert!(build_file_join_fix("$dir/$file/").is_none());
    }

    /// `file join` emits forward slashes, so a backslash-separated value
    /// (previously split like `/`) is no longer rewritten.
    #[test]
    fn file_join_fix_rejects_backslash() {
        assert!(build_file_join_fix("C:\\temp\\$x").is_none());
        assert!(build_file_join_fix("$dir\\$file").is_none());
    }

    /// A braced RHS substitutes nothing, so rewriting it into a live
    /// `[file join …]` would change semantics — no fix.
    #[test]
    fn file_join_fix_rejects_braced_value() {
        assert!(build_file_join_fix("{/tmp/$x}").is_none());
    }

    /// Glob characters and protocol-like prefixes stay unfixed (the
    /// segment whitelists reject `*`, `?`, and `:`).
    #[test]
    fn file_join_fix_rejects_glob_and_protocol() {
        assert!(build_file_join_fix("/tmp/*.log").is_none());
        assert!(build_file_join_fix("http://$host/path").is_none());
    }

    // end-to-end detection tests

    /// Baseline: `set p "/tmp/$x"` flags W201.
    #[test]
    fn flags_manual_path_with_var_interp() {
        let ws = warnings_for("set x 42\nset p \"/tmp/$x\"");
        assert!(
            ws.iter()
                .any(|w| w.variable == "p" && w.code == DiagCode::W201),
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

    /// The suppression is the *sanitiser*, not one spelling of it: a nested
    /// normalisation, the ensemble's unique-prefix abbreviation, and the
    /// `::`-qualified call all carry `PATH_NORMALISED` on the lattice, and
    /// each one the old text match rejected (issue #1391).
    #[test]
    fn file_normalize_suppression_is_spelling_independent() {
        for later in [
            "set p [file normalize [file join $p sub]]",
            "set p [file nor $p]",
            "set p [::file normalize $p]",
        ] {
            let ws = warnings_for(&format!("set x 42\nset p \"/tmp/$x\"\n{later}"));
            assert!(
                ws.iter().all(|w| w.variable != "p"),
                "{later} should suppress W201: {ws:?}",
            );
        }
    }

    /// A later assignment that is *not* a normalisation still reports.
    #[test]
    fn a_non_normalising_reassignment_does_not_suppress() {
        let ws = warnings_for("set x 42\nset p \"/tmp/$x\"\nset p [file join $p sub]");
        assert!(
            ws.iter().any(|w| w.variable == "p"),
            "[file join] is not a normalising sanitiser: {ws:?}",
        );
    }

    /// FP-STY-16: a literal space (or tab) in the rendered value marks
    /// prose, a protocol line, or display text — not a path.  An HTTP
    /// request line / usage message / prose-with-path must not flag W201.
    #[test]
    fn literal_space_marks_prose_no_w201() {
        for src in [
            "set host h\nset port p\nset bypass \"CONNECT $host:$port HTTP/1.1\"",
            "set exe e\nset msg \"Usage: [file tail $exe] script \"",
            "set dir d\nset x \"see $dir/readme for help\"",
        ] {
            let ws = warnings_for(src);
            assert!(
                ws.iter().all(|w| w.code != DiagCode::W201),
                "literal-space prose should not flag W201: {src:?} -> {ws:?}",
            );
        }
    }

    /// TP control: a genuine path concat with no literal whitespace still
    /// fires, and a command-sub segment's *internal* spaces (one CMD token)
    /// must not suppress.
    #[test]
    fn genuine_path_concat_still_fires_w201() {
        for src in [
            "set dir d\nset name n\nset f \"$dir/$name\"",
            "set dir d\nset path p\nset f \"$dir/[file tail $path]\"",
        ] {
            let ws = warnings_for(src);
            assert!(
                ws.iter().any(|w| w.code == DiagCode::W201),
                "genuine path concat must fire W201: {src:?} -> {ws:?}",
            );
        }
    }

    /// The emitted warning carries a buildable `[file join …]`
    /// replacement when the RHS decomposes cleanly. The lowering
    /// pipeline may brace the variable reference (`$x` → `${x}`), so
    /// both forms are accepted; the leading `/` must be preserved.
    #[test]
    fn warning_includes_replacement_when_buildable() {
        let ws = warnings_for("set x 42\nset p \"/tmp/$x\"");
        let w = ws
            .iter()
            .find(|w| w.variable == "p")
            .expect("expected W201 on p");
        let replacement = w.replacement.as_deref().unwrap_or("");
        assert!(
            replacement == "[file join /tmp $x]" || replacement == "[file join /tmp ${x}]",
            "unexpected replacement: {replacement:?}",
        );
    }

    /// A firing shape whose RHS does not decompose cleanly (mixed
    /// `$n.log` segment) keeps the warning but carries no replacement.
    #[test]
    fn warning_omits_replacement_for_mixed_segment() {
        let ws = warnings_for("set n x\nset p \"/var/log/$n.log\"");
        let w = ws
            .iter()
            .find(|w| w.variable == "p")
            .expect("expected W201 on p");
        assert!(
            w.replacement.is_none(),
            "no replacement expected: {:?}",
            w.replacement
        );
    }
}
