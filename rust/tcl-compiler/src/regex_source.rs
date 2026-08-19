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

//! Regex-source tracking: find the def-site string literal(s) that flow into a
//! `regexp` / `regsub` pattern via a variable, so the semantic-token layer can
//! highlight the *originating* literal as a regex.
//!
//! Example — `set my_re ".*abc"; regexp $my_re $s` — the `.*abc` at the `set`
//! is the pattern's real text, and this module returns its source span so it
//! reads as a regex rather than a plain string.
//!
//! # How it works
//!
//! This is a dataflow query over an already-built [`CompilationUnit`] (the
//! SSA/SCCP tier), mirroring how [`crate::minify`] resolves a `$var` to its
//! constant value:
//!
//! 1. Walk every function's CFG statements in parallel with its SSA statements
//!    (keyed by [`BlockId`]), maintaining the in-scope SSA version of each
//!    variable (`entry_versions` + per-statement `uses`/`defs`, exactly as
//!    `minify::collect_folds_for_scope`).
//! 2. Record, per SSA value `(name, version)`:
//!    * the source span of the assigned **value word** for a literal
//!      assignment (`AssignConst` / `AssignValue`), and
//!    * the incoming versions of a φ (control-flow merge), so a conditionally
//!      / re-assigned pattern resolves to *all* its reaching literals.
//! 3. Record every `regexp` / `regsub` call whose pattern argument is a bare
//!    `$var`, with the variable's SSA version at that use.
//! 4. For each such use, gate on the SCCP lattice proving the value is a
//!    string constant (`Const(String)` or an all-`String` `ConstSet`), then
//!    resolve its reaching def-site value spans (recursing through φs).
//!
//! The pattern grammar is version-invariant across Tcl 8.4–9.0, and `set` /
//! variable resolution are identical across versions, so this query needs no
//! dialect gating (the `dialect` argument only configures the segmenter that
//! recovers a value word's boundaries).
//!
//! # Scope
//!
//! The query covers every function the [`CompilationUnit`] lowers: the
//! top-level script, each `proc` body, each `TclOO` method / constructor /
//! destructor body (lowered to its own `FunctionUnit` under
//! [`CompilationUnit::methods`]), and each synthetic *body unit* — `apply`
//! lambda bodies and `namespace eval` blocks (under
//! [`CompilationUnit::body_units`]).  So a `set`/`regexp` flow is tracked inside
//! a method, a lambda, or a namespace-eval body exactly as inside a proc.

use std::collections::{HashMap, HashSet};

