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

//! The single-command shape lints: unbraced words, non-ASCII and bidirectional text, literal contents, variable-name arguments and expression rewrites.

use super::*;

fn w114_codes(src: &str) -> usize {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W114)
        .count()
}

#[test]
fn w311_flags_binary_encoding_with_translation() {
    assert!(has_code(
        "fconfigure $ch -encoding binary -translation lf\n",
        "tcl8.6",
        "W311",
    ));
    assert!(has_code(
        "chan configure $ch -encoding binary -translation crlf\n",
        "tcl8.6",
        "W311",
    ));
    // `-translation binary` is consistent — no warning.
    assert!(!has_code(
        "fconfigure $ch -encoding binary -translation binary\n",
        "tcl8.6",
        "W311",
    ));
}

#[test]
fn w200_binary_modifier_is_dialect_gated() {
    // `cu` / `su` modifiers need Tcl 8.5+; flagged under 8.4 only.
    assert!(has_code("binary format cu1 $x\n", "tcl8.4", "W200"));
    assert!(has_code("binary scan $d su v\n", "tcl8.4", "W200"));
    assert!(!has_code("binary format cu1 $x\n", "tcl8.6", "W200"));
    // No modifier — never flagged.
    assert!(!has_code("binary format c1 $x\n", "tcl8.4", "W200"));
    // `s` is the short-integer specifier, not a signedness modifier: TIP 275
    // added only `u`. `ss` is two short fields on every release (verified on
    // tclsh 8.4.20, 8.5.19, 8.6.18 and 9.0.4), so it must stay clean even
    // under an 8.4 target, while a genuine `u` still fires.
    assert!(!has_code("binary format ss 1 2\n", "tcl8.4", "W200"));
    assert!(!has_code("binary scan $x ss a b\n", "tcl8.4", "W200"));
    assert!(has_code("binary format su 1\n", "tcl8.4", "W200"));
}

#[test]
fn w202_binary_field_letters_are_dialect_gated() {
    // `t n m r R q Q` are `bad field specifier` on tclsh 8.4.20 and accepted
    // from 8.5.19 on, for both `format` and `scan`.
    for letter in ["t", "n", "m", "r", "R", "q", "Q"] {
        assert!(
            has_code(&format!("binary format {letter} 1\n"), "tcl8.4", "W202"),
            "{letter} should be gated on 8.4",
        );
        assert!(
            !has_code(&format!("binary format {letter} 1\n"), "tcl8.5", "W202"),
            "{letter} should be clean on 8.5",
        );
    }
    assert!(has_code("binary scan $d q v\n", "tcl8.4", "W202"));
    // Letters that exist on every release are never gated.
    for letter in [
        "a", "A", "b", "B", "h", "H", "c", "s", "S", "i", "I", "w", "W", "f", "d",
    ] {
        assert!(
            !has_code(&format!("binary format {letter} 1\n"), "tcl8.4", "W202"),
            "{letter} exists on 8.4 and must stay clean",
        );
    }
    // One diagnostic per format string even with several gated letters, and
    // the two codes stay independent: a gated letter is not a W200, a gated
    // suffix is not a W202.
    assert_eq!(
        count_code_in("binary format qrm 1 2 3\n", "W202", "tcl8.4"),
        1
    );
    assert!(!has_code("binary format q 1\n", "tcl8.4", "W200"));
    assert!(!has_code("binary format cu 1\n", "tcl8.4", "W202"));
    // A gated letter carrying a gated suffix earns both, once each.
    assert_eq!(count_code_in("binary format qu 1\n", "W200", "tcl8.4"), 1);
    assert_eq!(count_code_in("binary format qu 1\n", "W202", "tcl8.4"), 1);
}

#[test]
fn w121_flags_noncontiguous_subnet_mask() {
    assert!(has_code("set m 255.255.255.1\n", "tcl8.6", "W121"));
    assert!(has_code("set m 255.0.255.0\n", "tcl8.6", "W121"));
    // Valid contiguous masks are fine.
    assert!(!has_code("set m 255.255.255.0\n", "tcl8.6", "W121"));
    assert!(!has_code("set m 255.255.254.0\n", "tcl8.6", "W121"));
}

