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

//! Analysing valid multi-byte UTF-8 must never abort, and must never report a
//! span that lands inside a character (issue #1325).
//!
//! The trigger is a **compound** body word — a braced group welded to more word
//! characters, `{body}x`.  Real Tcl rejects that outright ("extra characters
//! after close-brace"), but the analyser accepts it so the braced part still
//! gets diagnosed, and the word's *value* is then the brace content
//! concatenated with the trailing fragment, closing `}` dropped.  Rebasing that
//! value by a single offset slides every token past the dropped brace one byte
//! left: invisible on ASCII, mid-character on anything else, at which point the
//! first `source[start..end]` in the analyser panics.
//!
//! Found in the wild on `erkac/f5-irules`'s `expire-url-faster.irule`, whose 40
//! zero-width spaces (`U+200B` — the usual residue of a copy-paste out of a web
//! page) sit immediately after almost every brace.  `cargo xtask fp-sweep`
//! aborted on it; the LSP contained the panic and published **nothing**, so the
//! file read as clean while a `WARNING IRULE3102` URL-evasion finding went
//! missing.

use tcl_compiler::analyser::Analyser;

/// The minimised repro: one compound body word with one zero-width space
/// welded to its closing brace.  Every larger shape in this file reduces to it.
const MINIMAL: &str = "if {1} {puts hi}\u{200b}\n";

/// The corpus file's shape — `when` → `if` → `switch`, with a run of
/// zero-width spaces after every opening and closing brace.  Kept because the
/// original panic needed two levels of body nesting to reach the failing
/// slice, and the minimised case exercises only one.
const NESTED: &str = concat!(
    "when CACHE_REQUEST {\u{200b}\u{200b}\u{200b}\u{200b}\n",
    "  if {\u{200b}\u{200b}\u{200b}\u{200b} [CACHE::age] > 60 }\u{200b}\u{200b}\u{200b}\u{200b} \
     {\u{200b}\u{200b}\u{200b}\u{200b}\n",
    "    switch -glob [string tolower [HTTP::uri]] {\u{200b}\u{200b}\u{200b}\u{200b}\n",
    "      \"/test\" -\n",
    "      \"/obsadenost\" {\u{200b}\u{200b}\u{200b}\u{200b}\n",
    "        CACHE::expire\n",
    "      }\u{200b}\u{200b}\u{200b}\u{200b}\n",
    "    }\u{200b}\u{200b}\u{200b}\u{200b}\n",
    "  }\u{200b}\u{200b}\u{200b}\u{200b}\n",
    "}\u{200b}\u{200b}\u{200b}\u{200b}",
);

/// Analyse `src` and assert every span it reports is sliceable out of `src`.
///
/// The analysis itself is the panic assertion — a non-boundary offset aborts
/// inside the analyser long before this returns.  The span audit then catches
/// the quieter half of the same bug: an off-by-one span that happens to stay on
/// a boundary is still a wrong position handed to the editor.
fn assert_spans_are_sliceable(src: &str, dialect: &str) {
    let mut analyser = Analyser::new();
    let result = analyser.analyse(src, dialect);
    for diagnostic in &result.diagnostics {
        let (start, end) = (
            diagnostic.span.start() as usize,
            diagnostic.span.end() as usize,
        );
        assert!(
            src.get(start..end).is_some(),
            "{} span {start}..{end} is not a sliceable range of the source",
            diagnostic.code,
        );
    }
}

#[test]
fn compound_body_word_with_a_multibyte_tail_does_not_abort() {
    assert_spans_are_sliceable(MINIMAL, "tcl8.6");
}

#[test]
fn nested_irule_bodies_with_zero_width_spaces_do_not_abort() {
    assert_spans_are_sliceable(NESTED, "f5-irules");
}

#[test]
fn a_multibyte_tail_is_not_specific_to_zero_width_space() {
    // Any multi-byte character welded to the closing brace reaches the same
    // slice; the zero-width space is only how it turned up in the wild.
    for tail in ["\u{200b}", "é", "→", "🎈", "\u{202e}"] {
        assert_spans_are_sliceable(&format!("if {{1}} {{puts hi}}{tail}\n"), "tcl8.6");
        assert_spans_are_sliceable(&format!("proc p {{}} {{\n  set x 1\n}}{tail}\n"), "tcl8.6");
        assert_spans_are_sliceable(
            &format!("switch -glob $x {{\n  a {{puts one}}{tail}\n}}\n"),
            "tcl8.6",
        );
    }
}

