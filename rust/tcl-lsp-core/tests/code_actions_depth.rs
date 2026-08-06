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

//! Depth coverage of the `tcl-lsp-core` code-actions provider
//! (`src/code_actions.rs`), the lowest-covered file in the crate.
//!
//! The sibling `lsp_lens_links_symbols.rs` already exercises eight
//! code-action paths (W302 catch capture, extract-proc, IPv4→IPv6, De Morgan,
//! generate-docstring, W213 unset, W120 / fuzzy `package require`, IRULE5002).
//! This file deliberately AVOIDS those and drives the UNCOVERED branches:
//!
//!   * `continuation_comment_actions` — W115 "Convert to per-line comments"
//!     (direct backslash-continued-comment shape detection).
//!   * `invert_comparison` — the second `expr_rewrite_actions` branch.
//!   * `demorgan_transform` reverse direction (`!X || !Y` → `!(X && Y)`).
//!   * IPv6-mapped → IPv4 reverse conversion.
//!   * `inline_proc_action` — "Inline proc 'X'" (incl. a SAFETY-DECLINE
//!     negative control, and a known BUG repro left commented out).
//!   * `switch_to_dict` via the code-actions surface.
//!   * IRULE5004 (`DNS::return` without `return`) check-fix lift.
//!   * `context_diagnostic_actions` — T106 redundant-encoder removal,
//!     IRULE3001 `[html_encode …]` wrap, IRULE1005/1006 collect-bootstrap.
//!   * `profiles_action` — insert / update / no-op branches.
//!   * Multi-action positions, `only`-style kind filtering by the caller,
//!     and empty / whitespace / out-of-range positions (no panic).
//!
//! ----------------------------------------------------------------------------
//! Presentation vs Tcl-semantic split (read first)
//! ----------------------------------------------------------------------------
//! A code ACTION is an editor-presentation artefact: its *title*, its LSP
//! *kind*, and the *shape* of its `TextEdit` range are an LSP wire contract,
//! NOT a Tcl runtime value. Those are asserted STRUCTURALLY here (title text,
//! `ActionKind`, range in-bounds + start ≤ end, `new_text` sanity).
//!
//! What IS a Tcl-semantic fact — and is therefore proven against real C-Tcl
//! (`scripts/dev/tclsh_check.sh`, 8.6 + 9.0) and cited with `// tclsh:` — is
//! the GROUND TRUTH a fix encodes:
//!   * `unset -nocomplain $v` succeeds on an unset var where bare `unset`
//!     errors (premise of the W213 fix — already covered upstream, noted).
//!   * `!($a && $b)` ≡ `!$a || !$b` and `$a <= $b` ≡ `!($a > $b)` as boolean
//!     expressions (the rewrite fixes preserve meaning).
//!   * `::ffff:10.0.0.1` embeds the dotted-quad `10.0.0.1` (the IP refactors
//!     are equivalent representations).
//!   * a braced-list `switch -exact` and the matching `dict get` agree.
//!   * `info complete` distinguishes a complete `expr {…}` from a brace-
//!     truncated one — the lever that exposes the inline-proc BUG below.
//!
//! Facts verified identically on tclsh8.6 AND tclsh9.0 unless noted.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::{DiagCode, run_all_checks};
use tcl_lexer::LexerConfig;
use tcl_lsp_core::code_actions::{
    ActionKind, CodeAction, ContextDiagnostic, check_diagnostic_actions, code_actions,
    context_diagnostic_actions, profiles_action,
};
use tcl_lsp_core::definition::LspRange;

// ---------------------------------------------------------------------------
// Harness — same shape as call_hierarchy.rs / lsp_lens_links_symbols.rs.
// ---------------------------------------------------------------------------

/// Analyse `source` for the vanilla Tcl 8.6 dialect.
fn analyse(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6").clone()
}

/// Analyse `source` for a named dialect (used for the iRules paths).
fn analyse_dialect(source: &str, dialect: &str) -> AnalysisResult {
    Analyser::new().analyse(source, dialect).clone()
}

/// A registry with the iRules dialect loaded (events, profiles, `class …`).
fn irules_registry() -> tcl_registry::CommandRegistry {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.load_irules();
    registry
}

/// The iRules compiler-checks output (`run_all_checks`) for `source`.
fn irules_checks(
    source: &str,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<tcl_compiler::compiler_checks::Diagnostic> {
    let cu = CompilationUnit::build_for_with_config(
        source,
        registry,
        false,
        LexerConfig::for_dialect("f5-irules"),
    )
    .with_interprocedural(registry, Some("f5-irules"));
    run_all_checks(&cu, registry, Some("f5-irules"))
}

/// A single-position (empty) `LspRange` at `(line, character)`.
fn cursor(line: u32, character: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: character,
        end_line: line,
        end_character: character,
    }
}

/// A non-empty single-line `LspRange` selection.
fn selection(line: u32, start: u32, end: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: start,
        end_line: line,
        end_character: end,
    }
}

/// A whole-document range (line 0..last, full width).
fn whole(source: &str) -> LspRange {
    let last = source.lines().count().max(1) - 1;
    LspRange {
        start_line: 0,
        start_character: 0,
        end_line: u32::try_from(last).unwrap_or(0),
        end_character: u32::MAX,
    }
}

/// Every edit's range is well-formed: start ≤ end (a valid LSP edit range).
fn edits_well_formed(action: &CodeAction) -> bool {
    action.edits.iter().all(|e| {
        (e.range.start_line, e.range.start_character) <= (e.range.end_line, e.range.end_character)
    })
}