#[test]
fn subnet_mask_helpers() {
    assert!(is_valid_subnet_mask(255, 255, 255, 0));
    assert!(is_valid_subnet_mask(0, 0, 0, 0));
    assert!(!is_valid_subnet_mask(255, 255, 255, 1));
    assert!(looks_like_subnet_mask(255, 255, 255, 1));
    assert!(!looks_like_subnet_mask(10, 0, 0, 1)); // ordinary IP
    // 24 leading 1-bits → /24.
    assert_eq!(
        nearest_valid_mask(255, 255, 255, 1).as_deref(),
        Some("255.255.255.0")
    );
}

#[test]
fn dotted_quad_scanner_matches_regex_behaviour() {
    let q = |t, n| {
        super::super::find_dotted_quads(t, n)
            .into_iter()
            .map(|q| (q.start, q.octets))
            .collect::<Vec<_>>()
    };
    // A clean quad is found with its octet substrings and start.
    assert_eq!(q("ip 192.168.1.1!", 3), vec![(3, ["192", "168", "1", "1"])]);
    // A 4-digit octet defeats the 3-digit cap (no leading boundary
    // realignment), exactly like `\b\d{1,3}`.
    assert_eq!(q("1234.1.1.1", 3), [] as [(usize, [&str; 4]); 0]);
    // The 4-digit cap accepts `999` and a 4-digit octet.
    assert_eq!(q("192.168.1.999", 4), vec![(0, ["192", "168", "1", "999"])]);
    // Two quads, non-overlapping; an embedding word char blocks the
    // boundary (`a10.0.0.1` has no leading `\b`).
    assert_eq!(q("a10.0.0.1", 3), [] as [(usize, [&str; 4]); 0]);
}

fn w108(src: &str, dialect: &str) -> Vec<(u32, usize)> {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W108)
        .map(|d| {
            let ch = d
                .message
                .chars()
                .find(|c| !c.is_ascii())
                .map_or(0, |c| c as u32);
            (ch, d.fixes.len())
        })
        .collect()
}

#[test]
fn w108_flags_confusables_and_artifacts() {
    // Smart quotes (auto-fix artifacts) → two W108 with fixes.
    assert_eq!(
        w108("set x \u{201c}hi\u{201d}\n", "tcl8.6"),
        vec![(0x201c, 1), (0x201d, 1)],
    );
    // NBSP and em-dash → W108 with an ASCII fix.
    assert_eq!(w108("set x \u{a0}y\n", "tcl8.6"), vec![(0xa0, 1)]);
    assert_eq!(w108("set x \u{2014}\n", "tcl8.6"), vec![(0x2014, 1)]);
}

#[test]
fn w108_confusables_mode_ignores_benign_unicode() {
    // `é` is not a confusable / artifact → silent in confusables mode.
    assert_eq!(w108("puts caf\u{e9}\n", "tcl8.6"), [] as [(u32, usize); 0]);
    // Plain ASCII → silent.
    assert_eq!(w108("set x hello\n", "tcl8.6"), [] as [(u32, usize); 0]);
    // The command word itself is not scanned.
    assert_eq!(w108("\u{440}uts x\n", "tcl8.6"), [] as [(u32, usize); 0]);
}

#[test]
fn w108_strict_mode_flags_all_non_ascii_for_irules() {
    // F5 iRules default to strict — every non-ASCII char fires,
    // including `é` (which has no ASCII equivalent → no fix).
    assert_eq!(w108("puts caf\u{e9}\n", "f5-irules"), vec![(0xe9, 0)]);
}

fn w108_mode(src: &str, dialect: &str, mode: crate::analyser::NonAsciiMode) -> Vec<u32> {
    let mut a = crate::analyser::Analyser::new().with_non_ascii_mode(mode);
    let mut out: Vec<u32> = a
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W108)
        .map(|d| {
            d.message
                .chars()
                .find(|c| !c.is_ascii())
                .map_or(0, |c| c as u32)
        })
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn is_benign_unicode_matches_reference() {
    for cp in [
        0x00E9, 0x00B0, 0x00B5, 0x2212, 0x4E2D, 0xFFFD, 0x2014, 0x2026,
    ] {
        assert!(
            super::super::is_benign_unicode(char::from_u32(cp).unwrap()),
            "U+{cp:04X}"
        );
    }
    for cp in [0x200B, 0x00A0, 0x0007, 0x202E] {
        assert!(
            !super::super::is_benign_unicode(char::from_u32(cp).unwrap()),
            "U+{cp:04X}"
        );
    }
}