use tcl_lexer::{Span, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::patterns::PatternType;

use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::Statement;
use crate::segmenter::segment_commands_with_offset_and_config;
use crate::ssa::Version;

/// Source spans of the def-site value literals that feed a `regexp` / `regsub`
/// pattern through a provably-constant string variable.  Sorted by start,
/// de-duplicated.  Empty when the unit has no such flow (the common case), so
/// callers pay nothing extra.
#[must_use]
pub fn regex_source_literal_spans(
    source: &str,
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut units: Vec<&FunctionUnit> =
        Vec::with_capacity(cu.procedures.len() + cu.methods.len() + cu.body_units.len() + 1);
    units.push(&cu.top_level);
    units.extend(cu.procedures.values());
    // `TclOO` method / constructor / destructor bodies are lowered to their own
    // `FunctionUnit`s (keyed `{class}::{method}`); walk them too so a
    // `set re "…"; regexp $re` inside a method tracks like one in a proc.
    units.extend(cu.methods.values());
    // Synthetic body units — `apply` lambda bodies and `namespace eval` blocks —
    // are lowered to their own fresh-frame `FunctionUnit`s (§ Scope). Walking
    // them lets a `set re "…"; regexp $re` *inside* a lambda or namespace-eval
    // body track its def-site literal the same way a proc body does.
    units.extend(cu.body_units.values());
    for fu in units {
        collect_in_function(source, fu, registry, dialect, &mut spans);
    }
    spans.sort_by_key(|s| (s.start(), s.end()));
    spans.dedup();
    spans
}

/// Per-function scan state.
#[derive(Default)]
struct Scan {
    /// `(name, version)` → absolute source span of the assigned value word,
    /// for a literal-assignment def.
    const_def_span: HashMap<(String, Version), Span>,
    /// `(name, version)` → φ incoming versions (control-flow merge defs).
    phi_incoming: HashMap<(String, Version), Vec<Version>>,
    /// `(pattern-variable name, SSA version at the use)` for each `regexp` /
    /// `regsub` call whose pattern operand is a bare `$var`.
    regex_uses: Vec<(String, Version)>,
}

fn collect_in_function(
    source: &str,
    fu: &FunctionUnit,
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
    out: &mut Vec<Span>,
) {
    let mut scan = Scan::default();

    for (block_id, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_id) else {
            continue;
        };
        if !fu.sccp.executable_blocks.contains(block_id) {
            continue;
        }
        // φ definitions at the block head.
        for phi in &ssa_block.phis {
            let key = (fu.ssa.var_name(phi.name).to_owned(), phi.version);
            let incoming: Vec<Version> = phi.incoming.values().copied().collect();
            scan.phi_incoming.entry(key).or_insert(incoming);
        }

        // Running name → version map, seeded from the block entry.
        let mut vars: HashMap<String, Version> = ssa_block
            .entry_versions
            .iter()
            .map(|(sym, ver)| (fu.ssa.var_name(*sym).to_owned(), *ver))
            .collect();

        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            let Some(ssa_stmt) = ssa_block.statements.get(stmt_idx) else {
                continue;
            };
            // Versions visible *at* this statement (uses resolve to their
            // reaching version, defs to the new version afterwards).
            let mut uses = vars.clone();
            for (sym, ver) in &ssa_stmt.uses {
                uses.insert(fu.ssa.var_name(*sym).to_owned(), *ver);
            }

            // A literal assignment records its value-word span, keyed by the
            // SSA version it defines.
            if is_literal_assignment(stmt)
                && let Some(vspan) = value_word_span(source, dialect, fu.abs_span(stmt.span()))
            {
                for (sym, ver) in &ssa_stmt.defs {
                    scan.const_def_span
                        .entry((fu.ssa.var_name(*sym).to_owned(), *ver))
                        .or_insert(vspan);
                }
            }

            // A `regexp`/`regsub` call with a bare-`$var` pattern records the
            // variable and the version reaching this use.
            if let Some(var) = regex_pattern_var(stmt, registry)
                && let Some(&ver) = uses.get(&var)
            {
                scan.regex_uses.push((var, ver));
            }

            for (sym, ver) in &ssa_stmt.defs {
                vars.insert(fu.ssa.var_name(*sym).to_owned(), *ver);
            }
        }
    }

    // Resolve each pattern-variable use to its def-site literal span(s), gated
    // on the SCCP lattice proving a string constant.
    for (name, use_ver) in &scan.regex_uses {
        if !is_string_constant(fu, name, *use_ver) {
            continue;
        }
        let mut visited: HashSet<(String, Version)> = HashSet::new();
        resolve_def_spans(&scan, name, *use_ver, &mut visited, out);
    }
}

/// True when the SCCP lattice proves `(name, version)` is a string constant —
/// a single `Const(String)` or an all-`String` `ConstSet`.
fn is_string_constant(fu: &FunctionUnit, name: &str, version: Version) -> bool {
    let Some(sym) = fu.ssa.var_symbol(name) else {
        return false;
    };
    match fu.sccp.values.get(&(sym, version)) {
        Some(LatticeValue::Const(ConstValue::String(_))) => true,
        Some(LatticeValue::ConstSet(vs)) => {
            !vs.is_empty() && vs.iter().all(|v| matches!(v, ConstValue::String(_)))
        }
        _ => false,
    }
}