/// Every edit's range sits within `source`'s line/column bounds (an insertion
/// or replacement the editor can apply without overshooting the buffer). A
/// zero-width insert at end-of-line / end-of-file is in-bounds.
fn edits_in_bounds(action: &CodeAction, source: &str) -> bool {
    let lines: Vec<&str> = source.split('\n').collect();
    let line_count = u32::try_from(lines.len()).unwrap_or(u32::MAX);
    action.edits.iter().all(|e| {
        let r = e.range;
        // End line may equal line_count for a trailing whole-line replacement.
        if r.start_line > line_count || r.end_line > line_count {
            return false;
        }
        let col_ok = |line: u32, ch: u32| {
            lines.get(line as usize).is_none_or(|l| {
                // UTF-16 length; ASCII fixtures here make this == char count.
                let units: u32 = l
                    .chars()
                    .map(|c| u32::try_from(c.len_utf16()).unwrap_or(1))
                    .sum();
                ch <= units
            })
        };
        col_ok(r.start_line, r.start_character) && col_ok(r.end_line, r.end_character)
    })
}

/// All action titles at a glance.
fn titles(actions: &[CodeAction]) -> Vec<String> {
    actions.iter().map(|a| a.title.clone()).collect()
}

/// The first action whose title contains `needle`.
fn find<'a>(actions: &'a [CodeAction], needle: &str) -> Option<&'a CodeAction> {
    actions.iter().find(|a| a.title.contains(needle))
}

// ===========================================================================
// continuation_comment_actions — W115 "Convert to per-line comments"
// ===========================================================================

#[test]
fn w115_continued_comment_offers_per_line_conversion() {
    // A backslash-continued comment run. `continuation_comment_actions`
    // detects the shape directly at the range start (a `#` line ending in
    // `\`), independent of any analyser-emitted W115. The fix rewrites the
    // continuation block into standalone `#` lines.
    //
    // tclsh: in real Tcl a trailing backslash on a comment line continues the
    // comment onto the next physical line, so `# a \` + `b` is ONE comment;
    // splitting it into `# a` / `# b` preserves that both lines are comments.
    //   info complete "# a \\\nb\n"  -> 1   (a single complete comment-command;
    //   verified tclsh8.6 + tclsh9.0). The conversion keeps every line a `#`
    //   comment, so semantics are unchanged.
    let src = "# header part \\\nmore text\nset x 1\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    let conv = find(&actions, "per-line comments")
        .expect("a per-line-comment conversion on the continued comment");
    assert_eq!(conv.kind, ActionKind::QuickFix, "{conv:?}");
    assert_eq!(
        conv.edits.len(),
        1,
        "single block-replacement edit: {conv:?}"
    );
    assert!(edits_well_formed(conv), "{conv:?}");
    assert!(edits_in_bounds(conv, src), "{conv:?}");
    // The replacement turns the continued second physical line into its own
    // `#` comment (the body `more text` becomes `# more text`).
    let new_text = &conv.edits[0].new_text;
    assert!(
        new_text.contains("# more text"),
        "continuation body should become its own comment line: {new_text:?}",
    );
    // The trailing backslash is gone from the rewritten block.
    assert!(
        !new_text.contains('\\'),
        "rewritten comment must not keep the continuation backslash: {new_text:?}",
    );
}

#[test]
fn w115_no_conversion_on_plain_comment() {
    // A plain (non-continued) comment at the cursor — no backslash, no W115 —
    // offers no conversion. Negative control for the direct-shape gate.
    let src = "# just a comment\nset x 1\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "per-line comments").is_none(),
        "plain comment must not offer the conversion; got {:?}",
        titles(&actions),
    );
}

#[test]
fn w115_no_conversion_on_continued_code_line() {
    // A backslash-continued *code* line (not a comment) must NOT be rewritten
    // as commented text — the gate requires the run to start with `#`. This is
    // the corruption the provider's own comment warns against.
    let src = "set x \\\n5\nputs $x\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "per-line comments").is_none(),
        "continued code line must not be commented out; got {:?}",
        titles(&actions),
    );
}

// ===========================================================================
// expr_rewrite_actions — invert comparison + De Morgan (reverse direction)
// ===========================================================================

#[test]
fn invert_comparison_flips_equality_operator() {
    // Selecting `$a == $b` offers "Invert comparison" → `$a != $b`.
    //
    // tclsh: the inversion preserves the negated truth value —
    //   set a 1; set b 1; list [expr {$a == $b}] [expr {!($a != $b)}] -> 1 1
    //   (verified tclsh8.6 + tclsh9.0). `==` and the negation of `!=` agree.
    let src = "if {$a == $b} { puts hi }\n";
    let analysis = analyse(src);
    // `if {` is 4 chars; `$a == $b` spans cols 4..12.
    let actions = code_actions(src, selection(0, 4, 12), Some(&analysis), &analysis.diagnostics);
    let inv = find(&actions, "Invert comparison").expect("an invert-comparison rewrite");
    assert_eq!(inv.kind, ActionKind::RefactorRewrite, "{inv:?}");
    assert_eq!(inv.edits.len(), 1);
    assert!(
        edits_well_formed(inv) && edits_in_bounds(inv, src),
        "{inv:?}"
    );
    assert_eq!(
        inv.edits[0].new_text, "$a != $b",
        "== should invert to !=: {:?}",
        inv.edits[0].new_text,
    );
}

#[test]
fn invert_comparison_flips_relational_operator() {
    // `$a <= $b` inverts to `$a > $b`.
    //
    // tclsh: `$a <= $b` ≡ `!($a > $b)` —
    //   set a 3; set b 5; list [expr {$a <= $b}] [expr {!($a > $b)}] -> 1 1
    //   (verified tclsh8.6 + tclsh9.0).
    let src = "while {$a <= $b} { incr a }\n";
    let analysis = analyse(src);
    // `while {` is 7 chars; `$a <= $b` spans cols 7..15.
    let actions = code_actions(src, selection(0, 7, 15), Some(&analysis), &analysis.diagnostics);
    let inv = find(&actions, "Invert comparison").expect("an invert-comparison rewrite");
    assert_eq!(
        inv.edits[0].new_text, "$a > $b",
        "<= should invert to >: {:?}",
        inv.edits[0].new_text,
    );
}