#[test]
fn the_zero_width_irule_still_reports_its_security_finding() {
    // The whole point of not aborting: IRULE3102 (URL evasion via a
    // non-normalised `HTTP::uri`) is a real security finding, and the panic
    // used to take it — and every other diagnostic for the file — with it.
    let mut analyser = Analyser::new();
    let result = analyser.analyse(NESTED, "f5-irules");
    let codes: Vec<String> = result
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "IRULE3102"),
        "IRULE3102 missing from {codes:?}",
    );
}

#[test]
fn a_well_formed_braced_body_is_analysed_whole() {
    // The clamp that fixes the compound word must not shorten an ordinary
    // braced body: `contiguous_prefix` returns it unchanged, so the nested
    // command is still reached.
    let src = "when HTTP_REQUEST {\n  if {1} {\n    log local0. \"[HTTP::uri]\"\n  }\n}\n";
    let mut analyser = Analyser::new();
    let result = analyser.analyse(src, "f5-irules");
    let codes: Vec<String> = result
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "IRULE3102"),
        "the nested `HTTP::uri` was not reached: {codes:?}",
    );
}

#[test]
fn contiguous_prefix_keeps_a_verbatim_body_and_clamps_a_welded_one() {
    use tcl_compiler::segmenter::contiguous_prefix;

    // Verbatim: the body value is the source between the braces.
    let src = "if {1} {puts hi}\n";
    assert_eq!(contiguous_prefix(src, 8, "puts hi"), "puts hi");

    // Welded: the value drops the `}`, so only the braced part is contiguous.
    let src = "if {1} {puts hi}\u{200b}\n";
    assert_eq!(contiguous_prefix(src, 8, "puts hi\u{200b}"), "puts hi");

    // The clamp lands on a character boundary even when the divergence is
    // partway through a multi-byte sequence.
    let src = "a\u{200b}b";
    assert_eq!(contiguous_prefix(src, 0, "a\u{200c}b"), "a");

    // Nothing to compare against — an isolated fragment, or a handler called
    // with fabricated spans — leaves the text alone rather than truncating on
    // no evidence.  Never a panic either way.
    assert_eq!(contiguous_prefix("abc", 99, "abc"), "abc");
    assert_eq!(contiguous_prefix("", 0, "set x 1"), "set x 1");
    // A base that is itself mid-character is unverifiable, not a panic.
    assert_eq!(contiguous_prefix("a\u{200b}b", 2, "x"), "x");
}

#[test]
fn body_text_in_region_leaves_a_position_preserving_overlay_alone() {
    use tcl_compiler::segmenter::body_text_in_region;

    // `eval {} foo` is walked through `concat_script_window`, whose padded
    // window blanks the delimiters but keeps `foo` at its written offset.  It
    // is deliberately *not* byte-identical to the source, so only the length
    // check can tell it apart from a compound word — clamping it would lose
    // the call entirely (`checks.rs::eval_offsets_of_the_walked_script_land_
    // on_real_source_bytes_1051`).
    let src = "proc foo {} {}\neval {} foo\n";
    let window = "    foo";
    assert_eq!(body_text_in_region(src, 20, 27, window), window);

    // The compound word is one byte short of its region — that is the shape
    // the clamp exists for.
    let src = "if {1} {puts hi}\u{200b}\n";
    assert_eq!(
        body_text_in_region(src, 8, 19, "puts hi\u{200b}"),
        "puts hi"
    );

    // An ordinary braced body fills its region exactly and is untouched.
    let src = "if {1} {puts hi}\n";
    assert_eq!(body_text_in_region(src, 8, 15, "puts hi"), "puts hi");
}

// The lowering side of the same guard (issue #1393).
//
// IR spans are the contract every downstream consumer trusts — codegen slices
// the source with them, the LSP turns them into editor ranges — so the
// lowerer has to be as truthful a span producer as the analyser.  It rebases a
// body word's tokens by one offset in the same places and for the same
// reasons, and until #1393 it did so with no guard at all: the shapes that
// still reach a body rebase past #1375's literal gates (`namespace eval`,
// `eval`, `uplevel`, `apply`, `when`) slid every token of a compound `{body}x`
// word one byte left, exactly as `analyse_body` used to.

/// Every statement span the lowering emits for `src`, checked against `src`.
///
/// `str::get` is the whole assertion: it returns `None` both for a span that
/// runs past the end and for one that lands inside a UTF-8 sequence — the two
/// failure modes of an unguarded rebase.  The lowering's own
/// `debug_assert_spans_in_source` fires first on a test build; this catches
/// the same fault from the outside, and names the statement when it does.
fn assert_lowered_spans_are_sliceable(src: &str, dialect: &str) {
    use tcl_compiler::ir::for_each_statement;

    let registry = tcl_registry::registry_for_dialect(dialect);
    let module = tcl_compiler::lowering::lower_to_ir_with_config(
        src,
        registry,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    );
    let check = |label: &str, script: &tcl_compiler::ir::Script| {
        for_each_statement(script, &mut |stmt| {
            let span = stmt.span();
            assert!(
                src.get(span.as_range()).is_some(),
                "{label}: {}..{} is not a sliceable range of {src:?}",
                span.start(),
                span.end(),
            );
        });
    };
    check("top level", &module.top_level);
    for (name, procedure) in &module.procedures {
        check(name, &procedure.body);
    }
    for (name, unit) in &module.body_units {
        check(name, &unit.body);
    }
}