/// Push the def-site value spans reaching `(name, version)`, recursing through
/// φ merges.  `visited` guards against φ cycles (loops).
fn resolve_def_spans(
    scan: &Scan,
    name: &str,
    version: Version,
    visited: &mut HashSet<(String, Version)>,
    out: &mut Vec<Span>,
) {
    let key = (name.to_owned(), version);
    if !visited.insert(key.clone()) {
        return;
    }
    if let Some(span) = scan.const_def_span.get(&key) {
        out.push(*span);
        return;
    }
    if let Some(incoming) = scan.phi_incoming.get(&key) {
        for &inc in incoming {
            resolve_def_spans(scan, name, inc, visited, out);
        }
    }
}

/// True for a literal-assignment statement whose value word is a source
/// literal (`set VAR "…"` / `set VAR {…}` — `AssignConst`/`AssignValue`).
fn is_literal_assignment(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::AssignConst { .. } | Statement::AssignValue { .. }
    )
}

/// The absolute span of the *value word* of a `set VAR VALUE` command that
/// begins at `stmt_span.start()`.
///
/// The value word's **start** is recovered by re-segmenting the statement span
/// (arg 2 of the `set`), but the segmenter's representative-token span clamps
/// off a quoted/braced word's closing delimiter (and the IR statement span can
/// itself stop before it), so the word's **end** is recomputed by matching the
/// opening delimiter in the source.  `None` when the statement is not a `set`
/// with a value word.
fn value_word_span(source: &str, dialect: &'static tcl_dialect::DialectProfile, stmt_span: Span) -> Option<Span> {
    let start = stmt_span.start() as usize;
    let end = (stmt_span.end() as usize).min(source.len());
    let text = source.get(start..end.max(start))?;
    let seg = segment_commands_with_offset_and_config(
        text,
        u32::try_from(start).unwrap_or(0),
        tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
    )
    .into_iter()
    .next()?;
    if seg.texts.first().map(String::as_str) != Some("set") || seg.argv.len() < 3 {
        return None;
    }
    let word_start = seg.argv.get(2)?.span.start() as usize;
    Some(delimited_word_span(source, word_start))
}

/// The full span of the word beginning at byte `start`: for a `"…"` / `{…}`
/// word, extend to the matching close delimiter (honouring `\`-escapes and
/// brace nesting); a bareword ends at the next whitespace.  Recovers the
/// closing delimiter the segmenter's token span clamps off.
fn delimited_word_span(source: &str, start: usize) -> Span {
    let bytes = source.as_bytes();
    let s32 = u32::try_from(start).unwrap_or(0);
    let end = match bytes.get(start) {
        Some(b'"') => scan_to_close(bytes, start + 1, b'"', b'"'),
        Some(b'{') => scan_to_close(bytes, start + 1, b'{', b'}'),
        _ => {
            let mut i = start;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            i
        }
    };
    Span::new(s32, u32::try_from(end).unwrap_or(s32).max(s32))
}

/// Scan from `i` for the `close` delimiter, honouring `\`-escapes and (when
/// `open != close`) nesting.  Returns the index *past* the close, or the input
/// length when unterminated.
fn scan_to_close(bytes: &[u8], mut i: usize, open: u8, close: u8) -> usize {
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == open && open != close => {
                depth += 1;
                i += 1;
            }
            b if b == close => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    bytes.len()
}

/// Index of the first positional (**pattern**) argument of a `regexp` /
/// `regsub` call, after skipping leading option switches — `-start` consumes
/// a value word, every other flag is boolean, and `--` terminates the option
/// scan. `args` **excludes** the command word. Returns `None` when no
/// positional argument remains (only options were supplied).
///
/// The one canonical option-skip for `regexp` / `regsub`, shared by the taint
/// (T103), security (W306 / W303), regex-source-tracking, and const-string
/// harvesting paths so they can never drift out of agreement.
#[must_use]
pub(crate) fn regexp_pattern_index(args: &[String]) -> Option<usize> {
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if a == "-start" && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    (i < args.len()).then_some(i)
}