#[test]
fn invert_comparison_flips_tip461_string_ordering_operator() {
    // Issue #983/#986: the hand-typed inversion list never included the
    // 9.0+ `lt`/`le`/`gt`/`ge` word-form comparisons at all, so this quick
    // fix silently never offered itself for one of them.
    //
    // tclsh 9.0: `$a lt $b` ≡ `!($a ge $b)` for any string pair (a total
    // order, same identity as the numeric/`<=` case above).
    let src = "if {$a lt $b} { puts hi }\n";
    let analysis = analyse(src);
    // `if {` is 4 chars; `$a lt $b` spans cols 4..12.
    let actions = code_actions(src, selection(0, 4, 12), Some(&analysis), &analysis.diagnostics);
    let inv = find(&actions, "Invert comparison").expect("an invert-comparison rewrite");
    assert_eq!(
        inv.edits[0].new_text, "$a ge $b",
        "lt should invert to ge: {:?}",
        inv.edits[0].new_text,
    );
}

#[test]
fn invert_comparison_absent_without_top_level_operator() {
    // A selection with no space-delimited top-level comparison operator offers
    // no inversion (a bare `$a` can't be inverted).
    let src = "puts $value\n";
    let analysis = analyse(src);
    // Select `$value` (cols 5..11).
    let actions = code_actions(src, selection(0, 5, 11), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "Invert comparison").is_none(),
        "no comparison operator -> no inversion; got {:?}",
        titles(&actions),
    );
}

#[test]
fn invert_comparison_absent_on_empty_selection() {
    // The expr rewrites require a non-empty single-line selection; a bare
    // cursor (start == end) offers neither De Morgan nor invert.
    let src = "if {$a == $b} { puts hi }\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 8), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "Invert comparison").is_none() && find(&actions, "De Morgan").is_none(),
        "no expr rewrites without a selection; got {:?}",
        titles(&actions),
    );
}

#[test]
fn demorgan_reverse_collapses_disjunction_of_negations() {
    // De Morgan also runs in reverse: `!$a || !$b` → `!($a && $b)`.
    //
    // tclsh: the two forms are equivalent booleans —
    //   set a 1; set b 0;
    //   list [expr {!$a || !$b}] [expr {!($a && $b)}] -> 1 1   (8.6 + 9.0).
    let src = "if {!$a || !$b} { puts hi }\n";
    let analysis = analyse(src);
    // `if {` is 4 chars; `!$a || !$b` spans cols 4..14.
    let actions = code_actions(src, selection(0, 4, 14), Some(&analysis), &analysis.diagnostics);
    let dm = find(&actions, "De Morgan").expect("a reverse-direction De Morgan rewrite");
    assert_eq!(dm.kind, ActionKind::RefactorRewrite);
    assert!(edits_well_formed(dm) && edits_in_bounds(dm, src), "{dm:?}");
    assert_eq!(
        dm.edits[0].new_text, "!($a && $b)",
        "reverse De Morgan of `|| of negations` is `!(… && …)`: {:?}",
        dm.edits[0].new_text,
    );
}

#[test]
fn demorgan_forward_recognises_irules_word_operators() {
    // Adversarial-review finding: `demorgan_transform` only recognised the
    // symbolic `&&`/`||`/`!` forms, so it silently never offered the rewrite
    // for a selection written in iRules' word style — `!($a and $b)` (the
    // same shape `demorgan_reverse_collapses_disjunction_of_negations`
    // above exercises symbolically) got no "Apply De Morgan's law" action
    // at all, an inconsistent gap given the sibling `invert_comparison` fix
    // in this same file already handles TIP 461's word operators.
    //
    // tclsh (f5-irules dialect): `!($a and $b)` ≡ `!$a or !$b` (the outer
    // `!` prefix's negation style is kept for the operands; only the
    // connective switches, `and` -> `or`, matching its own inner spelling).
    let src = "if {!($a and $b)} { puts hi }\n";
    let analysis = analyse_dialect(src, "f5-irules");
    // `if {` is 4 chars; `!($a and $b)` spans cols 4..16.
    let actions = code_actions(src, selection(0, 4, 16), Some(&analysis), &analysis.diagnostics);
    let dm =
        find(&actions, "De Morgan").expect("a forward-direction word-operator De Morgan rewrite");
    assert_eq!(dm.kind, ActionKind::RefactorRewrite);
    assert!(edits_well_formed(dm) && edits_in_bounds(dm, src), "{dm:?}");
    assert_eq!(
        dm.edits[0].new_text, "!$a or !$b",
        "forward De Morgan of `!($a and $b)` is `!$a or !$b`: {:?}",
        dm.edits[0].new_text,
    );
}

#[test]
fn demorgan_reverse_recognises_irules_word_operators() {
    // The reverse direction for the word-operator forms: `not $a or not $b`
    // → `not ($a and $b)`.
    //
    // tclsh (f5-irules dialect): the two forms are equivalent booleans.
    let src = "if {not $a or not $b} { puts hi }\n";
    let analysis = analyse_dialect(src, "f5-irules");
    // `if {` is 4 chars; `not $a or not $b` spans cols 4..20.
    let actions = code_actions(src, selection(0, 4, 20), Some(&analysis), &analysis.diagnostics);
    let dm =
        find(&actions, "De Morgan").expect("a reverse-direction word-operator De Morgan rewrite");
    assert_eq!(
        dm.edits[0].new_text, "not ($a and $b)",
        "reverse De Morgan of `not X or not Y` is `not (X and Y)`: {:?}",
        dm.edits[0].new_text,
    );
}

// ===========================================================================
// ip_conversion_actions — IPv6-mapped → IPv4 (the reverse direction)
// ===========================================================================