/// The shapes that still reach a body rebase after #1375's literal gates, each
/// with a compound body word — the `{body}x` weld that slides the rebase.
fn compound_body_shapes(tail: &str) -> Vec<(String, &'static str)> {
    vec![
        (format!("namespace eval n {{set x 1}}{tail}\n"), "tcl8.6"),
        (
            format!("namespace eval n {{ proc p {{}} {{set x 1}} }}{tail}\n"),
            "tcl8.6",
        ),
        (format!("eval {{set x 1}}{tail}\n"), "tcl8.6"),
        (format!("uplevel 1 {{set x 1}}{tail}\n"), "tcl8.6"),
        (format!("uplevel #0 {{set x 1}}{tail}\n"), "tcl8.6"),
        (format!("apply {{{{}} {{set x 1}}}}{tail}\n"), "tcl8.6"),
        (format!("apply {{{{}} {{set x 1}}{tail}}}\n"), "tcl8.6"),
        (
            format!("when HTTP_REQUEST {{set x 1}}{tail}\n"),
            "f5-irules",
        ),
        // …and the gated forms, which must stay in bounds however they are
        // resolved (barrier or lowered body).
        (format!("if {{1}} {{puts hi}}{tail}\n"), "tcl8.6"),
        (format!("while {{1}} {{puts hi}}{tail}\n"), "tcl8.6"),
        (format!("foreach i {{a b}} {{puts $i}}{tail}\n"), "tcl8.4"),
        (format!("proc p {{}} {{set x 1}}{tail}\n"), "tcl8.6"),
        (format!("catch {{set x 1}}{tail} msg\n"), "tcl8.6"),
        (format!("switch $x {{ a {{puts hi}}{tail} }}\n"), "tcl8.6"),
        (
            format!("oo::class create C {{ method m {{}} {{set x 1}}{tail} }}\n"),
            "tcl8.6",
        ),
    ]
}

#[test]
fn lowered_spans_survive_a_compound_body_word() {
    // The ASCII weld is the quiet half of #1325: every span past the dropped
    // `}` is one byte out, which `str::get` cannot see. It is here so the
    // matrix covers both halves; the multi-byte tails below are what turn the
    // same slide into an unsliceable span.
    for tail in ["x", "\u{200b}", "é", "→", "🎈"] {
        for (src, dialect) in compound_body_shapes(tail) {
            assert_lowered_spans_are_sliceable(&src, dialect);
        }
    }
}

#[test]
fn lowered_spans_survive_the_zero_width_irule() {
    // The corpus shape from #1325, lowered rather than analysed.
    assert_lowered_spans_are_sliceable(NESTED, "f5-irules");
    assert_lowered_spans_are_sliceable(MINIMAL, "tcl8.6");
}

#[test]
fn a_compound_body_lowers_at_the_offsets_it_is_written_at() {
    // The sliceability audit above catches the mid-character half of the bug.
    // This is the quiet half: with the welded `}` dropped and no clamp, the
    // inner `set x 1` used to lower one byte wide (`set x 1}`) on ASCII and
    // one byte left of its written position on anything wider.
    for (src, body_unit) in [
        ("namespace eval n {set x 1}x\n", true),
        ("namespace eval n {set x 1}\u{200b}\n", true),
        ("apply {{} {set x 1}}x\n", true),
        ("uplevel 1 {set x 1}x\n", false),
        ("eval {set x 1}x\n", false),
    ] {
        let registry = tcl_registry::registry_for_dialect("tcl8.6");
        let module = tcl_compiler::lowering::lower_to_ir(src, registry);
        let scripts: Vec<&tcl_compiler::ir::Script> = if body_unit {
            module.body_units.values().map(|unit| &unit.body).collect()
        } else {
            vec![&module.top_level]
        };
        let mut inner = Vec::new();
        for script in scripts {
            tcl_compiler::ir::for_each_statement(script, &mut |stmt| {
                if let Some(text) = src.get(stmt.span().as_range())
                    && text.starts_with("set")
                {
                    inner.push(text.to_owned());
                }
            });
        }
        assert_eq!(
            inner,
            vec!["set x 1".to_owned()],
            "the welded tail must be dropped, not lowered, in {src:?}",
        );
    }
}