#[test]
fn w108_comment_prose_is_not_flagged() {
    // FP fix: an em-dash (or any prose non-ASCII) inside a *comment* is
    // fine — comments are prose. Both a depth-1 body comment and a
    // depth-2 nested-body comment stay silent in confusables mode.
    assert_eq!(
        w108(
            "proc f {} {\n    # note \u{2014} dash\n    set y 1\n    puts $y\n}\nf\n",
            "tcl8.6"
        )
        .len(),
        0,
        "depth-1 body comment must not flag"
    );
    assert_eq!(
        w108(
            "proc f {c} {\n    if {$c} {\n        # nested \u{2014} dash\n        puts a\n    }\n}\n",
            "tcl8.6"
        ).len(), 0,
        "depth-2 nested body comment must not flag"
    );
}

#[test]
fn w108_leaves_comment_bidi_controls_to_w305() {
    // A bidi override in a comment is the trojan-source attack shape, and it
    // *is* still flagged — by W305, at error severity, from the whole-file
    // scan in `tcl_lsp_core::source_decode`.  W108 must not
    // report it as well: one character, one code, and a generic "non-ASCII
    // character" warning materially understates what a bidi override does.
    // The positive assertions live in `source_decode`'s own suite and in
    // `issue1326_encoding.rs`.
    let hits = w108(
        "proc f {} {\n    # ok \u{202e}live\n    set y 1\n    puts $y\n}\nf\n",
        "tcl8.6",
    );
    assert_eq!(hits.len(), 0, "W305 owns the bidi set now: {hits:?}");
    assert!(crate::analyser::confusables_table::is_bidi_control(
        '\u{202e}'
    ));
}

#[test]
fn w108_code_artifact_beside_comment_still_flags() {
    // TP control: the comment carve-out must not swallow code findings —
    // a smart quote in an actual argument still fires.
    let hits = w108(
        "proc f {} {\n    # note \u{2014} dash\n    set y \u{201c}hi\u{201d}\n    puts $y\n}\nf\n",
        "tcl8.6",
    );
    let codes: Vec<u32> = hits.iter().map(|(c, _)| *c).collect();
    assert!(
        codes.contains(&0x201c) && codes.contains(&0x201d) && !codes.contains(&0x2014),
        "code artifacts flag, comment prose does not: {codes:?}"
    );
}

#[test]
fn w108_strict_mode_still_flags_comment_prose() {
    // Strict (ASCII-only F5 platforms): the platform constraint covers
    // comment bytes too — the em-dash in a `when` body comment stays a
    // finding under the f5-irules default.
    let hits = w108(
        "when HTTP_REQUEST {\n    # bad \u{2014} dash\n    HTTP::respond 200\n}\n",
        "f5-irules",
    );
    assert!(
        hits.iter().any(|(c, _)| *c == 0x2014),
        "strict mode keeps flagging comment non-ASCII: {hits:?}"
    );
}

#[test]
fn w108_leaves_code_bidi_controls_to_w305() {
    // Same hand-off as in comments: the character is still
    // reported, just under the code that describes what it actually does.
    let hits = w108("set x a\u{202e}b\n", "tcl8.6");
    assert_eq!(hits.len(), 0, "W305 owns the bidi set now: {hits:?}");
    // ...and the neighbouring invisible characters that are *not* bidi
    // controls stay with W108, which is the boundary that matters.
    assert!(!crate::analyser::confusables_table::is_bidi_control(
        '\u{200b}'
    ));
    assert_eq!(
        w108_mode(
            "set x a\u{200b}b\n",
            "tcl8.6",
            crate::analyser::NonAsciiMode::Common
        ),
        vec![0x200b]
    );
}

#[test]
fn w305_analyser_reports_every_bidi_control_without_w108_duplication() {
    for (ch, name) in crate::analyser::confusables_table::BIDI_CONTROLS {
        let src = format!("set marker \"a{ch}b\"\n");
        let mut analyser = crate::analyser::Analyser::new()
            .with_non_ascii_mode(crate::analyser::NonAsciiMode::Off);
        let result = analyser.analyse(&src, "tcl9.0");
        let w305: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == tcl_core_types::DiagCode::W305)
            .collect();
        assert_eq!(w305.len(), 1, "U+{:04X}", *ch as u32);
        assert_eq!(w305[0].severity, tcl_core_types::Severity::Error);
        assert!(w305[0].message.contains(name));
        assert!(w305[0].message.contains("Trojan Source"));
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| d.code != tcl_core_types::DiagCode::W108),
            "W108 must not duplicate W305 for U+{:04X}",
            *ch as u32
        );
    }
}