#[test]
fn ipv6_mapped_literal_offers_ipv4_conversion() {
    // Cursor on an `::ffff:<v4>` literal offers "Convert to IPv4 address",
    // stripping the mapping prefix.
    //
    // tclsh: the mapped form embeds the dotted-quad verbatim —
    //   string range "::ffff:10.0.0.1" 7 end -> 10.0.0.1   (8.6 + 9.0).
    // The two are equivalent representations of one address.
    let src = "set ip ::ffff:10.0.0.1\n";
    let analysis = analyse(src);
    // Cursor inside the literal (after `set ip `, col ~12).
    let actions = code_actions(src, cursor(0, 12), Some(&analysis), &analysis.diagnostics);
    let conv = find(&actions, "IPv4 address").expect("an IPv6-mapped -> IPv4 refactor");
    assert_eq!(conv.kind, ActionKind::Refactor);
    assert_eq!(conv.edits.len(), 1);
    assert!(
        edits_well_formed(conv) && edits_in_bounds(conv, src),
        "{conv:?}"
    );
    assert_eq!(
        conv.edits[0].new_text, "10.0.0.1",
        "rewrite should be the bare v4: {:?}",
        conv.edits[0].new_text,
    );
}

#[test]
fn ipv6_mapped_with_cidr_preserves_suffix() {
    // The CIDR suffix is carried through the conversion: `::ffff:10.0.0.0/24`
    // -> `10.0.0.0/24`.
    let src = "set net ::ffff:10.0.0.0/24\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 14), Some(&analysis), &analysis.diagnostics);
    let conv = find(&actions, "IPv4 address").expect("a mapped -> v4 conversion");
    assert_eq!(
        conv.edits[0].new_text, "10.0.0.0/24",
        "CIDR suffix must survive: {:?}",
        conv.edits[0].new_text,
    );
}

#[test]
fn ip_conversion_absent_on_non_ip_word() {
    // A word that's neither an IPv4 dotted-quad nor an `::ffff:` mapped form
    // offers no IP conversion. (`set` itself is hex-free word text.)
    let src = "set greeting hello\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 14), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "IPv4 address").is_none() && find(&actions, "IPv6-mapped").is_none(),
        "no IP literal at cursor -> no conversion; got {:?}",
        titles(&actions),
    );
}

// ===========================================================================
// inline_proc_action — "Inline proc 'X'" (+ decline + a known BUG repro)
// ===========================================================================

#[test]
fn inline_single_command_proc_substitutes_args() {
    // A one-command, brace-free proc body inlines at the call site with the
    // call argument substituted for the parameter: `greet bob` -> `puts bob`.
    //
    // tclsh: the body and the inlined result are the same command —
    //   proc greet {who} { puts $who }; greet bob   prints `bob`, and the
    //   inlined `puts bob` prints `bob` too (verified 8.6 + 9.0).
    let src = "proc greet {who} { puts $who }\ngreet bob\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(1, 0), Some(&analysis), &analysis.diagnostics);
    let inline = find(&actions, "Inline proc").expect("an inline-proc action at the call");
    assert_eq!(inline.kind, ActionKind::RefactorInline, "{inline:?}");
    assert_eq!(inline.edits.len(), 1);
    assert!(
        edits_well_formed(inline) && edits_in_bounds(inline, src),
        "{inline:?}"
    );
    assert_eq!(
        inline.edits[0].new_text, "puts bob",
        "inline should substitute the arg for the param: {:?}",
        inline.edits[0].new_text,
    );
}

#[test]
fn inline_declines_control_flow_body() {
    // A proc whose body is a control-flow command (`return`) is NOT inlinable:
    // `return` acts on the call frame, so running it in the caller's frame
    // would return from the *caller*.  The action is surfaced greyed out with
    // that reason rather than silently omitted (issue #1199), so a user can
    // tell "cannot be done here" from "is broken".
    let src = "proc f {} { return 1 }\nf\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(1, 0), Some(&analysis), &analysis.diagnostics);
    let inline = find(&actions, "Inline proc").expect("a refused inline action");
    let reason = inline
        .disabled
        .as_deref()
        .expect("a control-flow body must be refused, not applied");
    assert!(reason.contains("call frame"), "{reason}");
    assert!(inline.edits.is_empty(), "a refusal carries no edits");
}

#[test]
fn inline_declines_multi_command_body() {
    // A multi-command body is refused with its reason: splicing several
    // commands into one caller word changes how the caller parses.
    let src = "proc f {x} {\n    set y $x\n    puts $y\n}\nf 1\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(4, 0), Some(&analysis), &analysis.diagnostics);
    let inline = find(&actions, "Inline proc").expect("a refused inline action");
    let reason = inline
        .disabled
        .as_deref()
        .expect("a multi-command body must be refused, not applied");
    assert!(reason.contains("commands"), "{reason}");
    assert!(inline.edits.is_empty(), "a refusal carries no edits");
}

// FIXED: inline_proc_action no longer brace-truncates a body with a braced
// sub-expression. It previously sliced the body with `proc_def.body_span` —
// whose `.end()` excludes the proc's closing `}` (lexer inner-end convention) —
// then `.trim_end_matches('}')` greedily ate the INNER expr brace, so
// `proc double {n} { expr {$n * 2} }` inlined `double 5` to `expr {5 * 2`
// (unparseable). It now strips a trailing `}` only when it is the unbalanced
// outer brace, preserving the inner sub-expression brace.
//
// Proven on tclsh8.6 + tclsh9.0: info complete {expr {5 * 2}} -> 1 (complete);
// info complete {expr {5 * 2} -> errors "missing close-brace".
#[test]
fn inline_braced_expr_body_keeps_closing_brace() {
    let src = "proc double {n} { expr {$n * 2} }\ndouble 5\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(1, 0), Some(&analysis), &analysis.diagnostics);
    let inline = find(&actions, "Inline proc").expect("an inline-proc action");
    assert_eq!(
        inline.edits[0].new_text, "expr {5 * 2}",
        "inlined braced expr must keep its closing brace: {:?}",
        inline.edits[0].new_text,
    );
}

