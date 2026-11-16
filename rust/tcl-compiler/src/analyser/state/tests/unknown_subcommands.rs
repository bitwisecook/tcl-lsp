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

//! W001: an unknown subcommand, its suggestion, fix and shadow suppression.

use super::*;

// -- W001 unknown-subcommand emitter

#[test]
fn analyse_emits_w001_for_unknown_string_subcommand() {
    let mut a = Analyser::new();
    let r = a.analyse("string bogus $x\n", "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w001[0].message.contains("'bogus'") && w001[0].message.contains("'string'"),
        "got {:?}",
        w001[0].message
    );
    assert!(matches!(
        w001[0].severity,
        crate::analyser::Severity::Warning
    ));
}

#[test]
fn analyse_w001_includes_did_you_mean_suggestion() {
    // ``string lenght`` — single-char typo for ``length``,
    // edit distance 2.  ``suggest_similar`` should surface
    // ``length``.
    let mut a = Analyser::new();
    let r = a.analyse("string lenght $x\n", "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w001[0].message.contains("did you mean 'length'"),
        "got {:?}",
        w001[0].message
    );
    assert!(
        w001[0].fixes.iter().any(|f| f.new_text == "length"),
        "got {:?}",
        w001[0].fixes
    );
}

#[test]
fn analyse_no_w001_for_known_subcommand() {
    // ``string length $x`` — known subcommand.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("string length $x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_dynamic_subcommand_position() {
    // ``string $sub $x`` — runtime-resolved subcommand;
    // can't statically check.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("string $sub $x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_command_substitution_in_subcommand_position() {
    // ``string [pick] $x`` — ``[…]`` in the subcommand
    // position is also a runtime-resolved value.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("string [pick] $x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_simple_command() {
    // ``set x 1`` — ``set`` has no SubcommandSig.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("set x 1\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_unknown_command() {
    // ``unknownthing foo`` — registry doesn't know the
    // command, so no signature lookup, no W001.  (W123
    // owns the unknown-command diagnostic.)
    let mut a = Analyser::new();
    let r = a.analyse("unknownthing foo\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w001_anchors_at_subcommand_token_only() {
    // The squiggle sits tightly on the offending word alone — not the
    // command name too — matching the "did you mean" fix's replacement
    // range and the KCS doc's documented "squiggle under the subcommand
    // token" behaviour.
    let mut a = Analyser::new();
    let src = "string bogus $x\n";
    let r = a.analyse(src, "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1);
    let span = w001[0].span;
    let text = &src[span.start() as usize..span.end() as usize];
    assert_eq!(text, "bogus", "got {text:?}");
}

#[test]
fn analyse_emits_w001_for_unknown_dict_subcommand() {
    // ``dict`` is also a SubcommandSig command — confirm
    // dispatch isn't ``string``-specific.
    let mut a = Analyser::new();
    let r = a.analyse("dict froob $d $k\n", "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w001[0].message.contains("'dict'"),
        "got {:?}",
        w001[0].message
    );
}

#[test]
fn analyse_w001_fix_replaces_wrapped_literal_subcommand() {
    // Wrapper tokens (``Str`` braced ``{lenght}`` / ``Esc``
    // quoted ``"lenght"``) carry the opening delimiter via
    // ``content_offset`` and the lexer span excludes the
    // closing delimiter; the W001 code-fix targets the
    // content range so the replacement preserves the
    // wrapping ``}`` / ``"`` rather than leaving a stray
    // trailing delimiter behind.
    for (src, expected) in [
        ("string {lenght} $x\n", "string {length} $x\n"),
        ("string \"lenght\" $x\n", "string \"length\" $x\n"),
        ("string lenght $x\n", "string length $x\n"),
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1, "src={src:?} got {:?}", r.diagnostics);
        assert!(
            w001[0].message.contains("did you mean 'length'"),
            "src={src:?} got {:?}",
            w001[0].message
        );

        let fix = w001[0]
            .fixes
            .iter()
            .find(|f| f.new_text == "length")
            .expect("expected replacement fix to 'length'");

        let mut fixed = src.to_string();
        let start = fix.span.start() as usize;
        let end = fix.span.end() as usize;
        fixed.replace_range(start..end, &fix.new_text);

        assert_eq!(fixed, expected, "src={src:?} fixes={:?}", w001[0].fixes);
        assert!(!fixed.contains("lenght"), "src={src:?} fixed={fixed:?}");
    }
}

#[test]
fn analyse_w001_diagnostic_span_matches_fix_span() {
    // The squiggle and the "did you mean" quick-fix must target the
    // identical range — otherwise accepting the fix visibly leaves part
    // of the squiggled text unchanged.
    let mut a = Analyser::new();
    let src = "string lenght $x\n";
    let r = a.analyse(src, "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    let fix = w001[0]
        .fixes
        .first()
        .expect("expected a 'did you mean' fix");
    assert_eq!(w001[0].span, fix.span, "got {:?}", w001[0]);
}

#[test]
fn analyse_w001_no_diagnostic_at_all_when_shadowed_by_proc() {
    // A same-file `proc string {...}` must suppress W001 (and its
    // subcommand-level W002 sibling) completely — not just avoid the
    // specific message text. The FP-STY-17 reproducers live in
    // `analyser/diagnostics/fp/sty.rs::fp_sty_17_*`.
    let mut a = Analyser::new();
    let src = "proc string {op args} { return $op }\nstring reverse hello\n";
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W001 | DiagCode::W002)),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w001_shadow_suppression_is_scoped_to_the_shadowed_name() {
    // Shadowing `string` must not suppress W001 for a genuinely unknown
    // subcommand on a different, unshadowed ensemble in the same file.
    let mut a = Analyser::new();
    let src = "proc string {op args} { return $op }\ninfo bogus\n";
    let r = a.analyse(src, "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(w001[0].message.contains("'info'"), "got {:?}", w001[0]);
}

#[test]
fn analyse_no_w001_for_expanded_literal_subcommand() {
    // `{*}{create a b}` splices the elements `create`, `a`, `b` into the
    // argument list (confirmed against tclsh 8.6.14) — the raw source
    // text "create a b" must never be compared against the subcommand
    // set. The FP-STY-18 reproducers live in
    // `analyser/diagnostics/fp/sty.rs::fp_sty_18_*`.
    let mut a = Analyser::new();
    let r = a.analyse("dict {*}{create a b}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w001_still_fires_for_unexpanded_unknown_subcommand() {
    // TP control for the `{*}`-expansion skip: the identical subcommand
    // set, without expansion, still fires on a genuine typo.
    let mut a = Analyser::new();
    let r = a.analyse("dict bogus a b\n", "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}