#[test]
fn w305_analyser_finds_comments_and_command_positions() {
    for src in [
        "# \u{202e} a comment that renders backwards\n",
        "\u{2066}puts hello\n",
        "proc \u{202d}name {} {}\n",
    ] {
        let result = crate::analyser::Analyser::new().analyse(src, "tcl9.0");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.code == tcl_core_types::DiagCode::W305)
                .count(),
            1,
            "{src:?}"
        );
    }
}

#[test]
fn w305_analyser_is_silent_on_ordinary_rtl_and_directional_marks() {
    for src in [
        "puts \"مرحبا بالعالم\"\n",
        "puts \"שלום עולם\"\n",
        "# تعليق عربي عادي\n",
        "puts \"\u{200e}1\u{200f}2\u{61c}3\"\n",
        "",
    ] {
        let result = crate::analyser::Analyser::new().analyse(src, "tcl9.0");
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| d.code != tcl_core_types::DiagCode::W305),
            "false positive on {src:?}"
        );
    }
}

/// W305 is emitted raw like every other finding: a disabled code is the
/// analyser's production skip, and an inline `# noqa` is a directive fact
/// the policy step reads, never a filter inside the producer — so the
/// finding stays in the analysis, and its line is in `suppressed_lines`.
#[test]
fn w305_analyser_skips_a_disabled_code_and_leaves_a_noqa_to_the_policy() {
    let disabled = ["W305".to_owned()].into_iter().collect();
    let result = crate::analyser::Analyser::with_disabled_diagnostics(disabled)
        .analyse("puts \"\u{202e}x\"\n", "tcl9.0");
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.code != tcl_core_types::DiagCode::W305)
    );

    let file_disabled = "# tcl-lsp: disable=W305\nputs \"\u{202e}x\"\n";
    let result = crate::analyser::Analyser::new().analyse(file_disabled, "tcl9.0");
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.code != tcl_core_types::DiagCode::W305),
        "file suppression failed: {result:?}"
    );

    for (src, recorded) in [
        ("# noqa: W305\nputs \"\u{202e}x\"\n", true),
        ("# noqa: W108\nputs \"\u{202e}x\"\n", false),
    ] {
        let result = crate::analyser::Analyser::new().analyse(src, "tcl9.0");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.code == tcl_core_types::DiagCode::W305)
                .count(),
            1,
            "the producer keeps the finding under {src:?}"
        );
        assert_eq!(
            crate::analyser::line_suppressed("W305", 1, &result.suppressed_lines),
            recorded,
            "the directive fact for {src:?}"
        );
    }
}

#[test]
fn w108_off_mode_disables_entirely() {
    use crate::analyser::NonAsciiMode::Off;
    // Even smart quotes / NBSP are silent when W108 is off.
    assert_eq!(
        w108_mode("set x \u{201c}hi\u{201d}\u{a0}\n", "tcl8.6", Off),
        [] as [u32; 0]
    );
    // ...and off wins even for iRules (which would otherwise be strict).
    assert_eq!(
        w108_mode("puts caf\u{e9}\n", "f5-irules", Off),
        [] as [u32; 0]
    );
}

#[test]
fn w108_strict_mode_explicit_flags_all_in_plain_tcl() {
    use crate::analyser::NonAsciiMode::Strict;
    // Explicit strict flags `é` even in a non-F5 dialect.
    assert_eq!(w108_mode("puts caf\u{e9}\n", "tcl8.6", Strict), vec![0xe9]);
}

#[test]
fn w108_common_mode_allows_intentional_unicode() {
    use crate::analyser::NonAsciiMode::Common;
    // Benign letters / symbols / punctuation in any script are allowed.
    assert_eq!(
        w108_mode("set x caf\u{e9}\n", "tcl8.6", Common),
        [] as [u32; 0]
    ); // é (Ll)
    assert_eq!(
        w108_mode("set x 90\u{b0}\n", "tcl8.6", Common),
        [] as [u32; 0]
    ); // ° (So)
    assert_eq!(
        w108_mode("set x \u{4e2d}\n", "tcl8.6", Common),
        [] as [u32; 0]
    ); // 中 (Lo)
}