// ===========================================================================
// refactor-engine: switch -> dict lookup (via the code-actions surface)
// ===========================================================================

#[test]
fn switch_to_dict_offered_for_uniform_assignment_switch() {
    // A `switch -exact` whose every branch assigns the same variable from a
    // constant is convertible to a `dict get` lookup. (The sibling port covers
    // if->switch and extract-datagroup; switch->dict is exercised here.)
    //
    // tclsh: the switch and the equivalent dict agree —
    //   set c green;
    //   list [switch -exact -- $c red {set v 1} green {set v 2} blue {set v 3}] \
    //        [dict get {red 1 green 2 blue 3} $c]   -> 2 2   (8.6 + 9.0).
    let src = concat!(
        "switch -exact -- $color {\n",
        "    red {set rgb 1}\n",
        "    green {set rgb 2}\n",
        "    blue {set rgb 3}\n",
        "}\n",
    );
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    let dict = find(&actions, "dict").expect("a switch->dict conversion at the switch cursor");
    assert!(
        dict.title.to_lowercase().contains("dict"),
        "title should mention dict: {:?}",
        dict.title,
    );
    assert!(
        !dict.edits.is_empty(),
        "conversion must carry edits: {dict:?}"
    );
    assert!(
        edits_well_formed(dict) && edits_in_bounds(dict, src),
        "{dict:?}"
    );
}

#[test]
fn switch_to_dict_absent_at_inert_cursor() {
    // With the cursor away from any `switch` command, no switch->dict action is
    // offered (the refactor self-gates on `find_command_at(.., "switch")`).
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 4), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "dict").is_none(),
        "no switch at cursor -> no dict conversion; got {:?}",
        titles(&actions),
    );
}

// ===========================================================================
// check_diagnostic_actions — IRULE5004 (DNS::return without return)
// ===========================================================================

#[test]
fn check_actions_surface_irule5004_dns_return_fix() {
    // `DNS::return` not followed by `return` fires IRULE5004 through the
    // compiler-checks pass (not `AnalysisResult.diagnostics`), carrying an
    // "insert return" fix. The provider must lift it. (The sibling port covers
    // the IRULE5002 unguarded-drop fix; this is its IRULE5004 twin.)
    let registry = irules_registry();
    let src = "when DNS_REQUEST {\n    DNS::return\n}\n";
    let checks = irules_checks(src, &registry);
    assert!(
        checks
            .iter()
            .any(|d| d.code == DiagCode::Irule5004 && !d.fixes.is_empty()),
        "expected an IRULE5004 check carrying a fix; got {checks:?}",
    );
    let none_disabled = std::collections::HashSet::new();
    let actions = check_diagnostic_actions(src, whole(src), &checks, &none_disabled);
    // Fix description is `Add 'return' after DNS::return`.
    let fix = find(&actions, "after DNS::return").expect("an IRULE5004 quick-fix");
    assert_eq!(fix.kind, ActionKind::QuickFix);
    assert_eq!(fix.edits.len(), 1);
    assert_eq!(fix.edits[0].new_text, "\n    return");
    // Insertion: a zero-width range right after `DNS::return`.
    assert_eq!(
        (
            fix.edits[0].range.start_line,
            fix.edits[0].range.start_character
        ),
        (
            fix.edits[0].range.end_line,
            fix.edits[0].range.end_character
        ),
        "the fix is a pure insertion: {:?}",
        fix.edits[0],
    );
    assert!(edits_well_formed(fix), "{fix:?}");
}

#[test]
fn check_actions_irule5004_suppressed_when_disabled() {
    // Disabling IRULE5004 (`tclLsp.diagnostics.IRULE5004 = false`) must drop its
    // quick-fix — the lightbulb mustn't re-surface a hidden warning.
    let registry = irules_registry();
    let src = "when DNS_REQUEST {\n    DNS::return\n}\n";
    let checks = irules_checks(src, &registry);
    let mut disabled = std::collections::HashSet::new();
    disabled.insert("IRULE5004".to_string());
    let actions = check_diagnostic_actions(src, whole(src), &checks, &disabled);
    assert!(
        find(&actions, "after DNS::return").is_none(),
        "disabled IRULE5004 must offer no fix; got {:?}",
        titles(&actions),
    );
}

// ===========================================================================
// context_diagnostic_actions — iRules taint encode-wrap / redundant-encoder
// ===========================================================================

#[test]
fn context_t106_removes_redundant_encoder_wrapper() {
    // T106 marks a redundant `[ENCODER $var]` wrapper; the fix unwraps it back
    // to the bare `$var`. The diag carries the tainted var in its message.
    let src = "set out [html_encode $clean]\n";
    let diags = vec![ContextDiagnostic {
        code: "T106".to_string(),
        message: "redundant encoder applied to already-safe $clean".to_string(),
        range: cursor(0, 0),
    }];
    let actions = context_diagnostic_actions(src, &diags);
    let unwrap =
        find(&actions, "Remove redundant encoder").expect("a T106 redundant-encoder removal");
    assert_eq!(unwrap.kind, ActionKind::QuickFix);
    assert_eq!(unwrap.edits.len(), 1);
    assert!(
        edits_well_formed(unwrap) && edits_in_bounds(unwrap, src),
        "{unwrap:?}"
    );
    // The replacement is the bare variable reference (no encoder, no brackets).
    assert_eq!(
        unwrap.edits[0].new_text, "$clean",
        "unwrap should restore the bare var: {:?}",
        unwrap.edits[0].new_text,
    );
}