/// When `stmt` is a `regexp` / `regsub` call whose (option-skipped) pattern
/// argument is a bare `$var`, return that variable's name.  Mirrors the
/// semantic-token layer's option-skip.
fn regex_pattern_var(stmt: &Statement, registry: &CommandRegistry) -> Option<String> {
    let Statement::Call {
        command,
        canonical_command,
        tokens: Some(toks),
        ..
    } = stmt
    else {
        return None;
    };
    let name = canonical_command.as_deref().unwrap_or(command.as_str());
    let is_regex = registry
        .get(name)
        .and_then(|s| s.pattern_type)
        .is_some_and(|p| p == PatternType::Regex);
    if !is_regex {
        return None;
    }
    // `argv[0]` is the command; skip leading option words to find the pattern.
    let args = toks.argv_texts.get(1..)?;
    let idx = regexp_pattern_index(args)?;
    let argv_idx = idx + 1; // shift back past the command word
    if toks.argv_kinds.get(argv_idx) != Some(&TokenType::Var) {
        return None;
    }
    bare_var_name(toks.argv_texts.get(argv_idx)?)
}

/// Extract a plain scalar variable name from a `$name` / `${name}` reference,
/// or `None` for an array element / computed / complex reference.
fn bare_var_name(text: &str) -> Option<String> {
    let inner = text
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
        .or_else(|| text.strip_prefix('$'))?;
    if inner.is_empty() || inner.contains(['(', '$', '[', ' ', ':']) {
        return None;
    }
    Some(inner.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans_text(source: &str) -> Vec<String> {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(source, &registry, false);
        regex_source_literal_spans(source, &cu, &registry, tcl_dialect::DialectProfile::by_name("tcl9.0"))
            .into_iter()
            .map(|s| source[s.start() as usize..s.end() as usize].to_owned())
            .collect()
    }

    #[test]
    fn single_literal_flows_to_regexp() {
        // The `.*abc` literal at the `set` is the regex source.
        let got = spans_text("set my_re \".*abc\"\nregexp $my_re $s\n");
        assert_eq!(got, vec!["\".*abc\"".to_owned()]);
    }

    #[test]
    fn regexp_pattern_index_skips_options() {
        let idx = |a: &[&str]| {
            let owned: Vec<String> = a.iter().map(|s| (*s).to_owned()).collect();
            regexp_pattern_index(&owned)
        };
        // No options — pattern is arg 0.
        assert_eq!(idx(&["pat", "s"]), Some(0));
        // Boolean flags are skipped one word each.
        assert_eq!(idx(&["-nocase", "-all", "pat", "s"]), Some(2));
        // `-start` consumes its value word.
        assert_eq!(idx(&["-start", "5", "pat", "s"]), Some(2));
        // `--` terminates the option scan; the very next word is the pattern
        // even if it starts with `-`.
        assert_eq!(idx(&["-nocase", "--", "-pat", "s"]), Some(2));
        // Only options, no pattern → None.
        assert_eq!(idx(&["-nocase"]), None);
        assert_eq!(idx(&["-start"]), None);
        // `-start` consumes the following word as its index value, so a lone
        // `-start pat` leaves no pattern behind → None.
        assert_eq!(idx(&["-start", "pat"]), None);
    }

    #[test]
    fn proc_wrapped_literal_flows_to_regexp() {
        // Same shape as `single_literal_flows_to_regexp`, but the `set`/
        // `regexp` pair lives inside a proc body rather than at top level —
        // the per-function CFG/SSA path must resolve it the same way.
        let got = spans_text(
            "proc ::bench::regex_check {} {\n    set my_re \".*abc\"\n    regexp $my_re $s\n}\n",
        );
        assert_eq!(got, vec!["\".*abc\"".to_owned()]);
    }

    /// Regression guard (issue #829 investigation): the retag must still fire
    /// when the enclosing proc sits alongside hundreds of unrelated ones in
    /// the same file, matching the `large_file_semantic_tokens_refresh_delivers_enriched_result`
    /// e2e fixture — proves the per-function CFG/SSA facts this depends on
    /// don't get lost or truncated at scale.
    #[test]
    fn literal_flows_to_regexp_in_a_large_multi_proc_file() {
        use std::fmt::Write as _;
        let mut s = String::new();
        s.push_str("namespace eval ::bench {\n    variable counter 0\n}\n\n");
        for i in 0..600 {
            let _ = write!(
                s,
                "# proc number {i}\n\
                 proc ::bench::step{i} {{a b}} {{\n\
                 \x20   set v{i} [expr {{$a + $b}}]\n\
                 \x20   set msg \"step {i} = $v{i}\"\n\
                 \x20   if {{$v{i} > 10}} {{\n\
                 \x20       set v{i} [expr {{$v{i} + 1}}]\n\
                 \x20   }}\n\
                 \x20   return $v{i}\n\
                 }}\n\n"
            );
        }
        s.push_str(
            "proc ::bench::regex_check {} {\n    set my_re \".*abc\"\n    regexp $my_re $s\n}\n",
        );
        let got = spans_text(&s);
        assert_eq!(got, vec!["\".*abc\"".to_owned()]);
    }

    #[test]
    fn braced_literal_flows_to_regexp() {
        let got = spans_text("set re {a+b}\nregexp $re $s\n");
        assert_eq!(got, vec!["{a+b}".to_owned()]);
    }

    #[test]
    fn regsub_pattern_variable_tracked() {
        let got = spans_text("set re {x+}\nregsub -all $re $s Y out\n");
        assert_eq!(got, vec!["{x+}".to_owned()]);
    }

    #[test]
    fn reassignment_resolves_to_reaching_def() {
        // The use reaches the SECOND assignment only.
        let got = spans_text("set re \"a\"\nset re \"b\"\nregexp $re $s\n");
        assert_eq!(got, vec!["\"b\"".to_owned()]);
    }

    #[test]
    fn conditional_resolves_to_all_reaching_literals() {
        // Both branches assign a constant → both literals are regex sources.
        let src = "if {$c} {\n  set re \"aa\"\n} else {\n  set re \"bb\"\n}\nregexp $re $s\n";
        let mut got = spans_text(src);
        got.sort();
        assert_eq!(got, vec!["\"aa\"".to_owned(), "\"bb\"".to_owned()]);
    }

    #[test]
    fn non_constant_source_is_not_tracked() {
        // `$x` is not a compile-time constant → nothing highlighted.
        let got = spans_text("set re $x\nregexp $re $s\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn literal_pattern_is_not_a_variable_source() {
        // An inline literal pattern is handled by the token layer, not here.
        let got = spans_text("regexp {abc} $s\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn subject_variable_is_not_treated_as_pattern() {
        // `$s` is the subject string, not the pattern — must not be tracked.
        let got = spans_text("set s \"literal\"\nregexp {abc} $s\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn pattern_inside_proc_body_tracked() {
        let src = "proc p {s} {\n  set re \".*x\"\n  regexp $re $s\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn pattern_inside_oo_method_body_tracked() {
        // TclOO method bodies are lowered to their own `FunctionUnit`
        // (`CompilationUnit::methods`), so the flow tracks inside a method too.
        let src = "oo::class create C {\n  method m {s} {\n    set re \".*x\"\n    regexp $re $s\n  }\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn pattern_inside_constructor_body_tracked() {
        let src = "oo::class create C {\n  constructor {s} {\n    set re {a+}\n    regexp $re $s\n  }\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["{a+}".to_owned()]);
    }

    #[test]
    fn pattern_inside_namespace_eval_body_tracked() {
        // A `namespace eval` body runs in its own frame, lowered to a synthetic
        // body unit (`CompilationUnit::body_units`), so the `set`/`regexp` flow
        // tracks inside it the same as in a proc.
        let src = "namespace eval ::ns {\n  set re \".*x\"\n  regexp $re $s\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn pattern_inside_apply_lambda_body_tracked() {
        // An `apply` lambda body is a fresh-frame body unit; a `set re`/`regexp`
        // wholly inside the lambda tracks its def-site literal.
        let src = "apply {{s} {\n  set re {a+b}\n  regexp $re $s\n}} foo\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["{a+b}".to_owned()]);
    }

    #[test]
    fn apply_lambda_param_is_not_a_def_site() {
        // The pattern variable is the lambda's *parameter* `re` (bound to the
        // caller's arg), not a body literal — so there is no def-site literal to
        // highlight. Proves the body unit binds params correctly (a bare `$re`
        // read resolves to the param, not to some caller scalar).
        let src = "apply {{re s} {\n  regexp $re $s\n}} {a+} foo\n";
        let got = spans_text(src);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn pattern_inside_array_for_body_tracked() {
        // `array for {k v} arr body` (Tcl 9.0) runs its body in the caller's
        // frame, so it is inlined into the caller unit with the loop vars bound;
        // a `set re`/`regexp` inside the body tracks its def-site literal.
        let src = "array for {k v} a {\n  set re \".*x\"\n  regexp $re $v\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn array_for_body_reads_caller_literal() {
        // The body runs in the caller frame (`vm.eval_source` in place), so a
        // `regexp $re` reading a *caller* literal resolves to it — the inline
        // lowering shares the caller unit's reaching defs (regression for the
        // fresh-frame-body-unit false-negative).
        let src = "proc p {} {\n  set re {a+}\n  array for {k v} a {regexp $re $v}\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["{a+}".to_owned()]);
    }

    #[test]
    fn array_for_loop_var_is_not_a_def_site() {
        // The pattern variable is the loop var `v` (bound per entry), not a body
        // literal — so there is no def-site literal to highlight. Proves the
        // inline lowering binds the loop vars so they shadow any caller scalar.
        let src = "array for {k v} a {\n  regexp $v $s\n}\n";
        let got = spans_text(src);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn namespace_eval_body_unit_does_not_change_bytecode() {
        // Recording a body unit is analysis-only: the module still emits the
        // `namespace` runtime barrier, and codegen never reads `body_units`, so
        // the presence of a body unit must not perturb compiled output. Assert
        // the body unit is recorded (coverage) but lives outside procedures.
        let registry = CommandRegistry::build_default();
        let src = "namespace eval ::ns { set re \".*x\"\n regexp $re $s }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        assert_eq!(cu.body_units.len(), 1, "one namespace-eval body unit");
        assert!(
            cu.body_units
                .keys()
                .all(|k| k.starts_with("::ns::namespace-eval#")),
            "the qname's enclosing namespace must be `::ns` (the block's own \
             target namespace), not global: {:?}",
            cu.body_units.keys().collect::<Vec<_>>()
        );
        // The body unit is not a real procedure (codegen would emit it).
        assert!(cu.procedures.is_empty(), "no procedures materialised");
    }

    fn spans_text_dialect(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> Vec<String> {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(source, &registry, false);
        regex_source_literal_spans(source, &cu, &registry, dialect)
            .into_iter()
            .map(|s| source[s.start() as usize..s.end() as usize].to_owned())
            .collect()
    }

    #[test]
    fn result_is_dialect_invariant() {
        // ARE pattern syntax, `set`, and variable resolution are identical
        // across Tcl 8.4–9.0, so the tracked source literal is the same under
        // every dialect.
        let src = "set re {a+b[0-9]*}\nregexp $re $s\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                spans_text_dialect(src, tcl_dialect::DialectProfile::by_name(dialect)),
                vec!["{a+b[0-9]*}".to_owned()],
                "dialect {dialect}"
            );
        }
    }

    #[test]
    fn wrong_dialect_option_does_not_misplace_pattern() {
        // `regsub -command` is a 9.0-only option; under an 8.6 target the
        // generic leading-option skip still steps over it, so the pattern
        // variable is located and its source literal tracked (option-dialect
        // validity is a diagnostics concern, not a highlighting one).
        let src = "set re {x+}\nregsub -command $re $s Y out\n";
        assert_eq!(spans_text_dialect(src, tcl_dialect::DialectProfile::by_name("tcl8.6")), vec!["{x+}".to_owned()]);
        assert_eq!(spans_text_dialect(src, tcl_dialect::DialectProfile::by_name("tcl9.0")), vec!["{x+}".to_owned()]);
    }
}