#[test]
fn w108_common_mode_flags_confusables_and_non_benign() {
    use crate::analyser::NonAsciiMode::Common;
    // Confusables / auto-fix artifacts still fire in common mode.
    assert_eq!(
        w108_mode("set x \u{201c}\n", "tcl8.6", Common),
        vec![0x201c]
    );
    // Non-benign characters (control / zero-width / format) fire even
    // without being confusables — these are the encoding-issue chars
    // `common` mode is meant to catch.
    assert_eq!(
        w108_mode("set x a\u{200b}b\n", "tcl8.6", Common),
        vec![0x200b]
    ); // ZWSP (Cf)
    // U+202E RLO is *not* here: bidi controls moved to W305,
    // so `common` mode reports the zero-width and control characters it was
    // always meant to catch and leaves the direction-altering set alone.
    assert_eq!(
        w108_mode("set x a\u{202e}b\n", "tcl8.6", Common),
        [] as [u32; 0]
    );
}

#[test]
fn w104_flags_space_padded_append() {
    assert_eq!(code_sevs("append x \" foo\"\n", "W104"), vec!["Hint"]);
    assert_eq!(code_sevs("append result \"item \"\n", "W104"), vec!["Hint"]);
    assert_eq!(
        code_sevs("append x foo\n", "W104"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        code_sevs("lappend x foo\n", "W104"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn w106_flags_unbraced_switch_body() {
    // Alternating unbraced body (no sub → WARNING; sub → ERROR).
    assert_eq!(code_sevs("switch $v a body\n", "W106"), vec!["Warning"]);
    assert_eq!(code_sevs("switch $v $pat $body\n", "W106"), vec!["Error"]);
    // Braced forms are fine.
    assert_eq!(
        code_sevs("switch $v {a {x} b {y}}\n", "W106"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        code_sevs("switch -regexp $v {a {x}}\n", "W106"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        code_sevs("switch $v { a body }\n", "W106"),
        [] as [std::string::String; 0]
    );
}

fn w100_sev(src: &str) -> Vec<String> {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W100)
        .map(|d| format!("{:?}", d.severity))
        .collect()
}

#[test]
fn w100_flags_unbraced_expr_with_substitution() {
    // ERROR when a `$`/`[` sub.
    assert_eq!(w100_sev("if $x {puts hi}\n"), vec!["Error"]);
    assert_eq!(w100_sev("while $cond {}\n"), vec!["Error"]);
    assert_eq!(w100_sev("expr $a + $b\n"), vec!["Error"]);
    assert_eq!(w100_sev("expr \"$a == $b\"\n"), vec!["Error"]);
    assert_eq!(w100_sev("for {set i 0} $i<10 {incr i} {}\n"), vec!["Error"]);
}

#[test]
fn w100_skips_braced_and_safe_literals() {
    assert_eq!(
        w100_sev("if {$x} {puts hi}\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(w100_sev("expr {$a + $b}\n"), [] as [std::string::String; 0]);
    assert_eq!(w100_sev("expr 1+2\n"), [] as [std::string::String; 0]);
    assert_eq!(w100_sev("if 1 {puts hi}\n"), [] as [std::string::String; 0]);
    assert_eq!(
        w100_sev("if {1} {puts hi}\n"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn is_safe_literal_expr_classifies() {
    assert!(is_safe_literal("42"));
    assert!(is_safe_literal("true"));
    assert!(!is_safe_literal("$x"));
    assert!(is_safe_literal_expr(
        "1 + 2",
        tcl_dialect::DialectProfile::find("tcl8.6").expect("tcl8.6 profile")
    ));
    assert!(!is_safe_literal_expr(
        "$a + $b",
        tcl_dialect::DialectProfile::find("tcl8.6").expect("tcl8.6 profile")
    ));
}

fn w212_count(src: &str) -> usize {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W212)
        .count()
}

#[test]
fn w212_flags_name_position_substitution() {
    assert_eq!(w212_count("set $x 1\n"), 1);
    assert_eq!(w212_count("incr $counter\n"), 1);
    assert_eq!(w212_count("info exists $v\n"), 1);
    assert_eq!(w212_count("upvar 1 a $b\n"), 1);
}

#[test]
fn w212_allows_indirect_read_through_a_computed_name_2499() {
    assert_eq!(
        w212_count(
            "proc f {sides} {\n    foreach side $sides {\n        set value [set $side]\n    }\n}\n"
        ),
        0
    );
    assert_eq!(w212_count("proc f {name} {set $name 1}\n"), 1);
}

#[test]
fn w212_ignores_plain_names() {
    assert_eq!(w212_count("set x 1\n"), 0);
    assert_eq!(w212_count("info exists v\n"), 0);
    // A `$`-value in a non-name position is fine.
    assert_eq!(w212_count("set x $y\n"), 0);
}

#[test]
fn w212_covers_registry_name_positions() {
    // Name positions supplied by the registry's VarWrite/VarRead roles, which
    // a hardcoded list misses.
    assert_eq!(w212_count("proc p {} { vwait $x }\n"), 1);
    assert_eq!(w212_count("proc p {} { catch {error e} $res }\n"), 1);
    assert_eq!(w212_count("proc p {l} { lassign $l $x }\n"), 1);
    assert_eq!(w212_count("proc p {s} { scan $s %d $x }\n"), 1);
    assert_eq!(w212_count("proc p {d} { dict with $d {} }\n"), 1);
    // TN controls: literal names in the same positions stay silent.
    assert_eq!(w212_count("proc p {} { vwait x }\n"), 0);
    assert_eq!(w212_count("proc p {} { catch {error e} res }\n"), 0);
    assert_eq!(w212_count("proc p {l} { lassign $l a b }\n"), 0);
    // A dynamic `catch $script` has no result-var position — nothing to flag.
    assert_eq!(w212_count("proc p {s} { catch $s }\n"), 0);
}

#[test]
fn w212_upvar_remote_name_may_be_computed() {
    // The *remote* (other-var) slot of `upvar` legitimately takes a computed
    // name, so `$remote` there must NOT fire W212 — only the local slot does.
    assert_eq!(w212_count("proc p {} { upvar 1 $remote local }\n"), 0);
    assert_eq!(w212_count("proc p {} { upvar 1 remote $local }\n"), 1);
}

#[test]
fn w216_upvar_local_name_is_indirect_array_idiom() {
    // FP guard: `${arr}(x)` in `upvar`'s local-name slot is the legitimate
    // indirect-array idiom (the same carve-out `set` / `vwait` get).  Separate
    // name-position lists drift — one omitting `upvar` is enough to break it.
    assert_eq!(w216_count("proc p {arr} { upvar 1 remote ${arr}(x) }\n"), 0);
    // TP control: `${arr}(x)` in a *value* position is a genuine broken read.
    assert_eq!(w216_count("proc p {arr} { puts ${arr}(x) }\n"), 1);
}

#[test]
fn variable_name_positions_are_registry_driven() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let pos = |cmd: &str, args: &[&str]| {
        let source = format!("{cmd} {}", args.join(" "));
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(&source, "tcl8.6");
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source,
            0,
            analysis.body_lexer_config.unwrap(),
        )
        .remove(0);
        let original = analyser
            .original_diagnostic_source_for_segment(&segment)
            .unwrap();
        Analyser::variable_name_positions(&original)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };
    // Name-position commands, resolved from the registry's VarWrite/VarRead
    // roles rather than a hardcoded list.
    assert_eq!(pos("set", &["a", "b"]), vec![0]);
    assert_eq!(pos("unset", &["-nocomplain", "a", "b"]), vec![1, 2]);
    assert_eq!(pos("info", &["exists", "v"]), vec![1]);
    assert_eq!(pos("info", &["level"]), Vec::<usize>::new());
    // `upvar` — only the *local* names (every other arg after the level word).
    assert_eq!(pos("upvar", &["1", "a", "b"]), vec![2]);
    assert_eq!(pos("upvar", &["a", "b"]), vec![1]); // no level word
    // Positions covered by the registry roles, which a hand-written list omits.
    assert_eq!(pos("vwait", &["v"]), vec![0]);
    assert_eq!(pos("catch", &["{script}", "res"]), vec![1]);
    assert_eq!(pos("catch", &["{script}", "res", "opts"]), vec![1, 2]);
    // A dynamic `catch $script` has no result-var position — nothing to flag.
    assert_eq!(pos("catch", &["$script"]), Vec::<usize>::new());
}

#[test]
fn upvar_local_positions_parity() {
    // Only the local names are strict name positions; the paired remote names
    // (indices 1, 3, …) are excluded so a computed `$remote` is not flagged.
    let registry = tcl_registry::CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    assert_eq!(
        registry.arg_indices_for_role("upvar", &["1", "a", "b"], tcl_registry::ArgRole::VarWrite,),
        vec![2],
    );
    assert!(
        registry
            .arg_indices_for_role(
                "upvar",
                &["$lvl", "a", "b"],
                tcl_registry::ArgRole::VarWrite
            )
            .is_empty(),
        "literal argv does not stand for an evaluated selector; this level is invalid"
    );
    assert_eq!(
        registry.arg_indices_for_role("upvar", &["1", "b"], tcl_registry::ArgRole::VarWrite,),
        vec![1],
    );
    let legacy = tcl_registry::CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
    assert!(
        legacy
            .arg_indices_for_role("upvar", &["1", "b"], tcl_registry::ArgRole::VarWrite)
            .is_empty(),
        "8.4 consumes the digit-leading selector and rejects the remaining unpaired name",
    );
    assert!(
        tcl_registry::CommandRegistry::build_default()
            .arg_indices_for_role("upvar", &["1", "b"], tcl_registry::ArgRole::VarWrite)
            .is_empty(),
        "an unselected native grammar cannot donate the modern parity layout",
    );
}

#[test]
fn w114_flags_nested_expr_in_expr_context() {
    assert_eq!(w114_codes("expr {[expr {$x + 1}]}\n"), 1);
    assert_eq!(w114_codes("if {[expr {$x}]} {puts hi}\n"), 1);
}

#[test]
fn w114_ignores_non_expr_context_and_plain_expr() {
    // `set y [expr {…}]` is a command substitution value, not a
    // nested expr context — no W114.
    assert_eq!(w114_codes("set y [expr {1+2}]\n"), 0);
    // A plain braced expr is fine.
    assert_eq!(w114_codes("expr {$x + 1}\n"), 0);
}

#[test]
fn w114_ignores_expr_nested_in_command_substitution() {
    // The `[expr {1+1}]` is an argument to `myCmd` (a fresh command
    // context), not a top-level command substitution in the `if` condition, so
    // it is NOT redundant and must not be flagged.
    assert_eq!(
        w114_codes("proc myCmd {a} {return $a}\nif {[myCmd [expr {1 + 1}]]} {puts hi}\n"),
        0,
    );
    // A top-level `[expr]` in the same condition position IS redundant.
    assert_eq!(w114_codes("if {[expr {1 + 1}]} {puts hi}\n"), 1);
    // And one mixed at top level alongside other operands still fires.
    assert_eq!(w114_codes("if {$x + [expr {1 + 1}]} {puts hi}\n"), 1);
}

#[test]
fn first_nested_expr_finds_bracketed_expr() {
    assert_eq!(first_nested_expr("{[expr {$x}]}"), Some((1, 11)));
    assert_eq!(first_nested_expr("{$x + 1}"), None);
    assert_eq!(first_nested_expr("[express]"), None); // not `expr` + ws
    // Nested inside another command substitution → not a top-level expr.
    assert_eq!(first_nested_expr("{[myCmd [expr {1+1}]]}"), None);
    // Top-level expr alongside another bracket sub is still found.
    assert_eq!(first_nested_expr("{[a] + [expr {$x}]}"), Some((7, 17)));
}

#[test]
fn w100_suggests_scan_for_a_hexadecimal_conversion_2507() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {hex} {\n    set r [expr 0x[string range $hex 0 1]]\n    set g [expr 0x$hex]\n    set b [expr $hex + 1]\n}\n",
        "tcl8.6",
    );
    let w100 = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W100)
        .collect::<Vec<_>>();
    assert_eq!(w100.len(), 3, "{w100:?}");
    assert_eq!(
        w100.iter()
            .filter(|d| d.message.contains("scan $hex %x var"))
            .count(),
        2,
        "{w100:?}"
    );
}