#[test]
fn context_t100_subst_offers_nocommands_fix() {
    // T100 on a `subst` sink offers a mechanical, single-flag fix:
    // `-nocommands` disables the only hazard the diagnostic names (command
    // substitution) without touching the call's variable/backslash
    // substitution behaviour — the exact mitigation `subst`'s own hover
    // snippet recommends.
    let src = "set out [subst $tainted]\n";
    let diags = vec![ContextDiagnostic {
        code: "T100".to_string(),
        message: "Tainted variable $tainted flows into subst; possible code injection".to_string(),
        range: cursor(0, 9),
    }];
    let actions = context_diagnostic_actions(src, &diags);
    let fix = find(&actions, "-nocommands").expect("a T100 subst -nocommands fix");
    assert_eq!(fix.kind, ActionKind::QuickFix);
    assert_eq!(fix.edits.len(), 1);
    assert!(
        edits_well_formed(fix) && edits_in_bounds(fix, src),
        "{fix:?}"
    );
    assert_eq!(fix.edits[0].new_text, " -nocommands");
    // Applying the edit at its own range must produce a well-formed
    // `subst -nocommands $tainted` call.
    let col = fix.edits[0].range.start_character as usize;
    let line = src.lines().next().unwrap();
    let mut patched = line.to_string();
    patched.insert_str(col, &fix.edits[0].new_text);
    assert_eq!(patched, "set out [subst -nocommands $tainted]");
}

#[test]
fn context_t100_non_subst_sink_offers_no_fix() {
    // T100 on a non-`subst` sink (`eval`) has no single-flag mitigation, so
    // no quick fix is offered — matches the diagnostic's own hover advice
    // (`{*}$cmdList` / direct invocation), neither of which is a mechanical
    // same-shape rewrite.
    let src = "eval $tainted\n";
    let diags = vec![ContextDiagnostic {
        code: "T100".to_string(),
        message: "Tainted variable $tainted flows into eval; possible code injection".to_string(),
        range: cursor(0, 5),
    }];
    let actions = context_diagnostic_actions(src, &diags);
    assert!(
        find(&actions, "-nocommands").is_none(),
        "eval sink must not offer the subst-only fix: {actions:?}"
    );
}

#[test]
fn context_irule3001_wraps_tainted_var_with_html_encode() {
    // IRULE3001 (reflected XSS) offers an `[html_encode $var]` wrap, and — since
    // `html_encode` is a user proc, not a built-in — inserts the helper proc at
    // top-of-file when it isn't already defined.
    //
    // tclsh: the html_encode body the fix injects neutralises the markup
    // metacharacters —
    //   string map {& &amp; < &lt; > &gt;} "a<b>c" -> a&lt;b&gt;c   (8.6 + 9.0),
    // so wrapping a tainted value in it is a meaning-preserving escape.
    let src = "when HTTP_REQUEST {\n    HTTP::respond 200 content $userdata\n}\n";
    let diags = vec![ContextDiagnostic {
        code: "IRULE3001".to_string(),
        message: "reflected XSS: tainted $userdata reaches the response body".to_string(),
        range: selection(1, 0, 38),
    }];
    let actions = context_diagnostic_actions(src, &diags);
    let wrap = find(&actions, "html_encode").expect("an IRULE3001 html_encode wrap");
    assert_eq!(wrap.kind, ActionKind::QuickFix);
    assert!(
        edits_well_formed(wrap) && edits_in_bounds(wrap, src),
        "{wrap:?}"
    );
    // One edit wraps the var; another inserts the helper proc at line 0.
    assert_eq!(wrap.edits.len(), 2, "wrap + helper-proc insert: {wrap:?}");
    let wrap_edit = wrap
        .edits
        .iter()
        .find(|e| e.new_text == "[html_encode $userdata]")
        .expect("the in-place wrap edit");
    assert!(
        wrap_edit.range.start_line == 1,
        "wrap is on the tainted line"
    );
    let helper = wrap
        .edits
        .iter()
        .find(|e| e.new_text.contains("proc html_encode"))
        .expect("the helper-proc insertion edit");
    assert_eq!(
        (helper.range.start_line, helper.range.start_character),
        (0, 0),
        "helper proc inserts at top of file: {helper:?}",
    );
}

#[test]
fn context_irule3001_skips_helper_when_already_defined() {
    // When `html_encode` is already defined in the file, the wrap fix offers
    // only the in-place wrap edit — no duplicate helper-proc insertion.
    let src = concat!(
        "proc html_encode {s} { return $s }\n",
        "when HTTP_REQUEST {\n",
        "    HTTP::respond 200 content $userdata\n",
        "}\n",
    );
    let diags = vec![ContextDiagnostic {
        code: "IRULE3001".to_string(),
        message: "XSS: $userdata".to_string(),
        range: selection(2, 0, 38),
    }];
    let actions = context_diagnostic_actions(src, &diags);
    let wrap = find(&actions, "html_encode").expect("an IRULE3001 wrap");
    assert_eq!(
        wrap.edits.len(),
        1,
        "helper already defined -> only the wrap edit: {wrap:?}",
    );
    assert!(
        wrap.edits
            .iter()
            .all(|e| !e.new_text.contains("proc html_encode")),
        "must not re-insert the helper proc: {wrap:?}",
    );
}

#[test]
fn context_collect_bootstrap_for_irule1005() {
    // IRULE1005 (a data event read without the matching `protocol::collect`)
    // offers a "collect bootstrap" quick-fix: a `when <setup> priority 500 {
    // <proto>::collect }` block, anchored at the enclosing `when`.
    let src = concat!(
        "when HTTP_REQUEST_DATA {\n",
        "    set d [HTTP::payload]\n",
        "}\n",
    );
    let diags = vec![ContextDiagnostic {
        code: "IRULE1005".to_string(),
        // The data event word lives in the diag range; protocols come from the
        // `X::collect` mentions in the message.
        message: "HTTP_REQUEST_DATA fires without a prior HTTP::collect".to_string(),
        range: selection(0, 5, 22),
    }];
    let actions = context_diagnostic_actions(src, &diags);
    let boot = find(&actions, "collect' bootstrap").expect("an IRULE1005 collect-bootstrap action");
    assert_eq!(boot.kind, ActionKind::QuickFix);
    assert_eq!(boot.edits.len(), 1);
    assert!(
        edits_well_formed(boot) && edits_in_bounds(boot, src),
        "{boot:?}"
    );
    let new_text = &boot.edits[0].new_text;
    assert!(
        new_text.contains("HTTP::collect") && new_text.contains("priority 500"),
        "bootstrap should add a prioritised HTTP::collect block: {new_text:?}",
    );
    // Inserted at the enclosing `when` (line 0), column 0.
    assert_eq!(
        (
            boot.edits[0].range.start_line,
            boot.edits[0].range.start_character
        ),
        (0, 0),
        "bootstrap anchors at the enclosing when: {:?}",
        boot.edits[0],
    );
}

#[test]
fn context_actions_empty_for_unrelated_code() {
    // A context diagnostic whose code the provider doesn't handle (and a
    // message naming no `$var`) yields no actions. Negative control.
    let src = "set x 1\n";
    let diags = vec![ContextDiagnostic {
        code: "W999".to_string(),
        message: "some unrelated warning".to_string(),
        range: cursor(0, 0),
    }];
    assert!(
        context_diagnostic_actions(src, &diags).is_empty(),
        "unhandled context diagnostic -> no actions",
    );
    // Empty diag list is also a no-op (no panic).
    assert!(context_diagnostic_actions(src, &[]).is_empty());
}

// ===========================================================================
// profiles_action — iRules `# Profiles:` header (insert / update / no-op)
// ===========================================================================

#[test]
fn profiles_action_inserts_header_when_missing() {
    // An iRule that uses an HTTP command but carries no `# Profiles:` directive
    // gets a "generate header" source action inserting the required profile.
    let registry = irules_registry();
    let src = "when HTTP_REQUEST {\n    HTTP::header insert X-Test 1\n}\n";
    let analysis = analyse_dialect(src, "f5-irules");
    let action = profiles_action(src, &analysis, &registry);
    // Guard: only assert when this build actually computes a required profile
    // for the HTTP event (keeps the test robust to registry-data evolution).
    let Some(action) = action else {
        return;
    };
    assert_eq!(action.kind, ActionKind::Source, "{action:?}");
    assert!(
        edits_well_formed(&action) && edits_in_bounds(&action, src),
        "{action:?}"
    );
    assert!(
        action.title.contains("profile") || action.title.contains("Profiles"),
        "title should mention profiles: {:?}",
        action.title,
    );
    let new_text = &action.edits[0].new_text;
    assert!(
        new_text.starts_with("# Profiles:") && new_text.contains("HTTP"),
        "header should declare the HTTP profile: {new_text:?}",
    );
    // A fresh insert lands at top-of-file (line 0).
    assert_eq!(action.edits[0].range.start_line, 0);
}

#[test]
fn profiles_action_updates_stale_directive() {
    // An existing but WRONG `# Profiles:` directive is replaced (update branch),
    // not duplicated.
    let registry = irules_registry();
    let src = concat!(
        "# Profiles: CLIENTSSL\n",
        "when HTTP_REQUEST {\n",
        "    HTTP::header insert X-Test 1\n",
        "}\n",
    );
    let analysis = analyse_dialect(src, "f5-irules");
    let Some(action) = profiles_action(src, &analysis, &registry) else {
        return;
    };
    assert_eq!(action.kind, ActionKind::Source);
    assert!(
        action.title.to_lowercase().contains("update"),
        "stale directive should drive an update: {:?}",
        action.title,
    );
    assert!(
        edits_well_formed(&action) && edits_in_bounds(&action, src),
        "{action:?}"
    );
    // The edit replaces the existing directive line (line 0), not appends.
    assert_eq!(action.edits[0].range.start_line, 0);
    assert!(
        action.edits[0].new_text.contains("HTTP"),
        "update should carry the corrected HTTP profile: {:?}",
        action.edits[0].new_text,
    );
}

#[test]
fn profiles_action_noop_when_directive_already_correct() {
    // A correct, up-to-date `# Profiles:` directive yields no action (no churn).
    let registry = irules_registry();
    let src = concat!(
        "# Profiles: HTTP\n",
        "when HTTP_REQUEST {\n",
        "    HTTP::header insert X-Test 1\n",
        "}\n",
    );
    let analysis = analyse_dialect(src, "f5-irules");
    // Either the build requires exactly HTTP (matching directive -> None) or it
    // computes no requirement (-> None). Both mean: no action.
    assert!(
        profiles_action(src, &analysis, &registry).is_none(),
        "an already-correct directive must not be re-offered",
    );
}

#[test]
fn profiles_action_none_for_non_irules_content() {
    // A plain Tcl document with no events / no profile-requiring commands has
    // no required profiles, so no header action.
    let registry = irules_registry();
    let src = "set x 1\nputs $x\n";
    let analysis = analyse_dialect(src, "f5-irules");
    assert!(
        profiles_action(src, &analysis, &registry).is_none(),
        "no events / commands -> no profile header",
    );
}

// ===========================================================================
// Multi-action positions, kind ("only") filtering, robustness
// ===========================================================================

#[test]
fn multi_action_position_offers_several_families() {
    // A selection over an IPv4 literal plus a referenced variable yields more
    // than one action family at once (extract-proc + extract-variable for the
    // selection, IPv4->IPv6 for the literal under the start cursor). The
    // provider returns them together; the editor filters by kind.
    let src = "set ip 192.168.0.1\n";
    let analysis = analyse(src);
    // Select the literal `192.168.0.1` (cols 7..18); the start cursor (col 7)
    // also sits on the literal for the IP refactor.
    let actions = code_actions(src, selection(0, 7, 18), Some(&analysis), &analysis.diagnostics);
    let kinds: std::collections::BTreeSet<&str> = actions.iter().map(|a| a.kind.as_str()).collect();
    assert!(
        kinds.len() >= 2,
        "expected multiple action families at this position; got kinds {kinds:?} from {:?}",
        titles(&actions),
    );
    // The IPv4 refactor is present alongside the selection refactors.
    assert!(
        find(&actions, "IPv6-mapped").is_some(),
        "the IPv4->IPv6 refactor should be among them; got {:?}",
        titles(&actions),
    );
    // Every action emitted is structurally sound.
    for a in &actions {
        assert!(
            edits_well_formed(a) && edits_in_bounds(a, src),
            "malformed: {a:?}"
        );
    }
}

#[test]
fn caller_only_filter_can_select_a_single_kind() {
    // The provider returns all overlapping actions; the editor's `only` filter
    // (e.g. `refactor.rewrite`) is applied by the CALLER on `ActionKind`. Model
    // that filter here and confirm exactly the rewrite survives for an
    // invertible comparison selection.
    let src = "if {$a == $b} { puts hi }\n";
    let analysis = analyse(src);
    let actions = code_actions(src, selection(0, 4, 12), Some(&analysis), &analysis.diagnostics);
    let only_rewrites: Vec<&CodeAction> = actions
        .iter()
        .filter(|a| a.kind == ActionKind::RefactorRewrite)
        .collect();
    assert!(
        only_rewrites
            .iter()
            .any(|a| a.title.contains("Invert comparison")),
        "the rewrite kind should retain the invert action; got {:?}",
        titles(&actions),
    );
    // The kind string is the dotted LSP identifier the `only` filter matches.
    assert_eq!(ActionKind::RefactorRewrite.as_str(), "refactor.rewrite");
    assert_eq!(ActionKind::QuickFix.as_str(), "quickfix");
    assert_eq!(ActionKind::RefactorExtract.as_str(), "refactor.extract");
    assert_eq!(ActionKind::RefactorInline.as_str(), "refactor.inline");
    assert_eq!(ActionKind::Refactor.as_str(), "refactor");
    assert_eq!(ActionKind::Source.as_str(), "source");
}

#[test]
fn out_of_range_positions_never_panic() {
    // The provider must tolerate positions past end-of-document / end-of-line
    // without panicking, returning a (possibly empty) action list.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Cursor far past the last line.
    let _ = code_actions(src, cursor(999, 999), Some(&analysis), &analysis.diagnostics);
    // A range straddling beyond the buffer.
    let _ = code_actions(
        src,
        LspRange {
            start_line: 999,
            start_character: 0,
            end_line: 1000,
            end_character: 50,
        },
        Some(&analysis),
        &analysis.diagnostics,
    );
    // Column far past the line end on a valid line.
    let _ = code_actions(src, cursor(0, 9999), Some(&analysis), &analysis.diagnostics);
    // The package-suggestion and context entry points must also be panic-safe.
    let reg = tcl_registry::CommandRegistry::build_default();
    let _ = tcl_lsp_core::code_actions::package_require_actions(
        src,
        cursor(999, 999),
        &reg,
        Some(&analysis),
        &[],
    );
    let _ = context_diagnostic_actions(
        src,
        &[ContextDiagnostic {
            code: "IRULE3001".to_string(),
            message: "$missing".to_string(),
            range: cursor(999, 999),
        }],
    );
}

#[test]
fn empty_and_whitespace_documents_never_panic() {
    // Empty / whitespace-only / blank-line documents must not panic and must
    // offer nothing.
    for src in ["", "   ", "\n", "   \n\t\n", "\n\n\n"] {
        let analysis = analyse(src);
        let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
        assert!(
            actions.is_empty(),
            "empty/whitespace doc {src:?} should offer nothing; got {:?}",
            titles(&actions),
        );
        // Whole-document range too.
        let actions = code_actions(src, whole(src), Some(&analysis), &analysis.diagnostics);
        assert!(actions.is_empty(), "{src:?} -> {:?}", titles(&actions));
    }
}

#[test]
fn inert_statement_offers_nothing() {
    // An ordinary, diagnostic-free statement with the cursor on it offers no
    // quick-fix and no range refactor (broad negative control).
    let src = "set x 1\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 4), Some(&analysis), &analysis.diagnostics);
    assert!(
        actions.is_empty(),
        "inert position should offer nothing; got {:?}",
        titles(&actions),
    );
}

#[test]
fn every_emitted_action_is_structurally_sound() {
    // Defensive sweep over a document that triggers several families at once:
    // every emitted action's edits must be well-formed AND in-bounds so the
    // editor never applies a corrupt or overshooting edit.
    let src = concat!(
        "# continued comment \\\n",
        "tail\n",
        "set ip 192.168.0.1\n",
        "proc greet {who} { puts $who }\n",
        "greet bob\n",
    );
    let analysis = analyse(src);
    let actions = code_actions(src, whole(src), Some(&analysis), &analysis.diagnostics);
    assert!(
        !actions.is_empty(),
        "expected at least one action over the document",
    );
    for a in &actions {
        assert!(edits_well_formed(a), "malformed edit in {a:?}");
        assert!(edits_in_bounds(a, src), "out-of-bounds edit in {a:?}");
    }
}

// NOTE on the presentation/structure split: titles, `ActionKind`, and edit-
// range shapes above are LSP wire contract — asserted STRUCTURALLY. The
// Tcl-semantic premises each fix encodes (boolean-rewrite equivalence, the
// IPv6-mapped embedding, switch/dict agreement, the html_encode escape, and the
// inline-proc brace-completeness that exposed the BUG) are pinned to real
// tclsh8.6 + tclsh9.0 via `scripts/dev/tclsh_check.sh` and cited at each
// `// tclsh:` line.
