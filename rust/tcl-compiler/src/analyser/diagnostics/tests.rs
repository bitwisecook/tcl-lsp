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

use super::*;
use crate::analyser::types::Diagnostic;
use tcl_core_types::DiagCode;
use tcl_lexer::Span;

fn w114_codes(src: &str) -> usize {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W114)
        .count()
}

fn code_sevs(src: &str, code: &str) -> Vec<String> {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .map(|d| format!("{:?}", d.severity))
        .collect()
}

fn has_code(src: &str, dialect: &str, code: &str) -> bool {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == code)
}

#[test]
fn e006_rejects_invalid_literal_formal_parameter_lists_from_registry_roles() {
    // Tcl 9.0.4 rejects a formal specifier with three fields, a namespace
    // qualified name, and an array element.  Duplicate parameter names remain
    // legal, and a value computed at runtime is deliberately outside the
    // analyser's proof boundary.
    for src in [
        "proc invalid {{a b c}} {}\n",
        "proc invalid {a::b} {}\n",
        "proc invalid {a(x)} {}\n",
    ] {
        assert!(has_code(src, "tcl9.0", "E006"), "expected E006 for {src:?}");
    }
    for src in [
        "proc duplicate {a a} {}\n",
        "set params {a::b}\nproc dynamic $params {}\n",
    ] {
        assert!(!has_code(src, "tcl9.0", "E006"), "must abstain for {src:?}");
    }
}

#[test]
fn e006_offers_only_the_structurally_unambiguous_parameter_repair() {
    let mut analyser = crate::analyser::Analyser::new();
    let result = analyser.analyse("proc invalid {{a b c}} {}\n", "tcl9.0");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::E006)
        .expect("E006 is emitted");

    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(diagnostic.fixes[0].new_text, "{a b c}");
    assert_eq!(
        diagnostic.fixes[0].safety,
        crate::analyser::types::FixSafety::RequiresReview
    );

    for source in ["proc invalid {a::b} {}\n", "proc invalid {a(x)} {}\n"] {
        let result = analyser.analyse(source, "tcl9.0");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::E006)
            .expect("E006 is emitted");
        assert!(diagnostic.fixes.is_empty(), "must abstain for {source:?}");
    }
}

#[test]
fn e006_discovers_definition_body_and_lambda_parameter_shapes() {
    // These are not ordinary top-level command arguments.  The TclOO method
    // shape comes from DefinitionBodyGrammar::member / MemberSpec::arg_roles;
    // `apply`'s nested first field comes from ArgRole::LambdaLiteral.
    assert!(has_code(
        "oo::class create C { method invalid {{a b c}} {} }\n",
        "tcl9.0",
        "E006",
    ));
    assert!(has_code(
        "oo::class create C {}\noo::define C method invalid {{a b c}} {}\n",
        "tcl9.0",
        "E006",
    ));
    assert!(has_code("apply {{{a b c}} {return 1}}\n", "tcl9.0", "E006",));
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
        super::find_dotted_quads(t, n)
            .into_iter()
            .map(|q| (q.start, q.octets))
            .collect::<Vec<_>>()
    };
    // A clean quad is found with its octet substrings and start.
    assert_eq!(q("ip 192.168.1.1!", 3), vec![(3, ["192", "168", "1", "1"])]);
    // A 4-digit octet defeats the 3-digit cap (no leading boundary
    // realignment), exactly like `\b\d{1,3}`.
    assert!(q("1234.1.1.1", 3).is_empty());
    // The 4-digit cap accepts `999` and a 4-digit octet.
    assert_eq!(q("192.168.1.999", 4), vec![(0, ["192", "168", "1", "999"])]);
    // Two quads, non-overlapping; an embedding word char blocks the
    // boundary (`a10.0.0.1` has no leading `\b`).
    assert!(q("a10.0.0.1", 3).is_empty());
}

#[test]
fn ipv6_candidate_scanner_extracts_runs() {
    let c = super::find_ipv6_candidates("addr fe80::1 end");
    assert_eq!(c, vec!["fe80::1"]);
    // A bare hextet pair (only one colon → <2 groups) is not a
    // candidate; a full address is.
    assert!(super::find_ipv6_candidates("ab:cd").is_empty());
    assert_eq!(
        super::find_ipv6_candidates("2001:db8::8a2e:370:7334"),
        vec!["2001:db8::8a2e:370:7334"]
    );
}

#[test]
fn redos_shape_detector() {
    assert!(super::has_redos_shape("(a+)+"));
    assert!(super::has_redos_shape("(a*)*"));
    assert!(super::has_redos_shape("(a|a)+"));
    assert!(super::has_redos_shape("(foo|bar){2}"));
    // No nested quantifier / overlapping alternation → safe.
    assert!(!super::has_redos_shape("^[a-z]+$"));
    assert!(!super::has_redos_shape("(abc)+"));
    assert!(!super::has_redos_shape("a|b|c"));
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
    assert!(w108("puts caf\u{e9}\n", "tcl8.6").is_empty());
    // Plain ASCII → silent.
    assert!(w108("set x hello\n", "tcl8.6").is_empty());
    // The command word itself is not scanned.
    assert!(w108("\u{440}uts x\n", "tcl8.6").is_empty());
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
            super::is_benign_unicode(char::from_u32(cp).unwrap()),
            "U+{cp:04X}"
        );
    }
    for cp in [0x200B, 0x00A0, 0x0007, 0x202E] {
        assert!(
            !super::is_benign_unicode(char::from_u32(cp).unwrap()),
            "U+{cp:04X}"
        );
    }
}

#[test]
fn w108_comment_prose_is_not_flagged() {
    // FP fix: an em-dash (or any prose non-ASCII) inside a *comment* is
    // fine — comments are prose. Both a depth-1 body comment and a
    // depth-2 nested-body comment stay silent in confusables mode.
    assert!(
        w108(
            "proc f {} {\n    # note \u{2014} dash\n    set y 1\n    puts $y\n}\nf\n",
            "tcl8.6"
        )
        .is_empty(),
        "depth-1 body comment must not flag"
    );
    assert!(
        w108(
            "proc f {c} {\n    if {$c} {\n        # nested \u{2014} dash\n        puts a\n    }\n}\n",
            "tcl8.6"
        )
        .is_empty(),
        "depth-2 nested body comment must not flag"
    );
}

#[test]
fn w108_leaves_comment_bidi_controls_to_w305() {
    // A bidi override in a comment is the trojan-source attack shape, and it
    // *is* still flagged — by W305, at error severity, from the whole-file
    // scan in `tcl_lsp_core::source_decode` (issue #1326).  W108 must not
    // report it as well: one character, one code, and a generic "non-ASCII
    // character" warning materially understates what a bidi override does.
    // The positive assertions live in `source_decode`'s own suite and in
    // `issue1326_encoding.rs`.
    let hits = w108(
        "proc f {} {\n    # ok \u{202e}live\n    set y 1\n    puts $y\n}\nf\n",
        "tcl8.6",
    );
    assert!(hits.is_empty(), "W305 owns the bidi set now: {hits:?}");
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
    // Same hand-off as in comments (issue #1326): the character is still
    // reported, just under the code that describes what it actually does.
    let hits = w108("set x a\u{202e}b\n", "tcl8.6");
    assert!(hits.is_empty(), "W305 owns the bidi set now: {hits:?}");
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

#[test]
fn w305_analyser_honours_code_and_line_suppression() {
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

    for (src, suppressed) in [
        ("# noqa: W305\nputs \"\u{202e}x\"\n", true),
        ("# noqa: W108\nputs \"\u{202e}x\"\n", false),
    ] {
        let result = crate::analyser::Analyser::new().analyse(src, "tcl9.0");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .all(|d| d.code != tcl_core_types::DiagCode::W305),
            suppressed,
            "line-local suppression mismatch for {src:?}"
        );
    }
}

fn codes_for_dialect(src: &str, dialect: &str) -> Vec<String> {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn w146_validates_modern_trace_operation_lists_by_registry_declared_type() {
    for (invalid, valid) in [
        (
            "trace add variable item {read rename write} callback\n",
            "trace add variable item {array read unset write} callback\n",
        ),
        (
            "trace remove command worker {delete enter rename} callback\n",
            "trace remove command worker {delete rename} callback\n",
        ),
        (
            "trace add execution worker {enter write leavestep} callback\n",
            "trace add execution worker {enter leave enterstep leavestep} callback\n",
        ),
    ] {
        assert!(
            codes_for_dialect(invalid, "tcl9.0").contains(&"W146".to_owned()),
            "invalid type-specific operation must fire: {invalid}"
        );
        assert!(
            !codes_for_dialect(valid, "tcl9.0").contains(&"W146".to_owned()),
            "the complete Tcl 9 operation set must stay silent: {valid}"
        );
    }
}

#[test]
fn w146_mixed_operation_list_fix_preserves_valid_members_and_requires_review() {
    let source = "trace add variable item {read bogus write} callback\n";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W146)
        .expect("mixed valid/invalid list fires W146");
    assert!(diagnostic.message.contains("'bogus'"));
    assert!(diagnostic.message.contains("array, read, unset, write"));
    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(diagnostic.fixes[0].new_text, "{read write}");
    assert_eq!(
        diagnostic.fixes[0].safety,
        crate::irules_checks::FixSafety::RequiresReview
    );
    assert_eq!(
        &source[diagnostic.fixes[0].span.as_range()],
        "{read bogus write}",
        "the edit replaces the whole written Tcl word, including delimiters"
    );
}

#[test]
fn w146_abstains_when_a_modern_operation_list_is_not_safely_fixable() {
    for source in [
        "trace add variable item $operations callback\n",
        "trace add variable item [operations] callback\n",
        "trace add variable item {*}$operations callback\n",
        "trace add variable item {read {unterminated} callback\n",
        "trace add not-a-type item {read bogus} callback\n",
        "trace add variable item\n",
    ] {
        assert!(
            !codes_for_dialect(source, "tcl9.0").contains(&"W146".to_owned()),
            "dynamic, malformed, incomplete, or invalid-discriminator input must abstain: {source:?}"
        );
    }

    for source in [
        "trace add command worker {} callback\n",
        "trace add command worker {enter write} callback\n",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W146)
            .expect("a complete empty/all-invalid list is still diagnosed");
        assert!(
            diagnostic.fixes.is_empty(),
            "removing every member would leave Tcl's invalid empty operation list"
        );
    }
}

#[test]
fn w146_honours_trace_prefix_and_legacy_version_rules() {
    assert!(
        !codes_for_dialect("trace add var item {read write} callback\n", "tcl9.0")
            .contains(&"W146".to_owned()),
        "the type discriminator accepts Tcl's unique-prefix rule"
    );
    assert!(
        codes_for_dialect("trace add variable item w callback\n", "tcl9.0")
            .contains(&"W146".to_owned()),
        "operation members use exact matching and may not be abbreviated"
    );
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        assert!(
            codes_for_dialect("trace variable item rwx callback\n", dialect)
                .contains(&"W146".to_owned()),
            "the deprecated rwua form is validated on {dialect}"
        );
    }
    assert!(
        !codes_for_dialect("trace variable item rwx callback\n", "tcl9.0")
            .contains(&"W146".to_owned()),
        "Tcl 9 removed the legacy form, so subcommand availability owns it"
    );
}

#[test]
fn w146_drops_registry_finding_when_a_user_command_shadows_trace() {
    let source =
        "proc trace args { return user-command }\ntrace add variable item {read bogus} callback\n";
    assert!(
        !codes_for_dialect(source, "tcl9.0").contains(&"W146".to_owned()),
        "post-walk command resolution must suppress registry argument facts for a shadowed builtin"
    );
}

/// FP guard (issue #923 audit, `ticklecharts` idx 51): a document that
/// installs the documented "`TclOO` Tricks" wiki helper
/// `proc ::oo::Helpers::callback` and calls it bare from a method body must
/// draw **no** diagnostic under 8.6, where `callback` is not a core command.
///
/// A `TclOO` member body's `namespace path` is unconditionally
/// `::oo::Helpers` (tclsh 8.6.14 / 9.0.4), so the helper really is the
/// command that bare word reaches — `esnap.tcl`'s `[callback ReadBrowser]`
/// runs. Before the shadow check consulted that implicit path the bare word
/// only ever qualified to `::callback`, so the helper did not count and the
/// call drew a W002 "disabled in the active dialect profile" for the 9.0
/// builtin the program never sees.
#[test]
fn oo_helpers_user_helper_shadows_the_dialect_gated_builtin() {
    let src = "proc ::oo::Helpers::callback {method args} {\n    \
               return [list my $method {*}$args]\n}\n\
               oo::class create Snap {\n    \
               method Start {} { return [callback ReadBrowser] }\n    \
               method ReadBrowser {} { return 1 }\n}\n";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        !codes.contains(&"W002".to_string()) && !codes.contains(&"W123".to_string()),
        "the file's own ::oo::Helpers::callback is what the method body \
         reaches: {codes:?}"
    );
}

/// TP control for the same gate: with no such helper anywhere, an 8.6
/// document really has no `callback`, and both the dialect-availability
/// report and the unknown-command report must still fire (tclsh 8.6.14:
/// `invalid command name "callback"`).
#[test]
fn oo_helpers_builtin_without_a_user_helper_still_reports() {
    let src = "oo::class create Snap {\n    \
               method Start {} { return [callback ReadBrowser] }\n    \
               method ReadBrowser {} { return 1 }\n}\n";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        codes.contains(&"W002".to_string()) && codes.contains(&"W123".to_string()),
        "8.6 core has no `callback` and nothing supplies it: {codes:?}"
    );
}

/// TN control: the implicit path is a `TclOO` member-body fact only, so a
/// `proc ::oo::Helpers::NAME` must shadow **only** inside such a body.
///
/// Uses `lseq` (Tcl 9.0-only, and not an `oo::Helpers` member of any
/// version) so the two positions differ in nothing but the namespace path:
/// inside a method body `::oo::Helpers::lseq` is genuinely on the search
/// path and shadows, at the top level nothing puts `::oo::Helpers` there
/// (tclsh 9.0.4: a bare word at the top level is looked up in `::` alone),
/// so the 8.6 dialect-availability report must stand.
#[test]
fn oo_helpers_user_helper_shadows_only_inside_a_member_body() {
    let helper = "proc ::oo::Helpers::lseq {args} { return $args }\n";
    let top_level = format!("{helper}lseq 1 5\n");
    assert!(
        codes_for_dialect(&top_level, "tcl8.6").contains(&"W002".to_string()),
        "a top-level bare word never reaches ::oo::Helpers: {:?}",
        codes_for_dialect(&top_level, "tcl8.6")
    );
    let in_method =
        format!("{helper}oo::class create C {{\n    method m {{}} {{ return [lseq 1 5] }}\n}}\n");
    assert!(
        !codes_for_dialect(&in_method, "tcl8.6").contains(&"W002".to_string()),
        "a method body resolves the helper: {:?}",
        codes_for_dialect(&in_method, "tcl8.6")
    );
}

#[test]
fn disabled_definer_nested_in_catch_reports_w002_once_without_cascade() {
    // The feature-detection idiom: a Tcl 9-only definer probed under 8.6.
    // W002 reports the availability once; the definer's member keywords
    // (`property`, `constructor`) and the defined class name must NOT
    // cascade into W123 — the definer grammar still resolves the body
    // structurally even though the dialect gate fails.
    let src = "if {![catch {oo::configurable create Greeter {
    property greeting
                   constructor {g} { my configure -greeting $g }
}}]} { puts ok }
Greeter new x
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert_eq!(
        codes.iter().filter(|c| *c == "W002").count(),
        1,
        "W002 exactly once: {codes:?}"
    );
    assert!(
        !codes.contains(&"W123".to_string()),
        "no unknown-command cascade for members or the class name: {codes:?}"
    );
}

#[test]
fn proc_nested_in_catch_registers_the_proc() {
    // `catch {proc p …}` still defines `p` — a later call is not unknown.
    let src = "if {[catch {proc p {} { return 1 }}]} { puts no }
p
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        !codes.contains(&"W123".to_string()),
        "the nested proc must register: {codes:?}"
    );
}

#[test]
fn plain_script_body_nested_in_catch_still_descends() {
    // FP guard for the definer carve-out: an ordinary control-flow body
    // nested in the substitution is still walked, so a genuinely unknown
    // command inside it keeps its W123.
    let src = "if {[catch {if {1} { zz9unknowncmd a b }}]} { puts no }
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        codes.contains(&"W123".to_string()),
        "unknown command inside a nested plain body still fires: {codes:?}"
    );
}

#[test]
fn enabled_definer_nested_in_catch_is_silent() {
    // Under a dialect where the definer exists there is no W002 and no
    // cascade either.
    let src = "if {![catch {oo::configurable create Greeter {
    property greeting
}}]}                { puts ok }
Greeter new x
";
    let codes = codes_for_dialect(src, "tcl9.0");
    assert!(
        !codes.contains(&"W002".to_string()) && !codes.contains(&"W123".to_string()),
        "enabled definer draws neither W002 nor a cascade: {codes:?}"
    );
}

/// FIX (issue-923 differential audit, finding idx 0) — a literal
/// `apply {{params} {body}}` inside a `[…]` command substitution reported
/// `Unknown command '<the parameter list>'`: the substitution collectors
/// re-segmented the whole `{params body}` list as if it were script source,
/// so the parameter-list word became a command head.
///
/// The fix is registry-role-driven, not `apply`-aware: `descend_command`
/// resolves `ArgRole::LambdaLiteral` arguments through
/// `lambda_literal::split_lambda_literal` and descends only the body
/// element, so the parameter list is never walked as code and the body is.
///
/// Source is the audit's own repro — the `validateHelper` lambda body
/// verbatim from `georgtree/argparse`'s `argparse.tcl:19-34`, with the outer
/// `{*}$validateHelper` variable indirection removed so `apply` is called
/// literally. Oracle: tclsh9.0.4 and tclsh8.6.14 both run it and print
/// `green`, so every command in it is real.
#[test]
fn apply_lambda_in_command_substitution_does_not_report_its_parameter_list() {
    let src = r#"set result [apply {{name opt args} {
    if {[dict exists $opt enum]} {
        set command [list tcl::prefix match -message "$name value" \
                             {*}[if {[uplevel 1 {info exists exact}]} {list -exact}] [dict get $opt enum]]
        set args [lmap arg $args {{*}$command $arg}]
    }
    return $args
}} widget [dict create enum {red green blue}] gr]
puts $result
"#;
    let mut a = crate::analyser::Analyser::new();
    let unknown: Vec<String> = a
        .analyse(src, "tcl9.0")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W123)
        .map(|d| d.message.clone())
        .collect();
    assert!(
        unknown.is_empty(),
        "the lambda's parameter list is not a command: {unknown:?}"
    );
}

/// FN half of the same fix: descending the lambda's *body* element is what
/// makes the commands inside it visible at all.  Before the fix a lambda
/// body nested in a `[…]` substitution was walked by nothing, so a genuinely
/// unknown command in it escaped W123 entirely.
#[test]
fn apply_lambda_body_in_command_substitution_is_walked() {
    for src in [
        "set r [apply {{a} {zz9unknowncmd}} 5]\nputs $r\n",
        "puts [apply {{a} {zz9unknowncmd}} 5]\n",
        "proc p {} { set r [apply {{a} {zz9unknowncmd}} 5]; return $r }\n",
        "set r [catch {apply {{a} {zz9unknowncmd}} 5}]\nputs $r\n",
        // Three-element lambda: element 2 is a namespace, not a script.
        "set r [apply {{a} {zz9unknowncmd} ::ns} 5]\nputs $r\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            codes.contains(&"W123".to_string()),
            "the lambda body's unknown command must be reported for {src:?}: {codes:?}"
        );
    }
    // TP control — the same command at the top level was already reported.
    assert!(
        codes_for_dialect("apply {{a} {zz9unknowncmd}} 5\n", "tcl9.0")
            .contains(&"W123".to_string())
    );
}

/// TN for the lambda carve-out: an ordinary registry `ArgRole::Body`
/// argument nested in the same substitution position is still a script and
/// is still walked — the fix narrows nothing but the lambda shape.
#[test]
fn plain_body_argument_in_command_substitution_still_walked() {
    for src in [
        "set r [if {1} {zz9unknowncmd}]\nputs $r\n",
        "set r [eval {zz9unknowncmd}]\nputs $r\n",
        "set r [uplevel 1 {zz9unknowncmd}]\nputs $r\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            codes.contains(&"W123".to_string()),
            "a plain body argument is still a script for {src:?}: {codes:?}"
        );
    }
}

/// FP guard — a lambda whose parameters are *named* like commands must draw
/// no W123: the parameter list is never a script, whatever the words in it
/// happen to spell.  Oracle (tclsh9.0.4, tclsh8.6.14): `apply {{set list}
/// {…}} a b` binds locals named `set` and `list` and runs fine — the words
/// are formal-parameter names, not calls.
#[test]
fn apply_lambda_parameters_named_like_commands_draw_no_unknown_command() {
    for src in [
        "set r [apply {{set list} {return \"$set$list\"}} a b]\nputs $r\n",
        // A defaulted parameter whose default value is a bareword.
        "set r [apply {{a {puts x}} {return $a}} 1]\nputs $r\n",
        // A parameter list whose words are not commands at all — the shape
        // that used to be reported as `Unknown command 'name opt args'`.
        "set r [apply {{name opt args} {return $name}} a b c]\nputs $r\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            !codes.contains(&"W123".to_string()),
            "parameter names are not command calls for {src:?}: {codes:?}"
        );
    }
}

/// Scope isolation for the substitution-position lambda (PR #1068 review).
///
/// `apply`'s lambda body runs in a **fresh call frame**, so a variable it sets
/// is a local of the lambda, never of the enclosing proc. Walking the body as
/// an ordinary registry `Body` argument would bind it in whatever scope the
/// `[…]` substitution sits in; routing the call through `apply`'s own analyser
/// hook is what keeps the frame separate.
///
/// Oracle (tclsh9.0.4) for the reviewer's own repro
/// `proc p {} { set r [apply {{} {gets stdin leaked}}]; puts $leaked }`:
/// `can't read "leaked": no such variable` from `puts $leaked` — the lambda's
/// `leaked` is invisible in `p`.
#[test]
fn apply_lambda_in_command_substitution_keeps_its_own_frame() {
    let scoped_vars = |src: &str| -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        let mut keys: Vec<String> = a.analyse(src, "tcl9.0").all_variables.into_keys().collect();
        keys.sort();
        keys
    };
    // TP — the reviewer's repro. `leaked` belongs to the lambda, not to `p`.
    let vars = scoped_vars(
        "proc p {} {\n    set r [apply {{} {gets stdin leaked}}]\n    puts $leaked\n}\n",
    );
    assert!(
        !vars.iter().any(|v| v == "p::leaked"),
        "the lambda's local must not leak into the enclosing proc: {vars:?}"
    );
    assert!(
        vars.iter()
            .any(|v| v.starts_with("apply@") && v.ends_with("::leaked")),
        "the lambda's local belongs to the lambda's own scope: {vars:?}"
    );
    // …and the read after the `try`-less lambda still warns, matching the
    // oracle's `can't read "leaked"`.
    assert!(
        codes_for_dialect(
            "proc p {} {\n    set r [apply {{} {gets stdin leaked}}]\n    puts $leaked\n}\n",
            "tcl9.0",
        )
        .contains(&"W210".to_string()),
        "an undefined read after the lambda must still warn"
    );
    // TP — a plain `set` inside the lambda body, same isolation.
    let vars = scoped_vars("proc p {} {\n    set r [apply {{} {set inner 1}}]\n    return $r\n}\n");
    assert!(
        !vars.iter().any(|v| v == "p::inner"),
        "a lambda-body `set` must not bind in the caller: {vars:?}"
    );
    // TP — the lambda's *parameters* are bound too, in the lambda's scope
    // (before the fix the substitution-position lambda registered none).
    let vars = scoped_vars("set r [apply {{name opt args} {return $name}} a b c]\n");
    for want in ["name", "opt", "args"] {
        assert!(
            vars.iter()
                .any(|v| v.starts_with("apply@") && v.ends_with(&format!("::{want}"))),
            "parameter {want} must bind in the lambda scope: {vars:?}"
        );
    }
    // TN — a genuine enclosing-scope binding from the same statement is
    // unaffected: `set r […]` still binds `r` in `p`.
    let vars = scoped_vars("proc p {} {\n    set r [apply {{} {set inner 1}}]\n    return $r\n}\n");
    assert!(
        vars.iter().any(|v| v == "p::r"),
        "the enclosing `set` still binds in the caller: {vars:?}"
    );
}

/// The lambda's namespace argument decides where its body's bareword calls
/// resolve — element 2, or the **global** namespace when absent, never the
/// caller's (`doc/apply.n`; `TclNRApplyObjCmd` in `generic/tclProc.c`
/// `::`-prefixes the word before the lookup).
///
/// Oracle (tclsh9.0.4 and tclsh8.6.14): with `helper` defined only in
/// `::myns`, `apply {{} {helper} ::myns}` returns its result, while
/// `apply {{} {helper}}` fails `invalid command name "helper"`.
///
/// The analyser records that namespace as a span-keyed override the LSP's
/// command resolution consults. A substitution-position lambda recorded none
/// at all before this fix (PR #1068 review).
#[test]
fn apply_lambda_in_command_substitution_records_its_namespace() {
    let overrides = |src: &str| -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl9.0")
            .namespace_overrides
            .into_iter()
            .map(|(_, ns)| ns)
            .collect()
    };
    // TP — explicit third element wins, in substitution position.
    assert_eq!(
        overrides("proc caller {} {\n    return [apply {{} {helper} ::myns}]\n}\n"),
        vec!["::myns".to_string()],
    );
    // TP — an unqualified namespace word is still resolved against the global
    // namespace, not the caller's.
    assert_eq!(
        overrides(
            "namespace eval ::outer {\n    proc caller {} {\n        return [apply {{} {helper} sub}]\n    }\n}\n"
        ),
        vec!["::sub".to_string()],
    );
    // TN — no third element means the global namespace, not `caller`'s.
    assert_eq!(
        overrides("proc caller {} {\n    return [apply {{} {helper}}]\n}\n"),
        vec!["::".to_string()],
    );
    // TN — the statement-position form was always right; it must stay
    // byte-identical to the substitution-position answer.
    assert_eq!(
        overrides("proc caller {} {\n    apply {{} {helper} ::myns}\n}\n"),
        overrides("proc caller {} {\n    return [apply {{} {helper} ::myns}]\n}\n"),
    );
}

#[test]
fn w211_deliberately_skips_destructuring_writer_outputs() {
    // Policy pin (review-2 audit): a command-output variable the script
    // never reads (`binary scan … rest`, `regexp … m`) is how Tcl spells
    // "ignore the remainder" — no W211.  A plain `set` of an unread
    // variable is the TP control.
    let src = "proc f {d} {
    binary scan $d H2H* type rest
    return $type
}
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        !codes.contains(&"W211".to_string()),
        "unread destructuring output must not draw W211: {codes:?}"
    );

    let src_tp = "proc g {} {
    set unread 1
    return 2
}
";
    let codes_tp = codes_for_dialect(src_tp, "tcl8.6");
    assert!(
        codes_tp.contains(&"W211".to_string()),
        "a plain unread set still draws W211: {codes_tp:?}"
    );
}

#[test]
fn w218_args_in_non_final_position_fires() {
    // TP: the classic pitfall — `args` before another parameter is an
    // ordinary parameter (C Tcl sets VAR_IS_ARGS only on the last formal).
    let mut a = crate::analyser::Analyser::new();
    let src = "proc p {args extra} { puts $extra }
";
    let r = a.analyse(src, "tcl8.6");
    let w218: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W218)
        .collect();
    assert_eq!(w218.len(), 1, "got {:?}", r.diagnostics);
    let expected = src.find("args").unwrap();
    assert_eq!(
        w218[0].span.start() as usize,
        expected,
        "anchor at the args parameter name"
    );
}

#[test]
fn w218_silent_for_final_args_and_plain_params() {
    // TN: `args` in final position is the variadic idiom; no `args` at all
    // is trivially fine; a lone `args` IS final.
    for src in [
        "proc p {a args} { puts $a }
",
        "proc p {a b} { puts $a$b }
",
        "proc p {args} { llength $args }
",
    ] {
        assert!(
            !codes_for_dialect(src, "tcl8.6").contains(&"W218".to_string()),
            "must be silent for {src:?}"
        );
    }
}

#[test]
fn w218_fires_for_method_and_apply_params() {
    // TP: the same pitfall inside a TclOO method parameter list and an
    // apply lambda.
    let method_src = "oo::class create C {
    method m {args other} { puts $other }
}
";
    assert!(
        codes_for_dialect(method_src, "tcl8.6").contains(&"W218".to_string()),
        "method params must be checked"
    );
    let apply_src = "apply {{args extra} { puts $extra }} 1 2
";
    assert!(
        codes_for_dialect(apply_src, "tcl8.6").contains(&"W218".to_string()),
        "apply lambda params must be checked"
    );
}

#[test]
fn w108_off_mode_disables_entirely() {
    use crate::analyser::NonAsciiMode::Off;
    // Even smart quotes / NBSP are silent when W108 is off.
    assert!(w108_mode("set x \u{201c}hi\u{201d}\u{a0}\n", "tcl8.6", Off).is_empty());
    // ...and off wins even for iRules (which would otherwise be strict).
    assert!(w108_mode("puts caf\u{e9}\n", "f5-irules", Off).is_empty());
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
    assert!(w108_mode("set x caf\u{e9}\n", "tcl8.6", Common).is_empty()); // é (Ll)
    assert!(w108_mode("set x 90\u{b0}\n", "tcl8.6", Common).is_empty()); // ° (So)
    assert!(w108_mode("set x \u{4e2d}\n", "tcl8.6", Common).is_empty()); // 中 (Lo)
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
    // U+202E RLO is *not* here: bidi controls moved to W305 (issue #1326),
    // so `common` mode reports the zero-width and control characters it was
    // always meant to catch and leaves the direction-altering set alone.
    assert!(w108_mode("set x a\u{202e}b\n", "tcl8.6", Common).is_empty());
}

#[test]
fn w104_flags_space_padded_append() {
    assert_eq!(code_sevs("append x \" foo\"\n", "W104"), vec!["Hint"]);
    assert_eq!(code_sevs("append result \"item \"\n", "W104"), vec!["Hint"]);
    assert!(code_sevs("append x foo\n", "W104").is_empty());
    assert!(code_sevs("lappend x foo\n", "W104").is_empty());
}

#[test]
fn w106_flags_unbraced_switch_body() {
    // Alternating unbraced body (no sub → WARNING; sub → ERROR).
    assert_eq!(code_sevs("switch $v a body\n", "W106"), vec!["Warning"]);
    assert_eq!(code_sevs("switch $v $pat $body\n", "W106"), vec!["Error"]);
    // Braced forms are fine.
    assert!(code_sevs("switch $v {a {x} b {y}}\n", "W106").is_empty());
    assert!(code_sevs("switch -regexp $v {a {x}}\n", "W106").is_empty());
    assert!(code_sevs("switch $v { a body }\n", "W106").is_empty());
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
    assert!(w100_sev("if {$x} {puts hi}\n").is_empty());
    assert!(w100_sev("expr {$a + $b}\n").is_empty());
    assert!(w100_sev("expr 1+2\n").is_empty());
    assert!(w100_sev("if 1 {puts hi}\n").is_empty());
    assert!(w100_sev("if {1} {puts hi}\n").is_empty());
}

#[test]
fn is_safe_literal_expr_classifies() {
    assert!(is_safe_literal("42"));
    assert!(is_safe_literal("true"));
    assert!(!is_safe_literal("$x"));
    assert!(is_safe_literal_expr("1 + 2", "tcl8.6"));
    assert!(!is_safe_literal_expr("$a + $b", "tcl8.6"));
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
fn w212_ignores_plain_names() {
    assert_eq!(w212_count("set x 1\n"), 0);
    assert_eq!(w212_count("info exists v\n"), 0);
    // A `$`-value in a non-name position is fine.
    assert_eq!(w212_count("set x $y\n"), 0);
}

#[test]
fn w212_covers_registry_name_positions() {
    // FN fixes: the old hardcoded list missed these name positions, which the
    // registry's VarWrite/VarRead roles now supply.
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

fn w216_count(src: &str) -> usize {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W216)
        .count()
}

#[test]
fn w216_upvar_local_name_is_indirect_array_idiom() {
    // FP fix: `${arr}(x)` in `upvar`'s local-name slot is the legitimate
    // indirect-array idiom (the same carve-out `set`/`vwait` already had). The
    // two name-position lists had drifted — W216's omitted `upvar`.
    assert_eq!(w216_count("proc p {arr} { upvar 1 remote ${arr}(x) }\n"), 0);
    // TP control: `${arr}(x)` in a *value* position is a genuine broken read.
    assert_eq!(w216_count("proc p {arr} { puts ${arr}(x) }\n"), 1);
}

#[test]
fn variable_name_positions_are_registry_driven() {
    let mut a = Analyser::new();
    a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
    let pos = |cmd: &str, args: &[&str]| {
        a.variable_name_positions(
            cmd,
            &args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        )
    };
    // Existing name-position commands (regression) — now resolved from the
    // registry's VarWrite/VarRead roles rather than a hardcoded list.
    assert_eq!(pos("set", &["a", "b"]), vec![0]);
    assert_eq!(pos("unset", &["-nocomplain", "a", "b"]), vec![1, 2]);
    assert_eq!(pos("info", &["exists", "v"]), vec![1]);
    assert_eq!(pos("info", &["level"]), Vec::<usize>::new());
    // `upvar` — only the *local* names (every other arg after the level word).
    assert_eq!(pos("upvar", &["1", "a", "b"]), vec![2]);
    assert_eq!(pos("upvar", &["a", "b"]), vec![1]); // no level word
    // FN fixes now covered by the registry roles that the old list omitted.
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
    assert_eq!(
        upvar_local_name_positions(&["1".into(), "a".into(), "b".into()]),
        vec![2],
    );
    assert_eq!(
        upvar_local_name_positions(&[
            "#0".into(),
            "r1".into(),
            "l1".into(),
            "r2".into(),
            "l2".into(),
        ]),
        vec![2, 4],
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
    // Issue #726: the `[expr {1+1}]` is an argument to `myCmd` (a fresh command
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
fn body_references_param_bare_dollar() {
    assert!(body_references_param("set y $x", "x"));
    assert!(body_references_param("return [expr {$a + $b}]", "a"));
    assert!(body_references_param("return [expr {$a + $b}]", "b"));
    assert!(body_references_param("puts [list $val 1]", "val"));
}

#[test]
fn body_references_param_braced_dollar() {
    assert!(body_references_param("set y ${x}", "x"));
    assert!(body_references_param("puts \"got ${val}!\"", "val"));
}

#[test]
fn body_references_param_no_match_for_substring_only() {
    // ``$abc`` must not match ``ab`` (boundary check).
    assert!(!body_references_param("set y $abc", "ab"));
    assert!(!body_references_param("puts $foobar", "foo"));
}

#[test]
fn body_references_param_skips_backslash_escape() {
    // ``\$x`` is a literal dollar — not a substitution.
    assert!(!body_references_param("puts \\$x", "x"));
}

#[test]
fn body_references_param_handles_multiple_uses() {
    assert!(body_references_param("set y $x; set z $x", "x"));
}

#[test]
fn body_references_param_misses_when_unused() {
    assert!(!body_references_param("puts hello", "x"));
    assert!(!body_references_param("return 42", "y"));
}

#[test]
fn body_references_param_braced_with_punct_after() {
    // ``${x}foo`` is a valid substitution — boundary not
    // required inside braces.
    assert!(body_references_param("set y ${x}foo", "x"));
}

#[test]
fn body_references_param_namespace_qualified() {
    // ``$ns::var`` is a qualified variable; the param name
    // is the leading identifier.  Boundary on ``::`` is
    // OK — both are part of the qualified name; the W214
    // emitter passes the bare param so this is a non-issue
    // in practice.  Test pins the boundary semantics.
    assert!(!body_references_param("set y $ns::var", "ns"));
}

fn diag(code: DiagCode, span: Span, msg: &str) -> Diagnostic {
    Diagnostic {
        code,
        span,
        message: msg.to_string(),
        severity: Severity::Warning,
        fixes: Vec::new(),
    }
}

#[test]
fn w004_fires_on_regsub_command_in_tcl86() {
    // `regsub -command` is Tcl 9.0+ (TIP 463); on Tcl 8.6 it
    // should produce a W004 dialect-availability warning.
    let mut a = Analyser::new();
    let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert!(
        !w004.is_empty(),
        "expected W004 on tcl8.6 regsub -command, got {:?}",
        result.diagnostics
    );
    assert!(w004[0].message.contains("-command"));
    assert!(w004[0].message.contains("regsub"));
}

#[test]
fn w004_skips_option_value_that_looks_like_a_flag() {
    // `-stride` is Tcl 9.0+ and takes a value.  On tcl8.6 the switch itself is
    // W004, but its value word — even when it looks like a flag (`-stride`
    // again) — must not be re-tested as a second gated option (Phase 4
    // value-skip).  Pre-fix this counted two W004s.
    assert_eq!(
        count_code("lsearch -stride -stride {a b} x", "W004"),
        1,
        "value word `-stride` was mistakenly re-tested as a second gated option"
    );
}

#[test]
fn w127_fires_on_invalid_option_enum_value() {
    // `-relief` carries a closed Tk value set; a literal outside it is W127.
    let bad = Analyser::new().analyse("button .b -relief bogus", "tk");
    assert!(
        bad.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W127 && d.message.contains("-relief")),
        "expected W127 on `-relief bogus`; got {:?}",
        bad.diagnostics
    );
    // A member value is accepted.
    let good = Analyser::new().analyse("button .b -relief raised", "tk");
    assert!(
        !good.diagnostics.iter().any(|d| d.code == DiagCode::W127),
        "`raised` is a valid relief; got {:?}",
        good.diagnostics
    );
    // A dynamic value is skipped (can't be checked statically).
    let dynamic = Analyser::new().analyse("button .b -relief $r", "tk");
    assert!(
        !dynamic.diagnostics.iter().any(|d| d.code == DiagCode::W127),
        "dynamic `-relief $r` must be skipped; got {:?}",
        dynamic.diagnostics
    );
}

#[test]
fn w127_fires_on_invalid_subcommand_closed_value() {
    fn has_w127(src: &str) -> bool {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W127)
    }
    // FN fix: `string is <class>` carries an exhaustive class set on the `is`
    // subcommand; a non-member is a runtime `bad class` error.
    assert!(has_w127("string is booleanx 1\n"));
    assert!(has_w127("string is xyz 1\n"));
    // C Tcl accepts a unique prefix — must NOT fire.
    assert!(!has_w127("string is boolean 1\n"));
    assert!(!has_w127("string is boo 1\n"));
    assert!(!has_w127("string is b 1\n"));
    assert!(!has_w127("string is dig 1\n"));
    // FN fix: an *ambiguous* prefix — one matching two or more classes — is a
    // runtime error in C Tcl (only a *unique* prefix abbreviates), so W127 must
    // fire, not silently accept it.
    assert!(has_w127("string is a 1\n")); // alnum / alpha / ascii
    assert!(has_w127("string is d 1\n")); // digit / double
    assert!(has_w127("string is w 1\n")); // wideinteger / wordchar
    // Dynamic class is skipped.
    assert!(!has_w127("string is $c 1\n"));
    // The message names the subcommand.
    let d = Analyser::new().analyse("string is booleanx 1\n", "tcl8.6");
    assert!(
        d.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W127 && d.message.contains("string is")),
        "message must name `string is`; got {:?}",
        d.diagnostics,
    );
}

/// Diagnostic codes for `src` under the Tcl 9.0 profile, in emission order.
fn all_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl9.0")
        .diagnostics
        .into_iter()
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn e003_tp_call_between_two_declarations_checks_the_first_signature() {
    // TP — issue #923 idx 45. `p a b` sits between a zero-parameter `proc p`
    // and a later one-parameter redefinition; tclsh 9.0.4 and 8.6.16 both
    // fail it with `wrong # args: should be "p"` — the *first* signature.
    // `all_procs` alone only remembers the second, so the call resolved to
    // nothing and the guaranteed failure went unreported.
    let src = "proc p {} { return 1 }\np a b\nproc p {a} { return $a }\n";
    assert!(
        all_codes(src).contains(&"E003".to_string()),
        "the in-between call is checked against the first definition: {:?}",
        all_codes(src)
    );
}

#[test]
fn e003_tn_call_between_two_declarations_matching_the_first_is_silent() {
    // TN — the paired guard: the same position with an argument count the
    // *first* definition accepts (and the second would reject) must stay
    // silent. tclsh: `p` there returns `first`.
    let src = "proc p {} { return 1 }\np\nproc p {a} { return $a }\n";
    assert!(
        !all_codes(src).contains(&"E003".to_string()),
        "a call the in-effect definition accepts is not an arity error: {:?}",
        all_codes(src)
    );
}

#[test]
fn e003_tp_call_after_both_declarations_checks_the_last_signature() {
    // TP — last-redefinition-wins is still right after the second header:
    // tclsh fails a bare `p` with `wrong # args: should be "p a"`.
    let src = "proc p {} { return 1 }\nproc p {a} { return $a }\np\n";
    assert!(
        all_codes(src).contains(&"E002".to_string()),
        "the trailing call is checked against the second definition: {:?}",
        all_codes(src)
    );
}

// E002 / E003 arity

#[test]
fn e003_not_emitted_for_leading_switches() {
    // Declared option flags must be skipped
    // before counting positional args.  `regsub` (max arity 4)
    // previously tripped a false E003 once any switch appeared.
    // These switches exist in every supported dialect.
    for snippet in [
        "regsub -all -line {x} $args {} str",
        "regsub -all {a} $b {} c",
        "regsub -nocase -all -- $pat $s {} out",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E003)
            .collect();
        assert!(e003.is_empty(), "unexpected E003 for {snippet:?}: {e003:?}");
    }
}

#[test]
fn e003_not_emitted_for_value_taking_leading_option() {
    // `-start` consumes a value word, so
    // `regsub -start 0 $exp $str $sub out` is `-start` + its value +
    // 4 positional (exp/string/subSpec/varName) = max arity 4 → valid.
    // A name-only leading-option skip miscounted the `0` value word as a
    // positional and tripped a false E003.
    for snippet in [
        "regsub -start 0 $exp $str $sub out",
        "regsub -all -start 3 {a} $b {} c",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E003)
            .collect();
        assert!(
            e003.is_empty(),
            "unexpected E003 for value-taking option in {snippet:?}: {e003:?}"
        );
    }
}

#[test]
fn e003_still_fires_past_value_taking_option() {
    // The value-word skip must not mask a genuine over-arity: after
    // `-start 0` there are 5 positional words (> max 4) → real E003.
    let mut a = Analyser::new();
    let result = a.analyse("regsub -start 0 a b c d e", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 past a value-taking option, got {:?}",
        result.diagnostics
    );
}

#[test]
fn interp_optional_path_subcommands_no_arity_error() {
    // `interp issafe`/`exists`/`hidden` take an optional
    // `?path?`; the zero-arg idiom must not trip E002.
    for snippet in ["interp issafe", "interp exists", "interp hidden"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let arity_err: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .collect();
        assert!(
            arity_err.is_empty(),
            "unexpected arity error for {snippet:?}: {arity_err:?}"
        );
    }
    // The one-arg form is still accepted.
    let mut a = Analyser::new();
    let ok = a.analyse("interp issafe child", "tcl8.6");
    assert!(
        !ok.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003),
    );
}

#[test]
fn interp_create_with_options_no_arity_error() {
    // `interp create -safe -- name` — the option words are
    // skipped, leaving one positional (`name`) within the 0..=1 bound.
    let mut a = Analyser::new();
    let result = a.analyse("interp create -safe -- child", "tcl8.6");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003),
        "unexpected arity error for `interp create -safe -- child`: {:?}",
        result.diagnostics
    );
    // Two positional names is a genuine over-arity.
    let mut a2 = Analyser::new();
    let over = a2.analyse("interp create a b", "tcl8.6");
    assert!(
        over.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 for two-name `interp create`, got {:?}",
        over.diagnostics
    );
}

#[test]
fn e003_fires_on_genuine_over_arity() {
    // 5 positional args for `regsub` (max 4) is a real error.
    let mut a = Analyser::new();
    let result = a.analyse("regsub a b c d e", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003, got {:?}",
        result.diagnostics
    );
}

#[test]
fn e003_switch_options_are_dialect_filtered() {
    // `regsub -command` is Tcl 9.0+ (TIP 463).
    // Under 9.0 it is a real switch → skipped → 4 positional → OK.
    let mut a = Analyser::new();
    let r9 = a.analyse("regsub -command a b c d", "tcl9.0");
    assert!(
        !r9.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "unexpected E003 under tcl9.0: {:?}",
        r9.diagnostics
    );
    // Under 8.6 `-command` is unknown → counted positional →
    // 5 > max 4 → E003 (dialect-leak guard).
    let mut a2 = Analyser::new();
    let r8 = a2.analyse("regsub -command a b c d", "tcl8.6");
    assert!(
        r8.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 under tcl8.6, got {:?}",
        r8.diagnostics
    );
}

#[test]
fn e003_source_nopkg_is_tcl9_only() {
    // C Tcl 9.0 accepts `source -nopkg file`, then attempts to read `file`.
    // Tcl 8.6 rejects the same shape as a wrong-arity invocation. The registry
    // must therefore skip the flag only for the Tcl 9 dialect rather than
    // treating it as a universal source argument.
    let mut tcl9 = Analyser::new();
    let r9 = tcl9.analyse("source -nopkg missing.tcl", "tcl9.0");
    assert!(
        !r9.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "unexpected E003 under tcl9.0: {:?}",
        r9.diagnostics
    );

    let mut tcl86 = Analyser::new();
    let r86 = tcl86.analyse("source -nopkg missing.tcl", "tcl8.6");
    assert!(
        r86.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 under tcl8.6: {:?}",
        r86.diagnostics
    );
}

#[test]
fn e003_suppressed_by_expanded_word() {
    // `{*}$rest` expands to an unknown count, so the expanded word
    // is excluded from the positional lower bound: `regsub a b c d
    // {*}$rest` has 4 non-expanded positional words (≤ max 4) and
    // must not trip E003, whereas the same five literal words do.
    let mut a = Analyser::new();
    let expanded = a.analyse("regsub a b c d {*}$rest", "tcl8.6");
    assert!(
        !expanded
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E003),
        "expansion should suppress E003: {:?}",
        expanded.diagnostics
    );
    let mut b = Analyser::new();
    let literal = b.analyse("regsub a b c d e", "tcl8.6");
    assert!(
        literal.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "control: five literal words should fire E003: {:?}",
        literal.diagnostics
    );
}

#[test]
fn e003_arity_is_dialect_aware_via_expand_syntax() {
    // End-to-end proof that the
    // document dialect reaches the analyser's segmenter (and thus the
    // lexer's `expand_syntax` flag).  `{*}` is the expansion operator
    // on 8.5+ but a literal brace word on 8.4, so for
    // `regsub a b c d {*}$rest`:
    //   * tcl8.4 — `{*}$rest` is a 5th literal positional word; 5 > max
    //     4 → E003 fires.
    //   * tcl9.0 — `{*}$rest` expands, contributing an unbounded count;
    //     the 4 non-expanded words are ≤ max 4 → E003 is suppressed.
    // Before the dialect → `LexerConfig` wiring the analyser always
    // lexed with `expand_syntax` on, so 8.4 wrongly behaved like 9.0
    // (no E003) — this asserts the two now diverge.
    let codes = |dialect: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse("regsub a b c d {*}$rest", dialect)
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect()
    };
    let on_84 = codes("tcl8.4");
    assert!(
        on_84.iter().any(|c| c == "E003"),
        "8.4 treats `{{*}}` as a literal word → 5 positional args → E003: {on_84:?}",
    );
    let on_90 = codes("tcl9.0");
    assert!(
        !on_90.iter().any(|c| c == "E003"),
        "9.0 expands `{{*}}` → 4 positional words ≤ max → no E003: {on_90:?}",
    );
}

// -- subcommand-level E003 arity (per-subcommand signatures) -----

#[test]
fn e003_fires_on_subcommand_over_arity() {
    // `string length` takes exactly one argument — three positional
    // words must trip E003.
    let mut a = Analyser::new();
    let result = a.analyse("string length a b c", "tcl8.6");
    let e003: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert!(
        !e003.is_empty(),
        "expected E003 for `string length a b c`, got {:?}",
        result.diagnostics
    );
    assert!(
        e003[0].message.contains("string length"),
        "message should name the subcommand: {:?}",
        e003[0].message
    );
}

#[test]
fn e003_fires_on_file_link_over_arity() {
    // `file link ?-linktype? linkName ?target?` — `link` accepts at
    // most two positional args, so three literal targets is E003.
    let mut a = Analyser::new();
    let result = a.analyse("file link $a $b $c", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 for `file link $a $b $c`, got {:?}",
        result.diagnostics
    );
}

#[test]
fn e003_namespace_which_extra_positional_fires() {
    // `namespace which ?-command? ?-variable? name` — exactly one trailing
    // `name`, the flags declared as options. Verified vs tclsh 9.0: a second
    // positional (`foo bar`) errors, but the flag forms are fine.
    fn e003(src: &str) -> bool {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E003)
    }
    // FN fix: a bare second positional is now flagged.
    assert!(
        e003("namespace which foo bar"),
        "extra positional must fire E003"
    );
    // Valid forms stay silent — the flags are skipped before counting.
    assert!(!e003("namespace which foo"), "one name is valid");
    assert!(
        !e003("namespace which -command foo"),
        "-command name is valid"
    );
    assert!(
        !e003("namespace which -variable foo"),
        "-variable name is valid"
    );
}

#[test]
fn e003_silent_for_subcommand_leading_options() {
    // Per-subcommand options (`file link -symbolic` / `-hard`,
    // `string match -nocase`) must be skipped before counting
    // positionals, so these well-formed calls stay silent.
    for snippet in [
        "file link -symbolic $a $b",
        "file link -hard $a $b",
        "string match -nocase $a $b",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E003)
            .collect();
        assert!(e003.is_empty(), "unexpected E003 for {snippet:?}: {e003:?}");
    }
}

#[test]
fn w147_reports_registry_declared_mutually_exclusive_options() {
    let mut a = Analyser::new();
    let result = a.analyse("source -encoding utf-8 -nopkg file.tcl\n", "tcl9.0");
    let conflicts: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W147)
        .collect();
    assert_eq!(
        conflicts.len(),
        1,
        "expected one W147: {:?}",
        result.diagnostics
    );
    assert!(conflicts[0].message.contains("-encoding, -nopkg"));
    assert!(conflicts[0].fixes.is_empty(), "intent is ambiguous");
    let mut legacy = Analyser::new();
    let legacy_result = legacy.analyse("source -encoding utf-8 -nopkg file.tcl\n", "tcl8.6");
    assert!(
        !legacy_result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W147),
        "Tcl 9-only relationship must not leak into Tcl 8.6"
    );
}

#[test]
fn w147_is_generic_and_covers_glob_without_source_logic() {
    let mut a = Analyser::new();
    let result = a.analyse("glob -directory root -path prefix *.tcl\n", "tcl8.6");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W147)
            .count(),
        1,
        "expected generic option conflict: {:?}",
        result.diagnostics
    );
}

#[test]
fn subcommand_arity_skips_unknown_and_dynamic_subcommands() {
    // An unknown subcommand is W001's job, not E003; a dynamic
    // subcommand word (`$sub`) can't be resolved, so neither path
    // should emit E003.
    for snippet in ["string $sub a b c", "string [x] a b c"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
            "unexpected E003 for {snippet:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn after_integer_ms_is_not_unknown_subcommand() {
    // Regression for #720: ``after`` dispatches on cancel/idle/info, but its
    // first word may instead be a millisecond delay. An integer first word is
    // a valid time argument, not an unknown subcommand, so no W001 fires.
    for snippet in ["after 200 {puts \"Hello world!\"}", "after 0", "after 1000"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "unexpected W001 for {snippet:?}: {:?}",
            result.diagnostics
        );
    }
    // A non-integer, non-subcommand first word is still a genuine error.
    let mut a = Analyser::new();
    let result = a.analyse("after bogus {puts hi}", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "W001 expected for `after bogus`: {:?}",
        result.diagnostics
    );
}

#[test]
fn w001_accepts_unique_prefix_subcommand_abbreviations() {
    // Tcl ensemble dispatch accepts unique-prefix abbreviations; the analyser
    // must not flag them as unknown subcommands (W001).
    for snippet in [
        "string le $s",   // length
        "string leng $s", // length
        "info ex v",      // exists
        "dict k $d",      // keys
        "string rev $s",  // reverse
    ] {
        assert!(
            !has_code(snippet, "tcl8.6", "W001"),
            "unique-prefix abbreviation {snippet:?} must not trip W001"
        );
    }
    // A genuinely unknown word still fires.
    assert!(
        has_code("string zzz $s", "tcl8.6", "W001"),
        "an unknown subcommand must still fire W001"
    );
    // An ambiguous prefix (`string t` → tolower/totitle/toupper/trim…) is not
    // a valid abbreviation. Since #1234 it gets its own diagnostic naming the
    // candidate set (W145) instead of the unknown-subcommand guess (W001).
    assert!(
        has_code("string t $s", "tcl8.6", "W145"),
        "an ambiguous prefix must fire W145"
    );
    assert!(
        !has_code("string t $s", "tcl8.6", "W001"),
        "W145 replaces the W001 guess for an ambiguous prefix"
    );
}

#[test]
fn command_version_gates_fire_w123() {
    // Whole commands introduced after 8.4 must be unknown (W123) in older
    // dialects and known once available.
    let cases = [
        ("apply {{} {return 1}}", "tcl8.5", "tcl8.4"),
        ("lreverse {a b c}", "tcl8.5", "tcl8.4"),
        ("lrepeat 3 x", "tcl8.5", "tcl8.4"),
        // NB: `const` is intentionally universal (valid in iRules), so it is
        // deliberately NOT version-gated — see const_.rs.
    ];
    for (snippet, ok, old) in cases {
        assert!(
            Analyser::new()
                .analyse(snippet, old)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123),
            "expected W123 for {snippet:?} on {old}"
        );
        assert!(
            !Analyser::new()
                .analyse(snippet, ok)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123),
            "unexpected W123 for {snippet:?} on {ok}"
        );
    }
}

#[test]
fn subcommand_version_gates_fire_w002() {
    // A subcommand added or removed across 8.4-9.1 still *exists* — just not in
    // every dialect.  Used in a dialect where it is absent it must warn with
    // W002 ("disabled in the active dialect profile"), the subcommand-level
    // analogue of the whole-command W002 check, NOT W001 ("Unknown
    // subcommand"), which is reserved for a name that exists in no dialect at
    // all (a genuine typo).  This is issue #812: `info cmdtype` is a real Tcl
    // 9.0 subcommand, so flagging it as "unknown" under the default 8.6 profile
    // was wrong.
    let added = [
        // (snippet, first dialect it exists in, an older dialect)
        ("string reverse abc", "tcl8.5", "tcl8.4"),
        ("package prefer stable", "tcl8.5", "tcl8.4"),
        ("encoding dirs", "tcl8.5", "tcl8.4"),
        ("binary encode base64 abc", "tcl8.6", "tcl8.5"),
        ("binary decode base64 abc", "tcl8.6", "tcl8.5"),
        ("interp bgerror {}", "tcl8.5", "tcl8.4"),
        ("interp limit {} time", "tcl8.5", "tcl8.4"),
        // interp debug is documented from Tcl 8.5's interp.n (present in
        // the 8.5 SYNOPSIS/body, absent from 8.4's subcommand list).
        ("interp debug {}", "tcl8.5", "tcl8.4"),
        ("interp cancel", "tcl8.6", "tcl8.5"),
        ("interp children", "tcl8.6", "tcl8.5"),
        ("clock add 0 1 day", "tcl8.5", "tcl8.4"),
        ("clock microseconds", "tcl8.5", "tcl8.4"),
        ("clock milliseconds", "tcl8.5", "tcl8.4"),
        // Issue #812: `info cmdtype` is new in Tcl 9.0.
        ("info cmdtype foo", "tcl9.0", "tcl8.6"),
    ];
    for (snippet, ok, old) in added {
        let old_diags = Analyser::new().analyse(snippet, old).diagnostics;
        assert!(
            old_diags.iter().any(|d| d.code == DiagCode::W002),
            "expected W002 for {snippet:?} on {old}"
        );
        assert!(
            !old_diags.iter().any(|d| d.code == DiagCode::W001),
            "unexpected W001 (should be W002) for {snippet:?} on {old}"
        );
        assert!(
            !Analyser::new()
                .analyse(snippet, ok)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W001 || d.code == DiagCode::W002),
            "unexpected W001/W002 for {snippet:?} on {ok}"
        );
    }
    // `trace variable/vdelete/vinfo` were removed in 9.0: known in 8.6, gone in
    // 9.0.  In 9.0 they still resolve in an older dialect, so they are W002
    // (disabled here), not W001 (unknown everywhere).
    for snippet in [
        "trace variable v w {}",
        "trace vdelete v w {}",
        "trace vinfo v",
        // Tcl 8.6 introduced preferred `interp children`; Tcl 9 still
        // accepts legacy `interp slaves` as a deprecated compatibility form.
        "interp slaves",
    ] {
        assert!(
            !Analyser::new()
                .analyse(snippet, "tcl8.6")
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W001 || d.code == DiagCode::W002),
            "unexpected W001/W002 for {snippet:?} on tcl8.6"
        );
        let nine = Analyser::new().analyse(snippet, "tcl9.0").diagnostics;
        assert!(
            nine.iter().any(|d| d.code == DiagCode::W002),
            "expected W002 for {snippet:?} on tcl9.0 (removed, but exists in 8.x)"
        );
        assert!(
            !nine.iter().any(|d| d.code == DiagCode::W001),
            "unexpected W001 (should be W002) for {snippet:?} on tcl9.0"
        );
    }
    // A subcommand that exists in *no* dialect is still a genuine W001.
    let typo = Analyser::new()
        .analyse("string bogusxyz abc", "tcl8.6")
        .diagnostics;
    assert!(
        typo.iter().any(|d| d.code == DiagCode::W001),
        "expected W001 for a genuinely unknown subcommand"
    );
    assert!(
        !typo.iter().any(|d| d.code == DiagCode::W002),
        "unexpected W002 for a genuinely unknown subcommand"
    );
}

#[test]
fn info_frame_is_dialect_gated_to_8_5_plus() {
    // `info frame` was introduced in Tcl 8.5 (TIP 280); it does not exist in
    // 8.4.  Because it *does* exist in 8.5+, using it under 8.4 is W002
    // ("disabled in the active dialect profile"), not W001 ("Unknown
    // subcommand") — the subcommand exists, just not in that dialect (#812).
    assert!(
        has_code("info frame\n", "tcl8.4", "W002"),
        "info frame should be disabled-in-dialect (W002) in tcl8.4"
    );
    assert!(
        !has_code("info frame\n", "tcl8.4", "W001"),
        "info frame is a real subcommand, so not W001 in tcl8.4"
    );
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(
            !has_code("info frame\n", dialect, "W001"),
            "info frame should be known in {dialect}"
        );
        assert!(
            !has_code("info frame\n", dialect, "W002"),
            "info frame should be enabled in {dialect}"
        );
    }
}

#[test]
fn namespace_unknown_is_dialect_gated_to_8_5_plus() {
    // `namespace unknown` was added in 8.5, like the sibling `namespace path`;
    // 8.4's `namespace` ensemble has no `unknown` subcommand.  Using it under
    // 8.4 is W002 (disabled-in-dialect), not W001 (the subcommand exists), and
    // it is clean from 8.5 on.
    assert!(
        has_code("namespace unknown handler\n", "tcl8.4", "W002"),
        "namespace unknown should be disabled-in-dialect (W002) in tcl8.4"
    );
    assert!(
        !has_code("namespace unknown handler\n", "tcl8.4", "W001"),
        "namespace unknown is a real subcommand, so not W001 in tcl8.4"
    );
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(
            !has_code("namespace unknown handler\n", dialect, "W002"),
            "namespace unknown should be enabled in {dialect}"
        );
    }
}

#[test]
fn e001_fires_for_bare_subcommand_command() {
    // A subcommand-dispatch command (`string`, `dict`, `info`) invoked
    // with no subcommand at all is E001.
    for snippet in ["string", "dict", "info"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e001: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E001)
            .collect();
        assert_eq!(
            e001.len(),
            1,
            "expected one E001 for {snippet:?}: {result:?}",
        );
        assert_eq!(
            e001[0].message,
            format!("'{snippet}' requires a subcommand")
        );
        assert_eq!(e001[0].severity, Severity::Error);
    }
}

#[test]
fn e001_quiet_when_subcommand_present() {
    let mut a = Analyser::new();
    let result = a.analyse("string length abc", "tcl8.6");
    assert!(!result.diagnostics.iter().any(|d| d.code == DiagCode::E001));
}

#[test]
fn e001_suppressed_by_shadowing_user_proc() {
    // A user proc named `string` shadows the builtin ensemble — a bare
    // `string` call resolves to the proc, so no E001.
    let mut a = Analyser::new();
    let result = a.analyse("proc string {} { return x }\nstring", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::E001),
        "shadowing proc should suppress E001: {result:?}"
    );
}

#[test]
fn e001_fn_true_negative_bare_history_has_a_default_subcommand() {
    // FP regression: `history` is a `WithSubcommands` registry command
    // (`add`/`change`/`clear`/`event`/`info`/`keep`/`nextid`/`redo`) but,
    // unlike `string`/`dict`/`info`, a bare call is well-defined Tcl —
    // history(n): "If no option is specified, the default is info."
    // Confirmed stable since Tcl 7.x; still true under Tcl 9. The registry
    // spec records this as `arity: Arity::at_least(0)` and the analyser
    // must honour it instead of assuming every ensemble-shaped command
    // requires its dispatch word.
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        assert!(
            !has_code("history", dialect, "E001"),
            "bare `history` defaults to `history info` — not E001 in {dialect}"
        );
    }
}

#[test]
fn e001_tp_bare_history_subcommand_sibling_still_required() {
    // Every *other* WithSubcommands command in the same family keeps
    // requiring its dispatch word — the `history` carve-out must not leak
    // into commands whose registry `arity.min` is genuinely 1.
    for snippet in [
        "array",
        "chan",
        "clock",
        "encoding",
        "file",
        "namespace",
        "package",
        "trace",
    ] {
        assert!(
            has_code(snippet, "tcl8.6", "E001"),
            "bare `{snippet}` should still require a subcommand"
        );
    }
}

#[test]
fn e001_tn_history_with_subcommand_is_unaffected() {
    // Sanity: a `history` call that *does* supply a subcommand is untouched
    // by the bare-call carve-out — still no E001, and a genuine unknown
    // subcommand is still W001.
    assert!(!has_code("history info", "tcl8.6", "E001"));
    assert!(!has_code("history clear", "tcl8.6", "E001"));
    assert!(has_code("history bogus", "tcl8.6", "W001"));
    assert!(!has_code("history bogus", "tcl8.6", "E001"));
}

#[test]
fn e002_fires_on_too_few_args() {
    // `regsub` requires at least 3 args (exp string subSpec).
    let mut a = Analyser::new();
    let result = a.analyse("regsub a b", "tcl8.6");
    let e002: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E002)
        .collect();
    assert!(
        !e002.is_empty(),
        "expected E002 for `regsub a b`, got {:?}",
        result.diagnostics
    );
    assert!(e002[0].message.contains("at least 3"));
}

#[test]
fn e003_shadow_is_namespace_scoped() {
    // A namespaced proc named `close` must
    // NOT suppress arity checks on a *global* `close` call (which
    // resolves to the builtin, max 2), but must suppress a `close`
    // call inside its own namespace (which resolves to the proc).
    let src = "proc ::ns::close {a b c d} {}\n\
                   close x y z\n\
                   namespace eval ::ns { close x y z }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected exactly one E003 (the global close), got {:?}",
        r.diagnostics
    );
    // The flagged call must be the top-level one, before the
    // `namespace eval` body (both call sites share the same text).
    let ns_eval_off = src.find("namespace eval").unwrap();
    let span = e003[0].span;
    assert!(
        (span.start() as usize) < ns_eval_off,
        "flagged the namespaced call instead of the global one: {:?}",
        &src[span.start() as usize..span.end() as usize],
    );
}

// reachable, in-order shadow gating

#[test]
fn e003_top_level_call_before_shadowing_proc_fires() {
    // A top-level `close x y z` *before* `proc close` resolves to
    // the builtin at load time (the proc does not exist yet), so the
    // builtin arity check must fire even though a same-named proc is
    // defined later in the file.  Exercises the in-order gate
    // (without it the post-walk flush silenced this).
    let src = "close x y z\nproc close {a b c d} {}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected E003 on the top-level close before its shadowing proc, got {:?}",
        r.diagnostics
    );
    // The flagged call is the top-level one on line 1, not the proc on line 2.
    // E003 highlights only the surplus arguments, so the span starts at the
    // first excess word rather than the command name — assert it lands on the
    // first line.
    let line2 = src.find('\n').unwrap();
    assert!(
        (e003[0].span.start() as usize) < line2,
        "wrong call flagged: {:?}",
        &src[e003[0].span.start() as usize..e003[0].span.end() as usize],
    );
}

#[test]
fn e003_top_level_call_after_shadowing_proc_suppressed() {
    // The mirror image: once `proc close` is defined, a later
    // top-level `close x y z` resolves to the 4-param user proc, so
    // the builtin arity check is suppressed.
    let src = "proc close {a b c d} {}\nclose x y z\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "no E003 expected — the call follows its shadowing proc, got {:?}",
        r.diagnostics
    );
}

#[test]
fn e003_proc_body_call_not_order_gated() {
    // A call inside a proc body resolves when that proc is *invoked*
    // — after the whole script has loaded — so a shadowing proc
    // defined later in the file still suppresses the builtin check.
    // Order is only enforced for top-level calls.
    let src = "proc foo {} { close x y z }\nproc close {a b c d} {}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "no E003 expected — proc-body calls are not order-gated, got {:?}",
        r.diagnostics
    );
}

#[test]
fn e003_static_rename_onto_builtin_name_suppresses_arity() {
    // `rename OLD NEW` moves OLD's identity onto NEW — `rename myimpl
    // close` makes `close` a real command backed by `myimpl`'s own
    // 3-parameter signature, not the registry `close` builtin (max 2), so
    // a 3-argument call must not fire E003.  Order-gated the same way as a
    // proc: the rename statement must lexically precede a top-level call.
    let src = "proc myimpl {a b c} { return $a }\nrename myimpl close\nclose 1 2 3\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "no E003 expected — close was renamed onto myimpl's 3-arg signature, got {:?}",
        r.diagnostics
    );
}

#[test]
fn e003_top_level_call_before_rename_onto_builtin_still_fires() {
    // The mirror image of `e003_top_level_call_before_shadowing_proc_fires`
    // for a rename target: a top-level call *before* the `rename` runs
    // still reaches the original builtin at load time.
    let src = "close 1 2 3\nproc myimpl {a b c} { return $a }\nrename myimpl close\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected E003 on the top-level close before the rename took effect, got {:?}",
        r.diagnostics
    );
    // Tight E003 span: the surplus argument of the line-1 call, not the
    // renamed proc on a later line.
    let line2 = src.find('\n').unwrap();
    assert!(
        (e003[0].span.start() as usize) < line2,
        "wrong call flagged: {:?}",
        &src[e003[0].span.start() as usize..e003[0].span.end() as usize],
    );
}

// E003 tight-range + registry arity-data corrections (verified against
// tclsh 9.0.4). See `rust/tcl-registry/src/commands/tcl/{lmap,global,
// variable,auto_load,auto_import}_.rs`.

/// Return the source text E003 highlights, so range tightness can be
/// asserted against the *problem* words rather than the whole command.
fn e003_highlighted_text(src: &str) -> String {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::E003)
        .unwrap_or_else(|| panic!("no E003 in {:?}", r.diagnostics));
    src[d.span.start() as usize..d.span.end() as usize].to_string()
}

#[test]
fn e003_highlights_only_the_surplus_arguments() {
    // TP + range precision: the squiggle covers exactly the excess words,
    // not the command name or the valid arguments.
    assert_eq!(
        e003_highlighted_text("lreverse {a b c} extra1 extra2\n"),
        "extra1 extra2",
        "E003 must highlight only the surplus arguments",
    );
    assert_eq!(
        e003_highlighted_text("string index abc 0 extra\n"),
        "extra",
        "E003 on a subcommand must highlight only the surplus word",
    );
    // Leading options are skipped: the surplus is measured after them.
    assert_eq!(
        e003_highlighted_text("puts -nonewline stdout hello extra\n"),
        "extra",
        "E003 must skip leading options when isolating the surplus",
    );
}

#[test]
fn e003_falls_back_to_whole_command_on_expansion() {
    // A `{*}`-expanded positional makes the first surplus word ambiguous, so
    // the highlight falls back to the whole command (still a TP).
    let src = "lreverse {a b c} {*}$extra more\n";
    let text = e003_highlighted_text(src);
    assert!(
        text.starts_with("lreverse"),
        "expansion case should fall back to the whole command, got {text:?}",
    );
}

#[test]
fn lmap_odd_even_parity_fires_e005() {
    // FN fix: `lmap` shares `foreach`'s odd/even grammar. An even count is
    // `wrong # args` (tclsh 9.0.4); a valid odd count is silent.
    assert!(has_code("lmap a b c d\n", "tcl8.6", "E005"));
    assert!(!has_code("lmap x {1 2 3} {incr x}\n", "tcl8.6", "E005"));
    assert!(!has_code(
        "lmap a {1 2} b {3 4} {expr {$a+$b}}\n",
        "tcl8.6",
        "E005",
    ));
}

#[test]
fn global_and_variable_zero_args_are_no_op_not_e002() {
    // FP fix: bare `global` / `variable` are valid no-ops (C Tcl has no
    // `Tcl_WrongNumArgs` on either).
    for src in ["global\n", "variable\n", "global a b c\n"] {
        assert!(
            !has_code(src, "tcl8.6", "E002"),
            "{src:?} must not draw E002",
        );
    }
}

#[test]
fn auto_load_and_auto_import_arity_bounds() {
    // auto_load cmd ?namespace? — 1..2 valid, 3 too many.
    assert!(!has_code("auto_load foo\n", "tcl8.6", "E003"));
    assert!(!has_code("auto_load foo ::ns\n", "tcl8.6", "E003"));
    assert!(has_code("auto_load a b c\n", "tcl8.6", "E003"));
    // auto_import pattern — exactly 1.
    assert!(!has_code("auto_import xyz*\n", "tcl8.6", "E003"));
    assert!(has_code("auto_import a b\n", "tcl8.6", "E003"));
}

// E005 — argument-count *shape* (parity/predicate) mismatches on the
// registry's key/value-pair and paired-argument commands (`dict create`,
// `dict replace`, `dict update`, `foreach`, `switch`). All confirmed
// against tclsh 8.6.14's "wrong # args" behaviour.

fn arity_shape_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, DiagCode::E002 | DiagCode::E003 | DiagCode::E005))
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn dict_create_odd_key_value_tail_fires_e005() {
    // `dict create ?key value ...?` — an odd tail has an unpaired key with
    // no value (tclsh 8.6.14: `dict create a` fails "wrong # args").
    assert_eq!(
        arity_shape_codes("dict create a\n"),
        vec!["E005".to_owned()]
    );
    assert_eq!(
        arity_shape_codes("dict create a b c\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn dict_create_even_key_value_tail_is_silent() {
    // TN: a paired (or empty) tail is the documented shape.
    assert_eq!(arity_shape_codes("dict create\n"), Vec::<String>::new());
    assert_eq!(arity_shape_codes("dict create a b\n"), Vec::<String>::new());
    assert_eq!(
        arity_shape_codes("dict create a b c d\n"),
        Vec::<String>::new()
    );
}

#[test]
fn dict_replace_even_tail_fires_e005() {
    // `dict replace dictionaryValue ?key value ...?` — the dict value
    // itself makes the *total* count odd; an even count leaves a
    // trailing key with no value (tclsh 8.6.14: `dict replace $d a` fails
    // "wrong # args").
    assert_eq!(
        arity_shape_codes("dict replace $d a\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn dict_replace_odd_tail_is_silent() {
    assert_eq!(arity_shape_codes("dict replace $d\n"), Vec::<String>::new());
    assert_eq!(
        arity_shape_codes("dict replace $d a b\n"),
        Vec::<String>::new()
    );
}

#[test]
fn dict_update_odd_total_fires_e005() {
    // `dict update dictVar key varName ?key varName ...? body` — total
    // count is always even (dict var + N pairs + body); an odd total
    // means an unpaired key or a missing body (tclsh 8.6.14: `dict update
    // d k v extra body` — 5 words — fails "wrong # args").
    assert_eq!(
        arity_shape_codes("dict update d k v extra body\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn dict_update_even_total_is_silent() {
    assert_eq!(
        arity_shape_codes("dict update d k v body\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        arity_shape_codes("dict update d k1 v1 k2 v2 body\n"),
        Vec::<String>::new()
    );
}

#[test]
fn foreach_unpaired_varlist_fires_e005() {
    // `foreach varList list ?varList list ...? body` — total count is
    // always odd (N varList/list pairs + body); an even total leaves an
    // unpaired trailing var-list with no source list (tclsh 8.6.14:
    // `foreach x $l y {puts $x}` — 4 words — fails "wrong # args").
    assert_eq!(
        arity_shape_codes("foreach x $l y {puts $x}\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn foreach_paired_varlists_are_silent() {
    assert_eq!(
        arity_shape_codes("foreach x $l {puts $x}\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        arity_shape_codes("foreach x $l1 y $l2 {puts \"$x $y\"}\n"),
        Vec::<String>::new()
    );
}

#[test]
fn foreach_expanded_tail_never_false_fires_e005() {
    // FP guard: `{*}`-expanded args make the true final count unknowable
    // — the parity check must abstain exactly like E002 does.
    assert_eq!(
        arity_shape_codes("foreach x $l {*}$rest\n"),
        Vec::<String>::new()
    );
}

#[test]
fn switch_unpaired_pattern_fires_e005() {
    // `switch ?options? string pattern body ?pattern body ...?` — a flat
    // (non-braced) form's total count (subject + patterns/bodies) is
    // always odd; an even count (here 4: subject + 3 more words) leaves
    // an unpaired trailing pattern with no body (tclsh 8.6.14: `switch $s
    // a b c` fails "wrong # args").
    assert_eq!(
        arity_shape_codes("switch $s a b c\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn switch_flat_pairs_are_silent() {
    assert_eq!(
        arity_shape_codes("switch $s a b c d\n"),
        Vec::<String>::new()
    );
}

#[test]
fn switch_single_braced_body_shorthand_is_silent() {
    // The `also_exact` union member: a single braced blob after the
    // subject (exactly 2 total args) is the documented shorthand for any
    // number of pattern/body pairs — never flagged, however many pairs
    // the braced blob's *content* logically holds.
    assert_eq!(
        arity_shape_codes("switch $s {a b c d e f}\n"),
        Vec::<String>::new()
    );
}

#[test]
fn switch_option_skip_does_not_shift_the_parity_check() {
    // Leading declared options (skipped before the parity check applies)
    // must not throw off the count `nargs_min` measures against.
    assert_eq!(
        arity_shape_codes("switch -exact -- $s a b c\n"),
        vec!["E005".to_owned()]
    );
    assert_eq!(
        arity_shape_codes("switch -exact -- $s a b c d\n"),
        Vec::<String>::new()
    );
}

#[test]
fn e005_does_not_double_fire_with_e002_or_e003() {
    // A genuinely too-few / too-many count is E002/E003's job, not
    // E005's — the shape check only applies once the count is already
    // within `[min, max]`.
    assert_eq!(arity_shape_codes("switch $s\n"), vec!["E002".to_owned()]);
    assert_eq!(arity_shape_codes("foreach x\n"), vec!["E002".to_owned()]);
}

// Same-file proc / TclOO forward / `interp alias` / `rename` arity
// (generalises E002/E003 beyond the builtin registry).

fn arity_codes(src: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn irules_arity_accepts_documented_pool_log_and_class_iteration_forms() {
    // Oracle: /Users/jimd/src/bigip-extract/man-21.0.0.1-0.0.13/
    // ltm_rule_command_{pool,log,class}.3. These forms were found in the
    // #1181 corpus by `cargo xtask fp-sweep` (issue #1316).
    for src in [
        "pool /Common/web member 192.0.2.10 443\n",
        "log -noname 192.0.2.20:514 local0.info message\n",
        "class nextelement -index -name -value -element -list -- /Common/users search_id\n",
        "class anymore /Common/users search_id\n",
        "class donesearch /Common/users search_id\n",
    ] {
        assert_eq!(
            arity_codes(src, "f5-irules"),
            Vec::<String>::new(),
            "{src:?}"
        );
    }

    // Keep the upper bounds useful after widening the three valid forms.
    for src in [
        "pool /Common/web member 192.0.2.10 443 extra\n",
        "log 192.0.2.20 local0.info message extra\n",
        "class nextelement /Common/users search_id extra\n",
    ] {
        assert_eq!(
            arity_codes(src, "f5-irules"),
            vec!["E003".to_owned()],
            "{src:?}"
        );
    }
}

#[test]
fn e003_fires_for_same_file_proc_call_too_many_args() {
    // The reported bug: a same-file call to a 7-parameter proc with 8
    // arguments produced no diagnostic at all. `arg8` inside the body
    // correctly fires W210 (undefined variable), but the call site
    // itself must fire E003 too.
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven eight
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected exactly one E003 for the 8-arg call to a 7-param proc, got {:?}",
        r.diagnostics
    );
    assert!(e003[0].message.contains("demonstrate"));
    assert!(e003[0].message.contains("expected at most 7"));
    assert!(e003[0].message.contains("got 8"));
}

#[test]
fn same_file_proc_call_with_correct_arity_is_silent() {
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven
";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "a correctly-arg-counted same-file proc call must not fire E002/E003"
    );
}

#[test]
fn same_file_proc_call_too_few_args_fires_e002() {
    let src = "proc need3 {a b c} {}\nneed3 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_proc_required_after_default_forces_exact_count() {
    // Same regression as `cross_file_arity_required_after_default_forces_exact_count`
    // (tcl-lsp-db), but exercised same-file: `proc opt {a {b 5} c}`
    // accepts exactly 3 arguments per real tclsh 9.0.4, not 2.
    let src = "proc opt {a {b 5} c} {}\nopt 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "2 args must still be too few — min is 3, not 2"
    );
    assert_eq!(
        arity_codes("proc opt {a {b 5} c} {}\nopt 1 2 3\n", "tcl8.6"),
        Vec::<String>::new()
    );
    assert_eq!(
        arity_codes("proc opt {a {b 5} c} {}\nopt 1 2 3 4\n", "tcl8.6"),
        vec!["E003".to_owned()]
    );
}

#[test]
fn same_file_proc_with_trailing_args_is_unbounded() {
    let src = "proc variadic {a args} {}\nvariadic 1 2 3 4 5\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "trailing `args` must accept any number of extra arguments"
    );
    assert_eq!(
        arity_codes("proc variadic {a args} {}\nvariadic\n", "tcl8.6"),
        vec!["E002".to_owned()],
        "the one required parameter before `args` must still be enforced"
    );
}

#[test]
fn after_default_form_braced_callback_is_arity_checked() {
    // `after ms script` appends zero args when it fires the script (unlike a
    // `-command` callback), so the callback is checked like an ordinary
    // in-body call, not the command-prefix mechanism — `cb` needs 2 args and
    // gets none. Body recursion only descends a *braced* word (`analyse_body`'s
    // `TokenType::Str` guard, the same convention `uplevel`'s resolver
    // documents), so the script must be braced here.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 {cb}\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "after's script word must recurse as a real call, not an opaque value"
    );
}

#[test]
fn after_idle_braced_callback_is_arity_checked() {
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter idle {cb}\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "after idle's script word must recurse exactly like after ms's"
    );
}

#[test]
fn after_multi_word_script_concatenation_abstains() {
    // Codex review finding (PR #852): `after ms script script script ...?`
    // concatenates every trailing word into ONE script before evaluating
    // it — confirmed against tclsh 9.0.4 (`after info` shows the
    // registered script as `cb 1 2`, space-joined). Marking only the first
    // word (`{cb}`) as Body would recurse into a truncated fragment and
    // wrongly flag a 2-arg `cb` as under-supplied; must abstain instead.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 {cb} 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "a multi-word after script must abstain, not mis-recurse the first word alone"
    );
    let src_idle = "proc cb {a b} { return [expr {$a+$b}] }\nafter idle {cb} 1 2\n";
    assert_eq!(
        arity_codes(src_idle, "tcl8.6"),
        Vec::<String>::new(),
        "after idle has the identical concatenation shape"
    );
}

#[test]
fn after_default_form_bareword_callback_is_now_arity_checked() {
    // TP — differential-audit finding idx 61 (main audit wave): a
    // *bareword* callback (no braces) is valid Tcl — equally callable, and
    // equally arity-checkable, as a braced one — but was invisible to
    // `command_invocations` entirely (a deliberate, but stale, decision
    // this test used to pin as `after_default_form_bareword_callback_
    // is_not_yet_checked`; its own comment called out that a future
    // change here must be deliberate, not silent — this is that
    // deliberate change): `dispatch_body_arguments` now dispatches a
    // genuinely-static bareword body (`Esc`-kind, single word, no `$`/`[`)
    // through the ordinary `process_command` path, so it gets full call
    // treatment, arity checking included, exactly like a braced one.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 cb\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_string()]);
}

#[test]
fn after_default_form_bareword_callback_correct_arity_is_silent() {
    // TN sibling — a bareword callback with the RIGHT arity (no extra
    // args needed) must not falsely fire E002.
    let src = "proc cb {} { return 1 }\nafter 1000 cb\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn after_correct_arity_callback_is_silent() {
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 {cb 1 2}\nafter idle {cb 3 4}\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn after_cancel_argument_is_not_arity_checked_as_a_callback() {
    // `after cancel` takes an id (or a script used only for *matching*, never
    // executed) — its argument must not be treated as a Body/call.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter cancel cb\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "after cancel's argument identifies a pending callback; it is never invoked"
    );
}

#[test]
fn same_file_args_not_last_is_an_ordinary_required_name() {
    // `args` only collects extra arguments when it's the *last*
    // parameter; here it's an ordinary required name, so the proc has
    // exact arity 3.
    let src = "proc f {args a b} {}\nf 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "args-not-last must not be treated as variadic"
    );
    assert_eq!(
        arity_codes("proc f {args a b} {}\nf 1 2 3\n", "tcl8.6"),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_interp_alias_arity_is_shifted_by_prepended_args() {
    // tclsh 9.0.4: `interp alias {} shortcut {} target 100` requires
    // exactly 2 more arguments at the `shortcut` call site (target's
    // arity of 3, minus the 1 prepended argument).
    let src = "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "shortcut needs 2 more args, only 1 given"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2 3\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_chained_alias_accumulates_prepended_shift() {
    // tclsh 9.0.4: `step2` (alias for `step1` with one prepended arg,
    // itself an alias for the 2-param `real`) needs exactly 1 more
    // argument.
    let src = "\
proc real {a b} {}
interp alias {} step1 {} real
interp alias {} step2 {} step1 9
step2
";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
    assert_eq!(
        arity_codes(
            "proc real {a b} {}\ninterp alias {} step1 {} real\ninterp alias {} step2 {} step1 9\nstep2 1\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_static_rename_inherits_original_arity() {
    // `rename` is a pure name move — `target_orig` must be checked
    // against the exact same arity as `target` always had.
    let src = "proc target {a b c} {}\nrename target target_orig\ntarget_orig 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "target_orig must still require exactly 3 arguments"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b c} {}\nrename target target_orig\ntarget_orig 1 2 3\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_rename_reestablished_after_deletion_checks_new_arity() {
    // `rename target {}` deletes `target` outright, but a fresh `proc
    // target` afterwards re-establishes the name with its own (here,
    // different) arity — confirmed against tclsh 9.0.4: `proc target {a
    // b} {}; rename target {}; proc target {a b c d} {}; target 1 2 3 4`
    // succeeds, and `target 1 2` fails "wrong # args" against the *new*
    // 4-arg signature, not the deleted 2-arg one. Before the timestamp
    // compare (`fact_superseded_by_deletion`), the single stored
    // `deleted_commands["::target"]` offset made every later call look
    // permanently dead, silently dropping this diagnostic (FN).
    let src = "proc target {a b} {}\nrename target {}\nproc target {a b c d} {}\ntarget 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "target was re-established with 4 params after its deletion"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b} {}\nrename target {}\nproc target {a b c d} {}\ntarget 1 2 3 4\n",
            "tcl8.6"
        ),
        Vec::<String>::new(),
        "4 args satisfy the re-established signature"
    );
    // Still correctly dead when there is no re-establishment at all.
    assert_eq!(
        arity_codes(
            "proc target {a b} {}\nrename target {}\ntarget 1 2\n",
            "tcl8.6"
        ),
        Vec::<String>::new(),
        "no re-establishment — target stays permanently deleted (TN)"
    );
}

// Issue #1007 — a `rename` / `interp alias` deletion recorded *inside* a
// proc body that's never called is conditional: it may never execute, so
// it must not supersede a fact established outside that body.
// `fact_superseded_by_deletion` previously reused `fact_in_effect`
// (call-site order-gating) to also decide whether the *deletion itself*
// was in effect, which only asks whether the call is order-gated against
// its own top-level/body status — never whether the deletion's own
// offset sits inside a different, possibly-never-invoked body. Fixed by
// adding the same `offset_is_inside_any_definition_body` guard the W123
// pass's `fact_live_for_call` already applies for the identical question
// (issue #973). All cases confirmed against tclsh 8.6.14.

#[test]
fn e003_fp_issue_1007_conditional_deletion_never_triggered_proc_stays_live() {
    let src = "proc target {a b} {}\nproc maybeDelete {} { rename target {} }\ntarget 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn e003_tp_issue_1007_conditional_deletion_never_triggered_still_checks_arity() {
    // `maybeDelete` is never called, so `target` keeps its original 2-arg
    // signature — an over-applied call must still fire E003, not
    // silently abstain as if the name were permanently gone.
    let src = "proc target {a b} {}\nproc maybeDelete {} { rename target {} }\ntarget 1 2 3\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn e003_tn_issue_1007_unconditional_deletion_before_call_stays_dead() {
    // Regression guard: an *unconditional* (top-level) deletion must
    // still permanently kill the name — the fix only exempts deletions
    // recorded inside a body.
    let src = "proc target {a b} {}\nrename target {}\ntarget 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn e003_tp_issue_1007_call_before_later_unconditional_deletion_still_checked() {
    // Regression guard: the already-correct call-site order gating for a
    // top-level, unconditional deletion is untouched by this fix.
    let src = "proc target {a b} {}\ntarget 1 2 3\nrename target {}\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn e003_tp_issue_1007_alias_conditional_deletion_never_triggered_still_checks_arity() {
    // Same fix, `interp alias` deletion form.
    let src = "proc target {a b} {}\ninterp alias {} short {} target\nproc maybeDelete {} { interp alias {} short {} }\nshort 1 2 3\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn same_file_rename_target_reestablished_after_further_rename_checks_new_arity() {
    // `rename target target_orig` moves `target`'s identity onward (like
    // `same_file_static_rename_inherits_original_arity`), but a *fresh*
    // `proc target` written afterwards re-establishes `target` itself as
    // a brand-new, independent command — it must be checked against its
    // own arity, not treated as still shadowed by the earlier rename.
    let src = "proc target {a b} {}\nrename target target_orig\nproc target {x} {}\ntarget 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E003".to_owned()],
        "the re-established target only takes 1 argument"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b} {}\nrename target target_orig\nproc target {x} {}\ntarget 1\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_alias_reestablished_after_deletion_checks_new_target_arity() {
    // `interp alias {} short {} target` then `interp alias {} short {}
    // {}` deletes the alias outright (tclsh 9.0.4: `short` then fails
    // "invalid command name"); a fresh `interp alias {} short {}
    // target2` afterwards re-establishes `short` against a
    // differently-aritied target — must be checked against `target2`,
    // not silently dropped as still-deleted.
    let src = "\
proc target {a b} {}
proc target2 {a b c} {}
interp alias {} short {} target
interp alias {} short {} {}
interp alias {} short {} target2
short 1 2
";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "short now aliases target2, which needs 3 arguments"
    );
    let ok = src.replace("short 1 2\n", "short 1 2 3\n");
    assert_eq!(arity_codes(&ok, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_dynamic_but_resolvable_rename_target_checks_arity() {
    // `$newname` is dynamic-*looking* but a known constant (`set newname
    // target_orig`) — issue #923 idx 3's constant-folding fix now
    // resolves it, so `target_orig` correctly inherits `target`'s arity
    // (a rename is a pure name move, never an arity change) and a call
    // with too few arguments is caught exactly like the fully-literal
    // `same_file_static_rename_inherits_original_arity` case. Before
    // that fix this whole rename was untracked (dynamic per a naive
    // `$`-in-the-text check), so `target_orig 1` was silently missed.
    let src =
        "proc target {a b c} {}\nset newname target_orig\nrename target $newname\ntarget_orig 1\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "newname's value is a known constant, so this rename must resolve and check arity"
    );
    let ok = src.replace("target_orig 1\n", "target_orig 1 2 3\n");
    assert_eq!(arity_codes(&ok, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_genuinely_dynamic_rename_target_does_not_false_positive() {
    // Unlike the sibling test above, `newname`'s value here is never a
    // tracked compile-time constant (piped through `gets`) — the rename
    // stays genuinely unresolvable and must never invent a diagnostic.
    let src =
        "proc target {a b c} {}\nset newname [gets stdin]\nrename target $newname\ntarget_orig 1\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "a genuinely dynamic rename target must not be resolved to target's arity"
    );
}

// The following regression tests were added in response to a code review
// (all verified against tclsh 9.0.4).

#[test]
fn same_file_call_to_renamed_away_name_does_not_false_positive() {
    // `rename target target_orig` removes `target` as a command entirely
    // (tclsh 9.0.4: calling it afterwards fails "invalid command name",
    // not a "wrong # args" against its original 2-arg signature) — a
    // call to the old name must abstain, not be checked against the
    // proc it used to denote.
    let src = "proc target {a b} {}\nrename target target_orig\ntarget 1\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "target is gone after the rename; must not fire on its old arity"
    );
    // A call to `target` *before* the rename executes is unaffected.
    let before = "proc target {a b} {}\ntarget 1\nrename target target_orig\n";
    assert_eq!(
        arity_codes(before, "tcl8.6"),
        vec!["E002".to_owned()],
        "target is still callable before the rename runs"
    );
}

#[test]
fn same_file_call_to_deleted_command_does_not_false_positive() {
    // `rename target {}` deletes `target` outright (tclsh 9.0.4: also
    // "invalid command name" afterwards) — same abstention as a rename
    // to a new name.
    let src = "proc target {a b} {}\nrename target {}\ntarget 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_chained_rename_still_resolves_original_arity() {
    // Chasing `c -> b -> a` through two renames still reaches `a`'s own
    // 2-arg signature, even though both `a` and `b` are themselves
    // recorded as "deleted" by the renames that moved them onward —
    // confirmed against tclsh 9.0.4.
    let src = "proc a {x y} {}\nrename a b\nrename b c\nc 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
    assert_eq!(
        arity_codes("proc a {x y} {}\nrename a b\nrename b c\nc 1 2\n", "tcl8.6"),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_top_level_call_before_alias_established_does_not_false_positive() {
    // `shortcut` doesn't exist yet at the point it's called — the
    // `interp alias` statement establishing it hasn't executed (tclsh
    // 9.0.4: fails "invalid command name", not an arity error against
    // the eventual alias target). Order matters at top level, unlike
    // inside a proc body.
    let src = "proc target {a b} {}\nshortcut 1\ninterp alias {} shortcut {} target\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "shortcut doesn't exist yet when called"
    );
    // Calling it after the alias is established is checked normally.
    let after = "proc target {a b} {}\ninterp alias {} shortcut {} target\nshortcut 1\n";
    assert_eq!(arity_codes(after, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_proc_body_call_to_later_alias_is_not_order_gated() {
    // Inside a proc body, the alias is established by the time the body
    // ever runs (the whole file loads first) regardless of where the
    // `interp alias` statement sits textually — order-gating would be
    // unsound here, unlike at top level.
    let src =
        "proc target {a b} {}\nproc caller {} { shortcut 1 }\ninterp alias {} shortcut {} target\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_alias_target_renamed_away_does_not_false_positive() {
    // An `interp alias` target is re-resolved *by name* on every call —
    // once that name is gone (renamed or deleted), the alias breaks too
    // (tclsh 9.0.4: `interp alias {} bar {} foo` then `rename foo baz` —
    // or `rename foo {}` — makes `bar` fail "invalid command name foo").
    // Unlike a rename-chase hop, an alias hop must still respect the
    // target's own deletion.
    let renamed_away = "proc foo {a} {}\ninterp alias {} bar {} foo\nrename foo baz\nbar 1\n";
    assert_eq!(arity_codes(renamed_away, "tcl8.6"), Vec::<String>::new());
    let deleted = "proc foo {a} {}\ninterp alias {} bar {} foo\nrename foo {}\nbar 1\n";
    assert_eq!(arity_codes(deleted, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_alias_to_renamed_target_still_resolves() {
    // By contrast, a rename that merely *moves* the target (not deletes
    // it) leaves any alias created against the new name working
    // normally — confirmed against tclsh 9.0.4.
    let src = "proc foo {a} {}\nrename foo bar\ninterp alias {} baz {} bar\nbaz 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn same_file_over_applied_alias_always_flags_regardless_of_argcount() {
    // `target` takes exactly 1 argument, but the alias already bakes in
    // 2 prepended words — every call fails at run time no matter how
    // many further arguments are supplied (confirmed against tclsh
    // 9.0.4: `bad`, `bad x`, and `bad x y` all fail "wrong # args").
    // Must never silently accept a zero-argument call as valid.
    let src = |call: &str| {
        format!("proc target {{a}} {{}}\ninterp alias {{}} bad {{}} target fixed extra\n{call}\n")
    };
    for call in ["bad", "bad x", "bad x y"] {
        let codes = arity_codes(&src(call), "tcl8.6");
        assert!(
            !codes.is_empty(),
            "over-applied alias must always flag '{call}', got {codes:?}"
        );
    }
}

#[test]
fn same_file_deleted_alias_call_does_not_false_positive() {
    // `interp alias {} bar {}` (target path present, target command
    // absent) deletes a previously-created alias — confirmed against
    // tclsh 9.0.4: a later `bar` call fails "invalid command name", not
    // a "wrong # args" against the alias's stale target arity. Must
    // abstain, not misdiagnose the failure as an arity mismatch.
    let src = "proc foo {a b} {}\ninterp alias {} bar {} foo\ninterp alias {} bar {}\nbar 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_deleted_alias_call_inside_proc_body_does_not_false_positive() {
    // Deletion is not order-gated inside a proc body, same convention as
    // every other fact here — the deletion is unconditionally in effect
    // by the time any proc body runs.
    let src = "\
proc foo {a b} {}
interp alias {} bar {} foo
interp alias {} bar {}
proc use {} { bar 1 }
";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_deleted_alias_call_is_unknown_command() {
    // Regression: the arity resolver already abstains for a call through a
    // deleted alias (`same_file_deleted_alias_call_does_not_false_positive`
    // above), but `command_aliases` itself was never pruned on deletion, so
    // W123 ("unknown command") still treated the deleted name as known —
    // the call went through completely unchecked, neither an arity
    // diagnostic nor an unknown-command one. Confirmed against tclsh 9.0.4:
    // a call through a deleted alias fails "invalid command name".
    let mut a = Analyser::new();
    let src = "interp alias {} bar {} puts\ninterp alias {} bar {}\nbar 1\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a call through a deleted alias must be flagged unknown; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn same_file_redeclared_alias_after_deletion_is_still_known() {
    // The inverse of the regression above: a name deleted and then
    // re-declared later in the file must stay known — the re-declaration
    // wins, exactly as `command_aliases`'s last-write-wins map already
    // implies for arity resolution.
    let mut a = Analyser::new();
    let src = "interp alias {} bar {} puts\ninterp alias {} bar {}\ninterp alias {} bar {} format\nbar 1\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a re-declared alias must not be flagged unknown; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn same_file_alias_query_form_is_not_a_deletion() {
    // `interp alias srcPath srcCmd` (no target path at all — 2 args
    // after `alias`) is a *query*, not a deletion; the alias must keep
    // resolving normally afterwards.
    let src = "proc foo {a b} {}\ninterp alias {} bar {} foo\ninterp alias {} bar\nbar 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_relative_qualified_call_resolves_against_current_namespace() {
    // `inner::p` is qualified but not absolute — Tcl resolves it against
    // the *current* namespace first, not global (confirmed against
    // tclsh 9.0.4: called from inside `namespace eval ::ns { … }`, it
    // reaches `::ns::inner::p`, not `::inner::p`).
    let src = "\
namespace eval ::ns {
    namespace eval inner {
        proc p {a b} {}
    }
    inner::p 1
}
";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
    let ok_src = "\
namespace eval ::ns {
    namespace eval inner {
        proc p {a b} {}
    }
    inner::p 1 2
}
";
    assert_eq!(arity_codes(ok_src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_relative_qualified_call_prefers_current_namespace_over_global() {
    // When both `::inner::p` (global) and `::ns::inner::p` (nested)
    // exist, a relative `inner::p` call from inside `::ns` resolves to
    // the nested one — confirmed against tclsh 9.0.4.
    let src = "\
proc ::inner::p {} {}
namespace eval ::ns {
    namespace eval inner {
        proc p {a b} {}
    }
    inner::p 1 2 3
}
";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E003".to_owned()],
        "must resolve against ::ns::inner::p (2 args), not ::inner::p (0 args)"
    );
}

// BODY role on iRules nesting scripts

#[test]
fn analyser_recurses_into_irules_nesting_script_bodies() {
    // clientside / serverside / peer / after now carry an
    // `ArgRole::Body`, so the analyser descends into the nesting
    // script and flags problems inside it.  A nested `set` with no
    // arguments trips E002 only when the body is actually analysed —
    // i.e. the generic body-walk picks the role up automatically.
    for src in [
        "when CLIENT_DATA { clientside { set } }",
        "when CLIENT_ACCEPTED { serverside { set } }",
        "when CLIENT_ACCEPTED { peer { set } }",
        "when RULE_INIT { after 1000 { set } }",
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "f5-irules");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E002 && d.message.contains("'set'")),
            "expected E002 from the nested `set` (body must be analysed) in {src:?}, got {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn w004_fires_on_lsearch_stride_in_tcl85() {
    // The W004 coverage requires the
    // option to exist in the registry.  `lsearch -stride` is
    // populated there.
    let mut a = Analyser::new();
    let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.5");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.5 lsearch -stride, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_version_gated_list_options() {
    // Options added after 8.4 must warn (W004) under an older dialect and stay
    // silent once available.
    let cases = [
        // (snippet, introduced-in dialect, a dialect that predates it)
        ("lsearch -index 0 {a b} x", "tcl8.5", "tcl8.4"), // -index: 8.5
        ("lsearch -nocase {a b} x", "tcl8.5", "tcl8.4"),  // -nocase: 8.5
        ("lsearch -bisect {a b} x", "tcl8.6", "tcl8.5"),  // -bisect: 8.6
        ("lsort -nocase {a b}", "tcl8.5", "tcl8.4"),      // -nocase: 8.5
        ("lsort -indices {a b}", "tcl8.5", "tcl8.4"),     // -indices: 8.5
        ("lsort -stride 2 {a b}", "tcl8.6", "tcl8.5"),    // -stride: 8.6
    ];
    for (snippet, ok_dialect, old_dialect) in cases {
        let mut a = Analyser::new();
        assert!(
            a.analyse(snippet, old_dialect)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W004),
            "expected W004 for {snippet:?} on {old_dialect}"
        );
        let mut a = Analyser::new();
        assert!(
            !a.analyse(snippet, ok_dialect)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W004),
            "unexpected W004 for {snippet:?} on {ok_dialect}"
        );
    }
}

#[test]
fn w004_fires_on_lsearch_stride_in_tcl86() {
    // `lsearch -stride` is Tcl 9.0-only — tclsh8.6.14 rejects it with
    // `bad option "-stride"` (the 8.6 / TIP 351 `-stride` belongs to `lsort`),
    // so W004 fires on tcl8.6 just as it does on tcl8.5 above.
    let mut a = Analyser::new();
    let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 lsearch -stride (9.0-only), got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_clock_scan_validate_in_tcl86() {
    // `clock scan -validate` is Tcl 9.0+ (TIP 532); the
    // subcommand-scoped option table consults the active
    // dialect via the W004 emitter's `sub_match` branch.
    let mut a = Analyser::new();
    let result = a.analyse("clock scan {today} -validate 1", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 clock scan -validate, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_fconfigure_nodelay_in_tcl86() {
    // `fconfigure -nodelay` is Tcl 9.0+ (TIP 528).
    let mut a = Analyser::new();
    let result = a.analyse("fconfigure $chan -nodelay 1", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 fconfigure -nodelay, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_chan_configure_inputmode_in_tcl86() {
    // Subcommand-scoped option: `chan configure -inputmode` is
    // Tcl 9.0+ (TIP 160).
    let mut a = Analyser::new();
    let result = a.analyse("chan configure $chan -inputmode raw", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 chan configure -inputmode, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_silent_on_regsub_command_in_tcl9() {
    // Same input on Tcl 9.0 — option is supported, no W004.
    let mut a = Analyser::new();
    let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl9.0");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "W004 should not fire on tcl9.0, got {:?}",
        result.diagnostics
    );
}

// --- Shadow suppression: a same-file proc / alias really is what gets
// called, so the registry builtin's dialect-restricted option no longer
// applies. Mirrors the E002/E003 arity suppression exactly (same queue,
// same resolution order).

#[test]
fn w004_suppressed_when_shadowed_by_user_proc_before_call() {
    let mut a = Analyser::new();
    let src = "proc lsearch {l args} { return $l }\nlsearch -stride 2 {a b c d}\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "a user proc shadowing lsearch should suppress the builtin's W004, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_still_fires_when_shadowing_proc_defined_after_top_level_call() {
    // Top-level calls run in source order during load; a proc defined
    // *after* this call hasn't shadowed it yet, so the builtin (and its
    // W004) is still in effect — same order-gating as arity.
    let mut a = Analyser::new();
    let src = "lsearch -stride 2 {a b c d}\nproc lsearch {l args} { return $l }\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004: the builtin is still in effect before the shadowing proc is defined, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_suppressed_when_shadowing_proc_defined_after_call_inside_proc_body() {
    // Inside a proc body, the whole file has already loaded by the time the
    // body runs, so a later top-level proc definition still shadows.
    let mut a = Analyser::new();
    let src =
        "proc caller {} { lsearch -stride 2 {a b c d} }\nproc lsearch {l args} { return $l }\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "a proc-body call resolves after full-file load, so the later proc \
definition should still suppress W004, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_suppressed_when_shadowed_by_namespaced_proc_called_unqualified() {
    // The shadowing proc is `::myns::lsearch`; the call inside the same
    // namespace resolves current-namespace-first, exactly like arity's
    // namespace resolution — not just a global `::lsearch` check.
    let mut a = Analyser::new();
    let src = "namespace eval myns {\n    proc lsearch {l args} { return $l }\n    lsearch -stride 2 {a b c d}\n}\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "a namespaced proc shadowing lsearch, called unqualified inside its \
own namespace, should suppress W004, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_suppressed_when_command_aliased_to_user_proc() {
    let mut a = Analyser::new();
    let src = "proc mylsearch {l args} { return $l }\ninterp alias {} lsearch {} mylsearch\nlsearch -stride 2 {a b c d}\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "lsearch aliased to a user proc should suppress the builtin's W004, got {:?}",
        result.diagnostics
    );
}

// --- Abbreviated-subcommand resolution: W004 now shares the registry's
// unique-prefix subcommand resolver instead of a hand-rolled exact-name
// match, so a legal Tcl ensemble abbreviation is still checked.

#[test]
fn w004_fires_on_abbreviated_chan_configure_inputmode() {
    // `configure` is `chan`'s only subcommand starting with `conf`, so real
    // Tcl ensemble dispatch accepts the abbreviation.
    let mut a = Analyser::new();
    let result = a.analyse("chan conf $chan -inputmode raw", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on the abbreviated 'chan conf -inputmode', got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_abstains_on_dynamic_subcommand() {
    for snippet in [
        "chan $sub -inputmode raw $chan",
        "chan [x] -inputmode raw $chan",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
            "unexpected W004 for dynamic subcommand {snippet:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn w004_abstains_on_expanded_subcommand_word() {
    let mut a = Analyser::new();
    let result = a.analyse("chan {*}$sub -inputmode raw $chan", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "unexpected W004 for a `{{*}}`-expanded subcommand word: {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_abstains_on_ambiguous_subcommand_prefix() {
    // `p` is ambiguous between `pending` / `pipe` / `pop` / `push` / `puts`
    // on `chan` — resolution must abstain rather than guess (and rather
    // than falling back to `chan`'s own unrelated top-level option table).
    let mut a = Analyser::new();
    let result = a.analyse("chan p $chan -inputmode raw", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "unexpected W004 for an ambiguous subcommand prefix: {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_abstains_on_unknown_subcommand_instead_of_scanning_parent_options() {
    let mut a = Analyser::new();
    let result = a.analyse("chan bogus $chan -inputmode raw", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "unexpected W004 for an unknown chan subcommand: {:?}",
        result.diagnostics
    );
}

// --- Quick fix: "Remove '-option'" deletes the flag and its value word(s).

#[test]
fn w004_fix_removes_option_and_its_value() {
    let mut a = Analyser::new();
    let src = "lsearch -stride 2 {a b c d} b";
    let result = a.analyse(src, "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert_eq!(w004.len(), 1, "{:?}", result.diagnostics);
    let fix = w004[0]
        .fixes
        .first()
        .expect("W004 should carry a remove-option fix");
    assert_eq!(fix.new_text, "");
    assert!(fix.description.contains("-stride"), "{:?}", fix.description);
    let mut applied = src.to_string();
    applied.replace_range(fix.span.start() as usize..fix.span.end() as usize, "");
    assert_eq!(applied, "lsearch {a b c d} b");
}

#[test]
fn w004_fix_removes_option_and_value_at_end_of_command() {
    let mut a = Analyser::new();
    let src = "lsearch -stride 2";
    let result = a.analyse(src, "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert_eq!(w004.len(), 1, "{:?}", result.diagnostics);
    let fix = w004[0].fixes.first().expect("fix");
    let mut applied = src.to_string();
    applied.replace_range(fix.span.start() as usize..fix.span.end() as usize, "");
    // No following argument to extend through, so one separator remains
    // before the deleted range — cosmetic only (Tcl treats runs of
    // whitespace between words identically).
    assert_eq!(applied.trim_end(), "lsearch");
}

#[test]
fn w004_fix_handles_braced_option_token_without_stray_closer() {
    // A braced flag/value (`{-stride}`) is a legal, if unusual, way to write
    // the same word; the fix must delete the whole wrapped token — never
    // leaving a stray `}` behind (kcs-issue-highlight-drops-closing-delimiter).
    let mut a = Analyser::new();
    let src = "lsearch {-stride} 2 {a b} x";
    let result = a.analyse(src, "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert_eq!(w004.len(), 1, "{:?}", result.diagnostics);
    let fix = w004[0].fixes.first().expect("fix");
    let mut applied = src.to_string();
    applied.replace_range(fix.span.start() as usize..fix.span.end() as usize, "");
    assert_eq!(applied, "lsearch {a b} x");
    assert_eq!(
        applied.matches('{').count(),
        applied.matches('}').count(),
        "braces must stay balanced: {applied:?}"
    );
}

#[test]
fn w003_fires_on_string_compare_in_tcl84() {
    // `lt` / `le` / `gt` / `ge` are Tcl 9.0+ (TIP 461); on
    // Tcl 8.4 / 8.5 / 8.6 they should produce W003.
    let mut a = Analyser::new();
    let result = a.analyse("if {$x lt $y} { puts hi }", "tcl8.4");
    let w003: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W003)
        .collect();
    assert!(
        !w003.is_empty(),
        "expected W003 on tcl8.4 'lt' operator, got {:?}",
        result.diagnostics
    );
    assert!(w003[0].message.contains("'lt'"));
}

#[test]
fn w003_silent_on_string_compare_in_tcl9() {
    let mut a = Analyser::new();
    let result = a.analyse("if {$x lt $y} { puts hi }", "tcl9.0");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 should not fire on tcl9.0, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w003_fires_on_in_operator_in_tcl84() {
    // `in` / `ni` are Tcl 8.5+ (TIP 201).
    let mut a = Analyser::new();
    let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.4");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "expected W003 on tcl8.4 'in' operator, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w003_fires_on_exponentiation_operator_in_tcl84() {
    // FN fix: `**` (exponentiation) is Tcl 8.5+ (TIP 123) — a symbolic
    // operator the word-shaped gated set used to miss entirely.
    let has_w003 = |src: &str, d: &str| {
        Analyser::new()
            .analyse(src, d)
            .diagnostics
            .iter()
            .any(|x| x.code == DiagCode::W003)
    };
    assert!(has_w003("expr {2 ** 3}", "tcl8.4"));
    // Available from 8.5 onward — silent.
    assert!(!has_w003("expr {2 ** 3}", "tcl8.6"));
    assert!(!has_w003("expr {2 ** 3}", "tcl9.0"));
    // Message cites the operator and its TIP.
    let d = Analyser::new().analyse("expr {2 ** 3}", "tcl8.4");
    assert!(
        d.diagnostics.iter().any(|x| x.code == DiagCode::W003
            && x.message.contains("'**'")
            && x.message.contains("TIP 123")),
        "message must cite `**` + TIP 123; got {:?}",
        d.diagnostics,
    );
}

#[test]
fn w003_fires_on_tab_separated_operator() {
    // The prefilter must tolerate any
    // whitespace, not just literal spaces.  `if {$x\tlt\t$y}` is
    // valid Tcl 8.4 syntax that the expr parser handles — the
    // analyser must not skip it because we only checked for
    // space-delimited operators.
    let mut a = Analyser::new();
    let result = a.analyse("if {$x\tlt\t$y} { puts hi }", "tcl8.4");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 must fire on tab-separated 'lt', got {:?}",
        result.diagnostics
    );
}

#[test]
fn w003_fires_on_newline_separated_operator() {
    // Same shape with a newline boundary — also valid Tcl.
    let mut a = Analyser::new();
    let result = a.analyse("if {$x\nin\n{a b c}} { puts hi }", "tcl8.4");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 must fire on newline-separated 'in', got {:?}",
        result.diagnostics
    );
}

#[test]
fn contains_gated_word_handles_boundaries() {
    // No false positives on identifiers that contain the keyword.
    assert!(!contains_gated_word("$alt"));
    assert!(!contains_gated_word("$align"));
    assert!(!contains_gated_word("inner"));
    assert!(!contains_gated_word("$gem"));
    // Real matches at word boundaries.
    assert!(contains_gated_word("$x lt $y"));
    assert!(contains_gated_word("$x\tlt\t$y"));
    assert!(contains_gated_word("($x)lt($y)"));
    assert!(contains_gated_word("lt $y"));
    assert!(contains_gated_word("$x lt"));
}

#[test]
fn w003_silent_on_in_operator_in_tcl85() {
    let mut a = Analyser::new();
    let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.5");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 should not fire on tcl8.5, got {:?}",
        result.diagnostics
    );
}

/// Every W003 diagnostic for `src` analysed under `dialect`, alongside
/// the exact source substring its span covers — the tight-highlight
/// assertions below check that text directly rather than trusting
/// hand-computed byte offsets.
fn w003_hits(src: &str, dialect: &str) -> Vec<(String, Diagnostic)> {
    let mut a = Analyser::new();
    let result = a.analyse(src, dialect);
    result
        .diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W003)
        .map(|d| {
            let text = src[d.span.start() as usize..d.span.end() as usize].to_string();
            (text, d)
        })
        .collect()
}

#[test]
fn w003_tight_span_covers_only_the_operator_in_a_braced_if() {
    // The whole condition is `$x lt $y` (11 chars); the diagnostic must
    // highlight just the 2-byte `lt`, not the condition or the `if`.
    let hits = w003_hits("if {$x lt $y} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "lt");
}

#[test]
fn w003_tight_span_covers_only_the_operator_in_bare_expr() {
    let hits = w003_hits("expr {2 in {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "in");
}

#[test]
fn w003_distinct_operators_each_get_their_own_tight_span() {
    // Two *different* gated operators in one expression used to collapse
    // onto one coarse diagnostic covering the whole condition; each must
    // now get its own diagnostic at its own span.
    let hits = w003_hits("if {$a lt $b && $c in $d} { puts hi }", "tcl8.4");
    let mut texts: Vec<&str> = hits.iter().map(|(t, _)| t.as_str()).collect();
    texts.sort_unstable();
    assert_eq!(texts, vec!["in", "lt"], "{hits:?}");
    // The two spans must not overlap.
    assert_ne!(hits[0].1.span, hits[1].1.span);
}

#[test]
fn w003_repeated_same_operator_gets_a_diagnostic_per_occurrence() {
    let hits = w003_hits("if {$a in $b && $c in $d} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 2, "{hits:?}");
    assert_eq!(hits[0].0, "in");
    assert_eq!(hits[1].0, "in");
    // Same text, but must anchor at two different source positions.
    assert_ne!(hits[0].1.span.start(), hits[1].1.span.start());
}

#[test]
fn w003_message_cites_the_relevant_tip() {
    let hits = w003_hits("expr {2 in {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1);
    assert!(
        hits[0].1.message.contains("TIP 201"),
        "{}",
        hits[0].1.message
    );
    let hits = w003_hits("if {$x lt $y} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 1);
    assert!(
        hits[0].1.message.contains("TIP 461"),
        "{}",
        hits[0].1.message
    );
}

#[test]
fn w003_fires_on_all_six_gated_operators_pre_availability() {
    // `ni`/`le`/`gt`/`ge` were only ever exercised indirectly before
    // (only `lt` and `in` had a dedicated test); cover the full set.
    for (src, op) in [
        ("if {$x ni {a b c}} { puts hi }", "ni"),
        ("if {$x le $y} { puts hi }", "le"),
        ("if {$x gt $y} { puts hi }", "gt"),
        ("if {$x ge $y} { puts hi }", "ge"),
    ] {
        let hits = w003_hits(src, "tcl8.4");
        assert_eq!(hits.len(), 1, "{src}: {hits:?}");
        assert_eq!(hits[0].0, op, "{src}");
    }
}

#[test]
fn w003_silent_on_ni_le_gt_ge_when_dialect_supports_them() {
    // `ni` only needs 8.5+; `le`/`gt`/`ge` need 9.0+.
    assert!(w003_hits("if {$x ni {a b c}} { puts hi }", "tcl8.5").is_empty());
    for src in [
        "if {$x le $y} { puts hi }",
        "if {$x gt $y} { puts hi }",
        "if {$x ge $y} { puts hi }",
    ] {
        assert!(w003_hits(src, "tcl9.0").is_empty(), "{src}");
        assert!(w003_hits(src, "tcl8.5").len() == 1, "{src}");
    }
}

#[test]
fn w003_fires_on_unbraced_multiword_expr_at_a_tight_span() {
    // `expr` is the only EXPR-role command that accepts an unbraced,
    // multi-word expression; the gated keyword is its own Tcl word, so
    // this exercises the separate `emit_w003_dialect_invalid_expr_words`
    // path rather than the single-argument one above.
    let hits = w003_hits("expr $a in $b", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "in");
    // No fix is offered for this shape (see doc comment on the emitter).
    assert!(hits[0].1.fixes.is_empty());
}

#[test]
fn w003_offers_lsearch_fix_for_in() {
    let hits = w003_hits("expr {2 in {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1);
    let fixes = &hits[0].1.fixes;
    assert_eq!(fixes.len(), 1, "{fixes:?}");
    assert_eq!(fixes[0].new_text, "([lsearch -exact {1 2 3} 2] >= 0)");
}

#[test]
fn w003_offers_lsearch_fix_for_ni() {
    let hits = w003_hits("expr {2 ni {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1);
    let fixes = &hits[0].1.fixes;
    assert_eq!(fixes.len(), 1, "{fixes:?}");
    assert_eq!(fixes[0].new_text, "([lsearch -exact {1 2 3} 2] < 0)");
}

#[test]
fn w003_offers_string_compare_fix_for_string_relational_ops() {
    for (src, expect) in [
        ("if {$x lt $y} { puts hi }", "([string compare $x $y] < 0)"),
        ("if {$x le $y} { puts hi }", "([string compare $x $y] <= 0)"),
        ("if {$x gt $y} { puts hi }", "([string compare $x $y] > 0)"),
        ("if {$x ge $y} { puts hi }", "([string compare $x $y] >= 0)"),
    ] {
        let hits = w003_hits(src, "tcl8.4");
        assert_eq!(hits.len(), 1, "{src}");
        let fixes = &hits[0].1.fixes;
        assert_eq!(fixes.len(), 1, "{src}: {fixes:?}");
        assert_eq!(fixes[0].new_text, expect, "{src}");
    }
}

#[test]
fn w003_fix_span_replaces_exactly_the_gated_application() {
    let src = "expr {2 in {1 2 3}}";
    let hits = w003_hits(src, "tcl8.4");
    let fix = &hits[0].1.fixes[0];
    assert_eq!(
        &src[fix.span.start() as usize..fix.span.end() as usize],
        "2 in {1 2 3}"
    );
}

#[test]
fn w003_no_fix_when_operator_nested_in_a_larger_expression() {
    // Only `in` is gated here (`&&` is fine everywhere); it is the sole
    // W003 occurrence, but it is not the *whole* condition — rewriting
    // just the `in` sub-expression while leaving `&& $c` around it is
    // exactly the "nested" shape the fix deliberately declines.
    let hits = w003_hits("if {$a in $b && $c} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].1.fixes.is_empty());
}

#[test]
fn w003_no_fix_when_more_than_one_occurrence() {
    let hits = w003_hits("if {$a lt $b && $c in $d} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 2);
    assert!(hits[0].1.fixes.is_empty());
    assert!(hits[1].1.fixes.is_empty());
}

#[test]
fn w003_no_fix_when_operand_is_a_call() {
    // `max($a, $b)` contains an unprotected space after the comma —
    // splicing its rendered text bare into `lsearch`'s argument list
    // would silently mis-word-split, so `is_simple_operand` excludes
    // `Call` and no fix is offered even though this is the sole,
    // top-level occurrence.
    let hits = w003_hits("expr {max($a, $b) in $list}", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].1.fixes.is_empty());
}

#[test]
fn w003_silent_on_variable_named_like_a_gated_operator() {
    // `$in` is a variable reference, not the `in` operator — the
    // lexical prefilter alone can't tell the two apart, but the real
    // expr parse must.
    assert!(w003_hits("if {$in} { puts hi }", "tcl8.4").is_empty());
    assert!(w003_hits("if {$ni && $lt} { puts hi }", "tcl8.4").is_empty());
}

#[test]
fn w003_silent_on_array_element_named_like_a_gated_operator() {
    assert!(w003_hits("if {$arr(in)} { puts hi }", "tcl8.4").is_empty());
}

#[test]
fn w003_silent_on_quoted_string_literal_operator_word() {
    // `"in"` is a quoted string literal, not the bareword operator.
    let hits = w003_hits(r#"if {"in" eq $x} { puts hi }"#, "tcl8.4");
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn w003_silent_on_malformed_expression() {
    // `lt` with no right-hand operand doesn't parse — `parse_expr`
    // falls back to `Raw`, so W003 must stay silent rather than guess.
    assert!(w003_hits("if {$x lt} { puts hi }", "tcl8.4").is_empty());
}

#[test]
fn w003_dialect_is_file_wide_regardless_of_namespace_or_proc_nesting() {
    // The active dialect is resolved once per file/analysis, not
    // per-scope — a gated operator inside a deeply nested proc body
    // must still be flagged.
    let src = "namespace eval ::foo {\n  proc bar {} {\n    if {$x in $y} { return 1 }\n  }\n}\n";
    let hits = w003_hits(src, "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "in");
}

#[test]
fn w003_fires_inside_a_tcl_oo_method_body() {
    let src = "oo::class create Foo {\n  method bar {} {\n    if {$x in $y} { return 1 }\n  }\n}\n";
    let hits = w003_hits(src, "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "in");
}

#[test]
fn w003_fires_inside_a_sub_interpreter_eval_body() {
    // `interp eval`'s script argument is a registry BODY role, so it is
    // walked like any other nested script. Static analysis can't know
    // the sub-interpreter's own Tcl version, so it reasonably applies
    // the enclosing file's dialect uniformly.
    let src = "interp eval $safeInterp {\n  if {$x in $y} { return 1 }\n}\n";
    let hits = w003_hits(src, "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
}

#[test]
fn w003_suppressed_by_an_earlier_proc_shadowing_if() {
    // A user `proc if {...} {...}` defined *before* the call site
    // resolves at the call site instead of the builtin `::if` — Tcl's
    // own name resolution, mirrored by W002's existing shadow rule and
    // now shared by the EXPR-role dispatch (W100/W110/W003/W114 all at
    // once, via `dispatch_expr_arguments`'s shadow guard).
    let src = "proc if {c b} { return 1 }\nif {$x in $y} { puts hi }\n";
    assert!(
        w003_hits(src, "tcl8.4").is_empty(),
        "shadowed 'if' must suppress W003"
    );
}

#[test]
fn w003_not_suppressed_by_a_later_proc_shadowing_if() {
    // The shadowing proc is defined *after* this call site, so it
    // cannot have been in effect when Tcl resolved this particular
    // call — W003 must still fire here.
    let src = "if {$x in $y} { puts hi }\nproc if {c b} { return 1 }\n";
    assert_eq!(w003_hits(src, "tcl8.4").len(), 1);
}

#[test]
fn w003_correctly_gates_eda_vendor_dialects_by_documented_base_version() {
    // Regression for the registry fix (`DialectSet::expr_grammar_base_version`):
    // these vendor dialects are documented as running on top of a real
    // Tcl 8.5+ core (`docs/design/compiler/dialects-events.md`), so
    // `in`/`ni` (TIP 201, 8.5+) must NOT be flagged for them — the old
    // `DialectSet::TCL85_PLUS` check excluded them entirely and
    // over-fired.
    for dialect in [
        "f5-iapps",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "synopsys-eda-tcl",
        "expect",
    ] {
        assert!(
            w003_hits("expr {2 in {1 2 3}}", dialect).is_empty(),
            "{dialect} should support TIP 201 'in'"
        );
    }
    // None of them reach Tcl 9.0, so the string-relational operators
    // are still correctly flagged.
    for dialect in [
        "f5-iapps",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "synopsys-eda-tcl",
        "expect",
    ] {
        assert_eq!(
            w003_hits("if {$x lt $y} { puts hi }", dialect).len(),
            1,
            "{dialect} should still gate TIP 461 'lt'"
        );
    }
    // Cadence Innovus/Genus run an 8.4-safe core (owner decision), so —
    // like `f5-irules` — TIP 201 `in`/`ni` (8.5+) is *also* out of
    // grammar and must be flagged, not just TIP 461's `lt`/`le`/`gt`/`ge`.
    assert_eq!(
        w003_hits("expr {2 in {1 2 3}}", "cadence-eda-tcl").len(),
        1,
        "cadence-eda-tcl runs an 8.4 core — TIP 201 'in' must gate"
    );
    assert_eq!(
        w003_hits("if {$x lt $y} { puts hi }", "cadence-eda-tcl").len(),
        1,
        "cadence-eda-tcl should still gate TIP 461 'lt'"
    );
}

#[test]
fn w003_f5_irules_stays_gated_on_its_tcl_8_4_runtime() {
    // iRules advertises an 8.6-shaped command *signature* but its
    // runtime `expr` evaluator is a genuine embedded Tcl 8.4.6 — both
    // TIPs must be flagged, unlike the 8.5-base vendor dialects above.
    assert_eq!(w003_hits("expr {2 in {1 2 3}}", "f5-irules").len(), 1);
    assert_eq!(w003_hits("if {$x lt $y} { puts hi }", "f5-irules").len(), 1);
}

#[test]
fn w003_f5_tmsh_now_gates_tip_461_but_not_tip_201() {
    // Regression: `f5-tmsh` had no `DialectSet` bit at all, so
    // `DialectSet::parse` returned `None` and W003 silently never fired
    // for it — a false negative on its documented Tcl 8.5 base for
    // `lt`/`le`/`gt`/`ge`. `expr_grammar_base_version` fixes this
    // without touching `DialectSet::parse`'s existing per-command
    // dialect-gating semantics.
    assert!(w003_hits("expr {2 in {1 2 3}}", "f5-tmsh").is_empty());
    assert_eq!(w003_hits("if {$x lt $y} { puts hi }", "f5-tmsh").len(), 1);
}

#[test]
fn emit_variable_usage_diagnostics_is_a_noop() {
    // Hook is intentionally empty — running it must leave
    // the diagnostics list untouched.
    let mut a = Analyser::new();
    a.result
        .diagnostics
        .push(diag(DiagCode::W113, Span::new(0, 3), "x"));
    a.emit_variable_usage_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 1);
}

#[test]
fn emit_cfg_ssa_diagnostics_runs_without_panicking_on_empty_source() {
    // Smoke test — the orchestrator handles empty input
    // gracefully (an empty CompilationUnit yields no
    // diagnostics).
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("");
    assert!(a.result.diagnostics.is_empty());
}

/// Slice 4: a memoised `CompilationUnit` (built via `build_for_memoized`
/// and fed through the `cu_override` seam) must yield **byte-identical**
/// diagnostics to the whole-file path — both on a cold cache (all misses,
/// proving the refactor) and a warm cache (all hits, proving the cache key
/// captures every lattice input).
#[test]
fn memoized_compilation_unit_diagnostics_match_whole_file() {
    use crate::compilation_unit::{CompilationUnit, FunctionUnit};
    use std::collections::HashMap;
    use std::sync::Arc;
    let snippets = [
        "proc a {x} {\n  if {$x} { set y 1 }\n  return $y\n}\nproc b {} { a 1 }\n",
        "set g 0\nproc inc {} { global g; incr g }\ninc\nputs $g\n",
        "proc f {n} {\n  set acc 0\n  for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n  return $acc\n}\nproc g {} { set z 9; set z 10; return $z }\n",
        "namespace eval n {\n  proc p {a} { set b $a; return $b }\n}\nset r [n::p 3]\n",
        "oo::class create K {\n  method m {a} { set n $a; return $n }\n}\nproc top {} { set q 1; set q 2 }\n",
        // Constructor object typing through the memo: `set o [K new]` types
        // `o` as OBJECT(::K), so `$o gone` is validated (W308). Exercises
        // the `known_classes` thread on `LatticeRequest` + the memo key.
        "oo::class create K {\n  method m {} { return 1 }\n}\nproc top {} { set o [K new]; $o gone }\n",
    ];
    for src in snippets {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        if let Some(d) = tcl_registry::prelude::DialectSet::parse("tcl") {
            registry.load_dialect(d);
        }
        // Whole-file reference.
        let mut whole = Analyser::new();
        let want = whole.analyse(src, "tcl");

        // Build a memoised unit (cold cache) and run through the seam.
        // The callback mirrors the db's `function_lattice`: build the
        // offset-0 unit, keyed on the offset-0 body + name + params.
        let mut cache: HashMap<String, FunctionUnit> = HashMap::new();
        let build_cu = |cache: &mut HashMap<String, FunctionUnit>| {
            CompilationUnit::build_for_memoized(
                src,
                crate::compilation_unit::UnitBuildOptions {
                    registry: &registry,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::default(),
                    dialect: "tcl",
                    external_call_sites: None,
                },
                &mut |req: &crate::compilation_unit::LatticeRequest<'_>| -> FunctionUnit {
                    // Key + build mirror the db's `function_lattice` query,
                    // including the whole-unit `known_classes` /
                    // `traced_variables` fingerprints.
                    let key = format!(
                        "{}\u{0}{:?}\u{0}{:?}\u{0}{:?}\u{0}{:?}\u{0}{:?}\u{0}{:?}",
                        req.qname,
                        req.body,
                        req.params,
                        req.param_constants,
                        req.known_classes,
                        req.traced_variables,
                        req.has_dynamic_variable_trace,
                    );
                    if let Some(fu) = cache.get(&key) {
                        return fu.clone();
                    }
                    let cfg = crate::cfg_builder::build_cfg_function_with_upvars(
                        req.qname,
                        req.body,
                        true,
                        req.upvar_procs.clone(),
                        req.proc_params.clone(),
                        req.global_write_procs.clone(),
                    );
                    let pc = crate::compilation_unit::decode_param_constants(req.param_constants);
                    let known_classes: std::collections::HashSet<String> =
                        req.known_classes.iter().cloned().collect();
                    let traced_variables: std::collections::BTreeSet<String> =
                        req.traced_variables.iter().cloned().collect();
                    let trace_facts = crate::compilation_unit::ModuleTraceFacts {
                        traced_variables: &traced_variables,
                        has_dynamic_variable_trace: req.has_dynamic_variable_trace,
                    };
                    let fu = FunctionUnit::build_with_param_constants_and_classes(
                        req.qname,
                        cfg,
                        req.params,
                        &registry,
                        pc.as_ref(),
                        &known_classes,
                        trace_facts,
                    );
                    cache.insert(key, fu.clone());
                    fu
                },
            )
            .with_interprocedural(&registry, Some("tcl"))
        };

        let cold = build_cu(&mut cache);
        let mut a_cold = Analyser::new();
        a_cold.set_cu_override(Arc::new(cold));
        let got_cold = a_cold.analyse(src, "tcl");
        assert_eq!(
            want.diagnostics, got_cold.diagnostics,
            "cold-cache memoised diagnostics differ for:\n{src}"
        );
        assert!(!cache.is_empty(), "cache should have entries for:\n{src}");

        // Warm cache: every procedure body is a hit now.
        let warm = build_cu(&mut cache);
        let mut a_warm = Analyser::new();
        a_warm.set_cu_override(Arc::new(warm));
        let got_warm = a_warm.analyse(src, "tcl");
        assert_eq!(
            want.diagnostics, got_warm.diagnostics,
            "warm-cache memoised diagnostics differ for:\n{src}"
        );
    }
}

/// Shift-correctness: a body that is **unedited but shifted** (lines
/// inserted above it) must NOT take a stale cache hit — the cached unit's
/// spans are absolute, so its diagnostics must land at the new positions.
/// The position-independent key + span rebase makes this a hit at the new
/// offset, byte-identical to a fresh analyse.
#[test]
fn memoized_compilation_unit_shift_correctness() {
    use crate::compilation_unit::{CompilationUnit, FunctionUnit};
    use std::collections::HashMap;
    use std::sync::Arc;
    // Bodies span read-before-set, dead store, and every control-flow shape
    // (if/for/while/catch/switch) so the rebase traverses nested scripts +
    // sub-spans, not just flat statements.  All produce positioned
    // diagnostics whose spans must move with the shift.
    let bodies = "proc a {} { return $undef }\n\
             proc b {} { set y 1; set y 2; return $y }\n\
             proc c {x} {\n  if {$x} { set z 1; set z 2 }\n  for {set i 0} {$i < 3} {incr i} { set w $q }\n  return $z\n}\n\
             proc d {n} {\n  while {$n} { catch { set r $undef2 } }\n  switch -- $n { 1 { set s 1; set s 2 } default { return $missing } }\n}\n";
    let base = bodies.to_owned();
    // Same procs, shifted down by several prepended top-level lines.
    let shifted = format!("set top 0\nset top2 1\n# a comment line\n{bodies}");
    let base = base.as_str();
    let shifted = shifted.as_str();

    let mut registry = tcl_registry::CommandRegistry::build_default();
    if let Some(d) = tcl_registry::prelude::DialectSet::parse("tcl") {
        registry.load_dialect(d);
    }
    let mut cache: HashMap<String, FunctionUnit> = HashMap::new();
    let build = |s: &str, cache: &mut HashMap<String, FunctionUnit>| {
        let cu = CompilationUnit::build_for_memoized(
            s,
            crate::compilation_unit::UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::default(),
                dialect: "tcl",
                external_call_sites: None,
            },
            // Position-independent key: the body is normalised to offset 0
            // before the callback sees it, so a shifted-but-unedited proc
            // hits and the builder rebases the cached offset-0 unit.
            &mut |req: &crate::compilation_unit::LatticeRequest<'_>| -> FunctionUnit {
                let key = format!(
                    "{}\u{0}{:?}\u{0}{:?}\u{0}{:?}",
                    req.qname, req.body, req.params, req.param_constants
                );
                if let Some(fu) = cache.get(&key) {
                    return fu.clone();
                }
                let cfg = crate::cfg_builder::build_cfg_function_with_upvars(
                    req.qname,
                    req.body,
                    true,
                    req.upvar_procs.clone(),
                    req.proc_params.clone(),
                    req.global_write_procs.clone(),
                );
                let pc = crate::compilation_unit::decode_param_constants(req.param_constants);
                let fu = FunctionUnit::build_with_param_constants(
                    req.qname,
                    cfg,
                    req.params,
                    &registry,
                    pc.as_ref(),
                );
                cache.insert(key, fu.clone());
                fu
            },
        )
        .with_interprocedural(&registry, Some("tcl"));
        let mut a = Analyser::new();
        a.set_cu_override(Arc::new(cu));
        a.analyse(s, "tcl")
    };

    // Prime the cache on `base`, then analyse `shifted` reusing it.  The
    // procedure bodies are unchanged, so they hit the position-independent
    // cache and are rebased to their new offsets.
    let _ = build(base, &mut cache);
    let entries_after_base = cache.len();
    let got = build(shifted, &mut cache);
    let want = Analyser::new().analyse(shifted, "tcl");
    assert_eq!(
        want.diagnostics, got.diagnostics,
        "shifted-body diagnostics must match a fresh analyse (rebased hit)"
    );
    // The shifted build reused the cached bodies (no new entries for the
    // procedures), exercising the rebase path rather than rebuilding.
    assert_eq!(
        cache.len(),
        entries_after_base,
        "shifted bodies should reuse cached entries (position-independent key)"
    );
    assert!(
        !want.diagnostics.is_empty(),
        "test should exercise real positioned diagnostics"
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w220_on_set_once_never_read() {
    // ``set x 1`` set once and never read is a dead store *and* an unused
    // variable. Both checks anchor at the same assignment, so the co-located
    // W220 is deduped in favour of the more informative W211 — a single hint,
    // not two.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1 }");
    let lifecycle: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, DiagCode::W211 | DiagCode::W220))
        .map(|d| d.code)
        .collect();
    assert_eq!(
        lifecycle,
        vec![DiagCode::W211],
        "set-once-never-read must yield only W211; got {:?}",
        a.result.diagnostics,
    );

    // A dead store of a variable that *is* used elsewhere still fires a
    // standalone W220 (no W211, so nothing to dedup against).
    let mut c = Analyser::new();
    c.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 2\nreturn $x }");
    assert!(
        c.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220)
            && !c
                .result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W211),
        "the overwritten `set x 1` fires W220 without W211; got {:?}",
        c.result.diagnostics,
    );

    let mut b = Analyser::new();
    b.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nreturn $x }");
    assert!(
        !b.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220),
        "W220 must not fire when the assignment is read; got {:?}",
        b.result.diagnostics,
    );
}

#[test]
fn w220_suppressed_for_interpreter_special_variable_writes() {
    // Issue #831: `set auto_path …` at top level configures the runtime
    // package/auto-loader; the write is observed by the interpreter even
    // though the script never reads `$auto_path` back, so it is not a dead
    // store.  The special-variable set is sourced from the dialect-aware
    // `tcl_registry::special_vars` registry.
    // Neither the dead-store (W220) nor the unused-variable (W211) hint may
    // fire — both were false positives on these writes.
    for src in [
        "set auto_path ../\n",
        "lappend auto_path /some/dir\n",
        "set env(FOO) bar\n",
        "set tcl_precision 12\n",
        "set errorInfo {}\n",
    ] {
        let mut a = Analyser::new();
        let res = a.analyse(src, "tcl8.6");
        assert!(
            !res.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
            "W220/W211 must not fire for special-var write {src:?}; got {:?}",
            res.diagnostics,
        );
    }

    // Control: a genuine top-level user variable written and never read is
    // still flagged (the same shape as the issue's screenshot).
    let mut a = Analyser::new();
    let res = a.analyse("set myUnusedVar ../\n", "tcl8.6");
    assert!(
        res.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
        "a genuine dead/unused store must still be flagged; got {:?}",
        res.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w220_suppressed_for_substitution_hidden_reads() {
    // A variable read only inside a command substitution the
    // version-precise SSA can't see — a branch condition, an `expr`
    // value, or a read-modify-write buried in a `[…]` — is not a dead
    // store. gate.
    // Uses the full `analyse` entry so the command registry (which the
    // hidden-read recovery consults) is populated.
    for src in [
        // read in an `if` condition substitution
        "proc f {} { set x 1\nif {[string length $x]} { puts hi } }",
        // read in an expr value substitution
        "proc f {} { set x 1\nset y [expr {[string length $x]}]\nreturn $y }",
        // read-modify-write buried in a substitution keeps the feed alive
        "proc f {} { set i 0\nforeach j {1 2 3} { lappend r [incr i $j] }\nreturn $r }",
    ] {
        let mut a = Analyser::new();
        let res = a.analyse(src, "tcl");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::W220),
            "W220 must not fire for a substitution-hidden read; got {:?} for {src:?}",
            res.diagnostics,
        );
    }
    // Control: a normal later read does not hide anything, so an earlier
    // overwritten store is still a dead store.
    let mut a = Analyser::new();
    let res = a.analyse("proc f {} { set x 1\nset x 2\nputs $x }", "tcl");
    assert!(
        res.diagnostics.iter().any(|d| d.code == DiagCode::W220),
        "W220 must still fire for a genuine overwrite; got {:?}",
        res.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w220_dead_store_overwritten() {
    // ``set x 1\nset x 2\nputs $x`` — the first ``set x 1``
    // is overwritten before being read.  W220 should fire
    // at the first assignment.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("set x 1\nset x 2\nputs $x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        !w220s.is_empty(),
        "W220 expected for overwritten ``set x 1``; got {:?}",
        a.result.diagnostics,
    );
    assert!(w220s.iter().any(|d| d.message.contains("'x'")));
    assert_eq!(w220s[0].severity, Severity::Hint);
}

#[test]
fn w220_array_element_overwrite_not_dead() {
    // `set a(k) 1` is NOT a dead store even
    // though the later `set a(j) 2` bumps the name-level SSA version of the
    // base `a`.  The place model sees `a(k)` is read by `puts $a(k)` and
    // that `a(k)` ≠ `a(j)`, so the false W220 on the first write is
    // suppressed.  Goes through `analyse` (the production path) so the
    // registry — which the place bridge needs — is bound.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }", "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W220),
        "no W220 expected — a(k) is read by `puts $a(k)`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w220_scalar_overwrite_still_fires_via_analyse() {
    // Regression guard for the element-granular scope of the suppression:
    // a genuine *scalar* overwrite must still fire W220 with the place
    // model active (scalars don't fold, so the name-level verdict stands).
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { set x 1; set x 2; puts $x }", "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 && d.message.contains("'x'")),
        "scalar dead store must still fire; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w220_braced_literal_arg_is_not_a_read() {
    // A braced word performs no `$`-substitution at this call — but the
    // name-level scan deliberately keeps `{$a(k)}` as a conservative
    // liveness read (the word may be `eval`-ed / scheduled later, e.g.
    // `after 100 {incr x}`), so under per-element SSA the mention keeps
    // `a(k)` alive and no dead store fires. The control: an element with
    // NO mention of any kind still reports.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {} { set a(k) 1; set a(j) 2; puts $a(j); puts {$a(k)} }",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
        "a braced mention conservatively keeps the element alive; got {:?}",
        r.diagnostics,
    );
    let mut b = Analyser::new();
    let r = b.analyse("proc f {} { set a(k) 1; set a(j) 2; puts $a(j) }", "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
        "an unmentioned dead element write still reports; got {:?}",
        r.diagnostics,
    );
}

/// W220-IR-paths.  Variables prefixed with ``::`` are
/// externally consumed (other namespaces, the global frame
/// outside this file) — the dead-store check skips them.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_global_qualified_var() {
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("set ::x 1\nset ::x 2\nputs $::x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.is_empty(),
        "W220 must skip ``::``-prefixed globals; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``set x [foo]`` is a side-effecting
/// store: dropping the assignment would also drop the call
/// to ``foo``.  ``IRAssignValue`` values containing ``[``
/// are filtered out.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_command_substitution_value() {
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("set x [clock seconds]\nset x 2\nputs $x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.is_empty(),
        "W220 must skip ``set x [cmd]`` side-effecting stores; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``set x [expr {[foo]}]`` lowers as
/// ``IRAssignExpr`` with a command call inside — same
/// side-effecting reasoning as command-substitution
/// values.  ``IRAssignExpr`` whose tree contains an
/// ``IRExprCommand`` is filtered out.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_expr_with_command_call() {
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("set x [expr {[clock seconds] + 1}]\nset x 2\nputs $x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.is_empty(),
        "W220 must skip ``IRAssignExpr`` containing a command call; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``incr x`` is a side-effecting write
/// (it reads the current value first).  The dead-store
/// check only matches ``IRAssignConst`` /
/// ``IRAssignValue`` / ``IRAssignExpr`` — ``IRIncr`` and
/// ``IRCall.defs`` are skipped by exclusion.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_incr_writes() {
    let mut a = Analyser::new();
    // ``incr x`` reads x then writes x+1; even when later
    // overwritten, dropping the incr would also drop the
    // implicit read.  Of the three writes to ``x``, only
    // the ``incr`` qualifies as overwritten-before-read
    // (``set x 0`` is read by incr, ``set x 5`` is read
    // by puts), so any W220 on x must be from the incr,
    // and the IR-statement-type filter must drop it.
    a.emit_cfg_ssa_diagnostics("set x 0\nincr x\nset x 5\nputs $x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220 && d.message.contains("'x'"))
        .collect();
    assert!(
        w220s.is_empty(),
        "W220 must skip ``incr`` side-effecting writes; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``lassign $list a b`` defines ``a`` and
/// ``b`` via ``IRCall.defs`` — a side-effecting write that
/// can't be dropped without also dropping the call.
/// The dead-store check only matches the three
/// pure-assign IR shapes; ``IRCall`` is skipped by
/// exclusion.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_call_defs() {
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("lassign {1 2} a b\nset a 5\nputs $a");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.iter().all(|d| !d.message.contains("'a'")),
        "W220 must skip ``IRCall.defs`` side-effecting writes; got {w220s:?}",
    );
}

/// W220-IR-paths.  In a ``pkgIndex.tcl`` file, ``$dir`` is
/// set by the Tcl package loader before the script body
/// runs — even when the script reassigns it, the original
/// store can't be considered dead (the loader-supplied
/// value is the relevant initial state).
#[test]
fn emit_cfg_ssa_diagnostics_w220_pkgindex_dir_var_suppressed() {
    let mut a = Analyser::new();
    a.file_path = Some("/some/path/pkgIndex.tcl".to_string());
    a.emit_cfg_ssa_diagnostics("set dir foo\nset dir bar\nputs $dir");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.is_empty(),
        "W220 must suppress ``$dir`` in pkgIndex.tcl; got {w220s:?}",
    );
}

/// W220-IR-paths.  Outside ``pkgIndex.tcl``, ``$dir`` is
/// just a regular variable — no special suppression.
/// Negative control for the pkgIndex special-case.
#[test]
fn emit_cfg_ssa_diagnostics_w220_dir_var_not_suppressed_outside_pkgindex() {
    let mut a = Analyser::new();
    a.file_path = Some("/some/path/script.tcl".to_string());
    a.emit_cfg_ssa_diagnostics("set dir foo\nset dir bar\nputs $dir");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        !w220s.is_empty(),
        "W220 must fire on ``$dir`` outside pkgIndex.tcl; got {:?}",
        a.result.diagnostics,
    );
    assert!(w220s.iter().any(|d| d.message.contains("'dir'")));
}

/// W220-IR-paths.  Variables shared across iRule events
/// via ``::when::*`` procs (collected in
/// ``ConnectionScope::cross_event_imports``) may be read
/// in a different event from where they're set — the
/// local "no use" verdict is unsafe.
#[test]
fn emit_cfg_ssa_diagnostics_w220_irules_cross_event_var_suppressed() {
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::by_name("f5-irules");
    // ``HTTP_REQUEST`` writes ``v``, ``HTTP_RESPONSE``
    // reads ``v`` — ``v`` is a cross-event def.  The
    // ``set v 1\nset v 2`` shape inside ``HTTP_REQUEST``
    // would normally fire W220 on the first ``set v 1``,
    // but cross-event suppression should drop it.
    a.emit_cfg_ssa_diagnostics(
        "when HTTP_REQUEST {\n  set v 1\n  set v 2\n}\nwhen HTTP_RESPONSE {\n  log local0. $v\n}",
    );
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.iter().all(|d| !d.message.contains("'v'")),
        "W220 must suppress vars shared across iRule events; got {w220s:?}",
    );
}

/// W220-IR-paths.  Negative control: a proc-local variable
/// (NOT shared across events) inside a ``::when::*`` proc
/// is still subject to W220.  Confirms the cross-event
/// filter is targeted, not a blanket
/// "skip everything in `::when::`*" rule.
#[test]
fn emit_cfg_ssa_diagnostics_w220_irules_proc_local_still_flagged() {
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::by_name("f5-irules");
    // ``local`` is only used inside HTTP_REQUEST — not a
    // cross-event var, so W220 should still fire on the
    // overwritten first assignment.
    a.emit_cfg_ssa_diagnostics(
        "when HTTP_REQUEST {\n  set local 1\n  set local 2\n  log local0. $local\n}",
    );
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.iter().any(|d| d.message.contains("'local'")),
        "W220 must still fire for proc-local vars in ::when::*; got {:?}",
        a.result.diagnostics,
    );
}

/// W220-IR-paths.  Dead stores in SCCP-unreachable blocks
/// are reported as O107 by the optimiser; the analyser
/// must not double-report them as W220.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_unreachable_block() {
    let mut a = Analyser::new();
    // ``if {0} { ... }`` makes the then-branch unreachable
    // under SCCP.  Any dead store inside is suppressed.
    a.emit_cfg_ssa_diagnostics("if {0} {\n  set x 1\n  set x 2\n  puts $x\n}\nputs done");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.is_empty(),
        "W220 must skip dead stores in SCCP-unreachable blocks; got {w220s:?}",
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w214_unused_param() {
    // ``proc foo {x y} { puts $x }`` — parameter ``y`` is
    // declared but never read in the body.  W214 should
    // fire on it.  Parameter ``x`` is read, so no W214.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {x y} { puts $x }");
    let w214s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W214)
        .collect();
    assert_eq!(
        w214s.len(),
        1,
        "expected exactly one W214 for unused param ``y``; got {:?}",
        a.result.diagnostics,
    );
    assert!(w214s[0].message.contains("'y'"));
    assert!(w214s[0].message.contains("'::foo'"));
    assert_eq!(w214s[0].severity, Severity::Hint);
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_unused_variable() {
    // ``proc foo {} { set y 1 }`` — y is set, never read,
    // and there's no other version → W211 fires.
    // Top-level test would be subject to global-scope
    // assumptions, so use a proc body where the local-only
    // verdict is safe.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { set y 1 }");
    let w211s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211)
        .collect();
    assert!(
        !w211s.is_empty(),
        "W211 expected for unused var ``y`` in proc foo; got {:?}",
        a.result.diagnostics,
    );
    assert!(w211s[0].message.contains("'y'"));
    assert!(w211s[0].message.contains("set but never used"));
    assert_eq!(w211s[0].severity, Severity::Hint);
}

#[test]
fn w211_not_emitted_for_command_output_vars() {
    // `scan` / `binary scan` / `regexp -> capture` write their targets
    // via the command, not a pure `set`; IRCall defs are excluded
    // from W211, so unused command outputs do not fire it.
    for src in [
        "proc f {} { scan $in \"%d\" n }",
        "proc f {} { binary scan $d cu4 b1 b2 b3 b4 }",
        "proc f {} { regexp {(\\w+)} $s -> word }",
    ] {
        let mut a = Analyser::new();
        let res = a.analyse(src, "tcl");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::W211),
            "W211 must not fire for command-output vars; got {:?} for {src:?}",
            res.diagnostics,
        );
    }
}

#[test]
fn w211_fires_once_per_variable_set_twice() {
    // A variable set twice and never read is one unused variable, reported once
    // (W211) at the earliest definition. The dead store at that same earliest
    // assignment does NOT also fire W220 — the co-located double-emit is
    // deduped in favour of the more informative W211 — but the *distinct*
    // second dead store (`set x 2`) still fires its own W220.
    let mut a = Analyser::new();
    let res = a.analyse("proc f {} { set x 1\nset x 2 }", "tcl");
    let w211: Vec<_> = res
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211)
        .collect();
    assert_eq!(w211.len(), 1, "expected one W211 for x; got {w211:?}");
    let w220: Vec<_> = res
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert_eq!(
        w220.len(),
        1,
        "the co-located W220 is deduped; only the distinct dead store remains; got {:?}",
        res.diagnostics,
    );
    // The surviving W211 and W220 anchor at different assignments.
    assert_ne!(w211[0].span, w220[0].span, "W211 and W220 must not overlap");
}

#[test]
fn w220_deduped_against_w211_on_single_dead_assignment() {
    // `set x 1` on a never-used variable is one dead assignment: it must emit
    // exactly one hint (W211 "never used"), not both W211 and a co-located
    // W220 ("never read"). The W220 is deduped in favour of W211.
    let mut a = Analyser::new();
    let res = a.analyse("proc f {} { set x 1 }", "tcl");
    let codes: Vec<_> = res
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, DiagCode::W211 | DiagCode::W220))
        .map(|d| d.code)
        .collect();
    assert_eq!(
        codes,
        vec![DiagCode::W211],
        "single dead assignment must yield only W211; got {:?}",
        res.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_w220_skipped_for_traced_var() {
    // A write trace makes `x` observable on every `set`, so neither
    // W211 (unused) nor W220 (dead store) may fire.  Both the 8.5+
    // `trace add variable` and 8.4 `trace variable` spellings count.
    for src in [
        "proc f {} { trace add variable x write cb; set x 1 }",
        "proc f {} { trace variable x w cb; set x 1 }",
    ] {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics(src);
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W211" | "W220")),
            "traced var must not fire W211/W220 for {src:?}; got {:?}",
            a.result.diagnostics,
        );
    }
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_skipped_for_textually_referenced() {
    // ``proc foo {} { set msg hello; puts "got $msg" }`` —
    // ``msg`` is referenced inside a quoted string; the
    // textual-reference filter should suppress W211 because
    // the def-use builder doesn't track ``"$msg"`` reads.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { set msg hello; puts \"got $msg\" }");
    let w211s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211 && d.message.contains("'msg'"))
        .collect();
    assert!(
        w211s.is_empty(),
        "W211 must not fire on var referenced via $-interpolation; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_skipped_for_global_aliased() {
    // ``proc foo {} { global config; set config 1 }`` —
    // ``config`` is global-aliased; the write goes to the
    // outer scope, so the local "no use" verdict is unsafe.
    // W211 must not fire.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { global config; set config 1 }");
    let w211s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211 && d.message.contains("'config'"))
        .collect();
    assert!(
        w211s.is_empty(),
        "W211 must not fire on global-aliased var; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_h300_repeated_assignment() {
    // ``proc foo {} { set x 1; set x 1 }`` — same var,
    // same literal value, consecutive statements.  The
    // first is a dead store; H300 fires on the second.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 1 }");
    let h300s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::H300)
        .collect();
    assert!(
        !h300s.is_empty(),
        "H300 expected for repeated ``set x 1``; got {:?}",
        a.result.diagnostics,
    );
    assert!(h300s[0].message.contains("'x'"));
    assert!(h300s[0].message.contains("Possible paste error"));
}

#[test]
fn emit_cfg_ssa_diagnostics_h300_skips_underscore_vars() {
    // Vars starting with ``_`` are excluded (the convention
    // for "intentionally unused").
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { set _x 1\nset _x 1 }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::H300),
        "H300 must not fire on underscore-prefixed vars",
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_h300_skips_distinct_values() {
    // Same var, different literal → not a paste error.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 2 }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::H300),
        "H300 must not fire when literal values differ",
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_read_before_set() {
    // ``proc foo {} { puts $undef }`` — undef is not a
    // parameter and not in scope; W210 fires at the use.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { puts $undef }");
    let w210s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W210 && d.message.contains("'undef'"))
        .collect();
    assert!(
        !w210s.is_empty(),
        "W210 expected for read of undef ``$undef``; got {:?}",
        a.result.diagnostics,
    );
    assert_eq!(w210s[0].severity, Severity::Warning);
    assert!(w210s[0].message.contains("read before it is set"));
}

#[test]
fn w210_not_fired_for_qualified_global_read() {
    // Regression for #725: ``$::myVar`` is an explicit global read; its
    // definition may live in another proc, namespace, or file, so it must
    // never be flagged read-before-set even when this unit never sets it.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { puts $::myVar }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must not fire on a fully-qualified global read; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn opaque_switch_recovers_exhaustive_arm_defs() {
    // A glob/regexp/fall-through switch is kept opaque, but it still
    // definitely-defines a variable assigned on *every* path. An exhaustive
    // switch (a `default` plus every arm setting `y`) defines `y`, so the
    // following `$y` read is NOT a read-before-set — no false W210.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(
        "proc f {x} { switch -glob $x {a* {set y 1} default {set y 2}}\n puts $y }",
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'y'")),
        "exhaustive opaque switch must define y (no W210); got {:?}",
        a.result.diagnostics,
    );

    // A non-exhaustive switch (no `default`) leaves `y` only maybe-defined,
    // so the read IS a read-before-set — W210 must still fire.
    let mut b = Analyser::new();
    b.emit_cfg_ssa_diagnostics(
        "proc g {x} { switch -glob $x {a* {set y 1} b* {set y 2}}\n puts $y }",
    );
    assert!(
        b.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'y'")),
        "non-exhaustive opaque switch leaves y maybe-undef (W210 expected); got {:?}",
        b.result.diagnostics,
    );
}

/// Helper: does the analyser emit a W210 for a read of `var` in `src`?
#[cfg(test)]
fn w210_fires_for(src: &str, var: &str) -> bool {
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(src);
    let needle = format!("'{var}'");
    a.result
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W210 && d.message.contains(&needle))
}

#[test]
fn fp_rbs_13_tailcall_is_a_terminator() {
    // Bug 1 / FP-RBS-13: `tailcall` replaces the current frame and never
    // returns (TclNRTailcallObjCmd always `return TCL_RETURN`), so it ends
    // straight-line flow exactly like `return`/`error`. A var set only on
    // the *other* branch of an `if {…} { tailcall … }` is therefore always
    // set at a read after the `if` — no false W210.

    // FP: `tailcall g` (with args) — only the else branch reaches `return`.
    assert!(
        !w210_fires_for(
            "proc f {cond} { if {$cond} { tailcall g } else { set result 1 }\n return $result }",
            "result",
        ),
        "tailcall g must terminate the then-branch (no W210 on result)",
    );

    // FP: bare `tailcall` is *also* a terminator (no args guard) — the C
    // impl returns TCL_RETURN regardless of arg count.
    assert!(
        !w210_fires_for(
            "proc f {cond} { if {$cond} { tailcall } else { set result 1 }\n return $result }",
            "result",
        ),
        "bare tailcall must terminate the then-branch (no W210 on result)",
    );

    // TP control: a non-terminating then-branch (`puts hi`) leaves `result`
    // maybe-unset at the read — W210 must still fire. Proves the
    // suppression is specific to the terminator, not the if/return shape.
    assert!(
        w210_fires_for(
            "proc f {cond} { if {$cond} { puts hi } else { set result 1 }\n return $result }",
            "result",
        ),
        "non-terminating then-branch must still fire W210 on result",
    );
}

#[test]
fn fp_rbs_14_opaque_switch_excludes_non_completing_arm() {
    // Bug 2 / FP-RBS-14: an opaque switch's must-define set excludes any arm
    // that cannot complete normally (it never reaches the code after the
    // switch). The default sets `y`; the `a*` arm exits, so `y` is defined
    // on every *reaching* path.

    // FP: returning arm excluded — default defines `y`.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 0 } default { set y 2 } }\n puts $y }",
            "y",
        ),
        "returning arm must be excluded from must-define (no W210 on y)",
    );

    // FP: erroring arm likewise cannot complete normally.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { error bad } default { set y 2 } }\n puts $y }",
            "y",
        ),
        "erroring arm must be excluded from must-define (no W210 on y)",
    );

    // TP control: a *completing* arm that omits `y` (`set z 9`) reaches the
    // code after the switch with `y` unset — W210 must fire.
    assert!(
        w210_fires_for(
            "proc f {x} { switch -glob $x { a* { set z 9 } default { set y 2 } }\n puts $y }",
            "y",
        ),
        "completing arm omitting y must still fire W210 on y",
    );

    // TP control: a `break` arm is a LOOP_JUMP, not a proc-exit —
    // it escapes the loop *without* `y`, so `y` is NOT defined on every
    // reaching path. The arm must be kept (with its empty pre-break defs) in
    // the must-define intersection, so `y` is dropped and W210 fires.
    assert!(
        w210_fires_for(
            "proc f {} { foreach x {a} { switch -glob $x { a* { break } default { set y 1 } } }\n puts $y }",
            "y",
        ),
        "break arm escapes the loop with y unset (W210 expected on y)",
    );
}

#[test]
fn fp_rbs_15_all_exiting_opaque_switch_is_a_terminator() {
    // Bug 3 / FP-RBS-15: an opaque switch with a `default` whose *every*
    // reachable arm body cannot complete normally never falls through, so
    // the code after it is dead — no W210 on a read in that dead code.

    // FP: every arm returns — the switch is a terminator, `puts $y` is dead.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 1 } default { return 2 } }\n puts $y }",
            "y",
        ),
        "all-returning opaque switch must terminate (puts $y is dead, no W210)",
    );

    // FP: a mix of error / tailcall arms is also all-exiting.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { error bad } default { tailcall g } }\n puts $y }",
            "y",
        ),
        "all error/tailcall opaque switch must terminate (no W210 on y)",
    );

    // TP control: drop the `default` — an unmatched subject falls through to
    // `puts $y` with `y` unset, so the read is reachable. W210 fires.
    assert!(
        w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 1 } b* { return 2 } }\n puts $y }",
            "y",
        ),
        "default-less opaque switch falls through (W210 expected on y)",
    );

    // TP control: one arm that *completes* (`set z 9`) lets the switch fall
    // through with `y` unset. W210 fires.
    assert!(
        w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 1 } default { set z 9 } }\n puts $y }",
            "y",
        ),
        "one completing arm lets the switch fall through (W210 expected on y)",
    );

    // TP control: an all-*break* switch is NOT a proc terminator —
    // it jumps to the enclosing loop's exit, so a `while 1` whose only exit
    // is the break reaches the post-loop read with the var unset. The switch
    // must wire its break edge to the loop exit (not promote to a Return),
    // so the loop exit is reachable and W210 fires.
    assert!(
        w210_fires_for(
            "proc f {x} { while 1 { switch -glob $x { a* { break } default { break } } }\n puts $y }",
            "y",
        ),
        "all-break opaque switch in while 1 reaches the loop exit (W210 expected on y)",
    );
}

#[test]
fn fp_rbs_16_dead_loop_exit_phi_operand_not_read_before_set() {
    // FP-RBS-16: a `while 1` loop-exit phi has an operand on the never-taken
    // `cond -> exit` edge carrying the var's version-0 (unset) origin. SCCP
    // marks that edge non-executable; the read-before-set phi-undef closure
    // must skip operands on dead predecessor edges, not just dead blocks.

    // FP: `while 1 { set y 1; break }` — only live exit is the break (y set).
    assert!(
        !w210_fires_for("proc f {} { while 1 { set y 1; break }\n puts $y }", "y"),
        "while 1 set+break: y set on the only live exit edge (no W210)",
    );

    // FP: the realistic `while 1 { ...; if {c} break }` early-exit idiom.
    assert!(
        !w210_fires_for(
            "proc compute {} {return 7}\nproc ok {a} {return 1}\nproc f {} { while 1 { set r [compute]; if {[ok $r]} break }\n return $r }",
            "r",
        ),
        "while 1 compute/if-break: r set before the only live exit (no W210)",
    );

    // FP-RBS-19 (#756): a non-constant condition may run the body zero times,
    // but the body unconditionally sets y, so a read after the loop is defined
    // whenever the loop ran. Matching C Tcl, we assume a may-run loop runs.
    assert!(
        !w210_fires_for("proc f {n} { while {$n>0} { set y 1 }\n puts $y }", "y"),
        "may-run while whose body defines y is silent after the loop (no W210)",
    );

    // TP control: only one of two break paths sets y, so the exit merges a
    // genuine unset version on a live break edge (not the loop-entry edge) —
    // FP-RBS-19 does not touch this; it still fires.
    assert!(
        w210_fires_for(
            "proc f {c} { while 1 { if {$c} { set y 1; break } else break }\n puts $y }",
            "y",
        ),
        "partial-def break path leaves y maybe-unset (W210 expected on y)",
    );
}

#[test]
fn fp_rbs_17_guaranteed_foreach_defines_body_vars() {
    // FP-RBS-17: a foreach over a non-empty *literal* list provably iterates
    // ≥1 time, so a body-assigned variable (or the loop variable) read after
    // the loop is defined. Analysis rotates the loop so the 0-iteration skip
    // is a dead entry-guard edge (FP-RBS-16 ignores its phi operand).

    // FP: body assigns y; the literal list guarantees ≥1 iteration.
    assert!(
        !w210_fires_for(
            "proc f {} { foreach x {1 2 3} { set y $x }\n puts $y }",
            "y"
        ),
        "foreach over a non-empty literal defines y (no W210)",
    );

    // FP: the loop variable itself is defined after a guaranteed foreach.
    assert!(
        !w210_fires_for("proc f {} { foreach x {a b c} {}\n puts $x }", "x"),
        "loop variable is defined after a guaranteed foreach (no W210)",
    );

    // TP control: an empty literal list runs zero times — y stays unset.
    assert!(
        w210_fires_for("proc f {} { foreach x {} { set y $x }\n puts $y }", "y"),
        "empty foreach list runs zero times (W210 expected on y)",
    );

    // FP-RBS-19 (#756): a dynamic (`$i`) list may be empty, but the body
    // unconditionally sets y, so a read after the loop is defined whenever the
    // loop ran. Matching C Tcl, we assume a may-run loop runs.
    assert!(
        !w210_fires_for("proc f {i} { foreach x $i { set y $x }\n puts $y }", "y"),
        "may-run dynamic foreach whose body defines y is silent after the loop (no W210)",
    );

    // TP control: a first-iteration read-before-set inside the body still
    // fires — the suppression is confined to reads *after* the loop.
    assert!(
        w210_fires_for(
            "proc f {} { foreach x {1 2 3} { puts $acc; set acc $x } }",
            "acc"
        ),
        "first-iteration read of acc before its set still fires (W210)",
    );
}

#[test]
fn fp_rbs_18_guaranteed_for_defines_body_vars() {
    // FP-RBS-18: a `for` whose condition is statically true on
    // entry (evaluated against the init clause's *constant* bindings)
    // provably iterates ≥1 time. Init is processed in order, and a
    // non-constant write invalidates a stale constant binding.

    // FP: `for {set i 0} {$i<3} …` — 0 < 3 true on entry, body sets y.
    assert!(
        !w210_fires_for(
            "proc f {} { for {set i 0} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "for with statically-true entry condition defines y (no W210)",
    );

    // TP control: `for {set i 5} {$i<3} …` — 5 < 3 false, zero iterations.
    assert!(
        w210_fires_for(
            "proc f {} { for {set i 5} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "for with false entry condition runs zero times (W210 expected on y)",
    );

    // FP-RBS-19 (#756): a stale-constant init (`set i $n` overwrites `set i 0`)
    // leaves the loop may-run — but its body unconditionally sets y, so a read
    // after the loop is silent (we assume a may-run loop runs). The rotation
    // decision itself (that a stale-const init is NOT claimed guaranteed) is
    // pinned directly on the CFG shape in
    // `cfg::for_rotation_requires_a_non_stale_constant_init`.
    assert!(
        !w210_fires_for(
            "proc f {n} { for {set i 0; set i $n} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "may-run for whose body defines y is silent after the loop (no W210)",
    );

    // TP control: an `incr` in the init resolves the loop var to `i = 5`, so
    // SCCP folds `5 < 3` to false and the body is provably dead — a genuine
    // zero-iteration loop that leaves y unset. Still fires.
    assert!(
        w210_fires_for(
            "proc f {} { for {set i 0; incr i 5} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "provably-empty for (incr init makes 5<3 false) leaves y unset (W210 on y)",
    );

    // FP-RBS-19 (#756): an init call writing the loop var through `upvar` leaves
    // the loop may-run (SCCP cannot see through the call), so the after-loop
    // read of the body-defined y is silent. The invalidation itself (the loop
    // is NOT claimed guaranteed) is pinned on the CFG shape in
    // `cfg::for_rotation_requires_a_non_stale_constant_init`.
    assert!(
        !w210_fires_for(
            "proc setter {} { upvar 1 i i; set i 5 }\nproc f {} { for {set i 0; setter} {$i < 3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "may-run for (upvar-writing init call) whose body defines y is silent after the loop (no W210)",
    );

    // FP guard: a *benign* call in the init (one that does not write the
    // loop var) must NOT invalidate the constant — the loop stays guaranteed
    // and `y` is defined, so no false W210 is introduced by the upvar fix.
    assert!(
        !w210_fires_for(
            "proc f {} { for {set i 0; puts hi} {$i < 3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "benign init call must not invalidate the constant (no W210 on y)",
    );
}

#[test]
fn fp_rbs_15_continue_in_opaque_switch_stays_silent() {
    // Companion to fp_rbs_15: a benign `continue` arm inside an
    // opaque switch in a guaranteed foreach stays silent when the variable
    // is set before the switch on every iteration.
    assert!(
        !w210_fires_for(
            "proc f {} { foreach x {1 2 3} { set y 1; switch -glob $x { a* { continue } default {} } }\n puts $y }",
            "y",
        ),
        "continue-arm switch with y set before it every iteration (no W210)",
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_skipped_for_lappend_autocreate() {
    // `lappend` / `append` auto-create their target, so a first use is
    // not a read-before-set (excluding RMW targets).
    for src in [
        "proc foo {} { lappend items a\nputs $items }",
        "proc foo {} { append buf x\nputs $buf }",
    ] {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics(src);
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W210),
            "W210 must not fire for auto-creating RMW; got {:?} for {src:?}",
            a.result.diagnostics,
        );
    }
    // `unset` (without -nocomplain) is destructive, not auto-creating —
    // its missing-variable case must still raise W213.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { unset gone }");
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "W213 must still fire for unset of a possibly-undef var; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_skipped_for_real_param() {
    // ``proc foo {x} { puts $x }`` — x IS a real parameter,
    // so W210 must not fire.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {x} { puts $x }");
    let w210s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W210 && d.message.contains("'x'"))
        .collect();
    assert!(
        w210s.is_empty(),
        "W210 must not fire on real param ``x``; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w213_unset_on_possibly_undef() {
    // ``proc foo {} { unset xs }`` — ``xs`` may not exist;
    // ``unset`` without ``-nocomplain`` would error at
    // runtime.  W213 fires (instead of W210) at the unset
    // statement.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { unset xs }");
    let w213s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W213)
        .collect();
    assert!(
        !w213s.is_empty(),
        "W213 expected for ``unset xs`` on possibly-undef var; got {:?}",
        a.result.diagnostics,
    );
    assert!(w213s[0].message.contains("'xs'"));
    assert!(w213s[0].message.contains("unset -nocomplain"));
    assert_eq!(w213s[0].severity, Severity::Warning);
    // The squiggle narrows to the offending variable word `xs`, not the whole
    // `unset xs` command.
    let src = "proc foo {} { unset xs }";
    let span = w213s[0].span;
    assert_eq!(&src[span.start() as usize..span.end() as usize], "xs");
    // A quick fix is attached that inserts `-nocomplain` right after `unset`.
    assert_eq!(w213s[0].fixes.len(), 1, "W213 must carry one fix");
    let fix = &w213s[0].fixes[0];
    assert_eq!(fix.new_text, " -nocomplain");
    assert_eq!(fix.span.start(), fix.span.end(), "insertion is zero-width");
    // Splicing the fix produces `unset -nocomplain xs`.
    let at = fix.span.start() as usize;
    let spliced = format!("{}{}{}", &src[..at], fix.new_text, &src[at..]);
    assert!(
        spliced.contains("unset -nocomplain xs"),
        "spliced: {spliced}"
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_read_after_unset() {
    // ``set a 1; unset a; puts $a`` — the `unset` kills `a`, so the
    // later `$a` read is read-before-set. W210 fires on the read line
    // (the killed real version is undef at its use, like a version-0
    // origin).
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc f {} {\n    set a 1\n    unset a\n    puts $a\n}");
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'a'")),
        "W210 expected for read after unset; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn try_handler_edge_merges_pretry_when_body_falls_through() {
    // A body that *can* fall through normally but also contains an
    // explicit throw (`if {$c} { error e }; set y 2`) reaches the handler
    // by an abnormal completion at *any* point, so `y` is only *maybe*
    // defined — W210 must still fire on the handler's `$y` read. Sourcing
    // the on-error edge only from the explicit-throw block (after `set y 2`
    // failed to run) would wrongly suppress it.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(
            "proc f {c} {\n try {\n  if {$c} { error e }\n  set y 2\n } on error {} {\n  puts $y\n }\n}\n",
        );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'y'")),
        "W210 expected for handler read of maybe-undef y; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn try_handler_edge_suppressed_when_var_set_before_sole_throw() {
    // `set x 1; error boom` — the body has no normal fall-through and `x`
    // is set before the sole throw, so the handler sees `x` defined; no
    // W210. tclsh 9.0.3 reads x == 1 here.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(
        "proc f {} {\n try {\n  set x 1\n  error boom\n } on error {} {\n  puts $x\n }\n}\n",
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must not fire when x is set before the sole throw; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w213_skipped_with_nocomplain() {
    // ``unset -nocomplain xs`` is the safe form — W213
    // must not fire.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {} { unset -nocomplain xs }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "W213 must not fire when ``-nocomplain`` is present; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_fires_at_top_level() {
    // Top-level RBS now fires when no
    // proc writes the variable.  ``puts $undef`` reads
    // ``undef`` without any preceding write.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("puts $undef");
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must fire at top-level when no proc writes the var; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_suppressed_when_proc_writes_global() {
    // A helper proc ``init`` writes ``::counter`` via ``set``,
    // so the top-level read should not flag W210 — the proc
    // may run before the read.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc init {} { set ::counter 0 }\nputs $counter");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must be suppressed for globals written by procs; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_suppressed_via_global_alias() {
    // ``proc init {} { global counter; set counter 0 }`` — the
    // ``global`` declaration aliases the proc-local ``counter``
    // to the global.  Top-level read should not flag W210.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc init {} { global counter; set counter 0 }\nputs $counter");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must be suppressed via global-alias case; got {:?}",
        a.result.diagnostics,
    );
}

// info exists / array exists

fn codes_for(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(src);
    a.result
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

// TclOO/snit method-body CFG/SSA diagnostics (issue #923 idx 77)

#[test]
fn method_body_read_before_set_now_flags_w210() {
    // TP — the finding's own real repro shape, reduced: `nico-robert_tomato`'s
    // Vector3d.tcl `method * {type}` reads `$other`, a variable belonging to
    // a *sibling* method (`DotProduct {other}`), never bound in `*`'s own
    // scope — tclsh8.6/9.0.4 both crash with `can't read "other": no such
    // variable` the moment `*` runs on an object operand. The exact same
    // unbound-read shape inside a plain `proc` already fired W210; a TclOO
    // method body previously got zero diagnostics at all, because the whole
    // CFG/SSA dataflow family (`emit_cfg_ssa_diagnostics_for_function_full`)
    // was only ever run over `cu.procedures`, never `cu.methods`.
    let src = "oo::class create Vector3d {\n    variable _x\n    constructor {x} { set _x $x }\n    method DotProduct {other} { return [expr {$_x * $other}] }\n    method Buggy {type} { return [my DotProduct $other] }\n}\n";
    let codes = codes_for(src);
    assert_eq!(
        codes,
        vec!["W210".to_string()],
        "the unbound `$other` read inside Buggy must flag exactly once: {codes:?}"
    );
}

#[test]
fn method_body_instance_variable_read_does_not_false_positive_w210() {
    // FP guard — `variable _x` declared at class level auto-binds `_x` in
    // *every* method's scope with no visible `variable` statement in the
    // method body itself (real TclOO semantics); a naive fix that just
    // iterates `cu.methods` without threading `MethodDef::instance_vars`
    // into the read-before-set suppression set would flood a false W210 on
    // every ordinary instance-variable read — this is that read, alone,
    // with no sibling bug present.
    let src = "oo::class create P {\n    variable _x\n    constructor {x} { set _x $x }\n    method X {} { return $_x }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "a legitimate instance-variable read must not flag W210: {codes:?}"
    );
}

#[test]
fn method_body_own_parameter_read_does_not_false_positive_w210() {
    // FP guard — a method's *own* declared parameter must not flag W210
    // either. `emit_read_before_set_diagnostics` / `emit_return_phi_undef_w210`
    // both special-case a real parameter via a separate `ir_module.procedures`
    // lookup keyed by the function's qualified name, which a method's
    // qualified name is never in (methods live in `ir_module.methods`, a
    // different map) — so without also folding `MethodDef::params` into the
    // suppression set, every method parameter would falsely read-before-set
    // (caught empirically while building this fix: an earlier version that
    // only threaded `instance_vars` still flagged `other` in `DotProduct
    // {other}` itself, and `x` in the constructor).
    let src =
        "oo::class create P {\n    method DotProduct {other} { return [expr {1 * $other}] }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "a method's own used parameter must not flag W210: {codes:?}"
    );
}

#[test]
fn method_body_setter_write_with_no_local_read_does_not_false_positive_dead_store() {
    // FP guard — a "setter" method that writes an instance variable with no
    // local read is not a dead store (W220) or unused variable (W211):
    // another method reads the value later. Mirrors the pre-existing
    // cross-function-global suppression (`globals_written_by_procs`), just
    // object-instance-scoped.
    let src = "oo::class create P {\n    variable _x\n    method SetX {v} { set _x $v }\n    method GetX {} { return $_x }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W220".to_string()) && !codes.contains(&"W211".to_string()),
        "a setter's instance-variable write must not flag dead-store/unused: {codes:?}"
    );
}

#[test]
fn method_body_ordinary_unused_local_still_flags_w211() {
    // TN — the fix must not blanket-suppress every diagnostic inside a
    // method body: a genuinely unused *local* (not a param, not an instance
    // var) still flags W211, exactly as it would inside a plain proc.
    let src = "oo::class create P {\n    method Foo {} {\n        set unused 1\n        return ok\n    }\n}\n";
    let codes = codes_for(src);
    assert!(
        codes.contains(&"W211".to_string()),
        "a genuinely unused local inside a method must still flag W211: {codes:?}"
    );
}

// Issue #1172 — method units now come from the registry definer grammars,
// so `oo::objdefine`, snit, and itcl method bodies get the same CFG/SSA
// diagnostic family.

#[test]
fn objdefine_method_body_unbound_read_now_flags_w210() {
    // FN now caught — an `oo::objdefine $obj { method … }` body previously
    // produced no method unit at all, so a genuinely unbound read inside it
    // got zero diagnostics (issue #1172 item 1).
    let src = "oo::class create C {}\nset k [C new]\noo::objdefine $k {\n    method probe {} { return $neverBound }\n}\n";
    let codes = codes_for(src);
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read inside an objdefine method must flag W210: {codes:?}"
    );
}

#[test]
fn objdefine_per_object_variable_does_not_false_positive_w210() {
    // FP guard — the objdefine block's own `variable z` auto-binds `z` in
    // the per-object method's frame exactly as a class-level declaration
    // does for class methods; reading it must not flag.
    let src = "oo::class create C {}\nset k [C new]\noo::objdefine $k {\n    variable z\n    method probe {} { if {[info exists z]} { return $z }\nreturn {} }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "a per-object instance-variable read must not flag W210: {codes:?}"
    );
}

#[test]
fn snit_method_body_unbound_read_now_flags_w210() {
    // FN now caught — snit method bodies previously never became method
    // units (issue #1172 item 2), so this unbound read was invisible.
    let src = "snit::type Dog {\n    method bark {} { return $neverBound }\n}\n";
    let codes = codes_for(src);
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read inside a snit method must flag W210: {codes:?}"
    );
}

#[test]
fn snit_method_implicits_and_declared_vars_do_not_false_positive_w210() {
    // FP guard — snit injects `self` / `selfns` / `type` / `options` into
    // every method body, and a type-level `variable` declaration is
    // auto-linked; none of those reads may flag.
    let src = "snit::type Dog {\n    variable name\n    method describe {} { return \"$self $type $name $options(-color)\" }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "snit implicit / declared variable reads must not flag W210: {codes:?}"
    );
}

#[test]
fn snit_widget_win_and_hull_do_not_false_positive_w210() {
    // FP guard — the widget definers additionally inject `win` / `hull`
    // (registry data: SNIT_WIDGET_GRAMMAR's implicit_vars).
    let src = "snit::widget Bar {\n    method redraw {} { return \"$win $hull\" }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "widget `win`/`hull` reads must not flag W210: {codes:?}"
    );
}

#[test]
fn itcl_method_body_unbound_read_now_flags_w210() {
    // FN now caught — itcl method bodies previously never became method
    // units either (issue #1172 item 2).
    let src = "itcl::class Toaster {\n    public method toast {} { return $neverBound }\n}\n";
    let codes = codes_for(src);
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read inside an itcl method must flag W210: {codes:?}"
    );
}

#[test]
fn itcl_instance_state_and_this_do_not_false_positive_w210() {
    // FP guard — itcl auto-binds `this`, every `variable`, and every
    // `common` in method bodies (including modifier-wrapped declarations).
    let src = "itcl::class Toaster {\n    variable crumbs 0\n    protected common heat 3\n    public method status {} { return \"$this $crumbs $heat\" }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "itcl `this`/variable/common reads must not flag W210: {codes:?}"
    );
}

#[test]
fn method_body_unbound_read_inside_switch_arm_still_flags_w210() {
    // TN — a sanity check that the new loop reaches a method's full CFG,
    // not just a shallow top-level scan: the unbound read is nested inside
    // a `switch` arm.
    let src = "oo::class create P {\n    method Foo {n} {\n        switch $n {\n            1 { return $missing }\n        }\n        return 0\n    }\n}\n";
    let codes = codes_for(src);
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read nested inside a switch arm must still flag W210: {codes:?}"
    );
}

#[test]
fn info_exists_control_still_flags_w210() {
    // Baseline: a plain read of an unset local flags W210.
    assert!(codes_for("proc f {} { puts $u }").contains(&"W210".to_string()));
}

#[test]
fn info_exists_guard_narrows_read_in_then_arm() {
    // Reads inside `if {[info exists X]}`
    // are guarded — X provably exists there, so no W210.
    let codes = codes_for("proc f {} { if {[info exists u]} { puts $u } }");
    assert!(
        !codes.contains(&"W210".to_string()),
        "guarded read must not flag W210; got {codes:?}",
    );
}

#[test]
fn info_exists_read_outside_guard_still_flags_w210() {
    // The narrowing is scoped to the guarded arm: a read after the
    // `if` (not dominated by the guard) still flags W210.
    let codes = codes_for("proc f {} { if {[info exists u]} { puts hi }\nputs $u }");
    assert!(
        codes.contains(&"W210".to_string()),
        "read outside the guarded arm must still flag W210; got {codes:?}",
    );
}

#[test]
fn info_exists_negated_guard_narrows_false_arm() {
    // The false arm of `![info exists X]`
    // is guarded.
    let codes = codes_for("proc f {} { if {![info exists u]} { puts no } else { puts $u } }");
    assert!(
        !codes.contains(&"W210".to_string()),
        "false-arm read of `![info exists X]` must not flag W210; got {codes:?}",
    );
}

#[test]
fn info_exists_query_word_not_read_before_set() {
    // The existence-query word is
    // not a read-before-set — bare call and command-sub forms.
    assert!(!codes_for("proc f {} { info exists u }").contains(&"W210".to_string()));
    assert!(!codes_for("proc f {} { array exists u }").contains(&"W210".to_string()));
    let codes = codes_for("proc f {} { set y [info exists u]; puts $y }");
    assert!(
        !codes.contains(&"W210".to_string()),
        "`set y [info exists u]` must not flag W210 on u; got {codes:?}",
    );
}

#[test]
fn i230_message_quotes_bare_var_as_source_spells_it() {
    // Message fidelity: `if $n …` must render the condition as `$n`, not
    // the segmenter's re-braced `${n}` reconstruction.
    let src = "set n 1\nif $n { puts a } else { puts b }\n";
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(src);
    let i230: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "I230")
        .collect();
    assert!(
        !i230.is_empty(),
        "expected I230: {:?}",
        a.result.diagnostics
    );
    assert!(
        i230[0].message.contains("'$n'"),
        "message must quote the source spelling `$n`, got {:?}",
        i230[0].message
    );
    assert!(
        !i230[0].message.contains("${n}"),
        "message must not re-brace to `${{n}}`, got {:?}",
        i230[0].message
    );
}

#[test]
fn i230_message_keeps_braced_var_spelling() {
    // A source-braced `${n}` stays `${n}` — fidelity cuts both ways.
    let src = "set n 1\nif ${n} { puts a } else { puts b }\n";
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(src);
    let i230: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "I230")
        .collect();
    assert!(
        !i230.is_empty(),
        "expected I230: {:?}",
        a.result.diagnostics
    );
    assert!(
        i230[0].message.contains("'${n}'"),
        "message must keep the source's braced spelling, got {:?}",
        i230[0].message
    );
}

#[test]
fn info_exists_folds_false_for_never_defined_local() {
    // A never-defined non-parameter never
    // exists → predicate folds false → I230.
    let codes = codes_for("proc f {a} { if {[info exists b]} { puts hi } }");
    assert!(
        codes.contains(&"I230".to_string()),
        "`info exists` of a never-defined local should fold to I230; got {codes:?}",
    );
}

#[test]
fn info_exists_folds_true_for_parameter() {
    // A parameter always exists → predicate folds true → I230.
    let codes = codes_for("proc f {a} { if {[info exists a]} { puts hi } }");
    assert!(
        codes.contains(&"I230".to_string()),
        "`info exists` of a parameter should fold to I230; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_conditionally_set_var() {
    // A var that is set on some path is not provably set/unset —
    // no fold, no false I230.
    let codes =
        codes_for("proc f {flag} { if {$flag} { set u 1 } ; if {[info exists u]} { puts $u } }");
    assert!(
        !codes.contains(&"I230".to_string()),
        "conditionally-set var must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_namespaced_or_array() {
    // Namespaced vars (and elements of namespaced arrays) may be populated
    // outside the function's view — never fold them.  A *local* array's
    // element guard folds on the base name instead (issue #1173, below).
    assert!(
        !codes_for("proc f {} { if {[info exists ::env(PATH)]} { puts hi } }")
            .contains(&"I230".to_string())
    );
}

// Array-element guards (issue #1173): an element of an array this body never
// touches is provably absent — tclsh 9.0.4 / 8.6.16:
//   proc f {} { info exists Params(key) }; f   ;# → 0
// The fold is decided on the *array* name, one-sided (false only), with the
// same scope-alias / instance-state / dynamic-write abstentions as a simple
// name.

#[test]
fn info_exists_folds_false_for_element_of_never_touched_array() {
    // TP — the SpiceGenTcl-adjacent idiom: guarding optional state through an
    // element of an array nothing in the proc ever creates.
    let codes = codes_for("proc f {} { if {[info exists Params(key)]} { puts hi } }");
    assert!(
        codes.contains(&"I230".to_string()),
        "an element guard on a never-touched array should fold to I230; got {codes:?}",
    );
}

#[test]
fn info_exists_folds_false_for_dynamic_element_of_never_touched_array() {
    // TP — the guard is about the array, so even a runtime-selected element
    // is provably absent when the array itself was never created.
    let codes = codes_for("proc f {k} { if {[info exists Params($k)]} { puts hi } }");
    assert!(
        codes.contains(&"I230".to_string()),
        "a dynamic element guard on a never-touched array should fold; got {codes:?}",
    );
}

#[test]
fn info_exists_element_fold_abstains_when_the_array_is_touched() {
    // FP guards — any touch of the base, under either spelling, abstains:
    // a sibling element write, a whole-array `array set`, an `upvar` alias,
    // a `global` / `variable` link, and a parameter base.
    for src in [
        "proc f {} { set Params(other) 1\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { array set Params {a 1}\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { upvar 1 src Params\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { global Params\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { variable Params\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {Params} { if {[info exists Params(key)]} { puts hi } }",
    ] {
        let codes = codes_for(src);
        assert!(
            !codes.contains(&"I230".to_string()),
            "a touched / aliased / parameter base must abstain; got {codes:?} for {src}",
        );
    }
}

#[test]
fn info_exists_element_fold_abstains_on_dynamic_writes() {
    // FP guard — `set $n …` may create exactly `Params(key)` (or `Params`),
    // so the never-touched proof is gone.
    let codes = codes_for("proc f {n} { set $n 1\n if {[info exists Params(key)]} { puts hi } }");
    assert!(
        !codes.contains(&"I230".to_string()),
        "a dynamic write must abstain the element fold; got {codes:?}",
    );
}

#[test]
fn info_exists_element_fold_abstains_on_instance_state_arrays() {
    // FP guard — a class-level `variable Params` binds the name to per-object
    // storage: an earlier call on the same instance may have populated it,
    // the same abstention the simple-name fold takes (issue #1129).
    let codes = codes_for(
        "oo::class create C {\n variable Params\n method setit {} { set Params(key) 1 }\n \
         method m {} { if {[info exists Params(key)]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance-state array element must abstain; got {codes:?}",
    );
}

#[test]
fn info_exists_element_fold_survives_an_unrelated_array() {
    // TP control for the touched-base skip: touching a *different* array must
    // not blanket-disable the fold.
    let codes =
        codes_for("proc f {} { set Other(key) 1\n if {[info exists Params(key)]} { puts hi } }");
    assert!(
        codes.contains(&"I230".to_string()),
        "an unrelated array's write must not disable the fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_unset_parameter() {
    // A parameter that is `unset` before the check can't be assumed
    // to exist.
    let codes = codes_for("proc f {a} { unset a; if {[info exists a]} { puts hi } }");
    assert!(
        !codes.contains(&"I230".to_string()),
        "unset parameter must not fold true; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_scope_alias_locals() {
    // A local bound to out-of-frame storage exists iff the *linked* variable
    // does — runtime state this function cannot see, so the fold must skip
    // it for every alias kind.  tclsh 8.6: `namespace eval ns {variable s
    // ok}; proc t {} {namespace upvar ns s a; info exists a}; t` → 1, and →
    // 0 once `ns::s` is unset.  Pre-fix, `namespace upvar` (whose lowered
    // Call carries no defs, unlike `global`/`variable`/`upvar`) folded
    // "always false" — the `::safe::CheckInterp` guard shape (safe.tcl:109).
    for src in [
        // The two `namespace upvar` forms are the regression; the other
        // alias kinds pin the behaviour they already had via `Call::defs`.
        "proc f {} { namespace upvar ::ns state alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { namespace upvar ::ns arr(k) alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { upvar 1 src alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { global alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { variable alias\n if {[info exists alias]} { puts hi } }",
    ] {
        let codes = codes_for(src);
        assert!(
            !codes.contains(&"I230".to_string()),
            "existence of a scope-alias local must not fold; got {codes:?} for {src}",
        );
    }
}

#[test]
fn info_exists_fold_survives_unrelated_scope_alias() {
    // TP control for the alias skip: an alias binding for one name must not
    // blanket-disable the fold — a *different*, never-defined local still
    // folds to I230.
    let codes = codes_for(
        "proc f {} { namespace upvar ::ns state alias\n if {[info exists other]} { puts hi } }",
    );
    assert!(
        codes.contains(&"I230".to_string()),
        "never-defined `other` must still fold beside an unrelated alias; got {codes:?}",
    );
}

// `info exists` over TclOO instance state (issue #1129)
//
// Oracle, tclsh 9.0.4 (TclOO 1.3.1) and tclsh 8.6.14 (TclOO 1.1.0), identical
// on both:
//
// ```tcl
// oo::class create C { variable x; constructor {} { set x 1 }
//                      method m {} { puts [info exists x] } }
// [C new] m                                            ;# → 1
// oo::class create D { variable x
//                      method m {} { puts [info exists x] } }
// [D new] m                                            ;# → 0
// oo::class create F { variable x; method setit {} { set x 42 }
//                      method m {} { puts [info exists x] } }
// set f [F new]; $f m ;# → 0     $f setit; $f m        ;# → 1
// oo::class create G { variable a; constructor {} { set b 7 } }
// oo::define G { variable b; method m {} {
//     puts [list a [info exists a] b [info exists b]] } }
// [G new] m                                            ;# → a 0 b 1
// ```
//
// So a class-level `variable` declaration does *not* create the variable —
// but any earlier method call on the same instance may have, which is a
// per-instance runtime fact no per-method fold can decide.  Abstain.

#[test]
fn info_exists_does_not_fold_instance_var_assigned_in_constructor() {
    // FP guard — `[C new] m` prints 1 on both runtimes, but the fold saw no
    // assignment to `x` inside `m`'s own body and called the guard always
    // false.
    let codes = codes_for(
        "oo::class create C {\n variable x\n constructor {} { set x 1 }\n \
         method m {} { if {[info exists x]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance variable assigned in the constructor must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_instance_var_assigned_in_sibling_method() {
    // FP guard — existence depends on call order on the instance
    // (`$f m` → 0 before `$f setit`, → 1 after), so neither direction folds.
    let codes = codes_for(
        "oo::class create F {\n variable x\n method setit {} { set x 42 }\n \
         method m {} { if {[info exists x]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance variable a sibling method assigns must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_instance_var_declared_in_later_define_block() {
    // FP guard, cross-definition-block shape (#1131): `variable b` is declared
    // by an `oo::define` block that lowering may walk *after* the method that
    // queries it.  `MethodDef::instance_vars` is the per-class union precisely
    // so this is order-free — the fold must abstain either way.
    let codes = codes_for(
        "oo::class create G {\n variable a\n constructor {} { set b 7 }\n}\n\
         oo::define G {\n variable b\n \
         method m {} { if {[info exists b]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance variable declared in a later oo::define block must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_my_variable_local_in_method() {
    // FP guard — `my variable x` binds the object's variable into the method
    // frame; existence is the object's, not the frame's.  Already covered by
    // the registry-driven scope-alias skip (`my variable` resolves through the
    // spec's own `ArgRole::VarWrite` resolver), pinned here so the TclOO
    // shape cannot regress with the rest of #1129.
    let codes = codes_for(
        "oo::class create E {\n method m {} { my variable x\n \
         if {[info exists x]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "a `my variable` local must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_still_folds_never_set_non_instance_local_in_method() {
    // TP guard — the abstention is name-scoped, not a blanket "no folds in
    // method bodies": `zzz` is neither instance state nor a parameter nor ever
    // assigned, so it still folds false exactly as it would inside a proc.
    let codes = codes_for(
        "oo::class create C {\n variable x\n constructor {} { set x 1 }\n \
         method m {} { if {[info exists zzz]} { puts hi } }\n}\n",
    );
    assert!(
        codes.contains(&"I230".to_string()),
        "a never-set non-instance local in a method body must still fold; got {codes:?}",
    );
}

#[test]
fn info_exists_folds_true_not_false_for_method_parameter() {
    // The second false-positive shape of #1129, found while building the
    // fix: the analyser looked a body's parameters up in
    // `ir_module.procedures`, which a *method*'s qualified name is never in
    // (methods live in `ir_module.methods`), so every method parameter read
    // as a never-defined local and the guard folded **always false** —
    // `[C new] m x` runs the `then` arm on both tclsh 9.0.4 and 8.6.14.
    // The optimiser's copy of the fold never had this bug
    // (`build_method_units` always passed `MethodDef::params`), so the two
    // consumers disagreed; both now read the same `MethodDef`.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(
        "oo::class create C {\n method m {p} { if {[info exists p]} { puts hi } }\n}\n",
    );
    let i230: Vec<&str> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::I230)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(i230.len(), 1, "a method parameter must fold; got {i230:?}");
    assert!(
        i230[0].contains("always true"),
        "a method parameter exists at entry — the fold must be true, got {i230:?}",
    );
}

#[test]
fn info_exists_does_not_fold_upvar_defined_local_across_my_dispatch() {
    // FP guard (issue #1177) — the callee reached via `my` is a method,
    // never in the upvar-procs table, so its `upvar 1 $refvar ref`
    // caller-frame definition was invisible and the guard folded always
    // false.  Oracle (tclsh 9.0.4 and 8.6.14): after `my Reference?
    // $lookup ref` returns true, `[info exists ref]` in the caller is 1;
    // on a miss, 0 — live code either way, so the fold must abstain.
    let codes = codes_for(
        "oo::class create Formatter {\n \
         method Reference? {lookup refvar} {\n upvar 1 $refvar ref\n \
         if {$lookup eq \"x\"} { set ref 1; return 1 }\n return 0\n }\n \
         method ResolvableReference? {lookup} {\n \
         my Reference? $lookup ref\n \
         if {[info exists ref]} { puts hi }\n }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an upvar-defined local across a `my` dispatch must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_upvar_defined_local_across_next_dispatch() {
    // Abstention guard (issue #1177), `next`-chain shape.  `next` names no
    // target at all, so which implementation runs — and which frame its
    // `upvar 1` lands in — needs MRO modelling this analysis does not do
    // (issue #1164).  Oracle (tclsh 9.0.4): the chained implementation's
    // `upvar 1` actually SKIPS the calling implementation's frame and
    // lands in the frame of whoever invoked the whole method
    // (`in-sub:0`, `global:1` for a top-level `[Sub new] probe`) — so a
    // per-chain answer is provable only with the chain in hand.  Until
    // then the fold must abstain rather than claim either constant.
    let codes = codes_for(
        "oo::class create Base {\n \
         method probe {} { upvar 1 status status\n set status ok\n return 1 }\n}\n\
         oo::class create Sub {\n superclass Base\n \
         method probe {} {\n next\n \
         if {[info exists status]} { puts hi }\n }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an upvar-defined local across a `next` dispatch must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_still_folds_across_my_when_no_method_reaches_the_caller_frame() {
    // TN guard (issue #1177) — the widening is evidence-gated, not a
    // blanket "no folds near `my`": when no method in the module can reach
    // its caller's frame, a dispatch cannot create locals here, so a
    // never-set non-instance local still folds always false (tclsh 9.0.4 /
    // 8.6.14: `[info exists zzz]` is 0 after `my Helper`).
    let codes = codes_for(
        "oo::class create C {\n \
         method Helper {} { return 1 }\n \
         method m {} {\n my Helper\n \
         if {[info exists zzz]} { puts hi }\n }\n}\n",
    );
    assert!(
        codes.contains(&"I230".to_string()),
        "with complete dispatch evidence the fold must survive; got {codes:?}",
    );
}

#[test]
fn read_after_my_dispatch_to_an_upvar_sibling_draws_no_read_before_set() {
    // The W210 face of the same evidence rule (issue #1177): `$ref` after
    // the dispatch is genuinely defined on the hit path — tclsh 9.0.4 /
    // 8.6.14 run it to completion.
    let codes = codes_for(
        "oo::class create Formatter {\n \
         method Reference? {lookup refvar} {\n upvar 1 $refvar ref\n \
         set ref 1\n return 1\n }\n \
         method ResolvableReference? {lookup} {\n \
         my Reference? $lookup ref\n \
         return $ref\n }\n}\n",
    );
    assert!(
        !codes.contains(&"W210".to_string()),
        "a `my` dispatch to an upvar sibling defines the local; got {codes:?}",
    );
}

/// Every I230 message emitted for `src`, in emission order.
fn i230_messages(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics(src);
    a.result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::I230)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn info_exists_folds_true_for_method_parameter_shadowing_an_instance_var() {
    // Codex P2 on PR #1175, finding A — confirmed against the oracle.  A
    // formal parameter whose name collides with a class-level `variable`
    // declaration **shadows** it outright: the name is an ordinary local that
    // always exists, and writes through it never reach object state.  So the
    // instance-variable abstention must not swallow it.
    //
    // tclsh 9.0.4 (TclOO 1.3.1) and 8.6.14 (TclOO 1.1.0), identical:
    //
    //   oo::class create A1 { variable x; constructor {} { set x 42 }
    //                         method m {x} { set r [list [info exists x] $x]
    //                                        set x 9; return $r }
    //                         method peek {} { list [info exists x] $x } }
    //   set a [A1 new]; $a m hello  ;# → exists 1 value hello
    //   $a peek                     ;# → exists 1 value 42   (`set x 9` did
    //                                                         not reach it)
    //
    // and with the instance variable never assigned, the parameter still
    // binds (`A2 in-method: exists 1 value hello`, `A2 after m: 0`).
    let msgs = i230_messages(
        "oo::class create A {\n variable x\n constructor {} { set x 42 }\n \
         method m {x} { if {[info exists x]} { puts hi } }\n}\n",
    );
    assert_eq!(
        msgs.len(),
        1,
        "the shadowing parameter must still fold; got {msgs:?}",
    );
    assert!(
        msgs[0].contains("always true"),
        "a parameter shadowing an instance variable always exists — the fold \
         must be true, got {msgs:?}",
    );
}

#[test]
fn info_exists_still_abstains_on_the_non_shadowed_instance_vars() {
    // Companion control for the shadowing case: shadowing is per-name.  `x`
    // is shadowed by the parameter and folds; the sibling instance variable
    // `y`, which nothing in this method binds, must keep abstaining.
    let msgs = i230_messages(
        "oo::class create A {\n variable x y\n constructor {} { set x 42 }\n \
         method m {x} {\n if {[info exists x]} { puts hi }\n \
         if {[info exists y]} { puts ho }\n }\n}\n",
    );
    assert_eq!(
        msgs.len(),
        1,
        "only the shadowed name may fold; got {msgs:?}",
    );
    assert!(msgs[0].contains("always true"), "got {msgs:?}");
}

#[test]
fn info_exists_frame_facts_survive_a_proc_method_qname_collision() {
    // Codex P2 on PR #1175, finding B — confirmed against the oracle.  A
    // `TclOO` method and a namespace procedure can carry the same qualified
    // name, so the per-function dispatcher must take its frame identity from
    // the caller (which knows the map it is walking) rather than probing
    // `ir_module.procedures` / `.methods` by name.
    //
    // tclsh 9.0.4 and 8.6.14, identical — note `oo::class create C` does not
    // create namespace `::C`, so the procedure needs `namespace eval` first:
    //
    //   oo::class create C { variable x; constructor {} { set x 42 }
    //                        method m {p} { list method [info exists p] $p $x } }
    //   namespace eval ::C {}
    //   proc ::C::m {q} { list proc [info exists q] $q [info exists x] }
    //   [C new] m hello   ;# → method 1 hello 42
    //   ::C::m world      ;# → proc 1 world 0
    //
    // Both frames are wholly separate: the procedure sees none of the class's
    // instance state. Pre-fix the name probe gave the *method* the
    // procedure's parameter list (so its own `[info exists p]` folded "always
    // false") and gave the *procedure* the class's instance variables (so it
    // abstained on `x`, a name it fully owns as a never-set local).
    let msgs = i230_messages(
        "oo::class create C {\n variable x\n constructor {} { set x 42 }\n \
         method m {p} { if {[info exists p]} { puts inmethod } }\n}\n\
         namespace eval ::C {}\n\
         proc ::C::m {q} {\n if {[info exists q]} { puts inproc }\n \
         if {[info exists x]} { puts nope }\n}\n",
    );
    // Method: `p` is its own parameter → always true.
    // Proc: `q` is its own parameter → always true; `x` is a never-set local
    // of the *procedure* (the class's instance state is not in scope there)
    // → always false.
    assert_eq!(
        msgs.len(),
        3,
        "each frame must fold its own three guards; got {msgs:?}",
    );
    assert_eq!(
        msgs.iter().filter(|m| m.contains("always true")).count(),
        2,
        "both parameter guards must fold true; got {msgs:?}",
    );
    assert_eq!(
        msgs.iter().filter(|m| m.contains("always false")).count(),
        1,
        "the procedure's never-set `x` must fold false — it is not object \
         state in a procedure frame; got {msgs:?}",
    );
}

#[test]
fn info_exists_folds_true_for_an_apply_lambda_parameter() {
    // Follow-on from finding B's remedy: the lambda loop now hands the
    // dispatcher the body unit's own `Procedure` rather than letting it probe
    // `ir_module.procedures` for a key that map never holds, so a lambda's
    // parameters reach the fold like a proc's.  A lambda is an anonymous
    // procedure with a fresh frame whose bound names are exactly its
    // parameter list — tclsh 9.0.4 / 8.6.14: `apply {{a} {info exists a}} 1`
    // → 1.
    let msgs = i230_messages("apply {{a} { if {[info exists a]} { puts hi } }} 1\n");
    assert_eq!(msgs.len(), 1, "a lambda parameter must fold; got {msgs:?}");
    assert!(
        msgs[0].contains("always true"),
        "a lambda parameter exists at entry, got {msgs:?}",
    );
}

#[test]
fn analyse_w307_suppressed_for_known_class_constructor_chain() {
    // ``[Dog new] bark`` — ``Dog`` is a user class so
    // ``new`` returns an Object whose class is ``Dog``.
    // The W307 cmd-sub suppression should kick in.  Since
    // ``bark`` is declared on ``Dog``, no W308 either.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} { return woof } }\n[Dog new] bark",
        "tcl",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must not fire for [KnownClass new] method chain; got {:?}",
        r.diagnostics,
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
        "W308 must not fire when method is declared on the class; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w308_emitted_for_unknown_method_on_known_class_constructor() {
    // ``[Dog new] fly`` — ``fly`` isn't declared on ``Dog``.
    // W307 is suppressed (constructor returns Object) but
    // W308 fires for the missing method.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} { return woof } }\n[Dog new] fly",
        "tcl",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "W308 expected for unknown method on known class; got {:?}",
        r.diagnostics,
    );
}

// Issue #1010 (site 4) — `emit_cmd_command_diagnostics`'s constructor
// recognition (`[Cls new] method`) typed the result as `Object(Cls)`
// even when `Cls` was renamed or deleted away with no later
// re-establishment, producing a misleading "unknown method" W308 that
// implies `Cls` exists. Confirmed against tclsh 8.6.14 that the
// constructor call itself fails "invalid command name" first — fixed so
// the dispatch instead falls back to the conservative "non-literal,
// cannot statically analyze" (W307), with W123 on `Cls` itself as the
// real, primary diagnostic.
//
// `harvest_constructor_object_types` (the `set x [Cls new]` variable-
// assignment sibling of this same check) is fixed the same way. The
// deeper, independent source in `type_infer.rs`'s `constructor_object_type`
// — which types `x` as `Object(Cls)` for that shape via the SSA type
// lattice — is gated in `aggregate_object_types`, where both sources are
// unioned and the analyser's own deletion facts are in scope (issue #1013;
// see the `w308_*_issue_1013_*` cases below).

#[test]
fn w308_tp_issue_1010_deleted_class_constructor_falls_back_to_w307() {
    let src =
        "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\n[Dog new] fly";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W308"),
        "a deleted class must not draw the misleading 'unknown method' W308; got {codes:?}"
    );
    assert!(
        codes.contains("W123"),
        "the deleted class itself must draw W123 as the real diagnostic; got {codes:?}"
    );
}

#[test]
fn w308_tp_issue_1013_deleted_class_set_var_constructor_draws_no_w308() {
    // Issue #1013 primary repro: `type_infer.rs`'s `constructor_object_type`
    // is a second, independent source of `Object(Dog)` typing for the
    // `set x [Dog new]` shape, reached through the SSA type lattice rather
    // than #1010's `harvest_constructor_object_types`. It reads an
    // unfiltered "known classes" set with no deletion awareness, so this
    // still drew the misleading "unknown method" W308 after #1010's fix.
    // tclsh8.6 and 9.0 both fail the constructor first: `invalid command
    // name "Dog"` — `x` is never assigned an object at all.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\nproc foo {} {\n    set x [Dog new]\n    $x fly\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W308"),
        "a deleted class must not draw the misleading 'unknown method' W308; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_fp_issue_1013_reestablished_class_set_var_constructor_still_flags() {
    // FP guard for the same gate: a class deleted and then re-established
    // under the same name is live again at file end (the fresh definition
    // postdates the deletion), so `x` must still type as `Object(Dog)` and
    // the unknown method must still be flagged.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\noo::class create Dog { method bark {} { return woof } }\nproc foo {} {\n    set x [Dog new]\n    $x fly\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a re-established class must still type its constructor result; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_issue_1013_live_class_set_var_constructor_still_flags() {
    // TN control: with no deletion anywhere, the ordinary `set x [Dog new]`
    // shape must be entirely unaffected by the gate.
    let src = "oo::class create Dog { method bark {} { return woof } }\nproc foo {} {\n    set x [Dog new]\n    $x fly\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a live class must still validate its methods; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_copy_propagated_handle_with_a_trailing_rename_still_flags() {
    // TP restored by moving the class-liveness gate from file end to the
    // dispatch site. `foo` runs while `Dog` is alive; the rename is the last
    // line of the file and cannot affect a call that already happened.
    //
    // The file-end gate dropped `x`'s `Object(::Dog)` type outright, so the
    // dispatch lost its W308 *and* picked up a spurious W307 in its place.
    //
    // Oracle (tclsh8.6, `review-probes-sound/w308d.tcl`): exits 1 with
    // `unknown method "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nproc foo {} {\n    set y [Dog new]\n    set x $y\n    $x fly\n}\nfoo\nrename Dog {}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "the class is alive at the dispatch, so the unknown method must flag; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_class_renamed_to_a_name_keeps_typing_its_existing_objects() {
    // `rename Dog Cat` deletes the command *name* `Dog` but not the class —
    // the object in `d` is still a Dog and still rejects `fly`.
    //
    // Oracle (tclsh8.6 and tclsh9.0): after `set d [Dog new]` and `rename
    // Dog Cat`, `$d bark` returns `woof` while `$d fly` fails with `unknown
    // method "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nset d [Dog new]\nrename Dog Cat\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a class renamed to a name is re-established, not deleted; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_a_class_renamed_away_still_flags_its_old_name() {
    // The paired guard: making the *class* survive a rename must not make
    // the vacated *command name* resolve. Oracle (tclsh8.6,
    // `review-probes/cls1_run.tcl`): after `rename Dog Cat`, `Dog new`
    // fails with `invalid command name "Dog"`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nDog new\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        codes.contains("W123"),
        "the vacated name must still be unknown; got {codes:?}",
    );
}

#[test]
fn w123_tp_a_rename_in_a_dead_branch_is_not_an_unconditional_deletion() {
    // A `rename` nested in a control-flow body may never run, so it is not
    // evidence the command is gone. The analyser proves this very branch
    // dead in the same run (it emits I230 on it) yet honoured the deletion,
    // flagging a command that is demonstrably still callable and — through
    // the shared liveness facts — withdrawing the W308 as well.
    //
    // Oracle (tclsh8.6, `review-probes/cls5.tcl`): `Dog new` succeeds and
    // `$d fly` fails with `unknown method "fly": must be bark or destroy`.
    let src = "oo::class create Dog {\n    method bark {} { return woof }\n}\nif {0} { rename Dog {} }\nset d [Dog new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W123"),
        "the branch never runs, so `Dog` is still bound; got {:?}",
        r.diagnostics,
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "and the live class must still validate its methods; got {:?}",
        r.diagnostics,
    );
    assert!(
        codes.contains("I230"),
        "the dead branch itself is still reported; got {codes:?}",
    );
}

#[test]
fn w308_tp_a_renamed_class_name_still_types_its_constructor_1049() {
    // TP — `rename Dog Cat` moves the class *command*; the class itself is
    // unchanged, so `Cat new` builds a Dog and `$d fly` is still an unknown
    // method. Before #1049 the constructor did not type at all, so the
    // dispatch fell through to W307 noise instead.
    //
    // Oracle (tclsh8.6.14 and tclsh9.0.4): `oo::class create Dog { method
    // bark {} { return woof } }; rename Dog Cat; set d [Cat new]; $d fly`
    // fails with `unknown method "fly": must be bark or destroy`, while
    // `$d bark` returns `woof`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nset d [Cat new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a class reached through a rename still validates its methods; got {:?}",
        r.diagnostics,
    );
    // The object is typed, so the dispatch is not an unanalysable one.
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a typed dispatch must not also draw W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_a_renamed_class_still_accepts_its_real_methods_1049() {
    // TN — the same shape with a method the class really has draws nothing.
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `$d bark` → `woof`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nset d [Cat new]\n$d bark\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.is_empty(),
        "a valid method on a renamed class is silent; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_a_constructor_written_before_its_rename_does_not_type_1049() {
    // TN — the order gate. At top level the rename has not run yet when
    // `[Cat new]` executes, so `Cat` is simply an unknown command there and
    // nothing may be typed from it. The paired guard for
    // `w123_tp_a_class_renamed_away_still_flags_its_old_name`, from the
    // other side of the rename.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; Cat new`
    // — before any `rename` — fails with `invalid command name "Cat"`.
    // The out-of-order call itself is W128's ("called after it was renamed or
    // deleted earlier in this file" — the same order fact from the other
    // side); W123 owns the case where the name is never established at all,
    // as `w123_tp_a_class_renamed_away_still_flags_its_old_name` pins.
    let src = "oo::class create Dog { method bark {} { return woof } }\nCat new\nset d [Cat new]\nrename Dog Cat\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W308"),
        "nothing may be typed from a name the rename has not created yet; got {:?}",
        r.diagnostics,
    );
    assert!(
        codes.contains("W128"),
        "and the out-of-order use is still reported; got {codes:?}",
    );
}

#[test]
fn w308_tp_a_chain_of_renames_still_reaches_the_class_1049() {
    // TP — `rename Dog Cat; rename Cat Kitten` leaves the class reachable
    // under the last name; the hop walk follows the chain (bounded).
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; rename
    // Dog Cat; rename Cat Kitten; set d [Kitten new]` succeeds, and
    // `$d fly` fails `unknown method "fly"`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nrename Cat Kitten\nset d [Kitten new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a chained rename still reaches the class; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_an_interp_alias_of_a_class_types_its_constructor_1049() {
    // TP — `interp alias {} Cat {} Dog` makes `Cat` dispatch to the class
    // command, so `Cat new` builds a Dog exactly as `Dog new` does.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; interp
    // alias {} Cat {} Dog; set d [Cat new]` returns an `::oo::Obj*` handle.
    let src = "oo::class create Dog { method bark {} { return woof } }\ninterp alias {} Cat {} Dog\nset d [Cat new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "an aliased class command still types its constructor; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_an_alias_with_prepended_args_is_declined_1049() {
    // TN — `interp alias {} Cat {} Dog create` prepends words, so `Cat new`
    // is really `Dog create new`, not the constructor call the plain alias
    // would make. The hop declines rather than mistyping.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): with that alias, `Cat new` runs
    // `Dog create new` — it creates an *object command named `new`*, and the
    // value is `::new`, not a fresh anonymous handle.
    let src = "oo::class create Dog { method bark {} { return woof } }\ninterp alias {} Cat {} Dog create\nset d [Cat new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
        "a prepended-args alias must not be typed as a plain constructor; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_an_alias_to_a_renamed_class_name_reaches_the_class_1049() {
    // TP — the alias's target is not a directly-live class but a *rename
    // destination*: `Cat` resolves onward to the class still keyed `::Dog`.
    // The alias hop must keep walking the chain and judge liveness where it
    // terminates, not demand a class at the hop itself.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; rename
    // Dog Cat; interp alias {} Pup {} Cat; set d [Pup new]` — `$d bark` →
    // `woof`, `$d fly` fails `unknown method "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\ninterp alias {} Pup {} Cat\nset d [Pup new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "an alias to a rename destination still reaches the class; got {:?}",
        r.diagnostics,
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a typed dispatch must not also draw W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_a_chain_of_aliases_reaches_the_class_1049() {
    // TP — an alias whose target is another alias resolves hop by hop, the
    // way Tcl re-resolves each name at invocation.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `interp alias {} Cat {} Dog; interp
    // alias {} Pup {} Cat; set d [Pup new]; $d fly` fails `unknown method
    // "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\ninterp alias {} Cat {} Dog\ninterp alias {} Pup {} Cat\nset d [Pup new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a chain of aliases still reaches the class; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_an_alias_to_the_vacated_source_name_stays_untyped_1049() {
    // TN — the guard the chain walk must not lose: an alias pointing at a
    // rename *source* lands on a vacated name. The class survives under
    // `::Cat`, but the alias re-resolves `Dog` by name at each call and
    // finds nothing — the lenient rename-source rule that keeps existing
    // objects typed must not resurrect the name for an alias.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `rename Dog Cat; interp alias {}
    // Pup {} Dog; Pup new` fails `invalid command name "Dog"`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\ninterp alias {} Pup {} Dog\nset d [Pup new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
        "an alias to a vacated name resolves to nothing and must not type; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_a_renamed_class_types_a_direct_constructor_dispatch_1049() {
    // TP — the same fix at the direct-dispatch site (`[Cat new] fly`), which
    // types the constructor result inline rather than through a variable.
    let src =
        "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\n[Cat new] fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a direct dispatch on a renamed constructor still validates; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_fp_issue_1010_reestablished_class_constructor_still_flags_unknown_method() {
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\noo::class create Dog { method bark {} { return woof } }\n[Dog new] fly";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a class re-established after deletion must still resolve its constructor normally; got {:?}",
        r.diagnostics,
    );
}

/// Object-`of` constructor typing — the W307/W308 item.  Each
/// case asserts the exact set of expected diagnostic codes.
fn w30x_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let mut codes: Vec<String> = r
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .filter(|c| c.starts_with("W30"))
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

#[test]
fn w308_transitive_alias_of_constructor_object() {
    // `set b $a` copies the OBJECT(Dog) type through the lattice, so the
    // unknown method on `$b` is validated (W308).
    let src = "oo::class create Dog { method bark {} {return woof} }\n\
                   set a [Dog new]\nset b $a\n$b fly";
    assert_eq!(w30x_codes(src), vec!["W308".to_string()]);
}

#[test]
fn w308_transitive_alias_known_method_silent() {
    let src = "oo::class create Dog { method bark {} {return woof} }\n\
                   set a [Dog new]\nset b $a\n$b bark";
    assert!(w30x_codes(src).is_empty());
}

#[test]
fn w308_constructor_create_named_and_auto() {
    // `Dog create rex` / `Dog create %AUTO%` are constructor spellings too.
    for src in [
        "oo::class create Dog { method bark {} {return woof} }\nset a [Dog create rex]\n$a fly",
        "oo::class create Dog { method bark {} {return woof} }\nset a [Dog create %AUTO%]\n$a fly",
    ] {
        assert_eq!(w30x_codes(src), vec!["W308".to_string()], "src: {src}");
    }
}

#[test]
fn w308_namespace_scoped_class_constructor() {
    // Class defined in a namespace; the relative constructor head inside
    // that namespace resolves to the qualified class (OBJECT(::ns::Dog)).
    let qualified = "namespace eval ns { oo::class create Dog { method bark {} {return woof} } }\n\
                         set a [ns::Dog new]\n$a fly";
    assert_eq!(w30x_codes(qualified), vec!["W308".to_string()]);
    let relative = "namespace eval ns {\n oo::class create Dog { method bark {} {return woof} }\n \
                        proc mk {} { set a [Dog new]; $a fly }\n}";
    assert_eq!(w30x_codes(relative), vec!["W308".to_string()]);
}

#[test]
fn w307_suppressed_for_object_returning_proc_factory() {
    // A proc returning `[Dog new]` is an object factory; a `$o method`
    // dispatch on its result suppresses W307.  Since the object-type
    // lattice's proc-return edge became a W308 input (issues #994 / #1143),
    // the factory-returned handle also carries its *class*: a known method
    // stays silent and an unknown one is validated as W308 — the same
    // answer hover / go-to-definition give for `o`.
    let known = "oo::class create Dog { method bark {} {return woof} }\n\
                 proc mk {} { return [Dog new] }\nset o [mk]\n$o bark";
    assert!(
        w30x_codes(known).is_empty(),
        "factory-return dispatch of a declared method must be silent; got {:?}",
        w30x_codes(known)
    );
    let unknown = "oo::class create Dog { method bark {} {return woof} }\n\
                   proc mk {} { return [Dog new] }\nset o [mk]\n$o fly";
    assert_eq!(
        w30x_codes(unknown),
        vec!["W308".to_owned()],
        "an unknown method on the factory-typed handle is W308, not W307"
    );
}

#[test]
fn w307_nested_command_sub_dispatch_counts_for_multidispatch() {
    // `$x` is dispatched once at statement level and once inside a `[…]`
    // command substitution.  The substitution dispatch must be recorded as
    // a var-command site too, so the multi-dispatch (≥2) suppression sees
    // both and stays silent (a single recorded dispatch would fire W307).
    let src = "set x [getCmd]\nputs [$x foo]\n$x foo\n";
    assert!(
        w30x_codes(src).is_empty(),
        "multi-dispatch (one nested in `[…]`) must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn w308_double_colon_oo_class_constructor() {
    // `::oo::class` is the fully-qualified spelling of `oo::class`; the
    // class must be recognised so the constructor types as OBJECT and the
    // method is validated (W308 for the unknown one, silence for the known).
    let unknown = "::oo::class create ::Dog { method bark {} {return woof} }\n[::Dog new] fly";
    assert_eq!(w30x_codes(unknown), vec!["W308".to_string()]);
    let known = "::oo::class create ::Dog { method bark {} {return woof} }\n[::Dog new] bark";
    assert!(w30x_codes(known).is_empty());
}

// TclOO method / `forward` arity (generalises E002/E003 to object
// dispatch), all verified against real tclsh 9.0.4.

fn e00x_codes_for(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let mut codes: Vec<String> = r
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .filter(|c| c == "E002" || c == "E003")
        .collect();
    codes.sort();
    codes
}

#[test]
fn tcloo_method_arity_fires_and_stays_silent() {
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{ method bar {{x y}} {{ return \"$x+$y\" }} }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 bar 1")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$f1 bar 1 2")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 bar 1 2 3")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_loop_installed_method_arity_abstains() {
    // Issue #1277: a literal `foreach`-installed member's parameter list is
    // deliberately never re-derived (`params_computed`), so a call of any
    // arity must draw neither E002 nor E003 — tclsh 9.0.4 / 8.6.16 both
    // agree `alpha` really does take any number of arguments (`args`), but
    // even if it took none this must still abstain: the point is that the
    // analyser never claims to know.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{\n\
                 foreach m {{alpha beta gamma}} {{ method $m {{args}} {{ return $args }} }}\n\
             }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 alpha")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 alpha 1 2 3 4 5")),
        Vec::<String>::new()
    );
    // Non-vacuity: an *ordinary*, non-loop-installed method with the exact
    // same call shape still fires — proving this test would catch a
    // reversion of the fix (a fixed two-parameter method really does draw
    // E002/E003 on the same harness).
    let ordinary = |call: &str| {
        format!(
            "oo::class create Widget2 {{ method bar {{x y}} {{ return \"$x+$y\" }} }}\n\
             set f1 [Widget2 new]\n{call}\n"
        )
    };
    assert_eq!(
        e00x_codes_for(&ordinary("$f1 bar 1")),
        vec!["E002".to_owned()]
    );
}

#[test]
fn tcloo_method_with_default_and_trailing_args_is_unbounded() {
    // tclsh 9.0.4: `method baz {x {y 1} args}` accepts any count >= 1.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{ method baz {{x {{y 1}} args}} {{ return $x }} }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 baz 1")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 baz 1 2 3 4")),
        Vec::<String>::new()
    );
    assert_eq!(e00x_codes_for(&src("$f1 baz")), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_forward_arity_is_shifted_by_prepended_args() {
    // tclsh 9.0.4: `forward fwd target_orig` (no prepended args) inherits
    // target_orig's own 3-arg arity exactly.
    let src = |call: &str| {
        format!(
            "proc target_orig {{a b c}} {{}}\n\
             oo::class create Widget {{ forward fwd target_orig }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2 3")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 fwd 1 2 3 4")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_forward_to_bare_method_name_abstains() {
    // A bare method name is *never* a resolvable `forward` target — tclsh
    // 9.0.4 fails at run time with `invalid command name "base"`, since
    // `forward`'s TARGET is looked up as an ordinary command and a
    // `method` never creates one. Must abstain, not guess an arity.
    let src = "\
oo::class create Bad { method base {a b c} {} forward fwdBad base }
set b1 [Bad new]
$b1 fwdBad 1 2 3 4 5
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_forward_via_my_resolves_sibling_method_arity() {
    // `forward NAME my TARGET ?ARG…?` is the documented, working idiom
    // for forwarding to a sibling method (`self` is not usable here —
    // tclsh 9.0.4: it errors "self may only be called from inside a
    // method" since forwarding doesn't run inside a method body). `my`'s
    // target resolves through the receiver's own MRO, arity-shifted by
    // any args placed after it — confirmed against tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{\n\
               method base {{a b c}} {{ return \"$a$b$c\" }}\n\
               forward fwd my base\n\
               forward fwdShift my base fixedarg\n\
             }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2 3")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 fwd 1 2 3 4")),
        vec!["E003".to_owned()]
    );
    assert_eq!(
        e00x_codes_for(&src("$f1 fwdShift 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        e00x_codes_for(&src("$f1 fwdShift 1 2")),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for(&src("$f1 fwdShift 1 2 3")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_forward_via_my_resolves_inherited_method_arity() {
    // `my`'s target resolution walks the receiver's full MRO, so a
    // `forward … my …` declared on a subclass reaches a method defined
    // only on a superclass — confirmed against tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Base {{ method base {{a b c}} {{ return \"$a$b$c\" }} }}\n\
             oo::class create Derived {{ superclass Base\n forward fwd my base }}\n\
             set d1 [Derived new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$d1 fwd 1 2")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$d1 fwd 1 2 3")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$d1 fwd 1 2 3 4")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_method_arity_abstains_for_ambiguous_receiver_class() {
    // `$f1` could hold either `A` or `B` — disjoint arities for `same` —
    // must not guess; no E002/E003 either way.
    let src = "\
oo::class create A { method same {x} {} }
oo::class create B { method same {x y} {} }
if {$cond} { set f1 [A new] } else { set f1 [B new] }
$f1 same 1
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_instance_dispatch_abstains_for_class_side_method() {
    // A `self method` (stored in `class_methods`) is called on the class
    // object, never on an instance — confirmed against tclsh 9.0.4:
    // `set o [C new]; $o make 1` fails "unknown method", while `C make
    // 1 2` succeeds. Must abstain (no E002/E003), not compute arity from
    // a signature the instance dispatch could never actually reach.
    let src = "\
oo::class create C {
    self method make {a b} { return \"$a$b\" }
}
set o [C new]
$o make 1
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

// -- TclOO constructor call-site arity (`ClassName new` / `ClassName create`)

#[test]
fn tcloo_constructor_new_arity_fires_and_stays_silent() {
    let src =
        |call: &str| format!("oo::class create Widget {{ constructor {{a b}} {{ }} }}\n{call}\n");
    assert_eq!(
        e00x_codes_for(&src("Widget new 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(e00x_codes_for(&src("Widget new 1 2")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("Widget new 1 2 3")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_constructor_create_arity_accounts_for_mandatory_name() {
    // `create` consumes one mandatory word (the object name) ahead of the
    // constructor's own parameters — confirmed against tclsh 9.0.4:
    // `Widget create fido 1` fails "should be Widget create objectName a b".
    let src =
        |call: &str| format!("oo::class create Widget {{ constructor {{a b}} {{ }} }}\n{call}\n");
    assert_eq!(
        e00x_codes_for(&src("Widget create fido 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        e00x_codes_for(&src("Widget create fido 1 2")),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for(&src("Widget create")),
        vec!["E002".to_owned()],
        "the mandatory object-name word itself must be enforced"
    );
}

#[test]
fn tcloo_constructor_createwithnamespace_arity_accounts_for_two_mandatory_words() {
    // `createWithNamespace` consumes two mandatory words (the object name
    // and the target namespace) ahead of the constructor's own
    // parameters — same word layout as the sibling class-*definition*
    // shape `oo::class createWithNamespace Name ::ns body` (see
    // `oo_class_arg_roles`), just with constructor args standing in for
    // the definition body. Unlike `new`/`create`, `createWithNamespace` is
    // unexported by default (confirmed against tclsh 9.0.4), so the class
    // object must `self export` it for an external
    // call to even reach the constructor — see the companion
    // `..._is_not_checked_when_not_exported` test for the unexported case.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{ constructor {{a b}} {{ }}; self export createWithNamespace }}\n{call}\n"
        )
    };
    assert_eq!(
        e00x_codes_for(&src("Widget createWithNamespace fido ::ns 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        e00x_codes_for(&src("Widget createWithNamespace fido ::ns 1 2")),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for(&src("Widget createWithNamespace fido ::ns 1 2 3")),
        vec!["E003".to_owned()]
    );
    assert_eq!(
        e00x_codes_for(&src("Widget createWithNamespace fido ::ns")),
        vec!["E002".to_owned()],
        "the mandatory object-name and namespace words themselves must be enforced"
    );
}

#[test]
fn tcloo_constructor_createwithnamespace_is_not_checked_when_not_exported() {
    // `createWithNamespace` is unexported by default (confirmed against
    // tclsh 9.0.4) — an external call to a class that
    // never `export`s it raises "unknown method" at run time and never
    // reaches the constructor, so it must not be arity-checked. This is
    // a false positive.
    let src = "\
oo::class create Widget { constructor {a b} { } }
Widget createWithNamespace fido ::ns 1
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_is_inherited_through_superclass() {
    // `Sub` declares no constructor of its own — `Base`'s is inherited
    // (confirmed against tclsh 9.0.4: a subclass with no `constructor`
    // block uses the nearest ancestor's).
    let src = "\
oo::class create Base { constructor {a b} { } }
oo::class create Sub { superclass Base }
Sub new 1
";
    assert_eq!(e00x_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_own_overrides_inherited() {
    let src = "\
oo::class create Base { constructor {a b} { } }
oo::class create Sub { superclass Base; constructor {a} { } }
Sub new 1 2
";
    assert_eq!(
        e00x_codes_for(src),
        vec!["E003".to_owned()],
        "Sub's own 1-arg constructor must win over Base's 2-arg one"
    );
}

#[test]
fn tcloo_no_explicit_constructor_anywhere_is_never_arity_checked() {
    // TclOO's default (inherited from `oo::object`) constructor accepts and
    // ignores any number of arguments — confirmed against tclsh 9.0.4:
    // `oo::class create Foo {}` then `Foo new 1 2 3` succeeds.
    let src = "oo::class create Widget { method bar {} { } }\nWidget new 1 2 3 4 5\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_create_mandatory_name_is_checked_even_without_a_constructor() {
    // Regression: a class with no explicit constructor anywhere in its MRO
    // used to abstain from arity-checking `create` entirely, not just the
    // constructor's own (unconstrained) parameters. `create`'s mandatory
    // leading object-name word is enforced by the dispatcher itself,
    // independent of the constructor -- confirmed against tclsh 9.0.4:
    // `oo::class create Foo {}` then `Foo create` (no name) still raises
    // "wrong # args", even though any number of trailing args succeeds.
    let src = |call: &str| format!("oo::class create Widget {{ method bar {{}} {{ }} }}\n{call}\n");
    assert_eq!(
        e00x_codes_for(&src("Widget create")),
        vec!["E002".to_owned()],
        "the mandatory object-name word must still be enforced"
    );
    assert_eq!(
        e00x_codes_for(&src("Widget create fido")),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for(&src("Widget create fido 1 2 3 4 5")),
        Vec::<String>::new(),
        "the unconstrained default constructor still accepts any trailing args"
    );
    assert_eq!(
        e00x_codes_for(&src("Widget new")),
        Vec::<String>::new(),
        "`new` has no mandatory name word, so it stays unchecked as before"
    );
}

#[test]
fn tcloo_constructor_arity_snit_new_is_not_checked() {
    // A snit type's `new`/`create` (if it even has one — snit instantiates
    // via `TypeName instanceName ?args?`, not `new`/`create`) must never be
    // arity-checked against a `TclOO` constructor; snit is a wholly
    // different object system.
    let src = "\
snit::type Widget {
    constructor {a b} { }
}
Widget new 1
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_dynamic_expand_abstains() {
    // `{*}`-expanded args can't be counted statically — matches the same
    // any-uncertainty-abstains convention as every other arity path here.
    let src = "\
oo::class create Widget { constructor {a b} { } }
set args {1}
Widget new {*}$args
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_forward_reference_in_proc_body_resolves() {
    // A proc body runs after the whole file loads, so a class defined
    // *later* in the file still resolves — not order-gated, mirroring
    // `queue_user_call_arity_candidate`'s proc-body convention.
    let src = "\
proc make {} { Widget new 1 }
oo::class create Widget { constructor {a b} { } }
";
    assert_eq!(e00x_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_arity_via_oo_define_after_class_create() {
    // The constructor need not be declared inline in `oo::class create` —
    // a later `oo::define ClassName { constructor {...} {...} }` populates
    // the same `ClassDef::constructors` slot the arity check reads.
    let src = "\
oo::class create Widget {}
oo::define Widget {
    constructor {a b} { }
}
Widget new 1
";
    assert_eq!(e00x_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_arity_empty_body_is_not_a_real_constructor() {
    // `constructor {a b} {}` (a literally empty body) is TclOO's own way
    // of writing "no constructor" — confirmed against tclsh 9.0.4:
    // `info class constructor` returns empty and `new` with any argument
    // count succeeds. Codex review finding (PR #852): the arity check must
    // not treat this as a real, arity-enforcing constructor.
    let src = "oo::class create Widget { constructor {a b} {} }\nWidget new 1 2 3\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_whitespace_or_comment_body_is_still_real() {
    // By contrast, ANY body content — even a single space or a
    // comment-only body — keeps the constructor's arity fully enforced
    // (confirmed against tclsh 9.0.4). Must not over-correct the empty-body
    // exclusion into treating every trivial-looking body as absent.
    let src = "oo::class create Widget { constructor {a b} { } }\nWidget new 1\n";
    assert_eq!(e00x_codes_for(src), vec!["E002".to_owned()]);
    let src_comment = "oo::class create Widget {\n    constructor {a b} {\n        # just a comment\n    }\n}\nWidget new 1\n";
    assert_eq!(e00x_codes_for(src_comment), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_arity_honours_definition_order_for_top_level_call() {
    // Codex review finding (PR #852): a top-level call made *before* a
    // later `oo::define` adds the constructor sees the class as it stood
    // at that point — no constructor yet, so TclOO's permissive default
    // applies (confirmed against tclsh 9.0.4).
    let src = "\
oo::class create Widget {}
Widget new 1
oo::define Widget {
    constructor {a b} { }
}
";
    assert_eq!(
        e00x_codes_for(src),
        Vec::<String>::new(),
        "the call precedes the constructor's own definition, not just the class's"
    );
}

#[test]
fn tcloo_constructor_arity_redefinition_uses_the_one_in_effect_at_the_call() {
    // A call between two constructor redefinitions sees the one that was
    // in effect at that point, not the file's final constructor —
    // mirrors `resolve_indirect_call_target`'s identical convention for a
    // same-file proc redefinition.
    let src = "\
oo::class create Widget {
    constructor {a} { }
}
Widget new 1
oo::define Widget {
    constructor {a b} { }
}
Widget new 1 2
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_proc_body_call_not_order_gated_against_later_define() {
    // Inside a proc body, order doesn't matter (the whole file loads
    // before any proc body runs) — same convention as every other
    // arity path here.
    let src = "\
oo::class create Widget {}
proc make {} { Widget new 1 }
oo::define Widget {
    constructor {a b} { }
}
";
    assert_eq!(e00x_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn apply_lambda_arity_suppressed_when_apply_itself_is_shadowed() {
    // Codex review finding (PR #852): `apply` is an ordinary command name
    // and can be shadowed by a user proc — confirmed against tclsh 9.0.4,
    // a user-defined `apply` resolves ahead of the language builtin. The
    // lambda-literal-shaped argument must not be arity-checked against a
    // builtin `apply` that this call never actually reaches.
    let src = "proc apply {lambda x} { return $x }\napply {{a b} {}} 1\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_lambda_arity_still_checked_when_apply_is_not_shadowed() {
    // Regression guard: the shadowing fix above must not silence the
    // ordinary, non-shadowed case.
    let src = "apply {{a b} {return [expr {$a+$b}]}} 1\n";
    assert_eq!(e00x_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_arity_top_level_before_class_definition_abstains() {
    // A top-level call textually before the class exists fails at run time
    // ("invalid command name"), not with a constructor-arity mismatch —
    // matches the same order-gating convention as a shadowing proc.
    let src = "\
Widget new 1
oo::class create Widget { constructor {a b} { } }
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

// -- TclOO `next` / `nextto` call-site arity

#[test]
fn tcloo_next_arity_checked_against_superclass_override() {
    // `next` inside `Derived::speak` invokes `Base::speak` — a 2-param
    // method, so `next` itself needs exactly 2 arguments — confirmed
    // against tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Base {{ method speak {{a b}} {{ return \"$a$b\" }} }}\n\
             oo::class create Derived {{\n\
               superclass Base\n\
               method speak {{a b}} {{ {call} }}\n\
             }}\n\
             [Derived new] speak x y\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("next 1")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("next 1 2")), Vec::<String>::new());
    assert_eq!(e00x_codes_for(&src("next 1 2 3")), vec!["E003".to_owned()]);
}

#[test]
fn tcloo_nextto_arity_checked_against_named_target() {
    // `nextto Root` jumps straight to `Root::speak` (1 param) rather than
    // walking the MRO from `Derived`, skipping `Mid` — confirmed against
    // tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Root {{ method speak {{a}} {{ return $a }} }}\n\
             oo::class create Mid {{ superclass Root\n method speak {{a b}} {{ return \"$a$b\" }} }}\n\
             oo::class create Derived {{\n\
               superclass Mid\n\
               method speak {{a b}} {{ {call} }}\n\
             }}\n\
             [Derived new] speak x y\n"
        )
    };
    assert_eq!(
        e00x_codes_for(&src("nextto Root 1 2")),
        vec!["E003".to_owned()]
    );
    assert_eq!(e00x_codes_for(&src("nextto Root 1")), Vec::<String>::new());
}

#[test]
fn tcloo_next_arity_silent_when_no_further_provider() {
    // `Base` declares no superclass override of `speak` — `next` here
    // has no provider to resolve arity against (a real `next` in this
    // position is itself a runtime error, "no next" — not an arity
    // mismatch this check models). Must not invent E002/E003 either way.
    let src =
        "oo::class create Base { method speak {a b} { next 1 2 3 4 5 } }\n[Base new] speak x y\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_next_arity_silent_outside_method_body() {
    // A bareword `next` at top level (or inside an ordinary proc) is not
    // inside any method's calling frame — confirmed against tclsh 9.0.4:
    // it fails "next may only be called from inside a method", not an
    // arity mismatch. `current_method_context` returns `None`, so the
    // candidate is dropped, not checked against some unrelated method.
    let src = "\
oo::class create Base { method speak {a b} { return \"$a$b\" } }
proc helper {} { next 1 2 3 4 5 }
helper
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_nextto_arity_silent_for_unresolvable_target_class() {
    // `nextto` naming a class the analyser doesn't locally know (an
    // external / cross-file / dynamically-loaded class) must abstain
    // rather than guess.
    let src = "\
oo::class create Base { method speak {a b} { next 1 2 } }
oo::class create Derived {
    superclass Base
    method speak {a b} { nextto SomeExternalClass 1 2 3 }
}
[Derived new] speak x y
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_next_arity_expansion_never_false_fires_too_few() {
    // `{*}`-expanded args make the true count a lower bound only — must
    // never false-fire E002, matching every other arity check's
    // `{*}`-expansion convention.
    let src = "\
oo::class create Base { method speak {a b} { return \"$a$b\" } }
oo::class create Derived {
    superclass Base
    method speak {a b} { set rest {1}; next {*}$rest }
}
[Derived new] speak x y
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_next_arity_trait_bit_does_not_collide_with_structurally_checked_arity() {
    // Regression: `Traits::TCLOO_NEXT_CHAIN` once reused the same bit as
    // `Traits::STRUCTURALLY_CHECKED_ARITY`, so every command carrying the
    // latter (e.g. `if`) was also seen as carrying the former. An ordinary
    // `if` inside a TclOO method body would then get queued as a bogus
    // next/nextto arity candidate and checked against the superclass
    // override's arity — here `Base::speak` takes 5 params, so a
    // 2-argument `if $a {puts hi}` would misfire E002 ("too few
    // arguments") if the collision were still present.
    let src = "\
oo::class create Base { method speak {a b c d e} { return $a } }
oo::class create Derived {
    superclass Base
    method speak {a b c d e} { if {$a} { puts hi } }
}
[Derived new] speak 1 2 3 4 5
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

// -- `apply {{params} body}` direct-call arity

#[test]
fn apply_lambda_arity_fires_and_stays_silent() {
    assert_eq!(
        e00x_codes_for("apply {{a b} {return [expr {$a+$b}]}} 1\n"),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        e00x_codes_for("apply {{a b} {return [expr {$a+$b}]}} 1 2\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for("apply {{a b} {return [expr {$a+$b}]}} 1 2 3\n"),
        vec!["E003".to_owned()]
    );
}

#[test]
fn apply_lambda_with_default_param_is_silent_when_omitted() {
    let src = "apply {{a {b 2}} {return [expr {$a+$b}]}} 1\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_lambda_with_args_catchall_is_unbounded() {
    let src = "apply {{a args} {return $a}} 1 2 3 4 5\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_dynamic_lambda_abstains() {
    // `apply $lambda …` — the lambda literal isn't statically visible, so
    // nothing here can be counted; must not false-fire.
    let src = "set lambda {{a b} {return $a}}\napply $lambda 1\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_expand_args_abstains_too_few() {
    let src = "set rest {1}\napply {{a b} {return $a}} {*}$rest\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

// ---------------------------------------------------------------------
// E001 (`TclOO` form): `$obj` invoked with no method word at all.
// ---------------------------------------------------------------------

#[test]
fn tp_e001_bare_tcloo_object_dispatch_requires_method() {
    // `set o [Dog new]; $o` — tclsh 9.0.4: `wrong # args: should be "o
    // method ?arg ...?"`. The dispatcher checks argument count before any
    // method lookup, so this is unconditional — unlike an unknown *named*
    // method (W308), there is no `unknown`-handler fallback that could
    // save it.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               set o [Dog new]\n$o\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let e001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E001)
        .collect();
    assert_eq!(
        e001.len(),
        1,
        "expected one E001 for bare `$o`: {:?}",
        r.diagnostics
    );
    assert_eq!(e001[0].message, "'o' requires a method");
    assert_eq!(e001[0].severity, Severity::Error);
}

#[test]
fn tp_e001_bare_tcloo_object_dispatch_named_constructor() {
    // `Dog create rex` names the instance directly; `$rex` bare still
    // requires a method.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               set rex [Dog create rex]\n$rex\n";
    assert!(
        has_code(src, "tcl", "E001"),
        "bare dispatch on a named-constructor instance should require a method"
    );
}

#[test]
fn tp_e001_bare_dispatch_fires_even_when_class_is_ambiguous() {
    // Unlike per-method arity (which abstains under ambiguity — see
    // `tcloo_method_arity_abstains_for_ambiguous_receiver_class`), a
    // bare dispatch's "missing method" failure is universal across every
    // `TclOO` instance regardless of which candidate class it resolves
    // to, so ambiguity between two known `TclOO` classes does not
    // suppress it.
    let src = "\
oo::class create A { method same {x} {} }
oo::class create B { method same {x y} {} }
if {$cond} { set f1 [A new] } else { set f1 [B new] }
$f1
";
    assert!(
        has_code(src, "tcl", "E001"),
        "bare dispatch should still fire regardless of which known TclOO class it is: {:?}",
        {
            let mut a = Analyser::new();
            a.analyse(src, "tcl").diagnostics
        }
    );
}

#[test]
fn fp_e001_bare_snit_instance_dispatch_not_flagged() {
    // snit's generated dispatcher proc is a different mechanism this
    // analyser does not model precisely enough to make the same
    // guarantee — must not assume it shares `TclOO`'s unconditional
    // "wrong # args" behaviour on a bare call.
    let src = "snit::type Dog { method bark {} { return woof } }\nDog t\n$t\n";
    assert!(
        !has_code(src, "tcl", "E001"),
        "bare snit instance dispatch must not fire E001 (unmodelled dispatcher)"
    );
}

#[test]
fn tn_e001_bare_dispatch_silent_when_method_present() {
    // Sanity: adding the bare-dispatch check must not leak into the
    // ordinary `$obj method` shape — that path stays W308's job.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               set o [Dog new]\n$o bark\n";
    assert!(!has_code(src, "tcl", "E001"));
}

#[test]
fn tn_e001_bare_dispatch_silent_for_unclassified_variable() {
    // A bare `$x` where `x` is never proven to hold a `TclOO` object
    // takes the ordinary W307 (non-literal command) path, not this
    // `TclOO`-specific E001 — the two must not overlap.
    let src = "proc walk {x} { $x }\n";
    assert!(!has_code(src, "tcl", "E001"));
}

#[test]
fn tp_e001_bare_command_substitution_head() {
    // Issue #1200 (previously a documented gap): `[Dog new]` used directly
    // as a command runs `Dog new`, then invokes the produced object with no
    // method word — tclsh 9.0.3/9.0.4 fail with `wrong # args: should be
    // "::oo::Obj… method ?arg ...?"` (`-errorcode {TCL WRONGARGS}`).
    let src = "oo::class create Dog { method bark {} { return woof } }\n[Dog new]\n";
    assert!(
        has_code(src, "tcl", "E001"),
        "a bare `[Dog new]` command-substitution head must fire E001"
    );
}

#[test]
fn tp_e001_bare_cmd_head_named_constructor_and_alias() {
    // `[Dog create rex]` returns the object command too, and an
    // `interp alias` / `rename` reaching the class is the same constructor
    // (the indirection resolver types it) — both bare uses fail identically.
    let named = "oo::class create Dog { method bark {} { return woof } }\n[Dog create rex]\n";
    assert!(
        has_code(named, "tcl", "E001"),
        "bare `[Dog create rex]` head must fire E001"
    );
    let renamed = "oo::class create Dog { method bark {} { return woof } }\n\
                   rename Dog Cat\n[Cat new]\n";
    assert!(
        has_code(renamed, "tcl", "E001"),
        "bare `[Cat new]` after `rename Dog Cat` must fire E001"
    );
}

#[test]
fn tp_e001_bare_cmd_head_factory_proc_return() {
    // The proc-return flow: the lattice proves `make` object-returning
    // (`ObjectHandleFacts::returns_object`), so a bare `[make]` is the same
    // zero-word TclOO dispatch failure.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               proc make {} { return [Dog new] }\n\
               [make]\n";
    assert!(
        has_code(src, "tcl", "E001"),
        "bare `[make]` on a proven object-returning factory must fire E001"
    );
}

#[test]
fn tn_e001_cmd_head_with_method_word_stays_silent() {
    // `[Dog new] bark` has a method word — the W308 named-method path, not
    // E001; a valid method draws nothing at all.
    let src = "oo::class create Dog { method bark {} { return woof } }\n[Dog new] bark\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::E001 | DiagCode::W307)),
        "`[Dog new] bark` is a well-formed dispatch; got {:?}",
        r.diagnostics
    );
}

#[test]
fn fn_e001_cmd_head_factory_with_method_word_no_w307() {
    // The invariant from issue #1143 on the cmd-head shape: the lattice
    // resolves `[make]`'s class, so `[make] bark` must be validated (and
    // stay silent), never draw the W307 abstention warning.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               proc make {} { return [Dog new] }\n\
               [make] bark\n";
    assert!(
        w30x_codes(src).is_empty(),
        "`[make] bark` resolves through the factory return type; got {:?}",
        w30x_codes(src)
    );
    let bogus = "oo::class create Dog { method bark {} { return woof } }\n\
                 proc make {} { return [Dog new] }\n\
                 [make] fly\n";
    assert!(
        has_code(bogus, "tcl", "W308"),
        "`[make] fly` must be validated against ::Dog and fire W308"
    );
}

#[test]
fn tn_e001_cmd_head_non_object_and_unknown_stay_silent() {
    // A non-object substitution result and a genuinely unknown one must not
    // fire E001 (the unknown one keeps its ordinary W307 abstention story).
    for src in [
        // Known non-object return type.
        "[string length abc]\n",
        // Same-named user proc, no TclOO class in sight.
        "proc Dog {args} { return 1 }\n[Dog new]\n",
        // Dynamic head — nothing provable.
        "set h puts\n[$h hello]\n",
    ] {
        assert!(
            !has_code(src, "tcl", "E001"),
            "`{src}` must not fire the TclOO bare-dispatch E001"
        );
    }
}

#[test]
fn tn_e001_abstains_inside_a_child_interpreter_body() {
    // A child interpreter has its own command table: `Dog` does not exist
    // there, so the real failure is `invalid command name "Dog"` in the
    // child — main-interp object semantics must not be asserted (E001
    // abstains, both the cmd-head and the `$var` forms).
    for src in [
        "oo::class create Dog { method bark {} { return woof } }\n\
         interp create sub\n\
         interp eval sub {[Dog new]}\n",
        "oo::class create Dog { method bark {} { return woof } }\n\
         interp create -safe s\n\
         s eval {[Dog new]}\n",
        "oo::class create Dog { method bark {} { return woof } }\n\
         interp create sub\n\
         interp eval sub {set o [Dog new]\n$o}\n",
    ] {
        assert!(
            !has_code(src, "tcl", "E001"),
            "`{src}` runs in a child interpreter; E001 must abstain"
        );
    }
}

#[test]
fn tn_e001_abstains_on_a_deleted_class() {
    // `rename Dog {}` deletes the class command; `[Dog new]` then fails
    // `invalid command name` (W123's story), so no bare-dispatch E001.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               rename Dog {}\n\
               [Dog new]\n";
    assert!(
        !has_code(src, "tcl", "E001"),
        "a deleted class cannot produce the object; E001 must abstain"
    );
}

#[test]
fn tn_e001_cmd_head_snit_stays_silent() {
    // snit's generated dispatcher is a different mechanism — same carve-out
    // as the `$var` path (FP-OBJ-05).
    let src = "snit::type Dog { method bark {} { return woof } }\n[Dog create t]\n";
    assert!(
        !has_code(src, "tcl", "E001"),
        "bare snit `[Dog create t]` must not fire the TclOO E001"
    );
}

#[test]
fn e001_cmd_head_matches_var_path_under_tcloo_less_dialect() {
    // Dialect availability is W002's job: under tcl8.4 the `oo::class`
    // definition itself is flagged (`disabled in the active dialect
    // profile`), and the dispatch checks keep modelling the TclOO semantics
    // of the classes the file declares — the `$var` path has always fired
    // E001 here, and the cmd-head path must agree with it rather than
    // silently diverge on the same document.
    let cmd_head = "oo::class create Dog { method bark {} { return woof } }\n[Dog new]\n";
    let var_path = "oo::class create Dog { method bark {} { return woof } }\n\
                    set o [Dog new]\n$o\n";
    assert_eq!(
        has_code(cmd_head, "tcl8.4", "E001"),
        has_code(var_path, "tcl8.4", "E001"),
        "cmd-head and $var bare-dispatch must agree under tcl8.4"
    );
    assert!(
        has_code(cmd_head, "tcl8.4", "W002"),
        "the definition itself carries the dialect-availability diagnostic"
    );
}

#[test]
fn w307_suppressed_for_method_return_captured_handle() {
    // Issue #1143: `set b [$a make]` — a handle returned by a
    // `$var`-dispatched method and captured into a variable.  The lattice's
    // method-return edge types `b`, so the re-dispatch draws no W307 (the
    // information hover / go-to-definition already surface).
    let src = "oo::class create A { method make {} { return [B new] } }\n\
               oo::class create B { method greet {} { return \"hi\" } }\n\
               set a [A new]\n\
               set b [$a make]\n\
               $b greet\n";
    assert!(
        w30x_codes(src).is_empty(),
        "the method-return-captured handle must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn w308_fires_for_bogus_method_on_method_return_captured_handle() {
    // The positive half of the same edge: once `b` is provably a ::B, an
    // unknown method is W308 — the diagnostic and hover agree on the class.
    let src = "oo::class create A { method make {} { return [B new] } }\n\
               oo::class create B { method greet {} { return \"hi\" } }\n\
               set a [A new]\n\
               set b [$a make]\n\
               $b fly\n";
    assert_eq!(
        w30x_codes(src),
        vec!["W308".to_owned()],
        "an unknown method on the lattice-typed handle is W308, not W307"
    );
}

#[test]
fn tp_e001_bare_dispatch_on_method_return_captured_handle() {
    // Issue #1200's variable flow: the same lattice typing makes a bare
    // `$b` the unconditional TclOO zero-word failure.
    let src = "oo::class create A { method make {} { return [B new] } }\n\
               oo::class create B { method greet {} { return \"hi\" } }\n\
               set a [A new]\n\
               set b [$a make]\n\
               $b\n";
    assert!(
        has_code(src, "tcl", "E001"),
        "a bare dispatch on the method-return-typed handle must fire E001"
    );
}

#[test]
fn tn_w307_expanded_runtime_command_list_stays_abstaining() {
    // The REFUTED SpiceGenTcl shape from issue #1143 (idx 14), kept as the
    // TN that must stay abstaining: `{*}$element` expands a *runtime-built*
    // list into the callee inside a method body (SpiceGenTcl's
    // `buildTopNetlist`).  No W307/W308/E001, and no false resolution.
    let src = "oo::class create Netlist { method add {e} { return $e } }\n\
               oo::class create Builder {\n\
                 method build {cls args} {\n\
                   set topNetlist [Netlist new]\n\
                   set element [list $cls new {*}$args]\n\
                   $topNetlist add [{*}$element]\n\
                   return $topNetlist\n\
                 }\n\
               }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W307 | DiagCode::W308 | DiagCode::E001)),
        "the runtime-built `{{*}}$element` dispatch must stay a silent \
         abstention; got {:?}",
        r.diagnostics
    );
}

#[test]
fn w307_suppressed_for_braced_namespace_var_proc_param() {
    // `${namespace}::define::…` — the dispatched variable is the *braced*
    // name `namespace` (a proc parameter), not the whole composite head.
    // The var-name extraction must isolate `namespace` so the proc-param
    // dispatch suppression fires (a mangled name defeats it → false W307).
    let src = "proc Define {namespace class args} {\n  \
                   ${namespace}::dynamic_methods $class\n  \
                   ${namespace}::define::[lindex $args 0] {*}[lrange $args 1 end]\n}\n";
    assert!(
        w30x_codes(src).is_empty(),
        "braced-namespace-var proc-param dispatch must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn w307_suppressed_for_snit_instance_var_in_helper_proc() {
    // `mytree` is a snit instance variable dispatched inside a non-method
    // helper `proc` in the type body (`upvar`'d component object).  The
    // dispatch falls in the class body range and names an instance var, so
    // `is_snit_member` suppresses W307.
    let src = "snit::type T {\n  variable mytree\n  \
                   proc Check {id} { upvar 1 mytree mytree; if {![$mytree exists $id]} { return } }\n}\n";
    assert!(
        w30x_codes(src).is_empty(),
        "snit instance-var dispatch in a helper proc must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn irule2001_carries_matchclass_replacement_fix() {
    // `matchclass <item> <class>` → `class match <item> equals <class>`,
    // replacing the whole command. Raw source slices preserve `$var`. The
    // fix span is independent of the diagnostic span (which is the head).
    let mut a = Analyser::new();
    let r = a.analyse("matchclass $u ::lib\n", "f5-irules");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("IRULE2001");
    assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
    let fix = &d.fixes[0];
    assert_eq!(fix.new_text, "class match $u equals ::lib");
    assert_eq!(fix.description, "Replace with 'class match'");
    // Fix range spans the whole command (head + both args), not just the
    // diagnostic's head span.
    assert_eq!(fix.span.start(), d.span.start());
    assert!(fix.span.end() > d.span.end());
}

#[test]
fn irule2001_three_arg_matchclass_fix_preserves_operator_and_class() {
    // The 3-arg form `matchclass <item> <operator> <class>` is a 1:1 rename
    // to `class match <item> <operator> <class>` — it must NOT be forced to
    // `equals` (which dropped the real class).
    let mut a = Analyser::new();
    let r = a.analyse(
        "matchclass [HTTP::uri] starts_with $::admin_paths\n",
        "f5-irules",
    );
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("IRULE2001");
    assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
    assert_eq!(
        d.fixes[0].new_text,
        "class match [HTTP::uri] starts_with $::admin_paths"
    );
}

#[test]
fn irule2001_ambiguous_arity_matchclass_warns_without_fix() {
    // A 1-arg (or any non-2/3-arg) `matchclass` is ambiguous: still warn,
    // but offer no quick-fix rather than corrupt the command.
    let mut a = Analyser::new();
    let r = a.analyse("matchclass [HTTP::uri]\n", "f5-irules");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("IRULE2001");
    assert!(d.fixes.is_empty(), "expected no fix, got {:?}", d.fixes);
}

#[test]
fn analyse_w307_emitted_for_cmd_substitution_with_unknown_return_type() {
    // ``[bogus_cmd] foo`` — the inner command isn't in the
    // registry, so the return type is unknown.  W307 should
    // fire for the cmd-as-command site.
    let src = "[bogus_cmd] foo";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 expected for [unknown] method pattern; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_suppressed_for_my_self_dispatch() {
    // ``[my method]`` is OO self-dispatch — never trips W307.
    let src = "[my m] arg";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must not fire for OO self-dispatch; got {:?}",
        r.diagnostics,
    );
}

// Issue #1324 — `[self]` / `[self object]` used as a dispatch head
// (`[self] method`) is `TclOO`'s own same-object spelling: it must resolve
// like `my method` for W308 ("unknown method") too, not just for the
// LSP-layer consumers #1322 already fixed (highlighting, go-to-def,
// references, code lens). Before this fix, `emit_cmd_command_diagnostics`
// treated *any* self-dispatch/introspection head — `self`'s own included —
// as an opaque, unresolvable-class object handle and silently abstained,
// a false negative on invalid code.

#[test]
fn analyse_w308_tp_1324_bare_self_receiver_unknown_method() {
    // `[self] nosuchmethod` — the exact community repro (issue #1324).
    let src = "\
oo::class create Test1324A {
    method animTick {} {
        return {}
    }
    method anim {} {
        [self] nosuchmethod
        return {}
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("nosuchmethod")),
        "W308 expected for `[self] nosuchmethod`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w308_fp_1324_bare_self_receiver_real_method_silent() {
    // FP guard: `[self] animTick` dispatches to a real method on the
    // enclosing class — must NOT fire W308.
    let src = "\
oo::class create Test1324B {
    method animTick {} {
        return {}
    }
    method anim {} {
        [self] animTick
        return {}
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
        "W308 must not fire for `[self] animTick`, a real method; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w308_tp_1324_self_object_receiver_unknown_method() {
    // `[self object]` is `self`'s explicit-word spelling of the same
    // receiver `[self]` (bare) denotes — must resolve identically.
    let src = "\
oo::class create Test1324C {
    method animTick {} {
        return {}
    }
    method anim {} {
        [self object] nosuchmethod
        return {}
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("nosuchmethod")),
        "W308 expected for `[self object] nosuchmethod`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_suppressed_for_partial_interpolation_resolving_to_known_proc() {
    // ``set suffix _hi`` makes ``$suffix`` resolve to ``_hi``;
    // ``foo$suffix`` therefore resolves to ``foo_hi``, which
    // is a known proc.  W123 should not fire.
    let src = "\
proc foo_hi {} {}
set suffix _hi
foo$suffix
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 should be suppressed when partial interpolation resolves to a known proc; got {:?}",
        r.diagnostics,
    );
}

// Issue #1010 (site 1) — `resolve_interpolated_w123_diagnostics` deleted an
// already-correct W123 for an interpolated command head (`foo$suffix`)
// once SCCP folded it to a known-but-dead proc name, with no deletion
// gate at all. Fixed by reusing `fact_live_for_call` per candidate.
// Confirmed against tclsh 8.6.14.

#[test]
fn w123_tp_issue_1010_interpolated_head_folds_to_deleted_proc_stays_flagged() {
    let src = "proc foo_hi {} {}\nrename foo_hi {}\nset suffix _hi\nfoo$suffix\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an interpolated head folding to a deleted proc must still draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_issue_1010_interpolated_head_folds_to_reestablished_proc_resolves() {
    let src =
        "proc foo_hi {} {}\nrename foo_hi {}\nproc foo_hi {} {}\nset suffix _hi\nfoo$suffix\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a proc re-established after deletion must still resolve; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn extra_commands_suppress_w123() {
    // `tclLsp.extraCommands` names are treated as known commands.
    let src = "mylibsend foo bar\n";
    // Baseline: an unknown bare command fires W123.
    let mut base = Analyser::new();
    let baseline = base.analyse(src, "tcl8.6");
    assert!(
        baseline
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123),
        "baseline W123 expected for unknown command; got {:?}",
        baseline.diagnostics,
    );
    // With the command declared extra, W123 is suppressed.
    let mut a = Analyser::new().with_extra_commands(["mylibsend".to_owned()].into_iter().collect());
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "extraCommands should suppress W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn namespace_ensemble_explicit_command_option_suppresses_w123() {
    // `namespace ensemble create -command NAME` dispatches under an
    // explicit, differently-named command — not the enclosing namespace's
    // own name (the implicit form). A call through it must not draw a
    // spurious "unknown command" (baseline: it does, before the name is
    // recorded).
    let src = "\
namespace eval ::ns {
    namespace export foo
    proc foo {a b} { return ok }
    namespace ensemble create -command ::ens -map {f foo}
}
::ens f 1 2
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an explicit -command ensemble name must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_kept_when_partial_interpolation_resolves_to_unknown() {
    // ``set suffix _missing`` makes ``foo$suffix`` resolve
    // to ``foo_missing`` — not a known command — so W123
    // should still fire.
    let src = "\
set suffix _missing
foo$suffix
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 expected when partial interpolation resolves to an unknown command",
    );
}

// Issue #968 — W123 false-positived on every built-in `expr` math function
// (`sin(...)`, `max(...)`, ...): `record_expr_function_invocations` already
// resolved a math-function call to `::tcl::mathfunc::<name>`, but the W123
// pass only recognised that qualified name via a *user-defined*
// `proc ::tcl::mathfunc::<name>` (`proc_tail_names`) — never the built-in
// function table itself, so every stock call read as unresolved.

#[test]
fn w123_tp_unknown_function_name_inside_expr_still_fires() {
    // FP fix must not swallow a genuinely unknown function — `frobnicate` is
    // not a real `::tcl::mathfunc` name in any Tcl release.
    let src = "set x [expr {frobnicate(1)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an unknown expr function must still draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_math_function_before_its_release_dual_fires_with_w002() {
    // `min`/`max` are TIP 232 (8.5+); under 8.4 the name is not a real
    // `::tcl::mathfunc` command in *this* dialect — the same "known
    // elsewhere, disabled here" split the registry-builtin path draws for
    // e.g. `dict` under `tcl8.4` (both W002 *and* W123 fire — see
    // `build_w123_known_names`'s profile-filter comment; verified live
    // against `exec` under `f5-irules`).
    let src = "set x [expr {min(1, 2)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W002),
        "min() under tcl8.4 must draw W002 (disabled in dialect); got {:?}",
        r.diagnostics,
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "min() under tcl8.4 must also draw W123 (unresolved in this dialect); got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_expr_function_after_rename_away_is_unresolved() {
    // `rename ::tcl::mathfunc::sin {}` really does break `expr {sin(...)}}`
    // in C Tcl (`invalid command name "tcl::mathfunc::sin"` — see the WASM
    // runtime's `expr_routes_through_the_command_table` test) — a call after
    // the deletion must not silently resolve.
    let src = "rename ::tcl::mathfunc::sin {}\nset x [expr {sin(1.0)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a call through a renamed-away mathfunc must be flagged unresolved; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_bareword_call_to_a_mathfunc_shaped_name_is_still_unresolved() {
    // `sin` is only ever callable via `::tcl::mathfunc::sin` or the `expr`
    // function-call grammar — a bare top-level command call to `sin` (no
    // user `proc sin`, not inside `expr`) is `invalid command name` in real
    // Tcl, and the fix must not leak resolution into ordinary bareword
    // command dispatch.
    let src = "sin 1.0\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a bareword call to a mathfunc-shaped name must still be unresolved; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_original_issue_968_repro_is_silent() {
    // The exact shape reported in issue #968: built-in `expr` math
    // functions must resolve with no diagnostic at all.
    let src = "set x [expr {sin(1.0) + max(1, 2, 3)}]\nputs $x\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "issue #968: built-in expr math functions must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_nested_function_calls_both_resolve() {
    // `sqrt(abs($x))` records both the outer and the inner function as
    // separate mathfunc invocations (`expr_nested_function_calls_are_both_recorded`
    // in commands.rs) — both must resolve.
    let src = "set x [expr {sqrt(abs($y))}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "nested built-in expr math functions must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_version_gated_function_resolves_at_its_introducing_release() {
    // `isnan` is TIP 521 (9.0+) — available (and silent) exactly at tcl9.0,
    // unlike the tcl8.4 case above.
    let src = "set x [expr {isnan(1.0)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W002 | DiagCode::W123)),
        "isnan() under tcl9.0 must draw neither W002 nor W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_expr_function_resolves_inside_if_condition() {
    // The `expr`-function resolution path is driven by the registry's
    // `ArgRole::Expr` role, so it fires for *any* EXPR-role argument —
    // `if`'s condition, not only a literal `expr` command.
    let src = "if {sin($y) > 0} {\n    puts hi\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a math function inside an `if` condition must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_expr_function_resolves_inside_namespace_and_proc_body() {
    // Interprocedural context (a namespace-scoped proc body) must not
    // change resolution — the function dispatches through the global
    // `::tcl::mathfunc` namespace regardless of the caller's own scope.
    let src = "\
namespace eval ::acme {
    proc calc {y} {
        return [expr {sin($y) + cos($y)}]
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a math function inside a namespaced proc body must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tn_user_defined_mathfunc_override_still_resolves() {
    // Pre-existing behaviour (`expr_function_call_records_a_mathfunc_invocation`
    // in commands.rs), unaffected by the built-in-name fix: a user override
    // resolves via `proc_tail_names` exactly as before.
    let src = "proc ::tcl::mathfunc::myfunc {x} { return $x }\nset r [expr {myfunc(1)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a user-defined mathfunc override must resolve; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tn_unrelated_proc_sharing_a_mathfunc_name_is_unaffected() {
    // `proc abs {x} {...}` is an ordinary, unrelated command — the fix is
    // scoped to `expr` function-call invocations only (via the settled
    // `::tcl::mathfunc::<name>` qualified name), so a same-named regular
    // proc and its ordinary bareword call sites must resolve exactly as
    // they did before this fix, via the normal proc-tail path.
    let src = "proc abs {x} {\n    if {$x < 0} { return [expr {-$x}] }\n    return $x\n}\nputs [abs -5]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an unrelated proc sharing a mathfunc name must resolve normally; got {:?}",
        r.diagnostics,
    );
}

// The `expr`-function shortcut above is keyed on `is_mathfunc_call`, not
// merely on the settled qualified name matching `::tcl::mathfunc::<name>` —
// an *ordinary* call can resolve to that same shape by being made from
// inside the real `::tcl::mathfunc` namespace, and TIP 232's command
// wrappers (the mechanism that makes such a bareword call valid at all)
// only exist from Tcl 8.5 onward, independent of any individual function's
// own, earlier expr-grammar availability.

#[test]
fn w123_tp_ordinary_call_shaped_like_mathfunc_fires_under_84() {
    // `sin` is a valid *expr* function since 8.4, but the `::tcl::mathfunc`
    // *command* namespace TIP 232 introduced does not exist until 8.5 — an
    // ordinary bareword call to `sin` from inside that namespace is not a
    // real command under an 8.4-based dialect.
    let src = "namespace eval ::tcl::mathfunc {\n    sin 1\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an ordinary call shaped like a mathfunc dispatch must still draw \
         W123 under an 8.4-based dialect; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_ordinary_call_shaped_like_mathfunc_resolves_under_86() {
    // The same call is genuinely valid Tcl from 8.5 onward: `::tcl::mathfunc::sin`
    // really is a callable command there, so the bareword call must resolve.
    let src = "namespace eval ::tcl::mathfunc {\n    sin 1\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "the command wrapper genuinely exists under tcl8.6; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_custom_mathfunc_body_bareword_call_fires_under_84() {
    // The more realistic trigger: a custom math function's own body calling
    // a built-in bareword (not through `expr`'s function-call syntax) —
    // still invalid under an 8.4-based dialect for the same reason.
    let src = "proc ::tcl::mathfunc::myfunc {x} { return [sin $x] }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a bareword call to a builtin from inside a custom mathfunc body \
         must still draw W123 under an 8.4-based dialect; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_expr_function_call_unaffected_by_wrapper_availability_under_84() {
    // Regression guard: the wrapper-availability gate must apply only to
    // ordinary calls, never to a genuine `expr` function-call site — issue
    // #968's original fix must not regress under an 8.4-based dialect.
    let src = "set x [expr {sin(1.0)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "expr {{sin(1.0)}} is valid under tcl8.4 regardless of TIP 232's \
         command-wrapper availability; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_static_rename_does_not_set_has_dynamic_providers() {
    // A fully static `rename OLD NEW` is now recorded precisely
    // (`renamed_commands`) instead of blanket-poisoning
    // `has_dynamic_providers` for the whole document.
    let mut a = Analyser::new();
    let r = a.analyse("rename set myset\n", "tcl");
    assert!(
        !r.has_dynamic_providers,
        "a fully static rename must not set has_dynamic_providers"
    );
    assert_eq!(
        r.renamed_commands.get("::myset").map(String::as_str),
        Some("::set")
    );
}

#[test]
fn analyse_dynamic_but_resolvable_rename_does_not_set_has_dynamic_providers() {
    // `$x` is dynamic-*looking* but `x` is a known constant (`set x
    // set`) — issue #923 idx 3's constant-folding fix now resolves this
    // exactly like the fully-static
    // `analyse_static_rename_does_not_set_has_dynamic_providers` case,
    // instead of falling back to the conservative
    // `has_dynamic_providers` flag.
    let mut a = Analyser::new();
    let r = a.analyse("set x set\nrename $x myset\n", "tcl");
    assert!(
        !r.has_dynamic_providers,
        "x's value is a known constant, so this rename must resolve, not widen"
    );
    assert_eq!(
        r.renamed_commands.get("::myset").map(String::as_str),
        Some("::set")
    );
}

#[test]
fn analyse_genuinely_dynamic_rename_still_sets_has_dynamic_providers() {
    // Unlike the sibling test above, `x`'s value here is never a
    // tracked compile-time constant (piped through `gets`) — the
    // document still falls back to the conservative
    // `has_dynamic_providers` flag, matching `command_binding.rs`'s
    // wildcard-collapse convention for the identical shape.
    let mut a = Analyser::new();
    let r = a.analyse("set x [gets stdin]\nrename $x myset\n", "tcl");
    assert!(
        r.has_dynamic_providers,
        "a genuinely dynamic rename must still set has_dynamic_providers"
    );
    assert!(r.renamed_commands.is_empty());
}

#[test]
fn analyse_w123_emits_did_you_mean_suggestion() {
    // ``puta`` is one edit away from ``puts`` — the
    // emitter should attach a suggestion and a CodeFix.
    let mut a = Analyser::new();
    let r = a.analyse("puta hi", "tcl");
    let w123 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W123)
        .expect("W123 emitted");
    assert!(
        w123.message.contains("did you mean 'puts'"),
        "expected suggestion in message, got: {}",
        w123.message,
    );
    assert!(!w123.fixes.is_empty(), "expected CodeFix payload");
    let fix = &w123.fixes[0];
    assert_eq!(fix.new_text, "puts");
    assert!(fix.description.contains("puts"));
}

#[test]
fn analyse_w123_suppressed_for_inline_stub_declared_command() {
    // ``my_cmd`` is declared via inline stub — W123 must
    // not fire even though it isn't in the registry.
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub my_cmd {arg1:var body:body}
# tcl-lsp: stubs-end
my_cmd $x foo
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 must not fire for stub-declared commands; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_dispatch_target_from_unknown_proc_suppresses() {
    // ``foo`` is one of the switch arms inside a
    // user-defined ``unknown`` proc — the empty-stub gate
    // doesn't fire (body is non-empty), so W123 is
    // already suppressed.  Add a fixture that verifies
    // the dispatch_targets are also in the suggestion
    // candidate set when an empty-stub unknown is in play.
    let src = "\
proc unknown {cmd args} {}
foo
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    // Empty unknown means W123 still fires — but the
    // dispatch_targets membership doesn't apply (set is
    // empty).  Just sanity-check the test runs.
    assert!(r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_no_suggestion_when_far_from_any_known_command() {
    let mut a = Analyser::new();
    let r = a.analyse("xyzzy_unknown_cmd", "tcl");
    let w123 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W123)
        .expect("W123 emitted");
    assert!(
        !w123.message.contains("did you mean"),
        "no suggestion expected for far-away command name; got: {}",
        w123.message,
    );
    assert!(w123.fixes.is_empty());
}

#[test]
fn analyse_irule4005_racy_static_emitted_for_per_request_writes() {
    // ``static::counter`` written in HTTP_REQUEST and read
    // in HTTP_RESPONSE — both per-request events; the
    // cross-event flow is racy ⇒ IRULE4005 fires.
    let mut a = Analyser::new();
    let r = a.analyse(
        "when HTTP_REQUEST { incr static::counter }\n\
             when HTTP_RESPONSE { log local0. \"$static::counter\" }",
        "f5-irules",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::Irule4005),
        "IRULE4005 expected for racy static cross-event flow; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_irule4005_no_emit_for_rule_init_writes() {
    // ``static::config`` written in RULE_INIT is racy-safe
    // (RULE_INIT runs once at iRule load) — IRULE4005 must
    // not fire.
    let mut a = Analyser::new();
    let r = a.analyse(
        "when RULE_INIT { set static::config 1 }\n\
             when HTTP_REQUEST { log local0. \"$static::config\" }",
        "f5-irules",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::Irule4005),
        "IRULE4005 must not fire for RULE_INIT writes; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w124_ipv4_octet_overflow() {
    // ``proc foo {} { set ip 192.168.1.999 }`` — 999 > 255,
    // not a valid IP.  W124 fires at the assignment.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 192.168.1.999 }", "tcl");
    let w124s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W124)
        .collect();
    assert!(
        !w124s.is_empty(),
        "W124 expected for IPv4 octet > 255; got {:?}",
        r.diagnostics,
    );
    assert!(w124s[0].message.contains("999"));
    assert!(w124s[0].message.contains("exceeds 255"));
    assert_eq!(w124s[0].severity, Severity::Error);
}

#[test]
fn analyse_no_w124_for_valid_ipv4() {
    // ``proc foo {} { set ip 192.168.1.1 }`` — valid IP.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 192.168.1.1 }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W124),
        "W124 must not fire on valid IPv4; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w124_ipv4_leading_zero_warning() {
    // ``proc foo {} { set ip 192.168.01.1 }`` — leading
    // zero on octet 3; might be octal in some contexts.
    // Severity is Warning.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 192.168.01.1 }", "tcl");
    let w124s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W124)
        .collect();
    assert!(
        !w124s.is_empty(),
        "W124 expected for IPv4 leading-zero octet; got {:?}",
        r.diagnostics,
    );
    assert_eq!(w124s[0].severity, Severity::Warning);
    assert!(w124s[0].message.contains("leading zero"));
}

#[test]
fn analyse_no_w124_for_oid_chain() {
    // FP-STY-06: an LDAP PEN OID (`1.3.6.1.4.1.4203.1.11.3`) is a
    // hierarchical dotted chain, not IPv4 — the embedded `4203.1.11.3`
    // slice must NOT fire W124.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set oid 1.3.6.1.4.1.4203.1.11.3 }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W124),
        "W124 must not fire on an OID dotted chain; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w124_real_ipv4_shaped_still_fires() {
    // TP control: a genuine four-component dotted quad with an
    // out-of-range octet (not part of a longer chain) still fires.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 10.0.0.300 }", "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W124),
        "W124 must fire on a genuine over-255 IPv4 quad; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w302_fire_and_forget_bare_close_silent() {
    // FP-STY-05: `catch {close $fh}` is the documented fire-and-forget
    // idiom — no W302.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { catch {close $fh} }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "W302 must be suppressed on `catch {{close ...}}`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w302_fire_and_forget_ensemble_chan_close_silent() {
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { catch {chan close $fh} }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "W302 must be suppressed on `catch {{chan close ...}}`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w304_braced_switch_form_silent() {
    // FP-NAB-05: the two-arg braced switch form is unambiguous — no W304.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {x} { switch $x { -nocase {puts a} default {puts b} } }",
        "tcl",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W304),
        "W304 must not fire on a two-arg braced switch; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w304_split_switch_form_still_fires() {
    // TP control: the split (3+ arg) switch form with a dynamic string
    // before an explicit option still warrants `--`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {x} { switch $x -nocase {puts a} default {puts b} }",
        "tcl",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W304),
        "W304 must still fire on the split switch form; got {:?}",
        r.diagnostics,
    );
}

/// Helper: W210 codes for a snippet.
fn w210_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W210)
        .map(|d| d.message.clone())
        .collect()
}

fn w233_codes(src: &str) -> usize {
    let mut a = Analyser::new();
    a.analyse(src, "tcl")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W233)
        .count()
}

#[test]
fn w233_divide_by_zero_literal_and_const_var() {
    // Each case verified against tclsh 8.4–9.0 to raise "divide by zero".
    assert_eq!(w233_codes("proc f {} { return [expr {1 / 0}] }"), 1);
    assert_eq!(w233_codes("proc f {} { return [expr {10 % 0}] }"), 1);
    assert_eq!(
        w233_codes("proc f {} { set d 0\n return [expr {10 / $d}] }"),
        1
    );
}

#[test]
fn w233_silent_on_nonzero_unknown_and_guarded() {
    // Non-zero const + unknown divisor never fire.
    assert_eq!(
        w233_codes("proc f {} { set d 3\n return [expr {10 / $d}] }"),
        0
    );
    assert_eq!(w233_codes("proc f {n} { return [expr {10 / $n}] }"), 0);
    // Short-circuit / dead-arm guards make the division unreachable
    // (tclsh: these expressions do *not* raise — the RHS is skipped).
    assert_eq!(w233_codes("proc f {} { return [expr {0 && 1/0}] }"), 0);
    assert_eq!(w233_codes("proc f {} { return [expr {0 ? 1/0 : 7}] }"), 0);
    assert_eq!(w233_codes("proc f {c} { return [expr {$c && 1/0}] }"), 0);
    assert_eq!(w233_codes("proc f {} { return [expr {+0 && 1/0}] }"), 0);
    // Constant-truthy guard forces the arm — fires. Each verified against
    // tclsh 8.4–9.0 (`-1`, `1.0`, `!0` are truthy, so `1/0` is reached
    // and raises "divide by zero").
    assert_eq!(w233_codes("proc f {} { return [expr {1.0 && 1/0}] }"), 1);
    assert_eq!(w233_codes("proc f {} { return [expr {-1 && 1/0}] }"), 1);
    assert_eq!(w233_codes("proc f {} { return [expr {!0 && 1/0}] }"), 1);
}

#[test]
fn w233_silent_on_float_division() {
    // Float division by zero yields Inf (verified against tclsh 8.4–9.0:
    // `1.0/0.0` and `1/0.0` are *not* errors), so the integer-only
    // divide-by-zero check must not fire. A float-literal divisor never
    // narrows to the integer point `[0, 0]`.
    assert_eq!(w233_codes("proc f {} { return [expr {1.0 / 0.0}] }"), 0);
    assert_eq!(w233_codes("proc f {} { return [expr {1 / 0.0}] }"), 0);
}

#[test]
fn w210_phi_undef_if_arm_only_def_return() {
    // `v` is defined only when `$x > 0`; the unconditional `return $v`
    // reads it on the no-set path too.
    let got = w210_codes("proc f {x} { if {$x > 0} { set v 1 }\n return $v }");
    assert!(
        got.iter().any(|m| m.contains("'v'")),
        "phi-from-undef merge read must fire W210; got {got:?}"
    );
}

#[test]
fn w210_phi_undef_switch_no_default_return() {
    let got = w210_codes("proc f {x} { switch $x { a { set v 1 } b { set v 2 } }\n return $v }");
    assert!(
        got.iter().any(|m| m.contains("'v'")),
        "switch-no-default + return must fire W210; got {got:?}"
    );
}

#[test]
fn when_body_not_analysed_under_plain_tcl() {
    // `when` is an iRules-only builtin; under plain Tcl it is a disabled
    // foreign-dialect command whose braced argument is opaque *data*, not a
    // handler script.  The body must not be scanned: only W002 (disabled) +
    // W123 (unknown-in-dialect) on `when` itself — no W210 on the body's
    // `$undefvar`, no W123 naming the body command.
    let mut a = Analyser::new();
    let r = a.analyse("when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n", "tcl");
    let codes: Vec<&str> = r.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        !codes.contains(&"W210"),
        "disabled `when` body must not be analysed (no W210); got {:?}",
        r.diagnostics
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("boguscmd")),
        "disabled `when` body command must not draw W123; got {:?}",
        r.diagnostics
    );
    // The `when` head itself is still flagged disabled + unknown-in-dialect.
    assert!(codes.contains(&"W002"), "expected W002; got {codes:?}");
}

#[test]
fn when_body_analysed_under_irules() {
    // Under the iRules dialect `when` IS enabled, so its body is a real
    // handler script and the read-before-set inside it fires (the inverse
    // of `when_body_not_analysed_under_plain_tcl`).
    let mut a = Analyser::new();
    let r = a.analyse("when HTTP_REQUEST {\n    set x $undefvar\n}\n", "f5-irules");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("undefvar")),
        "iRules `when` body must be analysed (W210 on $undefvar); got {:?}",
        r.diagnostics
    );
}

#[test]
fn w210_interproc_dict_with_caller_literal() {
    // A caller passing a literal dict propagates to the callee's
    // `dict with $param` key check (interproc constant propagation).
    // Key present → silent.
    assert!(
        w210_codes("proc f {d} { dict with d { return $missing } }\nf {missing ok}\n").is_empty()
    );
    // Empty dict → no keys → the read fires.
    assert!(
        w210_codes("proc f {d} { dict with d { return $missing } }\nf {}\n")
            .iter()
            .any(|m| m.contains("'missing'"))
    );
    // Mixed callers → unknown shape → conservatively silent.
    assert!(
        w210_codes("proc f {d} { dict with d { return $missing } }\nf {}\nf {missing X}\n")
            .is_empty()
    );
}

#[test]
fn w210_provably_no_match_regexp_scan() {
    // Provably-no-match output reads fire.
    assert!(
        w210_codes("proc f {} { scan abc %d n\n puts $n }")
            .iter()
            .any(|m| m.contains("'n'"))
    );
    assert!(
        w210_codes("proc f {} { regexp {x} y -> v\n puts $v }")
            .iter()
            .any(|m| m.contains("'v'"))
    );
    // Embedded in a negated condition fires on the no-match arm.
    assert!(
        w210_codes("proc f {} { if {![regexp {x} y -> v]} { puts $v } }")
            .iter()
            .any(|m| m.contains("'v'"))
    );
}

#[test]
fn w210_regexp_expanded_whitespace_pattern_silent() {
    // `-expanded` ignores whitespace, so `{a b}` matches `ab` and writes
    // v — the no-match proof must bail (no false W210).
    assert!(w210_codes("proc f {} { regexp -expanded {a b} ab v\n puts $v }").is_empty());
    // A whitespace-free literal under -expanded is still safe → fires.
    assert!(
        w210_codes("proc f {} { regexp -expanded {x} X v\n puts $v }")
            .iter()
            .any(|m| m.contains("'v'"))
    );
}

#[test]
fn w210_matchable_regexp_scan_silent() {
    // A matchable / nocase-matchable regexp output is set — no W210.
    assert!(w210_codes("proc f {} { regexp -nocase {x} X v\n puts $v }").is_empty());
    assert!(w210_codes("proc f {} { scan 42 %d n\n puts $n }").is_empty());
    // The success arm of a positive condition reads a set var.
    assert!(w210_codes("proc f {} { if {[regexp {x} y -> v]} { puts $v } }").is_empty());
    // An unknown / unsafe switch can't prove no-match → silent.
    assert!(w210_codes("proc f {} { regexp -about {x} y v\n puts $v }").is_empty());
}

#[test]
fn w210_incr_on_uninit_is_silent() {
    // `incr z` initialises z to 0 (Tcl 8.5+) — not read-before-set.
    assert!(w210_codes("proc f {} { incr z\n return $z }").is_empty());
    // A genuine bare read of an unset local still fires.
    assert!(
        w210_codes("proc f {} { puts $z }")
            .iter()
            .any(|m| m.contains("'z'"))
    );
}

#[test]
fn w210_phi_undef_use_after_unset_return() {
    let got = w210_codes("proc f {} { set v 1\n unset v\n return $v }");
    assert!(
        got.iter().any(|m| m.contains("'v'")),
        "use-after-unset return must fire W210; got {got:?}"
    );
}

#[test]
fn w210_loop_body_accumulator_read_after_loop_silent() {
    // FP-RBS-19 (#756): the reporter's exact pattern — a `lappend` accumulator
    // built inside a dynamic `foreach`, returned after the loop. The body
    // defines `r` on every iteration, so a read after the loop is defined
    // whenever the loop ran. Matching C Tcl (which errors only when `$items` is
    // actually empty at runtime), we assume a may-run loop runs.
    let got = w210_codes("proc f {items} { foreach i $items { lappend r $i }\n return $r }");
    assert!(
        got.is_empty(),
        "after-loop return of a loop-body accumulator must be silent; got {got:?}"
    );
    // TP control: a *first-iteration* read of the accumulator, before its set,
    // is a genuine read-before-set inside the body and still fires.
    let inbody = w210_codes("proc f {items} { foreach i $items { puts $r; lappend r $i } }");
    assert!(
        inbody.iter().any(|m| m.contains("'r'")),
        "first-iteration in-body read before the set must still fire; got {inbody:?}"
    );
}

#[test]
fn w210_no_fire_when_both_merge_arms_define() {
    // Control: every merge predecessor defines `v` — not read-before-set.
    let got = w210_codes("proc f {x} { if {$x > 0} { set v 1 } else { set v 2 }\n return $v }");
    assert!(
        got.is_empty(),
        "both-arms-defined merge must be silent; got {got:?}"
    );
}

#[test]
fn w210_empty_dict_with_return_fires_but_known_key_silent() {
    // FP-DS-08: empty dict unpacks nothing — `return $missing` fires.
    let empty = w210_codes("proc f {} { set d {}\n dict with d {}\n return $missing }");
    assert!(
        empty.iter().any(|m| m.contains("'missing'")),
        "empty dict-with return must fire W210; got {empty:?}"
    );
    // Known-key dict unpacks `missing` — silent.
    let known = w210_codes("proc f {} { set d {missing ok}\n dict with d {}\n return $missing }");
    assert!(
        known.is_empty(),
        "known-key dict-with return must be silent; got {known:?}"
    );
    // Unknown-shape dict (param) — conservatively silent.
    let unknown = w210_codes("proc f {d} { dict with d {}\n return $missing }");
    assert!(
        unknown.is_empty(),
        "unknown dict-with return must be silent; got {unknown:?}"
    );
}

#[test]
fn w210_qualified_variable_alias_tail_return_silent() {
    // FP-RBS-04: `variable ${name}::graphAttr` declares the local alias
    // `graphAttr`; the bare tail read is not read-before-set.
    let got =
        w210_codes("proc ::ns::get {name key} { variable ${name}::graphAttr\n return $graphAttr }");
    assert!(
        got.is_empty(),
        "qualified variable-alias tail read must be silent; got {got:?}"
    );
}

#[test]
fn w210_no_false_fire_on_many_var_scan_return() {
    // D4-F2: the dynamic scan arg-role resolver marks every trailing
    // varName as a write, so `return $a19` is not read-before-set.
    let src = "proc f {} { scan {0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19} \
{%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} \
a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15 a16 a17 a18 a19\n return $a19 }";
    assert!(
        w210_codes(src).is_empty(),
        "20-var scan must not false-fire W210 on the tail var"
    );
}

#[test]
fn w210_no_false_fire_on_many_var_lassign_return() {
    let src = "proc f {l} { lassign $l a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 \
a15 a16 a17 a18 a19 a20\n return $a20 }";
    assert!(
        w210_codes(src).is_empty(),
        "21-var lassign must not false-fire W210 on the tail var"
    );
}

#[test]
fn w214_empty_body_stub_silent() {
    // FP-STY-08: `proc stub {a b} {}` is a signature placeholder — no
    // W214 on its necessarily-unused params.
    let mut a = Analyser::new();
    let r = a.analyse("proc stub {a b} {}", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W214),
        "W214 must be suppressed on an empty-body stub; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_quoted_keyword_marker_silent() {
    // FP-STY-08: a param named `"as"` is a snit-style keyword marker.
    let mut a = Analyser::new();
    let r = a.analyse("proc xyz {\"as\" v} { return $v }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W214),
        "W214 must not fire on a quoted-keyword param; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_dispatch_protocol_suppresses_peer_family() {
    // ≥3 peers sharing `{ctx token}` + an arity-compatible dispatcher
    // (`$cmd $ctx $token`, 2 args) — `token` is a protocol contract.
    let src = "namespace eval ::n {\n\
                   proc a {ctx token} { puts $ctx }\n\
                   proc b {ctx token} { puts $ctx }\n\
                   proc c {ctx token} { puts $ctx }\n\
                   proc dispatch {cmd ctx token} { $cmd $ctx $token }\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214 && d.message.contains("'token'")),
        "dispatch-protocol family must suppress W214 on protocol params; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_no_dispatcher_still_fires_on_peer_family() {
    // TP control: 3 peers sharing `{ctx token}` but NO dispatcher — the
    // shared shape is coincidence, so `token` still fires.
    let src = "namespace eval ::n {\n\
                   proc a {ctx token} { puts $ctx }\n\
                   proc b {ctx token} { puts $ctx }\n\
                   proc c {ctx token} { puts $ctx }\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214 && d.message.contains("'token'")),
        "without a dispatcher, an unused protocol-shaped param still fires; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_genuine_unused_param_still_fires() {
    // TP control: a normal unused param in a non-empty body still fires.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {a b} { return $a }", "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214 && d.message.contains("'b'")),
        "W214 must still fire on a genuine unused param; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w302_constructive_subcommand_still_fires() {
    // TP control: `chan configure` is constructive, not fire-and-forget.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {} { catch {chan configure $fh -blocking 0} }",
        "tcl",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "W302 must still fire on a constructive `chan configure`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_i230_constant_if_branch() {
    // ``proc foo {} { if {1} { puts hi } }`` — the ``if 1``
    // condition is constant, the false branch is unreachable.
    // I230 should fire.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { if {1} { puts hi } }", "tcl");
    let i230s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::I230)
        .collect();
    assert!(
        !i230s.is_empty(),
        "I230 expected for constant ``if 1``; got {:?}",
        r.diagnostics,
    );
    assert!(i230s[0].message.contains("always true"));
}

#[test]
fn analyse_no_i230_for_dynamic_condition() {
    // ``proc foo {x} { if {$x > 0} {} }`` — ``$x > 0`` is
    // not constant; no I230.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {x} { if {$x > 0} { puts hi } }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::I230),
        "I230 must not fire on dynamic condition; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_unknown_command() {
    // ``no_such_cmd hello`` — bare name that's not a
    // built-in / proc / class / alias.  W123 fires.
    let mut a = Analyser::new();
    let r = a.analyse("no_such_cmd hello", "tcl");
    let w123s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W123)
        .collect();
    assert!(
        !w123s.is_empty(),
        "W123 expected for unknown command; got {:?}",
        r.diagnostics,
    );
    assert!(w123s[0].message.contains("'no_such_cmd'"));
    assert_eq!(w123s[0].severity, Severity::Hint);
}

#[test]
fn analyse_no_w123_for_builtin_command() {
    // ``puts hello`` — ``puts`` is a built-in; no W123.
    let mut a = Analyser::new();
    let r = a.analyse("puts hello", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 must not fire on built-in command; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_no_w123_for_user_proc() {
    // User-defined proc, then call it.  Both go through
    // the analyser walk; the call site must NOT trip W123.
    let mut a = Analyser::new();
    let r = a.analyse("proc greet {} { puts hi }\ngreet", "tcl");
    let w123s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W123)
        .collect();
    assert!(
        w123s.is_empty(),
        "W123 must not fire on user-defined proc call; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_no_w123_for_qualified_command_name() {
    // Qualified names (``a::b``) skip W123 — defer to
    // per-namespace logic.
    let mut a = Analyser::new();
    let r = a.analyse("ns::cmd hello", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 must not fire on qualified command name; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_package_require_gate_suppresses_when_recorded() {
    // The ``package_requires`` gate suppresses W123 entirely
    // when any package require has been recorded.  The
    // analyser walk doesn't yet record ``package require``
    // (the handler is not implemented), so we exercise the
    // gate by pre-populating ``result.package_requires``
    // and re-running the post-pass directly.
    use crate::signature_scan::types::SignaturePackageRequire;
    use tcl_lexer::Span;
    let mut a = Analyser::new();
    a.result.package_requires.push(SignaturePackageRequire {
        name: "Tcl".to_string(),
        version: Some("8.6".to_string()),
        exact: false,
        range: Span::new(0, 24),
        conditional: false,
    });
    // Seed an invocation that would otherwise trip W123.
    a.result
        .command_invocations
        .push(crate::signature_scan::types::SignatureCommandInvocation {
            name: "random_cmd".to_string(),
            range: Span::new(25, 35),
            resolved_qualified_name: None,
            resolved_user_definition: false,
            resolution_candidates: Vec::new(),
            argc: Some(0),
            callback_arity: None,
            callback_baked_args: 0,
            indirect: false,
            rename_safe: true,
            existence_probe: false,
            is_mathfunc_call: false,
            ensemble_dispatch: None,
        });
    let registry = tcl_registry::CommandRegistry::build_default();
    a.emit_unresolved_command_diagnostics(&registry);
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123),
        "W123 must be fully suppressed when package_requires is non-empty; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn analyse_w123_filtered_by_disabled_diagnostics() {
    // ``# tcl-lsp: disable=W123`` at top of file silences
    // the diagnostic via the existing disable filter.
    let mut a = Analyser::new();
    let r = a.analyse("# tcl-lsp: disable=W123\nno_such_cmd hello", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 must be silenced by file-suppression directive; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_var_as_command() {
    // ``proc foo {} { $cmd arg1 }`` — ``$cmd`` (a non-parameter local
    // dispatched once) is used as command head with no static knowledge
    // of what it holds, so W307 fires.  Must go through ``analyse`` (not
    // raw ``emit_cfg_ssa_diagnostics``) because ``var_command_sites`` is
    // populated by the analyser's walk dispatch, not the emitter pipeline.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { $cmd arg1 }", "tcl");
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert!(
        !w307s.is_empty(),
        "W307 expected for ``$cmd arg1``; got {:?}",
        r.diagnostics,
    );
    assert_eq!(w307s[0].severity, Severity::Warning);
    assert!(w307s[0].message.contains("Non-literal command name"));
}

#[test]
fn analyse_w307_suppressed_for_proc_param_dispatch() {
    // A dispatch on a *parameter* of the enclosing proc is object dispatch
    // the user documented as the proc's API contract — W307 must stay
    // silent.  `$self configure` is the canonical method-dispatch
    // idiom on an opaque handle.
    let mut a = Analyser::new();
    let r = a.analyse("proc p {self} { $self configure -x 1 }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must be suppressed for a dispatch on proc parameter; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_suppressed_for_multi_dispatch_local() {
    // A non-parameter local dispatched ≥2 times demonstrates intent
    // (object usage), so W307 is suppressed even without a known value.
    let mut a = Analyser::new();
    let r = a.analyse("proc p {} { $tree visit\n$tree leaves }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must be suppressed for a local dispatched ≥2 times; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_fires_for_tainted_dispatch_despite_multi_use() {
    // Taint carve-out: a user-controlled command name dispatched multiple
    // times is still a command-injection risk — the dispatcher-suppression
    // must NOT apply, so W307 fires.
    let mut a = Analyser::new();
    let r = a.analyse("proc p {} { set c [gets stdin]\n$c one\n$c two }", "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must fire for a tainted dispatched command name; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_no_w307_for_static_known_command() {
    // ``proc foo {} { set cmd puts; $cmd hello }`` — ``cmd``
    // has constant value "puts" which IS a known command, so
    // W307 must be suppressed.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set cmd puts\n$cmd hello }", "tcl");
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert!(
        w307s.is_empty(),
        "W307 must be suppressed when var holds known command name; got {:?}",
        r.diagnostics,
    );
}

// Issue #1010 (site 2) — `is_known_command` (used by `w307_site_suppressed`)
// suppressed W307 whenever SCCP proved a dynamic-dispatch value equalled
// a proc/class name, with no deletion gate — so a variable holding a
// renamed-or-deleted-away command (no later re-establishment) still
// silenced the real "invalid command name" hazard. Fixed by threading
// the dispatch site's own offset through and reusing `fact_live_for_call`.
// Confirmed against tclsh 8.6.14.

#[test]
fn w307_tp_issue_1010_dispatch_value_is_deleted_proc_stays_flagged() {
    let src = "proc target {} {}\nrename target {}\nproc foo {} { set cmd target\n$cmd hello }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a dispatch value equalling a deleted proc must still draw W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w307_fp_issue_1010_dispatch_value_is_reestablished_proc_resolves() {
    let src = "proc target {} {}\nrename target {}\nproc target {} {}\nproc foo {} { set cmd target\n$cmd hello }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a proc re-established after deletion must still suppress W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_suppressed_per_ssa_version_after_reassignment() {
    // Per-SSA-version refinement: `cmd` is
    // reassigned from a non-command to a known command before the
    // dispatch.  The merged const-set {notacommand, puts} would
    // wrongly keep W307 alive; reading the value at the dispatch's
    // exact SSA use-version ("puts") suppresses it.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc foo {} { set cmd notacommand\nset cmd puts\n$cmd hello }",
        "tcl",
    );
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert!(
        w307s.is_empty(),
        "W307 must read the precise reaching version (\"puts\"); got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_still_fires_when_precise_version_not_a_command() {
    // The mirror case: the reaching version is a non-command, so
    // W307 must still fire (the refinement only suppresses when the
    // exact value is provably a known command).
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc foo {} { set cmd puts\nset cmd notacommand\n$cmd hello }",
        "tcl",
    );
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert!(
        !w307s.is_empty(),
        "W307 should fire when the reaching version isn't a command; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_var_command_sites_recorded_during_walk() {
    // Smoke: confirm the recording infrastructure populates
    // ``var_command_sites`` for ``$var`` heads.  Run analyse
    // (not just emit) so the apply_disabled_diagnostics +
    // dedupe don't matter — we inspect post-analyse state.
    let mut a = Analyser::new();
    let _ = a.analyse("proc foo {x} { $x arg }", "tcl");
    // After analyse, var_command_sites is consumed by the
    // post-pass but restored at the end (snapshot/restore
    // contract).
    assert!(
        a.var_command_sites.iter().any(|s| s.var_name == "x"),
        "var_command_sites should record ``$x`` head; got {:?}",
        a.var_command_sites,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_cmd_command_sites_recorded_during_walk() {
    // ``[cmd] arg`` records to ``cmd_command_sites`` even
    // though no W307 emitter consumes it yet.
    let mut a = Analyser::new();
    let _ = a.analyse("proc foo {} { [puts hi] arg }", "tcl");
    assert!(
        !a.cmd_command_sites.is_empty(),
        "cmd_command_sites should be populated for ``[cmd] arg``; got {:?}",
        a.cmd_command_sites,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w214_skips_args_param() {
    // The variadic ``args`` is conventional and frequently
    // declared without use; W214 must not fire on it.
    let mut a = Analyser::new();
    a.emit_cfg_ssa_diagnostics("proc foo {x args} { puts $x }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214),
        "W214 should not fire on ``args``; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn dedupe_drops_exact_duplicates() {
    // Same code + span + message + severity → kept once.
    let mut a = Analyser::new();
    a.source = "set x 1".to_string();
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(4, 5), "x not set"));
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(4, 5), "x not set"));
    a.dedupe_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 1);
}

#[test]
fn dedupe_keeps_distinct_diagnostics_at_different_spans() {
    let mut a = Analyser::new();
    a.source = "set x 1\nset y 2".to_string();
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(4, 5), "x"));
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(12, 13), "y"));
    a.dedupe_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 2);
}

#[test]
fn dedupe_drops_e002_on_e101_line() {
    // E101 fires on a line; any E002 on the same line is
    // a false positive (arity check confused by the
    // recovered switch) and gets dropped.
    let mut a = Analyser::new();
    a.source = "switch $x { foo {puts foo}".to_string();
    let switch_span = Span::new(0, 6);
    a.result
        .diagnostics
        .push(diag(DiagCode::E101, switch_span, "missing open brace"));
    a.result
        .diagnostics
        .push(diag(DiagCode::E002, switch_span, "too few args"));
    a.dedupe_diagnostics();
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E101)
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002)
    );
}

#[test]
fn dedupe_keeps_e002_on_unrelated_line() {
    // E101 on line 0, E002 on line 1 — different lines, so
    // the suppression rule doesn't fire.
    let mut a = Analyser::new();
    a.source = "switch $x {\nset y 1".to_string();
    a.result
        .diagnostics
        .push(diag(DiagCode::E101, Span::new(0, 6), "missing brace"));
    a.result
        .diagnostics
        .push(diag(DiagCode::E002, Span::new(12, 15), "too few args"));
    a.dedupe_diagnostics();
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002)
    );
}

#[test]
fn apply_disabled_diagnostics_removes_listed_codes() {
    let mut a =
        Analyser::with_disabled_diagnostics(["W113"].iter().map(|s| (*s).to_string()).collect());
    a.result
        .diagnostics
        .push(diag(DiagCode::W113, Span::new(0, 3), "shadows"));
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(0, 3), "unset"));
    a.apply_disabled_diagnostics();
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113)
    );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210)
    );
}

#[test]
fn apply_disabled_diagnostics_no_op_when_empty() {
    let mut a = Analyser::new();
    a.result
        .diagnostics
        .push(diag(DiagCode::W113, Span::new(0, 3), "x"));
    a.apply_disabled_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 1);
}

// W120: missing package require

#[test]
fn w120_fires_for_package_gated_command_without_require() {
    // `tcl::idna` carries `required_package = "tcl::idna"`.
    // Using it without a `package require` emits W120.
    let mut a = Analyser::new();
    let r = a.analyse("tcl::idna decode example.com\n", "tcl9.0");
    let w120: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W120)
        .collect();
    assert_eq!(w120.len(), 1, "expected one W120; got {:?}", r.diagnostics);
    assert!(w120[0].message.contains("package require tcl::idna"));
    // Carries a fix that inserts the require at the top.
    assert_eq!(w120[0].fixes.len(), 1);
    assert_eq!(w120[0].fixes[0].new_text, "package require tcl::idna\n");
    assert!(
        w120[0].fixes[0]
            .description
            .contains("Add 'package require")
    );
}

#[test]
fn w120_suppressed_when_package_required() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "package require tcl::idna\ntcl::idna decode example.com\n",
        "tcl9.0",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W120),
        "W120 must not fire when the package is required; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w120_fix_inserts_after_existing_require() {
    // With an unrelated `package require` present, the fix
    // inserts on the line after it.
    let src = "package require Tcl 8.6\ntcl::idna decode x\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let w120 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W120)
        .expect("W120 expected");
    let fix = &w120.fixes[0];
    // Insertion offset is past the first line's newline
    // (byte 23 = start of line 1).
    let off = fix.span.start() as usize;
    assert_eq!(&src[..off], "package require Tcl 8.6\n");
}

// NB: the workspace-level #723 behaviour — suppressing W120 when a required
// package (transitively) provides the gated package — is the LSP server's
// `package_resolver`-backed post-filter, tested in `tcl-lsp-server`. The
// analyser's single-file W120 here intentionally fires whenever the gated
// package is not required/provided *in this document*.

#[test]
fn w120_emitted_once_per_command_name() {
    let src = "tcl::idna decode a\ntcl::idna encode b\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let w120: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W120)
        .collect();
    assert_eq!(w120.len(), 1, "expected one W120 per name; got {w120:?}");
}

#[test]
fn w120_disabled_via_directive() {
    let mut a = Analyser::new();
    let r = a.analyse("# tcl-lsp: disable=W120\ntcl::idna decode x\n", "tcl9.0");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W120),
        "{:?}",
        r.diagnostics
    );
}

// -- security-injection checks (W300 / W301 / W309 / W312) --
//
// Each fixture asserts the expected security-diagnostic set.

fn sec_codes(src: &str, code: &str) -> usize {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .count()
}

#[test]
fn w300_source_with_variable_path() {
    assert_eq!(sec_codes("source $path\n", "W300"), 1);
    // `-encoding ENC` is skipped to find the file argument.
    assert_eq!(sec_codes("source -encoding utf-8 $path\n", "W300"), 1);
    // A command-substituted path is just as dynamic as a `$var` one.
    assert_eq!(sec_codes("source [locate_lib]\n", "W300"), 1);
    // A literal path is fine.
    assert_eq!(sec_codes("source ./lib.tcl\n", "W300"), 0);
    // FP gate: a `$var` that provably holds a compile-time literal path is a
    // known file — the same as the literal form above — so it is silent.
    assert_eq!(sec_codes("set p \"./lib.tcl\"\nsource $p\n", "W300"), 0);
    // But a `$var` reassigned dynamically before the use is still flagged.
    assert_eq!(
        sec_codes("set p \"./lib.tcl\"\nset p [get_path]\nsource $p\n", "W300"),
        1
    );
    // A proc parameter is not a compile-time literal — still flagged.
    assert_eq!(sec_codes("proc f {p} { source $p }\n", "W300"), 1);
}

#[test]
fn w309_eval_uplevel_with_subst() {
    let mut a = Analyser::new();
    let r = a.analyse("eval [subst $template]\n", "tcl8.6");
    let w309: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W309)
        .collect();
    assert_eq!(w309.len(), 1);
    assert_eq!(w309[0].severity, Severity::Error);
    assert!(w309[0].message.starts_with("eval with [subst]"));
    assert_eq!(sec_codes("uplevel [subst {$x}]\n", "W309"), 1);
    // No `[subst]` → no W309.
    assert_eq!(sec_codes("eval [list set x $y]\n", "W309"), 0);
}

#[test]
fn w301_uplevel_injection() {
    // Single unbraced substituted script.
    assert_eq!(sec_codes("uplevel 1 \"set x $y\"\n", "W301"), 1);
    // Multiple args concatenate.
    assert_eq!(sec_codes("uplevel $a $b\n", "W301"), 1);
    // Braced body and the `[list …]` idiom are safe.
    assert_eq!(sec_codes("uplevel 1 {set x 1}\n", "W301"), 0);
    assert_eq!(sec_codes("uplevel 1 [list set x $y]\n", "W301"), 0);
    // A single *pure* variable script is the safe single-substitution
    // idiom — no W301; a concatenation still fires.
    assert_eq!(sec_codes("proc f {body} { uplevel 1 $body }\n", "W301"), 0);
    assert_eq!(sec_codes("uplevel 1 $body\n", "W301"), 0);
    assert_eq!(sec_codes("uplevel 1 pre$body\n", "W301"), 1);
}

#[test]
fn w312_interp_eval_injection() {
    assert_eq!(sec_codes("interp eval $child $script\n", "W312"), 1);
    assert_eq!(sec_codes("interp eval $child \"set x $y\"\n", "W312"), 1);
    // Multiple script words concatenate.
    assert_eq!(sec_codes("interp eval $foo $a $b\n", "W312"), 1);
    // invokehidden flags the hidden command word.
    assert_eq!(
        sec_codes("interp invokehidden $child $cmd $arg\n", "W312"),
        1
    );
    // Braced body is safe.
    assert_eq!(sec_codes("interp eval $child {set x 1}\n", "W312"), 0);
}

#[test]
fn w312_message_names_subcommand() {
    let mut a = Analyser::new();
    let r = a.analyse("interp eval $child $script\n", "tcl8.6");
    let w312 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W312)
        .unwrap();
    assert!(
        w312.message.contains("interp eval $child {...}"),
        "{w312:?}"
    );
}

#[test]
fn w102_subst_variable_argument() {
    // Bare `$var` template fires; the message lists both kinds.
    let mut a = Analyser::new();
    let r = a.analyse("subst $x\n", "tcl8.6");
    let w102 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W102)
        .unwrap();
    assert!(
        w102.message.contains("any [cmd] and $var in the string"),
        "{w102:?}"
    );
    assert!(
        w102.message
            .contains("Add -nocommands -novariables to limit")
    );
    // A braced or quoted template is fine; both flags suppress it.
    assert_eq!(sec_codes("subst {literal $y}\n", "W102"), 0);
    assert_eq!(sec_codes("subst \"$x\"\n", "W102"), 0);
    assert_eq!(sec_codes("subst -nocommands -novariables $x\n", "W102"), 0);
}

#[test]
fn w102_message_narrows_with_flags() {
    let mut a = Analyser::new();
    let r = a.analyse("subst -nocommands $x\n", "tcl8.6");
    let w102 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W102)
        .unwrap();
    // Only `$var` remains active; only `-novariables` is suggested.
    assert!(w102.message.contains("any $var in the string"), "{w102:?}");
    assert!(!w102.message.contains("[cmd]"), "{w102:?}");
    assert!(w102.message.contains("Add -novariables to limit"));
}

#[test]
fn w103_open_pipeline() {
    // `|`-pipeline with substitution → WARNING (injection).
    let mut a = Analyser::new();
    let r = a.analyse("open \"|$cmd\"\n", "tcl8.6");
    let w103 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W103)
        .unwrap();
    assert_eq!(w103.severity, Severity::Warning);
    assert!(w103.message.contains("command injection"), "{w103:?}");
    // Literal `|`-pipeline → HINT.
    assert_eq!(code_sevs("open |ls\n", "W103"), vec!["Hint"]);
    assert_eq!(code_sevs("open \"|cat file\"\n", "W103"), vec!["Hint"]);
    // Bare `$var` argument → WARNING (may resolve to a pipeline).
    assert_eq!(code_sevs("open $f\n", "W103"), vec!["Warning"]);
    // A command-substituted argument is just as dynamic as a `$var` one.
    assert_eq!(code_sevs("open [pick_target]\n", "W103"), vec!["Warning"]);
    // A literal filename is fine.
    assert_eq!(sec_codes("open \"file.txt\"\n", "W103"), 0);
    // FP gate: a `$var` proven to hold a compile-time literal is treated like
    // that literal inline — a benign filename is silent, a `|`-prefixed literal
    // is the pipeline Hint.
    assert_eq!(sec_codes("set f \"data.txt\"\nopen $f\n", "W103"), 0);
    assert_eq!(code_sevs("set f \"|ls\"\nopen $f\n", "W103"), vec!["Hint"]);
    // A proc parameter is not a literal — still a Warning.
    assert_eq!(
        code_sevs("proc g {f} { open $f }\n", "W103"),
        vec!["Warning"]
    );
}

// -- registry-trait gate swaps (W101 / W103 / W309 / W312 / W302) --
//
// The security emitters resolve their eligible commands from registry
// traits/fields, never from command-name literals. Each swapped gate keeps
// one fire and one silent case here (beyond the per-code fixtures above).

#[test]
fn w101_gate_trait_pair_keeps_eval_only() {
    // Fire: the concat-reparse shape (EVALUATES_CODE + TAINT_SINK with a
    // static Body role at arg 0) — `eval`.
    assert_eq!(sec_codes("eval $cmd\n", "W101"), 1);
    // Silent: `uplevel` carries the trait pair but its script position is
    // resolver-driven (optional level word) — owned by W301, no W101.
    assert_eq!(sec_codes("uplevel 1 \"set x $y\"\n", "W101"), 0);
    // Silent: the coroutine injectors carry EVALUATES_CODE but take a
    // coroutine name + command *prefix* — nothing is re-parsed as script.
    assert_eq!(sec_codes("coroinject c $cmd\n", "W101"), 0);
    assert_eq!(sec_codes("coroprobe c $cmd\n", "W101"), 0);
}

#[test]
fn w309_inner_subst_head_resolves_through_registry() {
    // The `[subst …]` head check is a registry PERFORMS_SUBSTITUTION
    // lookup, so the fully-qualified spelling is caught too (`get`
    // resolves a leading `::`) — previously a literal-prefix miss.
    assert_eq!(sec_codes("eval [::subst $template]\n", "W309"), 1);
    // A non-substituting inner head stays silent.
    assert_eq!(sec_codes("eval [format %s $x]\n", "W309"), 0);
}

#[test]
fn w103_socket_is_not_a_pipeline_opener() {
    // `socket` carries OPENS_CHANNEL too, but its first argument is an
    // address (`Tcl_OpenTcpClient`) — never a `|` exec spec — and its spec
    // carries TAINT_SOURCE, the excluder the gate pairs with the trait.
    assert_eq!(sec_codes("socket $host 80\n", "W103"), 0);
    assert_eq!(sec_codes("socket -server accept 8080\n", "W103"), 0);
    // The file opener still fires.
    assert_eq!(code_sevs("open $f\n", "W103"), vec!["Warning"]);
}

#[test]
fn w312_registry_taint_list_widens_to_console_and_abbreviations() {
    // `console eval` / `consoleinterp eval|record` declare
    // `taint_interp_eval_subcommands` in the registry, so the same
    // injection check applies — an intentional widening from the former
    // `cmd_name == "interp"` gate.
    assert_eq!(sec_codes("console eval $script\n", "W312"), 1);
    assert_eq!(sec_codes("consoleinterp record $script\n", "W312"), 1);
    assert_eq!(sec_codes("console eval {set x 1}\n", "W312"), 0);
    // The subcommand word resolves via the ensemble rule, matching C Tcl:
    // `interp ev` is the unique-prefix spelling tclsh accepts…
    assert_eq!(sec_codes("interp ev $child $script\n", "W312"), 1);
    // …and `interp e` is ambiguous (eval/exists/expose), a tclsh error, so
    // no diagnostic pretends it dispatched.
    assert_eq!(sec_codes("interp e $child $script\n", "W312"), 0);
}

#[test]
fn w302_fire_and_forget_is_registry_destructive_data() {
    // The suppression reads FIRE_AND_FORGET_TEARDOWN from the registry —
    // deliberately NOT the wider HAS_DESTRUCTIVE_OPS + SubCommand.destructive
    // axis (W313's set): `file mkdir` / `file rename` failures are real
    // errors (permissions, missing source), exactly what W302 asks the
    // caller to capture, so their bare-catch forms keep the hint. Only the
    // teardown idioms — where "target already gone" is the expected,
    // intentionally-ignored failure — suppress.
    assert_eq!(
        sec_codes("proc f {d} { catch {file mkdir $d} }\n", "W302"),
        1
    );
    assert_eq!(
        sec_codes("proc f {a b} { catch {file rename $a $b} }\n", "W302"),
        1
    );
    // Ensemble subcommand words resolve via the registry's unique-prefix
    // rule, matching C Tcl's `Tcl_GetIndexFromObj` (`file del` works in
    // tclsh).
    assert_eq!(sec_codes("proc f {p} { catch {file del $p} }\n", "W302"), 0);
    // Non-destructive subforms of a destructive-capable ensemble still
    // fire.
    assert_eq!(
        sec_codes("proc f {p} { catch {file exists $p} }\n", "W302"),
        1
    );
    assert_eq!(
        sec_codes("proc f {} { catch {array set a {x 1}} }\n", "W302"),
        1
    );
    // Newly stamped bare commands and destructive subforms suppress.
    assert_eq!(sec_codes("proc f {} { catch {unset gone} }\n", "W302"), 0);
    assert_eq!(
        sec_codes("proc f {} { catch {rename foo {}} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {id} { catch {after cancel $id} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {} { catch {namespace delete ::tmp} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {} { catch {dict unset cfg key} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {c} { catch {interp delete $c} }\n", "W302"),
        0
    );
}

// -- registry-wide VarWrite binding (handle_var_binding_command) --
//
// The binder takes no command-name gate: every command whose registry spec
// marks VarWrite-role arguments binds its literal targets.

#[test]
fn var_binding_gets_defines_target_and_read_is_clean() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {chan} {\n    gets $chan line\n    puts $line\n}\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "gets writes its line variable — reading it back must not fire W210; got {:?}",
        r.diagnostics
    );
    assert!(
        r.all_variables.contains_key("f::line"),
        "gets' line target must be a recorded definition; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
}

#[test]
fn var_binding_binary_scan_defines_targets() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {d} {\n    binary scan $d c v\n    puts $v\n}\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "binary scan writes its capture variables; got {:?}",
        r.diagnostics
    );
    assert!(
        r.all_variables.contains_key("f::v"),
        "binary scan's target must be a recorded definition; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
}

#[test]
fn var_binding_vwait_defines_target() {
    // `vwait done` returns only after an event handler wrote `done`
    // (`Tcl_VwaitObjCmd` traces WRITES|UNSETS and never reads the value),
    // so the operand is a definition site, and a read after the wait is
    // clean.
    let mut a = Analyser::new();
    let r = a.analyse("vwait done\nputs $done\n", "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "a read after `vwait done` is guaranteed defined; got {:?}",
        r.diagnostics
    );
    assert!(
        r.all_variables.contains_key("::::done"),
        "vwait's operand must be a recorded definition; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
}

#[test]
fn var_binding_double_bind_with_set_is_idempotent() {
    // `set` has a dedicated handler that binds first; the generic
    // VarWrite binder re-binding the same token must not push a duplicate
    // self-reference or downgrade the dedicated handler's
    // `warn_if_unused = true`.
    let mut a = Analyser::new();
    let r = a.analyse("set x 5\n", "tcl8.6");
    let var = r
        .global_scope
        .variables
        .get("x")
        .expect("set must define x");
    assert!(
        var.references.is_empty(),
        "the definition site must not be double-recorded as a reference; got {:?}",
        var.references
    );
    assert!(
        var.warn_if_unused,
        "the generic binder's warn_if_unused=false must not downgrade set's true"
    );
}

#[test]
fn var_binding_scope_alias_and_destroy_families_are_excluded() {
    // CREATES_SCOPE_ALIAS: `global ::ns::v` binds the local alias tail
    // `v` (its dedicated handler's layout), never the qualified name a
    // flat VarWrite walk would record.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {} {\n    global ::ns::v\n    set v 1\n}\n",
        "tcl8.6",
    );
    assert!(
        r.all_variables.contains_key("f::v"),
        "global's dedicated handler binds the alias tail; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_variables.keys().any(|k| k.contains("ns::v")),
        "the generic binder must not record the qualified name; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
    // DESTROYS_VARIABLE: `unset`'s VarWrite role marks a removal target,
    // not a binding.
    let mut a = Analyser::new();
    let r = a.analyse("proc g {} {\n    unset zombie\n}\n", "tcl8.6");
    assert!(
        !r.all_variables.contains_key("g::zombie"),
        "unset must not record a definition for its removal target; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
}

#[test]
fn w303_redos_nested_quantifiers() {
    // Nested quantifier and overlapping alternation, in regexp /
    // regsub and a `switch -regexp` braced case list.
    assert_eq!(sec_codes("regexp {(a+)+} $str\n", "W303"), 1);
    assert_eq!(sec_codes("regexp {(a|a)+} $str\n", "W303"), 1);
    assert_eq!(sec_codes("regsub {(x*)*} $s y out\n", "W303"), 1);
    assert_eq!(
        sec_codes("switch -regexp $x {(a+)+ {body} default {x}}\n", "W303"),
        1
    );
    // Option flags before the pattern are skipped.
    assert_eq!(sec_codes("regexp -nocase {(a+)+} $s\n", "W303"), 1);
    // Anchored nested quantifier still fires.
    assert_eq!(sec_codes("regexp {(a+)+$} $x\n", "W303"), 1);
    // Safe patterns don't fire.
    assert_eq!(sec_codes("regexp {abc} $str\n", "W303"), 0);
    assert_eq!(sec_codes("regexp {[0-9]+} $s\n", "W303"), 0);
    // FP gate: glob-pattern commands are not regexes — the guard is the
    // spec's `pattern_type == Regex`, so regex-looking text under
    // `string match` / `lsearch` must not fire.
    assert_eq!(sec_codes("string match {(a+)+$} $x\n", "W303"), 0);
    assert_eq!(sec_codes("lsearch $l {(a+)+$}\n", "W303"), 0);
}

#[test]
fn w303_message_and_severity() {
    let mut a = Analyser::new();
    let r = a.analyse("regexp {(a+)+} $s\n", "tcl8.6");
    let w303 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W303)
        .unwrap();
    assert_eq!(w303.severity, Severity::Warning);
    assert!(w303.message.contains("catastrophic"), "{w303:?}");
}

#[test]
fn w310_hardcoded_credential_option() {
    // A literal value after a default credential option fires.
    assert_eq!(sec_codes("mycmd -password literalsecret123\n", "W310"), 1);
    assert_eq!(sec_codes("mycmd -token abc123\n", "W310"), 1);
    // Case-insensitive option matching.
    assert_eq!(sec_codes("mycmd -Password hunter2\n", "W310"), 1);
    // Only one diagnostic per command.
    assert_eq!(sec_codes("mycmd -pass a -secret b\n", "W310"), 1);
    // A `$var` / `[cmd]` value is not a hardcoded credential.
    assert_eq!(sec_codes("mycmd -password $env_pw\n", "W310"), 0);
    assert_eq!(sec_codes("mycmd -password [getpw]\n", "W310"), 0);
    // No credential option → nothing.
    assert_eq!(sec_codes("mycmd -name literalvalue\n", "W310"), 0);
}

#[test]
fn irule2002_flags_deprecated_irules_command() {
    // `HTTP::class` is deprecated → `CLASSIFY::application`.
    let mut a = Analyser::new();
    let r = a.analyse("when HTTP_REQUEST {\n  HTTP::class\n}\n", "f5-irules");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2002)
        .expect("IRULE2002");
    assert_eq!(d.severity, Severity::Warning);
    assert!(
        d.message.contains(
            "'HTTP::class' is deprecated in iRules. Use 'CLASSIFY::application' instead."
        ),
        "{d:?}"
    );
}

#[test]
fn irule2002_silent_in_plain_tcl_dialect() {
    // The deprecation check is iRules-only.
    assert!(!has_code("HTTP::class\n", "tcl8.6", "IRULE2002"));
}

#[test]
fn w310_message_names_option() {
    let mut a = Analyser::new();
    let r = a.analyse("mycmd -password literalsecret\n", "tcl8.6");
    let w310 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W310)
        .unwrap();
    assert!(
        w310.message
            .starts_with("Hardcoded credential in -password argument."),
        "{w310:?}"
    );
}

#[test]
fn w310_registry_credential_option() {
    // `http::geturl`'s registry `credential_options` adds `-headers`
    // to the default flag set (Strategy 1 augmentation).
    let mut a = Analyser::new();
    let r = a.analyse(
        "http::geturl $url -headers {Authorization \"Bearer abc123def456\"}\n",
        "tcl8.6",
    );
    let w310 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W310)
        .unwrap();
    assert!(
        w310.message
            .starts_with("Hardcoded credential in -headers argument."),
        "{w310:?}"
    );
}

#[test]
fn w310_subcommand_sensitive_header() {
    // `HTTP::header insert authorization <literal>` — the subcommand's
    // registry credential_arg + sensitive_headers (Strategy 2).
    let mut a = Analyser::new();
    let r = a.analyse(
        "HTTP::header insert authorization \"Bearer secrettoken123\"\n",
        "f5-irules",
    );
    let w310 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W310)
        .unwrap();
    assert!(
        w310.message
            .starts_with("Hardcoded credential in authorization header value."),
        "{w310:?}"
    );
    // A non-sensitive header is fine; a `$var` value is not literal.
    assert!(
        !a.analyse(
            "HTTP::header insert content-type \"text/html\"\n",
            "f5-irules"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W310)
    );
    assert!(
        !a.analyse("HTTP::header insert authorization $tok\n", "f5-irules")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W310)
    );
}

#[test]
fn w306_literal_expected_in_regexp_pattern() {
    fn has_w306(src: &str) -> bool {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W306)
    }
    // A quoted `"$var"` / `"${var}"` pattern is byte-for-byte identical at
    // runtime to the bare `$var` idiom — the quotes group nothing — so it is
    // the canonical parameterised-pattern form, not a foot-gun: exempt.
    assert!(!has_w306("regexp \"$pat\" $s\n"));
    assert!(!has_w306("regexp \"${pat}\" $s\n"));
    // A quoted `"[cmd]"` computes the pattern dynamically — the foot-gun: fires.
    assert!(has_w306("regexp \"[clock seconds]\" $s\n"));
    // A quoted var *concatenated* with literal text (`"prefix$pat"`) is no
    // longer a single pure reference — a literal *was* expected there: fires.
    assert!(has_w306("regexp \"prefix$pat\" $s\n"));
    // A bare `$var` is the canonical parameterised-pattern idiom — exempt.
    assert!(!has_w306("regexp $pat $s\n"));
    // A braced pattern suppresses substitution — exempt.
    assert!(!has_w306("regexp {[abc]+} $s\n"));
    // An escaped `\[` in a quoted pattern is a literal regex char — exempt.
    assert!(!has_w306("regexp \"\\[abc\\]+\" $s\n"));
    // A bare `[cmd]` pattern is the foot-gun (parsed as command sub) — fires.
    assert!(has_w306("regexp [join $parts] $s\n"));
}

#[test]
fn w101_anchors_at_the_substituted_argument() {
    fn w101_span_text(src: &str) -> String {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W101)
            .unwrap_or_else(|| panic!("no W101 in {:?}", r.diagnostics));
        src[d.span.start() as usize..d.span.end() as usize].to_string()
    }
    // The substitution is in the *second* argument — anchor on `$x`, not on
    // the safe literal prefix `"safeprefix"`.
    assert_eq!(w101_span_text("eval \"safeprefix\" $x\n"), "$x");
    // The common single-quoted-string shape still anchors on that string
    // (the substitution is inside it), starting at column 0.
    let mut a = Analyser::new();
    let r = a.analyse("eval \"cmd $a\"\n", "tcl8.6");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W101)
        .unwrap();
    assert_eq!(d.span.start(), 5, "should anchor on the quoted string arg");
}

#[test]
fn w304_does_not_cross_proc_param_shadow() {
    // The outer `set path -force` must NOT be attributed to the inner
    // `$path` use — the proc param `path` shadows it.  W304 may still
    // fire on the substituted `file delete $path`, but never claiming
    // the value is `-force`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "set path -force\nproc useit {path} { file delete $path }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W304 && d.message.contains("-force")),
        "{:?}",
        r.diagnostics
    );
    // Control: a top-level `$path` use *after* a complete proc still
    // resolves to the outer literal (no shadow crossing).
    let r2 = a.analyse(
        "set path -force\nproc p {path} {}\nfile delete $path\n",
        "tcl8.6",
    );
    assert!(
        r2.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W304 && d.message.contains("-force")),
        "{:?}",
        r2.diagnostics
    );
}

// --- Issue #703: `try` handler `-` fallthrough body ---------------------
//
// A `try` `on`/`trap` handler body may be a bare `-` to share the *next*
// handler's body (the same fallthrough mechanism `switch` uses for pattern
// bodies). The lowerer used to treat every handler body as a script, so the
// solo `-` compiled to a zero-argument call of the `-` command and tripped a
// spurious E002 ("Too few arguments for '-'"). These tests assert both sides:
// the fallthrough `-` raises no E002, and a genuine zero-arg `-` command is
// still flagged.

fn count_code(src: &str, code: &str) -> usize {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .count()
}

// The reporter's snippet (from Tcl's own `tools/findBadExternals.tcl`).
const ISSUE_703_SOURCE: &str = "\
proc main {argc argv} {
    lassign $argv libtcl
    try {
        switch $::tcl_platform(platform) {
            unix - macosx {
                exec nm --extern-only --defined-only $libtcl
            }
            windows {
                exec dumpbin /exports $libtcl
            }
        }
    } on ok result - trap NONE result {
        foreach line [split $result \\n] {
            puts $line
        }
        return 0
    } on error msg {
        puts stderr $msg
        return 1
    }
}
";

#[test]
fn issue_703_fallthrough_dash_has_no_e002() {
    // The solo `-` on the `on ok result - trap NONE result` line must not be
    // flagged as a zero-arg `-` command.
    assert_eq!(count_code(ISSUE_703_SOURCE, "E002"), 0);
}

#[test]
fn issue_703_braced_dash_has_no_e002() {
    let src =
        "proc p {} {\n    try {set x 1} on ok a {-} trap NONE b {\n        return $b\n    }\n}\n";
    assert_eq!(count_code(src, "E002"), 0);
}

#[test]
fn issue_703_genuine_dash_command_in_handler_still_flagged() {
    // A real zero-arg `-` *command* inside a handler body is still a genuine
    // arity error — the fix must not blunt E002 here.
    let src = "proc p {} {\n    try {set x 1} on error msg {\n        -\n    }\n}\n";
    assert_eq!(count_code(src, "E002"), 1);
}

#[test]
fn issue_703_genuine_dash_command_outside_try_still_flagged() {
    assert_eq!(count_code("proc f {} {\n    -\n}\n", "E002"), 1);
}

#[test]
fn issue_703_no_false_w210_on_fallthrough_group_var() {
    // When a fallthrough handler's var differs from the target's, the shared
    // body must not be flagged W210 for reading either var. Real Tcl is itself
    // inconsistent here — a byte-compiled `try` binds the *matching* handler's
    // var, the interpreted form the *target*'s — so the shared body is analysed
    // with the whole group's vars treated as defined.
    let reads_fallthrough_var =
        "proc p {} {\n    try {set v ok} on ok x - on error y {\n        return $x\n    }\n}\n";
    let reads_target_var =
        "proc p {} {\n    try {set v ok} on ok x - on error y {\n        return $y\n    }\n}\n";
    assert_eq!(count_code(reads_fallthrough_var, "W210"), 0);
    assert_eq!(count_code(reads_target_var, "W210"), 0);
}

#[test]
fn issue_703_genuine_read_before_set_still_fires_in_shared_body() {
    // A read of a variable bound by no handler in the group is still a genuine
    // read-before-set.
    let src = "proc p {} {\n    try {set v 1} on ok x - on error y {\n        return $undefinedvar\n    }\n}\n";
    assert_eq!(count_code(src, "W210"), 1);
}

#[test]
fn issue_703_backslash_escaped_dash_no_false_w210() {
    // Codex review on #706: a backslash-escaped `\-` handler body evaluates to
    // `-` and is a fallthrough, so the shared target body must not be flagged
    // W210 for reading the fallthrough handler's var (same as the bare `-`).
    let src =
        "proc p {} {\n    try {set v ok} on ok x \\- on error y {\n        return $x\n    }\n}\n";
    assert_eq!(count_code(src, "W210"), 0);
}

#[test]
fn scratch_dup_e003_per_item() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item("set var 10 10\n", "tcl8.6");
    let e003: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    eprintln!("per_item E003 count = {}", e003.len());
    for d in &e003 {
        eprintln!("  span={:?} msg={}", d.span, d.message);
    }
}

#[test]
fn catch_body_is_walked_for_syntactic_checks() {
    // `catch { … }` evaluates its script body, so the per-command syntactic
    // checks (here W100, unbraced `expr`) must reach inside it.
    // Regression for the missing body walk in
    // `handle_catch_command` (it defined the result/options vars but never
    // recursed into args[0]).
    assert_eq!(count_code("catch { expr $x+1 }\n", "W100"), 1);
    assert_eq!(count_code("catch { set y [expr $x+1] } res\n", "W100"), 1);
    // A dynamic body (`catch $cmd`) stays opaque — nothing to walk.
    assert_eq!(count_code("catch $cmd\n", "W100"), 0);
}

#[test]
fn tcltest_test_body_is_walked_when_imported() {
    // `tcltest::test` carries `Body` roles on its `-setup`/`-body`/`-cleanup`
    // option values and on the legacy positional body (penultimate arg), so
    // the analyser descends into the test script — both when the command is
    // called fully-qualified and when it is reached through a
    // `namespace import ::tcltest::*` by its bare name.
    let qualified = "package require tcltest\n\
                     tcltest::test t1 {d} { expr $x+1 } {}\n";
    assert_eq!(count_code(qualified, "W100"), 1);

    let imported = "package require tcltest\n\
                    namespace import -force ::tcltest::*\n\
                    test t1 {d} -body { expr $x+1 } -result {}\n";
    assert_eq!(count_code(imported, "W100"), 1);

    // The expected-result field is data, not a script, and must not be walked.
    let result_field = "package require tcltest\n\
                        namespace import -force ::tcltest::*\n\
                        test t1 {d} -body { puts ok } -result {[expr $x]}\n";
    assert_eq!(count_code(result_field, "W100"), 0);

    // Without the import, a bare `test` is an unknown command whose braced
    // argument is an opaque string — do not recurse (matches Tcl: `test` is
    // undefined until tcltest is loaded).
    assert_eq!(count_code("test t1 {d} { expr $x+1 } {}\n", "W100"), 0);
}

#[test]
fn append_and_lappend_define_their_target_variable() {
    // `append`/`lappend` create their first argument if absent, so the target
    // is a variable definition (it must surface in `symbols`/completion/hover).
    // Regression: previously only `set` /
    // `variable` / `global` / `incr` defined vars, so an `append`/`lappend`
    // target was dropped from the symbol table.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("lappend safe 1\nappend out hi\n", "tcl8.6");
    assert!(
        r.global_scope.variables.contains_key("safe"),
        "lappend target"
    );
    assert!(
        r.global_scope.variables.contains_key("out"),
        "append target"
    );
    // A read-modify-write target is not "set but never used": no W211.
    assert_eq!(count_code("lappend safe 1\n", "W211"), 0);
}

#[test]
fn nested_catch_result_var_is_defined() {
    // `catch SCRIPT ?resultVar? ?optionsVar?` binds its result/options vars even
    // when the `catch` is nested in a `[...]` substitution, so they must reach
    // the symbol table (symbols/completion/hover).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("set out [catch {error x} msg opts]\n", "tcl8.6");
    assert!(
        r.global_scope.variables.contains_key("msg"),
        "catch result var"
    );
    assert!(
        r.global_scope.variables.contains_key("opts"),
        "catch options var"
    );
    // The binding is a side effect of catch, not a "set but unused" target.
    assert_eq!(count_code("if {[catch {foo} e]} {puts $e}\n", "W211"), 0);
}

#[test]
fn catch_body_package_require_is_conditional() {
    // `catch { package require Foo }` is a guarded optional-dependency probe, so
    // the package requirement recorded from inside the catch body must be marked
    // conditional (not promoted to an unconditional fact). Codex review P2.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("catch { package require Foo 1.2 }\n", "tcl8.6");
    let foo = r
        .package_requires
        .iter()
        .find(|p| p.name == "Foo")
        .expect("Foo package require recorded");
    assert!(
        foo.conditional,
        "catch-body package require must be conditional"
    );

    // A top-level `package require` stays unconditional.
    let mut b = crate::analyser::Analyser::new();
    let r2 = b.analyse("package require Bar\n", "tcl8.6");
    let bar = r2
        .package_requires
        .iter()
        .find(|p| p.name == "Bar")
        .unwrap();
    assert!(
        !bar.conditional,
        "top-level package require must be unconditional"
    );
}

#[test]
fn tcltest_import_is_namespace_scoped() {
    // A `namespace import ::tcltest::*` made *inside* a namespace must not
    // resolve a bare `test` call in a sibling/parent namespace. Codex review P2.
    let inside_ns_top_level_call = "package require tcltest\n\
        namespace eval ns { namespace import -force ::tcltest::* }\n\
        test t {d} { expr $x+1 } {}\n";
    assert_eq!(count_code(inside_ns_top_level_call, "W100"), 0);

    // Same-namespace call still resolves and recurses.
    let same_ns = "package require tcltest\n\
        namespace eval ns { namespace import -force ::tcltest::* ; test t {d} { expr $x+1 } {} }\n";
    assert_eq!(count_code(same_ns, "W100"), 1);

    // A global-scope import still applies at top level (Tcl's `::` fallback).
    let top_level = "package require tcltest\n\
        namespace import -force ::tcltest::*\n\
        test t {d} { expr $x+1 } {}\n";
    assert_eq!(count_code(top_level, "W100"), 1);
}

/// tclpkg manifest whole-file scoped environment (`tcl_registry::scoped::
/// file_scope_env` keyed on the `tclpkg.tcl` basename): directives resolve
/// against the environment, never against same-named Tcl/Tk commands.
mod tclpkg_manifest_env {
    use super::*;

    const MANIFEST: &str = "\
package     demo-app
version     0.1.0
description \"A demo\"
license     MIT
author      \"Dev <dev@example.org>\"

tcl >=8.6

require json    1.0.0
require http    2.9.0

dev-require tcltest 2.5.0

provides demo::serve
entry    main.tcl
";

    fn diags_for(path: Option<&str>) -> Vec<Diagnostic> {
        let mut analyser =
            crate::analyser::Analyser::new().with_file_path(path.map(str::to_string));
        analyser.analyse(MANIFEST, "tcl8.6").diagnostics
    }

    /// TN: every directive resolves in the manifest environment — no
    /// unknown-command / unknown-subcommand / missing-package noise.
    #[test]
    fn manifest_directives_resolve_cleanly() {
        let diags = diags_for(Some("/proj/tclpkg.tcl"));
        assert!(
            diags.is_empty(),
            "a valid manifest must produce no diagnostics: {diags:?}"
        );
    }

    /// TP control: the same text WITHOUT the manifest file name is plain
    /// Tcl — the directives draw their usual diagnostics (the environment
    /// must never leak into ordinary documents).
    #[test]
    fn plain_tcl_document_still_flags_directives() {
        let diags = diags_for(Some("/proj/other.tcl"));
        assert!(
            diags.iter().any(|d| d.code == DiagCode::W123),
            "outside a manifest the directives are unknown commands: {diags:?}"
        );
    }

    /// TP: a typo'd directive still draws W123 — the environment is closed.
    #[test]
    fn manifest_typo_directive_fires_w123() {
        let mut analyser =
            crate::analyser::Analyser::new().with_file_path(Some("/proj/tclpkg.tcl".to_string()));
        let result = analyser.analyse("package demo\nverison 0.1.0\n", "tcl8.6");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123 && d.message.contains("verison")),
            "a typo'd directive must stay unknown: {:?}",
            result.diagnostics
        );
    }

    /// TP: a directive arity error is checked against the manifest
    /// grammar (`require <name> <minimum> ?-source <url>?`).
    #[test]
    fn manifest_directive_arity_checked() {
        let mut analyser =
            crate::analyser::Analyser::new().with_file_path(Some("/proj/tclpkg.tcl".to_string()));
        let result = analyser.analyse("package demo\nrequire json\n", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == DiagCode::E002),
            "`require` with one argument is below the directive's minimum: {:?}",
            result.diagnostics
        );
    }
}

mod did_you_mean_variables {
    //! W210 / W212 / W215 "did you mean…?" — the undefined-variable
    //! family suggests a close in-scope variable name (W215's own tests
    //! live with the emitter in `analyser::scope`).

    use super::*;

    fn find_code(src: &str, dialect: &str, code: DiagCode) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, dialect);
        r.diagnostics
            .iter()
            .find(|d| d.code == code)
            .unwrap_or_else(|| panic!("expected {code:?} in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn w210_suggests_close_defined_variable() {
        // TP: `countr` is an edit-distance-1 typo of the defined `counter`.
        let d = find_code("set counter 1\nputs $countr\n", "tcl8.6", DiagCode::W210);
        assert!(
            d.message.ends_with("; did you mean 'counter'?"),
            "expected a 'counter' suggestion: {:?}",
            d.message
        );
        // The suggestion is informational only — no fix (the analyser
        // cannot know which spelling was intended).
        assert!(d.fixes.is_empty(), "{:?}", d.fixes);
    }

    #[test]
    fn w210_suggests_inside_proc_bodies() {
        let d = find_code(
            "proc p {} {\n  set counter 1\n  puts $countr\n}\n",
            "tcl8.6",
            DiagCode::W210,
        );
        assert!(
            d.message.ends_with("; did you mean 'counter'?"),
            "expected a 'counter' suggestion: {:?}",
            d.message
        );
    }

    #[test]
    fn w210_case_mismatch_still_wins_over_edit_distance() {
        // `MYLIST` is 6 edits from `mylist` — far outside the scaled
        // budget — but the case-insensitive twin rule still names it.
        let d = find_code("set MYLIST 1\nputs $mylist\n", "tcl8.6", DiagCode::W210);
        assert!(
            d.message.ends_with("; did you mean 'MYLIST'?"),
            "expected the case twin: {:?}",
            d.message
        );
    }

    #[test]
    fn w210_no_suggestion_when_nothing_is_close() {
        // FP guard: `frobnicate` is nowhere near `totally`.
        let d = find_code(
            "set totally 1\nputs $frobnicate\n",
            "tcl8.6",
            DiagCode::W210,
        );
        assert!(
            !d.message.contains("did you mean"),
            "no suggestion expected: {:?}",
            d.message
        );
    }

    #[test]
    fn w210_short_name_never_fishes_a_full_replacement() {
        // FP guard: every other 1-char name is a full rewrite of `$u`,
        // not a typo correction — `m` must not be suggested.
        let d = find_code("set m 1\nputs $u\nputs $m\n", "tcl8.6", DiagCode::W210);
        assert!(
            !d.message.contains("did you mean"),
            "no suggestion expected for a 1-char read: {:?}",
            d.message
        );
    }

    #[test]
    fn w212_suggests_close_in_scope_variable_for_undefined_name() {
        // TP: `set $countr` where `countr` is undefined but `counter`
        // is in scope — the better correction is `counter`.
        let d = find_code("set counter 1\nset $countr 5\n", "tcl8.6", DiagCode::W212);
        assert!(
            d.message.contains("Did you mean 'counter'?"),
            "expected the close in-scope name: {:?}",
            d.message
        );
    }

    #[test]
    fn w212_keeps_de_sigil_suggestion_for_defined_name() {
        // `varname` is itself defined — dropping the `$` is the fix.
        let d = find_code("set varname x\nset $varname y\n", "tcl8.6", DiagCode::W212);
        assert!(
            d.message.contains("Did you mean 'varname'?"),
            "expected the de-sigiled name: {:?}",
            d.message
        );
    }

    #[test]
    fn w212_keeps_de_sigil_suggestion_when_nothing_is_close() {
        // FP guard: no in-scope name is close — the de-sigil suggestion
        // stands rather than fishing an unrelated variable.
        let d = find_code(
            "set totally 1\nincr $frobnicate\n",
            "tcl8.6",
            DiagCode::W212,
        );
        assert!(
            d.message.contains("Did you mean 'frobnicate'?"),
            "expected the de-sigiled name: {:?}",
            d.message
        );
    }
}

mod irule2002_drop_in_fix {
    //! IRULE2002 — a deprecated command whose registry spec marks the
    //! replacement as a drop-in rename carries a head-swap quick fix;
    //! non-mechanical replacements stay message-only.

    use super::*;

    fn irule2002_for(src: &str) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "f5-irules");
        r.diagnostics
            .iter()
            .find(|d| d.code == DiagCode::Irule2002)
            .unwrap_or_else(|| panic!("expected IRULE2002 in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn drop_in_replacement_carries_head_swap_fix() {
        // TP: `client_addr` → `IP::client_addr` is argument-compatible.
        let d = irule2002_for("when HTTP_REQUEST {\n  set a [client_addr]\n}\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        let fix = &d.fixes[0];
        assert_eq!(fix.new_text, "IP::client_addr");
        assert_eq!(fix.description, "Replace with 'IP::client_addr'");
        // The fix replaces exactly the command head token.
        assert_eq!(fix.span, d.span);
    }

    #[test]
    fn drop_in_fix_preserves_arguments() {
        // TP: an argument-taking drop-in (`decode_uri <s>` →
        // `URI::decode <s>`) swaps only the head; the argument words are
        // untouched (outside the fix span).
        let d = irule2002_for("when HTTP_REQUEST {\n  set u [decode_uri $raw]\n}\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "URI::decode");
        assert_eq!(d.fixes[0].span, d.span);
    }

    #[test]
    fn non_mechanical_replacements_get_no_fix() {
        // FP guards: `ip_addr` → `IP::addr` restructures its arguments
        // (`… mask …`); `matchclass` → `class` needs the `match`
        // subcommand (IRULE2001 carries the correct arity-aware fix);
        // `use` → `virtual` changes the statement shape entirely.
        for src in [
            "when HTTP_REQUEST {\n  set m [ip_addr 10.0.0.1 255.0.0.0]\n}\n",
            "when HTTP_REQUEST {\n  matchclass $u ::lib\n}\n",
            "when HTTP_REQUEST {\n  use pool aol_pool\n}\n",
        ] {
            let d = irule2002_for(src);
            assert!(
                d.fixes.is_empty(),
                "no fix expected for {src:?}: {:?}",
                d.fixes
            );
        }
    }
}

mod arity_usage_suffix {
    //! E002 / E003 / E005 — arity messages append the registry
    //! signature as a " — usage: …" suffix when the resolved spec
    //! declares a synopsis.

    use super::*;

    fn find_code(src: &str, code: DiagCode) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        r.diagnostics
            .iter()
            .find(|d| d.code == code)
            .unwrap_or_else(|| panic!("expected {code:?} in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn e002_appends_command_synopsis() {
        let d = find_code("lreplace\n", DiagCode::E002);
        assert_eq!(
            d.message,
            "Too few arguments for 'lreplace': expected at least 3, got 0 \
             — usage: lreplace list first last ?element element ...?"
        );
    }

    #[test]
    fn e003_appends_subcommand_synopsis() {
        // The subcommand path uses the resolved subcommand's synopsis,
        // not the parent ensemble's.
        let d = find_code("string length a b c\n", DiagCode::E003);
        assert!(
            d.message.ends_with(" — usage: string length string"),
            "expected the subcommand synopsis: {:?}",
            d.message
        );
    }

    #[test]
    fn e005_appends_command_synopsis() {
        let d = find_code("lmap a b c d\n", DiagCode::E005);
        assert!(
            d.message
                .ends_with(" — usage: lmap varlist1 list1 ?varlist2 list2 ...? body"),
            "expected the lmap synopsis: {:?}",
            d.message
        );
    }

    #[test]
    fn user_proc_arity_message_stays_count_only() {
        // A same-file proc has no registry synopsis — the message keeps
        // its count-only form.
        let d = find_code("proc pair {a b} {}\npair 1\n", DiagCode::E002);
        assert!(
            !d.message.contains("usage:"),
            "no usage suffix expected for a user proc: {:?}",
            d.message
        );
    }
}

mod w104_lappend_fix {
    //! W104 — the two-word leading-space `append` shape carries a
    //! whole-command `lappend` rewrite; every non-mechanical shape
    //! stays message-only.

    use super::*;

    fn w104_for(src: &str) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        r.diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W104)
            .unwrap_or_else(|| panic!("expected W104 in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn leading_space_var_piece_carries_whole_command_rewrite() {
        let src = "set item x\nset items {}\nappend items \" $item\"\n";
        let d = w104_for(src);
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        let fix = &d.fixes[0];
        assert_eq!(fix.new_text, "lappend items $item");
        assert_eq!(fix.description, "Rewrite with `lappend`");
        // The fix replaces exactly the whole `append` command.
        assert_eq!(
            &src[fix.span.start() as usize..fix.span.end() as usize],
            "append items \" $item\"",
        );
    }

    #[test]
    fn literal_piece_and_array_target_also_rewrite() {
        let d = w104_for("append out(k) \" item\"\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "lappend out(k) item");
    }

    #[test]
    fn trailing_space_shape_gets_no_fix() {
        // `append msg "item "` puts the separator *after* the piece;
        // `lappend` would move it before — not equivalent.
        let d = w104_for("append msg \"item \"\n");
        assert!(d.fixes.is_empty(), "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn non_mechanical_shapes_get_no_fix() {
        for src in [
            // Several pieces in one value word.
            "set a 1\nset b 2\nappend out \" $a $b\"\n",
            // Extra pad spaces.
            "set a 1\nappend out \"  $a\"\n",
            // Several value words.
            "set a 1\nappend out \" $a\" tail\n",
            // Braced value — a deliberate literal separator.
            "append banner { }\n",
            // Piece with a word/list metacharacter.
            "set a 1\nappend out \" [list $a]\"\n",
            // Pad-only value.
            "append out \" \"\n",
        ] {
            let d = w104_for(src);
            assert!(
                d.fixes.is_empty(),
                "no fix expected for {src:?}: {:?}",
                d.fixes
            );
        }
    }
}

mod w114_unwrap_fix {
    //! W114 — a redundant nested `[expr {…}]` inside a braced outer
    //! expression carries an unwrap fix; unbraced shapes and
    //! string-comparison contexts stay message-only.

    use super::*;

    fn w114_for(src: &str) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        r.diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W114)
            .unwrap_or_else(|| panic!("expected W114 in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn compound_inner_body_is_parenthesised() {
        let src = "set a 1\nset b 3\nif {$a && [expr {$b + 1}]} { puts hi }\n";
        let d = w114_for(src);
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        let fix = &d.fixes[0];
        assert_eq!(fix.new_text, "($b + 1)");
        assert_eq!(fix.description, "Unwrap the nested `expr`");
        // The fix replaces exactly the nested `[expr {…}]` — the
        // diagnostic's own span.
        assert_eq!(fix.span, d.span);
        assert_eq!(
            &src[fix.span.start() as usize..fix.span.end() as usize],
            "[expr {$b + 1}]",
        );
    }

    #[test]
    fn atom_inner_body_drops_the_parentheses() {
        let src = "set x 2\nset y [expr {[expr {$x}] + 1}]\n";
        let d = w114_for(src);
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "$x");
        assert_eq!(
            &src[d.fixes[0].span.start() as usize..d.fixes[0].span.end() as usize],
            "[expr {$x}]",
        );
    }

    #[test]
    fn literal_atom_also_drops_the_parentheses() {
        let d = w114_for("set y [expr {[expr {42}] + 1}]\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "42");
    }

    #[test]
    fn unbraced_inner_body_gets_no_fix() {
        // Inlining an unbraced body re-exposes it to substitution.
        let d = w114_for("set x 1\nif {[expr $x + 1] > 0} {}\n");
        assert!(d.fixes.is_empty(), "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn unbraced_outer_argument_gets_no_fix() {
        // `if [expr {$x}] …` — the whole argument is the nested call;
        // unwrapping to `($x)` would change the substitution pipeline.
        let d = w114_for("set x 1\nif [expr {$x}] {}\n");
        assert!(d.fixes.is_empty(), "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn string_comparison_context_gets_no_fix() {
        // `[expr {$s}]` normalises "007" to 7; `($s) eq "007"` would
        // not — the unwrap could flip the verdict, so no fix.
        let d = w114_for("set s 007\nif {[expr {$s}] eq \"007\"} {}\n");
        assert!(d.fixes.is_empty(), "no fix expected: {:?}", d.fixes);
    }

    /// TIP 461's `lt`/`le`/`gt`/`ge` share the exact same numeric-
    /// normalisation risk as `eq`/`ne` (issue #983/#986: this guard used to
    /// be a hand-typed 4-entry list that only named `eq`/`ne`/`in`/`ni`,
    /// missing these four entirely — a live safety gap in an *automatic*
    /// code fix, not just a cosmetic one).
    #[test]
    fn tip461_string_ordering_context_gets_no_fix() {
        let d = w114_for("set s 007\nif {[expr {$s}] lt \"010\"} {}\n");
        assert!(d.fixes.is_empty(), "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn multi_group_expr_arguments_get_no_fix() {
        // `[expr {a} {b}]` concatenates its arguments — not one braced
        // group, so no textual inline.
        let d = w114_for("set a 1\nif {[expr {$a} {+ 1}] > 0} {}\n");
        assert!(d.fixes.is_empty(), "no fix expected: {:?}", d.fixes);
    }
}

// -- issue #934: colon-named definitions --------------------------------

/// The count of W314 diagnostics `src` draws under the plain `tcl` dialect.
fn w314_count(src: &str) -> usize {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W314)
        .count()
}

#[test]
fn w314_fires_for_colon_named_proc_class_and_namespace() {
    // TP: an all-colon simple name has no absolute written form.
    assert_eq!(w314_count("proc : args { return hello }\n"), 1);
    assert_eq!(w314_count("oo::class create :\n"), 1);
    assert_eq!(w314_count("namespace eval : { }\n"), 1);
    // ... inside a namespace, still only reachable relatively.
    assert_eq!(w314_count("namespace eval x { proc : {} {} }\n"), 1);
}

#[test]
fn w314_does_not_fire_for_addressable_colon_shapes() {
    // A lone interior colon is an ordinary name character.
    assert_eq!(w314_count("proc a:b {} {}\n"), 0);
    assert_eq!(w314_count("proc :2 {} {}\n"), 0);
    // The empty-string name IS absolutely addressable (`::` / `::x::`).
    assert_eq!(w314_count("proc {} args {}\n"), 0);
    assert_eq!(w314_count("namespace eval x {}\nproc x:: {} {}\n"), 0);
    // A written all-colon word names the global `{}` proc — addressable.
    assert_eq!(w314_count("proc ::: args {}\n"), 0);
    // Ordinary namespaces never fire.
    assert_eq!(w314_count("namespace eval a::b { proc p {} {} }\n"), 0);
}

#[test]
fn w314_namespace_flags_once_without_per_definition_cascade() {
    // The namespace-eval site carries the warning; an addressable simple
    // name inside it does not re-fire per definition.
    assert_eq!(w314_count("namespace eval : { proc helper {} {} }\n"), 1);
    // A colon-named proc *inside* a colon namespace is its own defect too.
    assert_eq!(w314_count("namespace eval : { proc : {} {} }\n"), 2);
}

#[test]
fn colon_named_proc_keeps_its_simple_name_and_key() {
    // The reported #934 crash: `proc :` produced an empty simple name (the
    // documentSymbol name), because the key was collapsed into `::` and the
    // tail re-derived by a naive `rsplit`.
    let mut a = Analyser::new();
    let r = a.analyse("proc : args { return hello }\n", "tcl");
    let (key, def) = r
        .all_procs
        .iter()
        .find(|(_, p)| p.name_span.start() == 5)
        .expect("proc : recorded");
    assert_eq!(key, ":::", "constructed key is \"::\" + \":\"");
    assert_eq!(def.name, ":", "simple name preserved");
    assert_eq!(def.qualified_name, ":::");
}

#[test]
fn colon_named_namespace_keeps_nested_definitions_distinct() {
    // `namespace eval : { proc inner {} {} }` must home `inner` under the
    // `:`-named namespace — never collapse into the global namespace.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval : { proc inner {} {} }\nproc inner {} {}\n",
        "tcl",
    );
    assert!(
        r.all_procs.contains_key(":::::inner"),
        "nested key = ns key \":::\" + \"::\" + \"inner\": {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
    assert!(
        r.all_procs.contains_key("::inner"),
        "the global `inner` is a distinct definition"
    );
}

#[test]
fn colon_named_proc_resolves_from_bare_calls_not_written_runs() {
    // Construction symmetry: a bare `:` call settles onto the `:` proc; a
    // written `:::` names the global `{}` command instead (tclsh-pinned).
    let mut a = Analyser::new();
    let r = a.analyse("proc : args {}\nproc {} args {}\n: x\n::: y\n", "tcl");
    let resolved: Vec<(&str, Option<&str>)> = r
        .command_invocations
        .iter()
        .filter(|i| i.name == ":" || i.name == ":::")
        .map(|i| (i.name.as_str(), i.resolved_qualified_name.as_deref()))
        .collect();
    assert!(
        resolved.contains(&(":", Some(":::"))),
        "bare `:` settles onto the colon proc: {resolved:?}"
    );
    assert!(
        resolved.contains(&(":::", Some("::"))),
        "written `:::` settles onto the `{{}}` proc: {resolved:?}"
    );
}

// -- M7: command names carried in variables / dispatch tables ------------

// Issue #1009 — the constant-`$cmd` dispatch settlement's `known` /
// `user_defined` closures resolved through a proc/class/alias/rename
// target that was renamed or deleted away, with no later
// re-establishment, exactly like the pre-#973 bug in `scope.rs`. Fixed
// by reusing `fact_live_for_call` (widened to `pub(super)` so this
// sibling pass can call it) with the dispatch site's own offset as the
// call site. Confirmed this never caused a W123 false negative (the
// pass runs after W123 already fired), but did poison the
// `resolved_qualified_name` these invocations carry for hover /
// go-to-definition / find-references / rename-tracking. All cases
// confirmed against tclsh 8.6.14 (deletion semantics are identical
// whether a command is invoked literally or via a variable).

fn const_dispatch_target(src: &str) -> Option<(String, Option<String>)> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    r.command_invocations
        .into_iter()
        .find(|i| i.range.start() == dispatch && i.indirect)
        .map(|i| (i.name, i.resolved_qualified_name))
}

#[test]
fn const_dispatch_tp_issue_1009_proc_deleted_no_reestablishment_does_not_resolve() {
    let src = "proc target {} {}\nrename target {}\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        None,
        "a $cmd dispatch to a deleted proc with no re-establishment must not resolve"
    );
}

#[test]
fn const_dispatch_fp_issue_1009_proc_reestablished_after_deletion_resolves() {
    let src = "proc target {} {}\nrename target {}\nproc target {} {}\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("target".to_owned(), Some("::target".to_owned())))
    );
}

#[test]
fn const_dispatch_tn_issue_1009_proc_no_deletion_still_resolves() {
    let src = "proc target {} {}\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("target".to_owned(), Some("::target".to_owned())))
    );
}

#[test]
fn const_dispatch_tp_issue_1009_class_deleted_no_reestablishment_does_not_resolve() {
    let src = "oo::class create Target\nrename Target {}\nset cmd Target\n$cmd\n";
    assert_eq!(const_dispatch_target(src), None);
}

#[test]
fn const_dispatch_tp_issue_1009_rename_target_deleted_no_reestablishment_does_not_resolve() {
    let src = "proc helper {} {}\nrename helper ha2\nrename ha2 {}\nset cmd ha2\n$cmd\n";
    assert_eq!(const_dispatch_target(src), None);
}

#[test]
fn const_dispatch_fp_issue_1009_rename_target_no_deletion_still_resolves() {
    let src = "proc helper {} {}\nrename helper ha2\nset cmd ha2\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("ha2".to_owned(), Some("::ha2".to_owned())))
    );
}

#[test]
fn const_dispatch_tp_issue_1009_alias_deleted_no_reestablishment_does_not_resolve() {
    let src = "proc target {} {}\ninterp alias {} short {} target\ninterp alias {} short {}\nset cmd short\n$cmd\n";
    assert_eq!(const_dispatch_target(src), None);
}

#[test]
fn const_dispatch_fp_issue_1009_alias_no_deletion_still_resolves() {
    let src = "proc target {} {}\ninterp alias {} short {} target\nset cmd short\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("short".to_owned(), Some("::short".to_owned())))
    );
}

#[test]
fn const_dispatch_fp_issue_1009_deletion_inside_never_triggered_body_still_resolves() {
    // Same conditional-body guard #1006/#1007 already apply — a deletion
    // recorded inside a proc that's never called must not disqualify the
    // dispatch (tclsh: calling nothing that invokes `maybeDelete` leaves
    // `target` live).
    let src = "proc target {} {}\nproc maybeDelete {} { rename target {} }\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("target".to_owned(), Some("::target".to_owned())))
    );
}

#[test]
fn const_cmd_head_records_a_reference_to_the_dispatched_proc_m7() {
    let mut a = Analyser::new();
    let src = "proc target {} {}\nset cmd target\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.find("$cmd").unwrap()).unwrap();
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.range.start() == dispatch && i.name == "target");
    assert!(
        inv.is_some_and(|i| i.resolved_qualified_name.as_deref() == Some("::target") && i.indirect),
        "const $cmd dispatch must reference ::target (indirect): {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start(), i.indirect))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn const_cmd_head_resolves_in_the_dispatch_namespace_m7() {
    let mut a = Analyser::new();
    let src = "namespace eval ns { proc target {} {} }\n\
               namespace eval ns { set cmd target; $cmd }\n";
    let r = a.analyse(src, "tcl");
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::ns::target")),
        "{:?}",
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .map(|i| (&i.name, &i.resolved_qualified_name))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn const_cmd_head_abstains_on_unknown_or_dynamic_values_m7() {
    // Unknown value: no invocation appears (and no W123 arises from it).
    let mut a = Analyser::new();
    let r = a.analyse("set cmd nosuchcmd\n$cmd\n", "tcl");
    assert!(!r.command_invocations.iter().any(|i| i.name == "nosuchcmd"));
    // Computed value (command substitution): the value oracle abstains.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("proc target {} {}\nset cmd [pick]\n$cmd\n", "tcl");
    assert!(!r2.command_invocations.iter().any(|i| i.indirect));
    // Interpolated value (`x$suffix`): not a written constant — abstain.
    let mut a4 = Analyser::new();
    let r4 = a4.analyse(
        "proc target {} {}\nset suffix arget\nset cmd t$suffix\n$cmd\n",
        "tcl",
    );
    assert!(!r4.command_invocations.iter().any(|i| i.indirect));
    // A builtin value carries no navigable definition: abstain.
    let mut a3 = Analyser::new();
    let r3 = a3.analyse("set cmd puts\n$cmd hi\n", "tcl");
    assert!(!r3.command_invocations.iter().any(|i| i.indirect));
}

#[test]
fn const_cmd_head_resolves_through_a_pure_copy_chain_m7() {
    // `set x target; set cmd $x; $cmd` dispatches ::target — the copy
    // chain preserves provenance, so the *ultimate* literal (`target` in
    // `set x target`) is the writable reference (issue #945 fault 1:
    // renaming must rewrite that literal, keeping the dispatch alive).
    let mut a = Analyser::new();
    let src = "proc target {} {}\nset x target\nset cmd $x\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.find("$cmd").unwrap()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.indirect
            && i.range.start() == dispatch
            && i.resolved_qualified_name.as_deref() == Some("::target")
            && i.rename_safe),
        "copy-chained const dispatch must reference ::target: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start(), i.indirect))
            .collect::<Vec<_>>(),
    );
    let lit = u32::try_from(src.find("set x target").unwrap() + "set x ".len()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| !i.indirect
            && i.range.start() == lit
            && i.resolved_qualified_name.as_deref() == Some("::target")),
        "the ultimate literal must be the writable reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start(), i.indirect))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn branch_joined_const_dispatch_records_every_may_target_945() {
    // Issue #945 fault 2: `set cmd foo; if {$runtime} { set cmd bar };
    // $cmd` dispatches ::foo when the branch is not taken and ::bar when
    // it is (tclsh 9.0.4: runtime=0 → foo, runtime=1 → bar).  The value
    // at the dispatch point is the SSA φ-join {foo, bar} — never the
    // lexically last write — so BOTH procs are referenced, each with its
    // own writable literal.
    let mut a = Analyser::new();
    let src = "proc foo {} {return foo}\nproc bar {} {return bar}\n\
               set cmd foo\nif {$runtime} {\n    set cmd bar\n}\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    let heads: Vec<&str> = r
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads.contains(&"::foo") && heads.contains(&"::bar"),
        "both may-targets must be referenced at the join: {heads:?}",
    );
    // Each contributing literal is a writable reference to its own target.
    let foo_lit = u32::try_from(src.find("set cmd foo").unwrap() + "set cmd ".len()).unwrap();
    let bar_lit = u32::try_from(src.find("set cmd bar").unwrap() + "set cmd ".len()).unwrap();
    let lit_of = |target: &str| {
        r.command_invocations
            .iter()
            .find(|i| !i.indirect && i.resolved_qualified_name.as_deref() == Some(target))
            .map(|i| i.range.start())
    };
    assert_eq!(lit_of("::foo"), Some(foo_lit), "foo literal is writable");
    assert_eq!(lit_of("::bar"), Some(bar_lit), "bar literal is writable");
}

#[test]
fn switch_and_loop_joined_const_dispatch_records_every_may_target_945() {
    // The same φ-join soundness across `switch` arms…
    let mut a = Analyser::new();
    let src = "proc left {} {}\nproc right {} {}\nset cmd left\n\
               switch -- $x {\n  a { set cmd right }\n  default {}\n}\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    let heads: Vec<&str> = r
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads.contains(&"::left") && heads.contains(&"::right"),
        "switch join must keep both may-targets: {heads:?}",
    );
    // …and across a `while` back-edge (the loop-carried φ folds the
    // pre-loop literal with the in-loop reassignment).
    let mut a2 = Analyser::new();
    let src2 = "proc first {} {}\nproc next {} {}\nset cmd first\n\
                while {$go} {\n    $cmd\n    set cmd next\n}\n";
    let r2 = a2.analyse(src2, "tcl");
    let dispatch2 = u32::try_from(src2.find("$cmd").unwrap()).unwrap();
    let heads2: Vec<&str> = r2
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch2)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads2.contains(&"::first") && heads2.contains(&"::next"),
        "loop-carried join must keep both may-targets: {heads2:?}",
    );
}

#[test]
fn catch_and_try_body_writes_abstain_never_last_write_945() {
    // The CFG deliberately models a `catch` body as one opaque call with
    // summarised variable defs (`emit_opaque_catch`) — the body's writes
    // have no per-branch structure to join.  The provenance walk sees a
    // non-literal defining statement and **abstains**: no indirect
    // reference at all, and in particular never the old lexical map's
    // answer (the body's `set cmd risky` presented as the unconditional
    // value, issue #945 fault 2).  Sound abstention is the contract:
    // no false single-target definition, no destructive rename edit.
    let mut a = Analyser::new();
    let src = "proc safe {} {}\nproc risky {} {}\nset cmd safe\n\
               catch {\n    set cmd risky\n}\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.indirect && i.range.start() == dispatch),
        "an opaque catch write must abstain, not settle to a single target",
    );
    // A `try` body, by contrast, inlines with real CFG structure in
    // analysis builds, so its φ-join keeps BOTH may-targets — the body
    // may error before the write (`safe` survives) or complete
    // (`risky`) — and each side keeps its writable literal.
    let mut a2 = Analyser::new();
    let src2 = "proc safe {} {}\nproc risky {} {}\nset cmd safe\n\
                try {\n    set cmd risky\n} on error {} {}\n$cmd\n";
    let r2 = a2.analyse(src2, "tcl");
    let dispatch2 = u32::try_from(src2.rfind("$cmd").unwrap()).unwrap();
    let heads2: Vec<&str> = r2
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch2)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads2.contains(&"::safe") && heads2.contains(&"::risky"),
        "try join must keep both may-targets: {heads2:?}",
    );
}

#[test]
fn unprovable_const_dispatch_shapes_abstain_945() {
    // A proc parameter: the value flows in from the caller — abstain.
    let mut a = Analyser::new();
    let r = a.analyse("proc run {cmd} {\n    $cmd\n}\n", "tcl");
    assert!(!r.command_invocations.iter().any(|i| i.indirect));
    // A write reachable through `upvar`: the alias write is not a local
    // literal definition — abstain.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "proc target {} {}\nproc setit {v} {upvar 1 $v x; set x target}\n\
         proc go {} {\n    set cmd other\n    setit cmd\n    $cmd\n}\n",
        "tcl",
    );
    let dispatch = 0; // any indirect site inside `go` would be unsound
    let _ = dispatch;
    assert!(
        !r2.command_invocations
            .iter()
            .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::target")),
        "an upvar-mutated head must not settle to the callee's write",
    );
}

#[test]
fn namespace_qualified_const_value_resolves_absolutely_945() {
    let mut a = Analyser::new();
    let src = "namespace eval ns { proc target {} {} }\nset cmd ::ns::target\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.indirect
            && i.range.start() == dispatch
            && i.resolved_qualified_name.as_deref() == Some("::ns::target")),
        "an absolute value resolves to itself: {:?}",
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .map(|i| (&i.name, &i.resolved_qualified_name))
            .collect::<Vec<_>>(),
    );
    // The literal is the writable component.
    let lit = u32::try_from(src.find("::ns::target\n").unwrap()).unwrap();
    assert!(
        r.command_invocations
            .iter()
            .any(|i| !i.indirect && i.range.start() == lit),
        "the qualified literal is a writable reference",
    );
}

#[test]
fn traced_head_variable_abstains_945() {
    // A write trace can rewrite `cmd` at any moment (tclsh 9.0.4: the
    // callback fires on every write, and reads via `trace add variable …
    // read` can substitute values) — the reaching-definition walk cannot
    // see those writes, so a traced dispatch head abstains entirely.
    let mut a = Analyser::new();
    let src = "proc target {} {}\nproc redirect {n1 n2 op} {}\n\
               set cmd target\ntrace add variable cmd write redirect\n$cmd\n";
    let r = a.analyse(src, "tcl");
    assert!(
        !r.command_invocations.iter().any(|i| i.indirect),
        "a traced head must abstain: {:?}",
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .map(|i| &i.name)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn expanded_head_list_prefix_keeps_the_exact_writable_component_945() {
    // `{*}$cmd` expands the value as a command *prefix* — tclsh 9.0.4
    // dispatches its first list element with the rest appended as
    // arguments.  The writable provenance narrows to that element's
    // sub-span inside the defining literal (`helper` within
    // `{helper arg}`), so a rename rewrites exactly the command
    // component and keeps the baked argument.
    let mut a = Analyser::new();
    let src = "proc helper {a} {}\nset cmd {helper arg}\n{*}$cmd\n";
    let r = a.analyse(src, "tcl");
    let heads: Vec<&str> = r
        .command_invocations
        .iter()
        .filter(|i| i.indirect)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads.contains(&"::helper"),
        "the prefix's first element dispatches ::helper: {heads:?}",
    );
    let lit_start = u32::try_from(src.find("helper arg").unwrap()).unwrap();
    let lit_end = lit_start + u32::try_from("helper".len()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| !i.indirect
            && i.range.start() == lit_start
            && i.range.end() == lit_end
            && i.resolved_qualified_name.as_deref() == Some("::helper")),
        "the writable span covers exactly the command component: {:?}",
        r.command_invocations
            .iter()
            .filter(|i| !i.indirect && i.name.contains("helper"))
            .map(|i| (i.range.start(), i.range.end()))
            .collect::<Vec<_>>(),
    );
    // A plain (non-expanded) `$cmd` head treats the whole value as the
    // command name — a two-element value names no known command, so the
    // site abstains rather than resolving the first element.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("proc helper {a} {}\nset cmd {helper arg}\n$cmd\n", "tcl");
    assert!(
        !r2.command_invocations.iter().any(|i| i.indirect),
        "a non-expanded multi-word value is not a prefix dispatch",
    );
}

#[test]
fn dispatch_table_value_becomes_a_reference_when_consumed_m7() {
    let mut a = Analyser::new();
    let src = "proc do_add {a b} {}\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    let r = a.analyse(src, "tcl");
    let lit = u32::try_from(src.find("add do_add").unwrap() + 4).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.name == "do_add"
            && i.range.start() == lit
            && i.resolved_qualified_name.as_deref() == Some("::do_add")
            && !i.indirect),
        "table value literal must reference ::do_add: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dispatch_table_value_abstains_when_the_table_is_not_consumed_m7() {
    // The same table with no dispatch site is configuration data — its
    // values must not gain phantom references.
    let mut a = Analyser::new();
    let src = "proc do_add {a b} {}\narray set ops {add do_add}\n";
    let r = a.analyse(src, "tcl");
    assert!(
        !r.command_invocations.iter().any(|i| i.name == "do_add"),
        "{:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

// Issue #1010 (site 3) — `emit_dispatch_table_command_references`'s
// `known` closure synthesized a "live reference" for a dispatch-table
// literal even when the proc/class it named was renamed or deleted away
// with no later re-establishment. Fixed by reusing `fact_live_for_call`
// with the table entry's own position as the call site. Confirmed
// against tclsh 8.6.14.

#[test]
fn dispatch_table_tp_issue_1010_deleted_proc_draws_no_reference() {
    let src = "proc do_add {a b} {}\nrename do_add {}\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    // `rename do_add {}` itself intentionally draws its own self-reference
    // to the OLD argument's token (issue #923 idx 39 — go-to-definition on
    // that exact written word must still resolve, and real Tcl requires
    // `do_add` to exist at that point). That reference is not what this
    // test guards against; only a *dispatch-table*-synthesized reference
    // (via `array set ops {add do_add}` / `$ops($k)`) to the deleted,
    // never-re-established proc must be absent.
    let rename_self_ref_offset =
        u32::try_from(src.find("rename do_add").unwrap() + "rename ".len()).unwrap();
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.name == "do_add" && i.range.start() != rename_self_ref_offset),
        "a deleted proc with no re-establishment must draw no reference beyond rename's own self-reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dispatch_table_fp_issue_1010_reestablished_proc_still_references() {
    let src = "proc do_add {a b} {}\nrename do_add {}\nproc do_add {a b} {}\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.name == "do_add" && i.resolved_qualified_name.as_deref() == Some("::do_add")),
        "a proc re-established after deletion must still be referenced: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dict_set_table_value_becomes_a_reference_when_consumed_m7() {
    let mut a = Analyser::new();
    let src =
        "proc handle_get {} {}\ndict set handlers GET handle_get\n{*}[dict get $handlers GET]\n";
    let r = a.analyse(src, "tcl");
    let lit = u32::try_from(src.find(" handle_get\n").unwrap() + 1).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.name == "handle_get"
            && i.range.start() == lit
            && i.resolved_qualified_name.as_deref() == Some("::handle_get")),
        "{:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

// -- M9: source-site namespace propagation (seeded analysis) --------------

#[test]
fn seeded_analysis_homes_relative_defs_under_the_source_namespace_m9() {
    // `source` evaluates the file in the caller's namespace: analysing with
    // a seed must behave exactly like wrapping the file in
    // `namespace eval ::x { ... }` — relative names re-home, absolute stay.
    let mut a = Analyser::new();
    let r = a.analyse_with_source_namespace(
        "proc helper {} {}\nproc ::abs::keep {} {}\nnamespace eval rel { proc deep {} {} }\nhelper\n",
        "tcl",
        "::x",
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        r.all_procs.contains_key("::abs::keep"),
        "an absolute definition is not re-homed"
    );
    assert!(
        r.all_procs.contains_key("::x::rel::deep"),
        "a relative namespace nests under the seed: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.name == "helper"
                && i.resolved_qualified_name.as_deref() == Some("::x::helper")),
        "a bare call settles inside the seeded namespace first"
    );
}

#[test]
fn seeded_analysis_records_composed_source_site_namespaces_m9() {
    // A nested `source` inside the seeded file composes: its recorded site
    // namespace already carries the seed, so transitive re-homing needs no
    // separate composition step.
    let mut a = Analyser::new();
    let r = a.analyse_with_source_namespace(
        "source plain.tcl\nnamespace eval sub { source nested.tcl }\n",
        "tcl",
        "::x",
    );
    let sites: Vec<(&str, &str)> = r
        .source_targets
        .iter()
        .map(|s| (s.raw_path.as_str(), s.site_namespace.as_str()))
        .collect();
    assert!(sites.contains(&("plain.tcl", "::x")), "{sites:?}");
    assert!(sites.contains(&("nested.tcl", "::x::sub")), "{sites:?}");
}

#[test]
fn global_seed_is_the_plain_analysis_m9() {
    let mut a = Analyser::new();
    let r = a.analyse_with_source_namespace("proc helper {} {}\n", "tcl", "::");
    assert!(r.all_procs.contains_key("::helper"));
}

// ===========================================================================
// M11 (TIP 278) — cross-version namespace-scope variable fallback.
//
// C-Tcl facts, pinned live against tclsh8.6 (8.6.16) and tclsh9.0 (9.0.4)
// (the same table `rust/tcl-vm/tests/cross_version_vars_e2e.rs` executes
// under both binaries):
//   * `set g 1; namespace eval foo { set g }`      8.6 → 1, 9.0 → error
//   * `set v 1; namespace eval bar { variable v; set v }`  both → error
//     (a declared-but-unset `variable` blocks the fallback)
//   * `namespace eval a { set x 1; namespace eval b { set x } }`
//     both → error (an *intermediate* namespace is never consulted)
//   * `set g 1; proc p {} { set g }; p`            both → error
//     (proc frames never had the fallback)
// ===========================================================================

/// The recorded reference spans of the global-scope variable `name`.
fn global_var_refs(result: &crate::analyser::AnalysisResult, name: &str) -> usize {
    result
        .global_scope
        .variables
        .get(name)
        .map_or(0, |v| v.references.len())
}

#[test]
fn ns_scope_read_attaches_to_the_global_var_under_8x_only_m11() {
    let src = "set g 1\nnamespace eval foo { puts $g }\n";
    // TP: 8.x — the namespace-frame read is a genuine use of `::g`.
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "g"),
        1,
        "8.x: the `$g` read at namespace scope resolves to (and must be \
         recorded on) the global cell"
    );
    // TN: 9.0 removed the fallback — the read errors at runtime, so it must
    // NOT count as a reference of the global.
    let mut a = Analyser::new();
    let r90 = a.analyse(src, "tcl9.0");
    assert_eq!(
        global_var_refs(&r90, "g"),
        0,
        "9.0: no fallback — attaching the read would invent a reference"
    );
}

#[test]
fn ns_scope_declared_variable_blocks_the_8x_attach_m11() {
    // FP guard: `variable v` creates the cell in the namespace's table, so
    // even under 8.x the read resolves namespace-locally (tclsh8.6 errors on
    // the read — it does NOT see the global `v`).
    let src = "set v 1\nnamespace eval bar { variable v; puts $v }\n";
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "v"),
        0,
        "the declared namespace `v` shadows the global in both versions"
    );
}

#[test]
fn proc_read_never_attaches_via_the_ns_fallback_m11() {
    // TN: the fallback is a **namespace-frame** rule; a proc frame resolves
    // bare names locally in every Tcl version (tclsh8.6 and 9.0 both error).
    let src = "set g 1\nproc p {} { puts $g }\n";
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "g"),
        0,
        "a proc-body read must not be recorded on the global via the \
         namespace fallback"
    );
}

#[test]
fn ns_scope_read_never_attaches_to_an_intermediate_namespace_m11() {
    // FP guard: `$x` at `::a::b` scope sees `::a::b::x` and (8.x only) `::x`
    // — never `::a::x`, in ANY version (both tclshs error on this script).
    let src = "namespace eval a { set x 1\nnamespace eval b { puts $x } }\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let mut a = Analyser::new();
        let r = a.analyse(src, dialect);
        let ns_a = r
            .global_scope
            .children
            .iter()
            .find(|s| s.name == "a")
            .expect("namespace scope `a` exists");
        let x_refs = ns_a.variables.get("x").map_or(0, |v| v.references.len());
        assert_eq!(
            x_refs, 0,
            "[{dialect}] `::a::x` must gain no reference from the read at \
             `::a::b` scope"
        );
    }
}

// Dialect-profile availability (dialect-profile-model.md, Milestone 2): the
// composed (version|vendor) masks admit each vendor dialect's embedded Tcl
// core, the version ladder still gates later-version core, iRules stays
// subtractive, and unknown dialects stay permissive.

#[test]
fn w123_vendor_profiles_admit_their_embedded_tcl_core() {
    // FP-fix (the confirmed bare-bit defect): real embedded-core commands
    // must resolve cleanly — no W123 (unknown) and no W002 (disabled).
    let clean: &[(&str, &str)] = &[
        // iApps run a Tcl 8.5.13 host interpreter: 8.5 core is real.
        ("dict get {a 1} a", "f5-iapps"),
        ("lassign {1 2} a b", "f5-iapps"),
        ("apply {{x} {return $x}} 1", "f5-iapps"),
        // ... and the host interpreter is NOT the TMM sandbox: exec is real.
        ("exec /bin/true", "f5-iapps"),
        // Expect embeds Tcl 8.6: 8.5 and 8.6 core are real.
        ("dict get {a 1} a", "expect"),
        ("lmap x {1 2} {set x}", "expect"),
        ("coroutine c ::apply {{} {}}", "expect"),
        // EDA shells: 8.5 base (xilinx) and 8.6 base (synopsys).
        ("dict get {a 1} a", "xilinx-eda-tcl"),
        ("lmap x {1 2} {set x}", "synopsys-eda-tcl"),
    ];
    for (snippet, dialect) in clean {
        let codes = codes_for_dialect(snippet, dialect);
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "{dialect}: {snippet:?} is embedded-core and must not flag, got {codes:?}"
        );
    }
}

#[test]
fn ns_scope_array_element_read_attaches_under_8x_m11() {
    // The fallback covers array-element reads too (pinned vector: `set
    // arr(k) AV; namespace eval foo { set arr(k) }` reads AV on 8.6).
    let src = "set arr(k) AV\nnamespace eval foo { puts $arr(k) }\n";
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "arr"),
        1,
        "8.x: element read attaches"
    );
    let mut a = Analyser::new();
    let r90 = a.analyse(src, "tcl9.0");
    assert_eq!(global_var_refs(&r90, "arr"), 0, "9.0: no fallback");
}

#[test]
fn w123_vendor_profiles_still_gate_later_version_core() {
    // TN: the composed mask is the *embedded base* version, not a blanket
    // allow — core introduced after the base still flags.
    let flagged: &[(&str, &str)] = &[
        // lmap is 8.6; iApps embed 8.5.
        ("lmap x {1 2} {set x}", "f5-iapps"),
        ("coroutine c ::apply {{} {}}", "f5-iapps"),
        // lmap is 8.6; xilinx embeds 8.5.
        ("lmap x {1 2} {set x}", "xilinx-eda-tcl"),
        // zipfs is 9.0; expect embeds 8.6.
        ("zipfs root", "expect"),
        ("zipfs root", "synopsys-eda-tcl"),
    ];
    for (snippet, dialect) in flagged {
        let codes = codes_for_dialect(snippet, dialect);
        assert!(
            codes.iter().any(|c| c == "W123"),
            "{dialect}: {snippet:?} is post-base core and must draw W123, got {codes:?}"
        );
    }
}

#[test]
fn w123_vendor_profiles_still_flag_genuinely_unknown_commands() {
    // TP: the widened masks must not swallow real unknowns.
    for dialect in ["f5-iapps", "expect", "xilinx-eda-tcl"] {
        assert!(
            has_code("frobnicate_no_such_cmd a b\n", dialect, "W123"),
            "{dialect}: genuinely unknown command must still draw W123"
        );
    }
}

#[test]
fn irules_stays_subtractive_under_the_profile() {
    // The subtractive-iRules trap (§9): the profile keeps the bare IRULES
    // mask + disable list, so nothing moves for iRules files.
    // Banned 8.4 core: exists elsewhere → W002 (and W123 both fire).
    for banned in ["exec /bin/true", "file exists /tmp", "socket -server x 80"] {
        let codes = codes_for_dialect(banned, "f5-irules");
        assert!(
            codes.iter().any(|c| c == "W002"),
            "f5-irules: {banned:?} is banned and must draw W002, got {codes:?}"
        );
    }
    // 8.5+/8.6 core: never present at ANY BIG-IP version (D3).
    for versioned in ["dict get {a 1} a", "lmap x {1 2} {set x}"] {
        let codes = codes_for_dialect(versioned, "f5-irules");
        assert!(
            codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-irules: {versioned:?} must stay unavailable, got {codes:?}"
        );
    }
    // The universal core and F5 surface stay clean.
    for ok in ["set x 1", "log local0. hi"] {
        let codes = codes_for_dialect(ok, "f5-irules");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-irules: {ok:?} must stay clean, got {codes:?}"
        );
    }
}

#[test]
fn irules_alias_dialect_string_behaves_like_canonical() {
    // §2.4 alias canonicalisation: the legacy "irules" spelling used to fall
    // through DialectSet::parse to the permissive ALL_TCL view (a silent
    // false negative); via the profile catalog it now resolves like
    // f5-irules.
    let codes = codes_for_dialect("exec /bin/true", "irules");
    assert!(
        codes.iter().any(|c| c == "W002"),
        "alias 'irules' must ban exec like f5-irules, got {codes:?}"
    );
    assert!(
        !has_code("set x 1", "irules", "W123"),
        "universal core stays clean under the alias"
    );
}

#[test]
fn unknown_dialect_strings_stay_permissive() {
    // §8: the PLAIN_TCL sink — a typo'd dialect must flag nothing, exactly
    // as the old unwrap_or(ALL_TCL) fallbacks behaved.
    for snippet in ["dict get {a 1} a", "zipfs root", "exec /bin/true"] {
        let codes = codes_for_dialect(snippet, "definitely-not-a-dialect");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "unknown dialect must stay permissive for {snippet:?}, got {codes:?}"
        );
    }
}

#[test]
fn w001_subcommand_checks_use_the_profile_mask() {
    // Subcommand-level: once `dict` resolves under f5-iapps, its 8.5-valid
    // subcommands must not draw the W001/W002 subcommand diagnostics either.
    let codes = codes_for_dialect("dict keys {a 1}", "f5-iapps");
    assert!(
        !codes.iter().any(|c| c == "W001" || c == "W002"),
        "dict keys is 8.5-valid under f5-iapps, got {codes:?}"
    );
    // A genuinely unknown subcommand still fires W001 under the vendor mask.
    assert!(
        has_code("dict zzznotasub {a 1}", "f5-iapps", "W001"),
        "unknown dict subcommand must still draw W001 under f5-iapps"
    );
}

#[test]
fn tmsh_first_class_resolves_its_surface_and_gates_later_core() {
    // Milestone 6 (D8): f5-tmsh = TCL85|TMSH — a Tcl 8.5 host plus the
    // tmsh:: surface.
    // TP (the fix): the tmsh:: surface stops drawing unknown-command.
    for ok in [
        "tmsh::create ltm pool p1",
        "tmsh::log local0.info \"hi\"",
        "tmsh::list ltm virtual",
    ] {
        let codes = codes_for_dialect(ok, "f5-tmsh");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-tmsh: {ok:?} is the tmsh surface, got {codes:?}"
        );
    }
    // TN: the 8.5 core is real.
    for ok in [
        "dict get {a 1} a",
        "lassign {1 2} a b",
        "apply {{x} {return $x}} 1",
    ] {
        let codes = codes_for_dialect(ok, "f5-tmsh");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-tmsh: {ok:?} is 8.5 core, got {codes:?}"
        );
    }
    // Reverse-regression (§7.2, budgeted): 8.6/9.0 core is newly unknown
    // on the 8.5 base — the old interim ALL_TCL mask hid these.
    for gated in [
        "lmap x {1 2} {set x}",
        "coroutine c ::apply {{} {}}",
        "zipfs root",
    ] {
        let codes = codes_for_dialect(gated, "f5-tmsh");
        assert!(
            codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-tmsh: {gated:?} is later-than-8.5 core and must flag, got {codes:?}"
        );
    }
    // FP-guard: the iApp-only surface is NOT part of the tmsh shell — it
    // resolves nowhere under TCL85|TMSH, drawing the disabled-in-dialect
    // diagnostic (the spec exists, in the *iApps* pack).
    let codes = codes_for_dialect("iapp::conf save\n", "f5-tmsh");
    assert!(
        codes.iter().any(|c| c == "W002" || c == "W123"),
        "iapp:: is the iApps host surface, not tmsh: {codes:?}"
    );
}

#[test]
fn bpf_precise_mask_keeps_90_core_and_drops_8x_relics() {
    // Milestone 6 (D7): bpf = TCL90|BPF — a genuine Tcl 9.0 base.
    // TN: 9.0 core (including 8.5/8.6 additions carried into 9.0) resolves.
    for ok in [
        "dict get {a 1} a",
        "lmap x {1 2} {set x}",
        "coroutine c ::apply {{} {}}",
        "zipfs root",
    ] {
        let codes = codes_for_dialect(ok, "bpf");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "bpf: {ok:?} is real on the 9.0 base, got {codes:?}"
        );
    }
    // TP (reverse-regression, budgeted): 8.x-only relics removed at the
    // 9.0 boundary are correctly unknown now — the interim ALL_TCL|BPF
    // mask wrongly admitted them.
    for relic in ["tcltest::bytestring x", "case $x in a {puts hi}"] {
        let codes = codes_for_dialect(relic, "bpf");
        assert!(
            codes.iter().any(|c| c == "W123" || c == "W002"),
            "bpf: {relic:?} was removed at the 9.0 boundary, got {codes:?}"
        );
    }
}

#[test]
fn irules_subcommands_named_like_banned_commands_resolve_cleanly() {
    // FP-fix (the Milestone 5 retag): `DNS::header cd` (the DNS
    // Checking-Disabled flag) and `IP::stats in` (inbound stats) are real
    // iRules subcommands that were bulk mis-tagged by name collision with
    // the banned `cd` command and the `in` operator spelling, and so drew
    // spurious subcommand diagnostics. Exclusion is keyed on the resolved
    // spec, never on a bare name.
    for snippet in ["DNS::header cd", "IP::stats in"] {
        let codes = codes_for_dialect(snippet, "f5-irules");
        assert!(
            !codes
                .iter()
                .any(|c| c == "W001" || c == "W002" || c == "W123"),
            "f5-irules: {snippet:?} is a real subcommand and must stay \
             clean, got {codes:?}"
        );
    }
    // TP retained: a genuinely unknown subcommand of the same commands
    // still flags.
    assert!(
        has_code("DNS::header zzznotasub", "f5-irules", "W001"),
        "unknown DNS::header subcommand must still draw W001"
    );
}

// Behaviour axis (dialect-profile-model.md, Milestone 3): the expr grammar,
// mathfunc tiers, and octal policy resolve through the profile — including
// alias canonicalisation the string-keyed tables missed.

#[test]
fn w003_irules_alias_gates_like_the_canonical_profile() {
    // §2.4: `expr_grammar_base_version` had no arm for the legacy "irules"
    // spelling, so W003 silently never fired there — a false negative the
    // profile's alias canonicalisation fixes. Both TIPs gate on the 8.4
    // runtime, exactly as for "f5-irules".
    assert_eq!(w003_hits("expr {2 in {1 2 3}}", "irules").len(), 1);
    assert_eq!(w003_hits("if {$x lt $y} { puts hi }", "irules").len(), 1);
}

#[test]
fn w003_bpf_accepts_both_tips_on_its_tcl_9_runtime() {
    // bpf embeds Tcl 9.0 (D7): `in`/`ni` (TIP 201) and `lt`/`le`/`gt`/`ge`
    // (TIP 461) are all grammatical — no W003. (Previously bpf had no
    // documented base and W003 skipped it entirely; same outcome, now for
    // the modelled reason.)
    assert!(w003_hits("expr {2 in {1 2 3}}", "bpf").is_empty());
    assert!(w003_hits("if {$x lt $y} { puts hi }", "bpf").is_empty());
}

/// Issue #985: the 9 iRules word operators (`contains`, `and`, …) used to
/// evaluate with zero warning outside the iRules dialect — the lexer's word-
/// operator recognition, the parser, and the runtime evaluator
/// (`tcl_expr_eval.rs`'s `apply_irules_string_op`) all treat them as valid
/// regardless of dialect, so `if {$x contains "foo"}` silently ran (and
/// silently misbehaved, since core Tcl has no such operator) in every
/// non-iRules dialect. W003 now flags every one of them, the same family
/// that already flags TIP 201 (`in`/`ni`) and TIP 461 (`lt`/`le`/`gt`/`ge`).
#[test]
fn w003_fires_on_irules_word_operators_outside_irules() {
    for (src, op) in [
        (r#"if {$x contains "foo"} { puts hi }"#, "contains"),
        (r#"if {$x starts_with "foo"} { puts hi }"#, "starts_with"),
        (r#"if {$x ends_with "foo"} { puts hi }"#, "ends_with"),
        (r#"if {$x equals "foo"} { puts hi }"#, "equals"),
        (r#"if {$x matches_glob "foo*"} { puts hi }"#, "matches_glob"),
        (
            r#"if {$x matches_regex "foo.*"} { puts hi }"#,
            "matches_regex",
        ),
        ("if {$a and $b} { puts hi }", "and"),
        ("if {$a or $b} { puts hi }", "or"),
        ("if {not $a} { puts hi }", "not"),
    ] {
        let hits = w003_hits(src, "tcl9.0");
        assert_eq!(hits.len(), 1, "{src}: {hits:?}");
        assert_eq!(hits[0].0, op, "{src}");
        assert!(
            hits[0].1.message.contains("iRules"),
            "{}: {}",
            src,
            hits[0].1.message
        );
        // No safe mechanical rewrite exists for these — unlike `in`/`lt`,
        // there is no portable Tcl expression these fold into.
        assert!(hits[0].1.fixes.is_empty(), "{src}");
    }
}

#[test]
fn w003_silent_on_irules_word_operators_inside_irules() {
    for src in [
        r#"if {$x contains "foo"} { puts hi }"#,
        r#"if {$x starts_with "foo"} { puts hi }"#,
        r#"if {$x ends_with "foo"} { puts hi }"#,
        r#"if {$x equals "foo"} { puts hi }"#,
        r#"if {$x matches_glob "foo*"} { puts hi }"#,
        r#"if {$x matches_regex "foo.*"} { puts hi }"#,
        "if {$a and $b} { puts hi }",
        "if {$a or $b} { puts hi }",
        "if {not $a} { puts hi }",
    ] {
        assert!(w003_hits(src, "f5-irules").is_empty(), "{src}");
    }
}

#[test]
fn w003_fires_on_irules_word_operator_in_unbraced_multiword_expr() {
    // Exercises `emit_w003_dialect_invalid_expr_words`, the sibling path for
    // `expr`'s unbraced multi-word form (see
    // `w003_fires_on_unbraced_multiword_expr_at_a_tight_span` above).
    let hits = w003_hits("expr $a contains $b", "tcl9.0");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "contains");
    assert!(w003_hits("expr $a contains $b", "f5-irules").is_empty());
}

// Option-gating semantics (dialect-profile-model.md §5.2, Milestone 4):
// intersects membership + version ceiling, replacing the old `contains`
// rule that silently dropped inherited vendor options and never gated a
// version-ceiling leak.

#[test]
fn w004_version_gated_options_follow_the_profile_ceiling() {
    // TP: switch -nocase (8.5+) flags on the 8.4 base…
    assert!(
        has_code("switch -nocase a {a {} default {}}", "tcl8.4", "W004"),
        "switch -nocase is 8.5+ and must draw W004 under tcl8.4"
    );
    // …including the pinned-8.4 iRules runtime.
    assert!(
        has_code("switch -nocase a {a {} default {}}", "f5-irules", "W004"),
        "switch -nocase must draw W004 under f5-irules (8.4 base)"
    );
    // FP-fix: it is clean at/above 8.5 — the composed vendor profiles
    // included (the old contains rule could never satisfy a composed mask).
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "f5-iapps", "expect"] {
        assert!(
            !has_code("switch -nocase a {a {} default {}}", dialect, "W004"),
            "{dialect}: switch -nocase is real 8.5+ core"
        );
    }
}

#[test]
fn w004_later_version_options_never_leak_into_supersets() {
    // regsub -command is 9.0+: clean there, flagged below — for plain
    // versions and composed vendor profiles alike (§5.2's ceiling guard).
    assert!(!has_code(
        r#"regsub -command {x} "axb" {string toupper}"#,
        "tcl9.0",
        "W004"
    ));
    for dialect in ["tcl8.6", "f5-iapps", "expect"] {
        assert!(
            has_code(
                r#"regsub -command {x} "axb" {string toupper}"#,
                dialect,
                "W004"
            ),
            "{dialect}: regsub -command is 9.0-only and must draw W004"
        );
    }
}

// Issue #973 — a proc / class / rename / alias target that was renamed or
// deleted away, with no later re-establishment under the same name, must
// not still read as "known" for W123: calling it fails "invalid command
// name" in real Tcl. `finalise_invocation_resolutions`'s `known` predicate
// (scope.rs) and `build_w123_known_names` (unresolved.rs — the pass that
// actually decides W123) both gated deletion only for registry builtins /
// aliases / rename targets, never for user procs or classes themselves.
//
// Fixed by extending the same "was this fact re-established after its
// last deletion" question the arity resolver's `fact_superseded_by_deletion`
// already answers for E002/E003 (see
// `same_file_rename_reestablished_after_deletion_checks_new_arity` above)
// to the proc/class checks here — `fact_live_for_call` in unresolved.rs —
// with the same call-site + conditional-body awareness
// `qualified_name_deleted_before` already gives registry builtins: a
// deletion recorded inside a proc/class/method body is conditional (it
// executes only if that body is ever invoked) and a top-level call
// textually before a later deletion still resolves. All cases below
// confirmed against tclsh 8.6.14.
fn w123_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W123)
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn w123_tp_rename_of_a_nonexistent_command_is_flagged() {
    // TP — issue #923 idx 5. `rename OLD NEW` requires `OLD` to exist:
    // tclsh 9.0.4 and 8.6.16 both abort with `can't rename
    // "definitelyNotDefinedAnywhere": command doesn't exist` (exit 1). `OLD`
    // is recorded as an ordinary command reference, so W123 reports the
    // guaranteed failure at the token that causes it.
    let src = "rename definitelyNotDefinedAnywhere someAlias\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_deleting_rename_of_a_nonexistent_command_is_flagged() {
    // TP — the delete form of the same finding: tclsh 9.0.4 and 8.6.16 both
    // abort with `can't delete "totallyBogusCommand": command doesn't exist`.
    let src = "rename totallyBogusCommand {}\nputs done\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tn_rename_of_an_existing_proc_is_silent() {
    // TN — the paired guard: the source exists, so the rename is legal
    // (tclsh: `hello` then returns `hi`) and neither operand is flagged.
    let src = "proc greet {} { return hi }\nrename greet hello\nhello\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tn_rename_of_a_builtin_is_silent() {
    // TN — renaming a registry builtin away is the standard wrap idiom and
    // must not report the builtin as unknown.
    let src = "rename puts ::original_puts\nproc puts {args} { }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_proc_call_before_rename_resolves() {
    let src = "proc helper {} { return 1 }\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_proc_deleted_via_rename_no_reestablishment() {
    // The exact shape from issue #973's repro.
    let src = "\
namespace eval ::a {
    proc helper {} { return 1 }
}
rename ::a::helper {}
namespace eval ::a {
    proc caller {} { helper }
}
";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_proc_reestablished_after_deletion_resolves() {
    let src = "proc helper {} { return 1 }\nrename helper {}\nproc helper {} { return 2 }\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_class_deleted_via_rename_no_reestablishment() {
    let src = "oo::class create Helper\nrename Helper {}\nproc caller {} { Helper new }\n";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_class_call_before_rename_resolves() {
    let src = "oo::class create Helper\nproc caller {} { Helper new }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_class_reestablished_after_deletion_resolves() {
    let src = "oo::class create Helper\nrename Helper {}\noo::class create Helper\nproc caller {} { Helper new }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_interp_alias_deleted_no_reestablishment() {
    let src = "proc target {} {}\ninterp alias {} short {} target\ninterp alias {} short {}\nproc caller {} { short }\n";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_interp_alias_call_before_deletion_resolves() {
    let src = "proc target {} {}\ninterp alias {} short {} target\nproc caller {} { short }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_interp_alias_reestablished_after_deletion_resolves() {
    let src = "\
proc target {} {}
proc target2 {} {}
interp alias {} short {} target
interp alias {} short {}
interp alias {} short {} target2
proc caller {} { short }
";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_rename_to_new_name_call_to_old_name_stays_unknown() {
    // A rename that *moves* the name (not a deletion) leaves the old name
    // permanently gone — must stay unknown, same as before this fix.
    let src = "proc helper {} { return 1 }\nrename helper newhelper\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_top_level_call_before_later_proc_deletion_resolves() {
    // A top-level call textually before a *later* same-file deletion still
    // resolves — order matters at top level (tclsh: `proc helper {} {};
    // helper; rename helper {}` succeeds).
    let src = "proc helper {} { return 1 }\nhelper\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_deletion_inside_never_triggered_proc_body_resolves() {
    // A `rename` recorded inside a proc body that's never called is
    // conditional — it never executes, so the name stays live (tclsh:
    // calling `caller` without ever calling `maybeDelete` succeeds).
    let src = "proc helper {} { return 1 }\nproc maybeDelete {} { rename helper {} }\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn vendor_command_inherited_options_resolve_cleanly() {
    // expect_after's options inherit the command's EXPECT gate — under the
    // old contains rule every one of them was "unavailable" the moment the
    // active mask composed (version|vendor). No W004 under expect.
    let codes = codes_for_dialect("expect_after -re {pattern} {send ok}", "expect");
    assert!(
        !codes.iter().any(|c| c == "W004"),
        "inherited expect options must resolve under expect, got {codes:?}"
    );
}

// Issue #1006 — the W123 alias / rename-target checks used file-end-only
// gating (`fact_live_at_file_end`, no call site or conditional-body
// awareness), unlike the proc/class checks #973 already made call-site-
// and conditional-aware (`fact_live_for_call`). Fixed by replacing the
// plain `alias_names` / `rename_target_names` tail `HashSet`s'
// contribution to resolution with `alias_defs_by_tail` /
// `rename_defs_by_tail` (mirroring `proc_defs_by_tail` /
// `class_defs_by_tail`), checked per call site the same way. All cases
// confirmed against tclsh 8.6.14.

#[test]
fn w123_fp_issue_1006_alias_call_before_later_deletion_resolves() {
    let src = "proc target {} { return 1 }\ninterp alias {} short {} target\nshort\ninterp alias {} short {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1006_alias_deletion_inside_never_triggered_body_resolves() {
    let src = "proc target {} { return 1 }\ninterp alias {} short {} target\nproc maybeDelete {} { interp alias {} short {} }\nproc caller {} { short }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1006_rename_target_call_before_later_deletion_resolves() {
    let src = "proc helper {} { return 1 }\nrename helper ha2\nha2\nrename ha2 {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1006_rename_target_deletion_inside_never_triggered_body_resolves() {
    let src = "proc helper {} { return 1 }\nrename helper ha2\nproc maybeDelete {} { rename ha2 {} }\nproc caller {} { ha2 }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

// `fact_live_for_call`'s body-call escape hatch, Codex PR #1014 review
// comment #2 (`unresolved.rs:260`): a call *inside* a proc/class body
// carries no execution-order meaning from its own textual position — it
// was wrongly treated as automatically after every top-level deletion, so
// a body call whose enclosing definition demonstrably ran before a later
// deletion still drew a spurious W123. Confirmed against tclsh 8.6.14
// throughout.

#[test]
fn w123_fp_issue_1009_codex_review_body_call_before_later_deletion_resolves() {
    // FP guard (the confirmed regression): `caller`'s own top-level
    // invocation runs before `rename helper {}`, so the `helper` call
    // inside its body must still resolve — confirmed against tclsh 8.6.14
    // (the script prints "ok" and exits 0).
    let src = "proc helper {} {}\nproc caller {} { helper }\ncaller\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_1009_codex_review_deleted_before_definition_still_flags() {
    // TP regression: `helper` is deleted *before* `caller` is even
    // defined, with no re-establishment — the body call must still draw
    // W123 (confirmed against tclsh 8.6.14: `invalid command name
    // "helper"`).
    let src = "proc helper {} {}\nrename helper {}\nproc caller {} { helper }\ncaller\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_issue_1009_codex_review_deleted_between_definition_and_call_still_flags() {
    // TP regression: `helper` is deleted *after* `caller` is defined but
    // *before* `caller` is ever invoked — the body call must still draw
    // W123 (confirmed against tclsh 8.6.14: `invalid command name
    // "helper"`, even though `caller`'s own definition predates the
    // deletion).
    let src = "proc helper {} {}\nproc caller {} { helper }\nrename helper {}\ncaller\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_fp_issue_1009_codex_review_method_body_call_before_later_deletion_resolves() {
    // FP guard, TclOO variant: a method body call before a later
    // unconditional deletion of the command it calls must also resolve —
    // confirmed against tclsh 8.6.14 (the script prints "ok" and exits 0).
    let src = "proc helper {} { return hi }\noo::class create Widget {\n    method run {} { helper }\n}\nWidget create w1\nw1 run\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_1009_codex_review_escape_hatch_requires_specific_enclosing_call() {
    // FN guard: an unrelated proc's top-level invocation must not "lend"
    // liveness to a different, never-invoked enclosing definition — only
    // `unrelated` is called at the top level before the deletion, `caller`
    // itself never is, so the `helper` call inside `caller`'s body must
    // still draw W123 (the same base shape the original #973 fix already
    // covers when there is no competing top-level call at all).
    let src = "proc helper {} {}\nproc caller {} { helper }\nproc unrelated {} { return 1 }\nunrelated\nrename helper {}\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_fp_issue_1015_two_level_call_chain_before_later_deletion_resolves() {
    // FP guard (issue #1015): the escape hatch must follow a *chain* of
    // enclosing definitions, not one level. `inner` is never invoked at
    // the top level — only `outer` is — but `outer` calls `inner`, which
    // calls `helper`, all before the rename. tclsh8.6/9.0 both run this
    // clean (exit 0, no error).
    let src = "proc helper {} { return hi }\nproc inner {} { helper }\nproc outer {} { inner }\nouter\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1015_three_level_call_chain_before_later_deletion_resolves() {
    // FP guard, one level deeper still — the reachability query is a
    // fixpoint, so depth is not bounded by the number of hops.
    let src = "proc helper {} { return hi }\nproc l1 {} { helper }\nproc l2 {} { l1 }\nproc l3 {} { l2 }\nl3\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_1015_mutual_recursion_cycle_never_entered_still_flags() {
    // TP guard (issue #1015): `pingCaller`/`pongCaller` call each other and
    // nothing calls either at the top level, so neither is ever reached —
    // the cycle must terminate as "unreachable" rather than looping, and
    // the `helper` call inside must still draw W123. tclsh8.6 loads the
    // file fine precisely *because* the cycle is never entered; calling
    // into it after the rename fails `invalid command name "helper"`.
    let src = "proc helper {} {}\nproc pingCaller {} { helper\n pongCaller }\nproc pongCaller {} { pingCaller }\nrename helper {}\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_a_dead_body_edge_does_not_lower_a_later_top_level_offset() {
    // TP guard (Codex review of PR #1045, adversarial soundness review):
    // `a` runs before the rename, but its only call to `b` sits inside `if
    // {0} { … }` and never executes. `b`'s real first invocation is the
    // top-level one *after* the rename, so `b`'s `helper` call fails.
    //
    // The unrestricted fixpoint let the dead `a` -> `b` edge lower `b`'s
    // offset to `a`'s, which read as "reached before the deletion" and
    // withdrew the warning. A body edge may no longer undercut a callee's
    // own top-level offset.
    //
    // Oracle (tclsh8.6, `review-probes-sound/r1.tcl`): exits 1 with
    // `invalid command name "helper"` from `b`, invoked at line 6.
    let src = "proc helper {} { return hi }\nproc b {} { helper }\nproc a {} { if {0} { b } }\na\nrename helper {}\nb\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_an_empty_enclosing_body_leaves_the_later_top_level_offset() {
    // The paired FP guard for the test above: same shape with `a`'s body
    // empty, so there is no `a` -> `b` edge to drop in the first place.
    // Both must warn, or the fix would be indistinguishable from "the edge
    // never mattered".
    //
    // Oracle (tclsh8.6, `review-probes-sound/r2.tcl`): exits 1, same error.
    let src = "proc helper {} { return hi }\nproc b {} { helper }\nproc a {} { }\na\nrename helper {}\nb\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_fp_a_live_body_edge_still_reaches_a_callee_with_no_top_level_call() {
    // FP guard for the restriction: `b` has no top-level call site of its
    // own, so the `a` -> `b` edge is the only evidence there is and must
    // still resolve. This is issue #1015's shape, and the restriction is
    // written to leave it alone — without it, every #1015 chain would
    // regress to a false positive.
    let src = "proc helper {} { return hi }\nproc b {} { helper }\nproc a {} { b }\na\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1015_mutual_recursion_cycle_entered_at_top_level_resolves() {
    // FP guard: the same cycle, but entered by a real top-level call
    // before the deletion — every member is then reachable, so the nested
    // `helper` call resolves. Confirms the cycle handling short-circuits
    // without also poisoning the reachable case.
    let src = "proc helper {} { return hi }\nproc pingCaller {} { helper }\nproc pongCaller {} { pingCaller }\nproc entry {} { pongCaller }\nentry\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

/// FP — issue #1070. A lambda's own parameters are bound by `apply`, so
/// reading one in the body is never a read-before-set.  The bare-statement
/// spelling used to draw `W210` on every parameter because the enclosing
/// frame's SSA read-scan walked the whole lambda literal as if the body ran
/// in the caller's frame.
///
/// Parameter binding is `proc` semantics — defaults and `args` included.
/// tclsh 9.0.4 / 8.6.14, identical:
///
/// ```text
/// apply {{a b} {return $a-$b}} 1 2       → 1-2
/// apply {{a {b 99}} {return $a-$b}} 1    → 1-99
/// apply {{a args} {return $a-$args}} 1 2 3 → 1-2 3
/// apply {{a args} {return $a-[llength $args]}} 1 → 1-0
/// ```
#[test]
fn apply_lambda_parameters_are_bound_not_read_before_set() {
    for src in [
        "apply {{a} {puts $a}} 5\n",
        "apply {{a b} {puts $a$b}} 5 6\n",
        "apply {{a args} {puts $a$args}} 5 6\n",
        "apply {{a args} {puts $a[llength $args]}} 5\n",
        "apply {{a {b 1}} {puts $a$b}} 5\n",
        // The lambda's namespace element must not shift the parameter binding.
        "apply {{a} {puts $a} ::ns} 5\n",
        // Nested inside a proc body, and inside a control-flow body.
        "proc p {} { apply {{a} {puts $a}} 5 }\n",
        "if {1} { apply {{a} {puts $a}} 5 }\n",
        // The command-substitution spelling was already clean; keep it so.
        "set r [apply {{a} {puts $a}} 5]\nputs $r\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            !codes.contains(&"W210".to_string()),
            "a bound lambda parameter must not fire W210 for {src:?}: {codes:?}"
        );
    }
}

/// TN — the same change must not blind the lambda body.  A name the body
/// reads without binding is a genuine error (tclsh 9.0.4 / 8.6.14:
/// `apply {{a} {return $zznever}} 5` → `can't read "zznever": no such
/// variable`), and the caller's locals are *not* visible inside the lambda
/// (`set x 7; apply {{} {puts $x}}` → `can't read "x": no such variable`), so
/// its body units carry the read-before-set family in their own right.
#[test]
fn apply_lambda_body_unbound_read_still_fires_w210() {
    for src in [
        "apply {{a} {puts $zznever}} 5\n",
        "apply {{a} {set b $zznever; puts $b}} 5\n",
        "proc p {} { apply {{a} {puts $zznever}} 5 }\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            codes.contains(&"W210".to_string()),
            "an unbound read in a lambda body must still fire W210 for {src:?}: {codes:?}"
        );
    }
}

/// TN — a `namespace eval` body unit is deliberately *not* in the
/// read-before-set family: its variables belong to the namespace and are
/// routinely written by a different body that opens the same namespace, so
/// the closed-frame argument a lambda gives does not hold there.
#[test]
fn namespace_eval_body_is_not_read_before_set_analysed() {
    let src = "namespace eval ::demo { variable counter }\n\
               namespace eval ::demo { variable counter; incr counter }\n";
    let codes = codes_for_dialect(src, "tcl9.0");
    assert!(
        !codes.contains(&"W210".to_string()),
        "a namespace-eval body must not be read-before-set analysed: {codes:?}"
    );
}

/// FN control — a *dynamic* lambda word is a real caller-frame read, so it
/// must keep firing.  `apply $lambda` reads `lambda` in the caller's frame
/// (tclsh 9.0.4 / 8.6.14 both raise `can't read "zzlambda"` for an unset one).
#[test]
fn dynamic_apply_lambda_word_is_still_a_caller_frame_read() {
    let codes = codes_for_dialect("proc p {} { apply $zzlambda 5 }\n", "tcl9.0");
    assert!(
        codes.contains(&"W210".to_string()),
        "`apply $lambda` must still read `lambda` in the caller's frame: {codes:?}"
    );
}

// ---------------------------------------------------------------------------
// Issue #1329 — bareword `my <method>` was never recorded as a dispatch site,
// so W308 ("unknown method") could not fire for `TclOO`'s commonest
// same-object spelling. Issue #1330 — the spans every `CmdCommandSite`- and
// `VarCommandSite`-anchored diagnostic reports.
//
// The #1330 assertions are deliberately *exact ranges*: those diagnostics
// already fired before the fix and only their span was wrong, so a test that
// checked the code alone would have passed against the bug.
// ---------------------------------------------------------------------------

/// Analyse `src` on both the whole-file and the per-item (incremental) path
/// and return the `(code, text-under-span)` pairs for the dispatch codes,
/// asserting the two paths agree.
///
/// The agreement check is the point: issue #1330's user-visible symptom only
/// appeared on the per-item path, which is the one the LSP serves from, so a
/// whole-file-only assertion would have missed it entirely.
fn dispatch_diags_both_paths(src: &str) -> Vec<(String, String, Span)> {
    let collect = |r: &crate::analyser::types::AnalysisResult| -> Vec<(String, String, Span)> {
        r.diagnostics
            .iter()
            .filter(|d| matches!(d.code, DiagCode::W307 | DiagCode::W308 | DiagCode::E001))
            .map(|d| {
                let text = src
                    .get(d.span.start() as usize..d.span.end() as usize)
                    .unwrap_or("<out of bounds>")
                    .to_string();
                (d.code.as_str().to_string(), text, d.span)
            })
            .collect()
    };
    let whole = collect(&crate::analyser::Analyser::new().analyse(src, "tcl"));
    let per_item = collect(&crate::analyser::Analyser::new().analyse_per_item(src, "tcl"));
    assert_eq!(
        whole, per_item,
        "whole-file and per-item paths must report identical dispatch \
         diagnostics and spans (issue #1330) for:\n{src}"
    );
    whole
}

/// Just the codes, for the presence/absence assertions.
fn dispatch_codes(src: &str) -> Vec<String> {
    dispatch_diags_both_paths(src)
        .into_iter()
        .map(|(code, _, _)| code)
        .collect()
}

/// A class whose only declared members are `animTick` and the method body
/// under test — the ticket's own shape.
fn cls_1329(body: &str) -> String {
    format!(
        "oo::class create Test1329 {{\n    method animTick {{}} {{ return {{}} }}\n    \
         method anim {{}} {{\n{body}    }}\n}}\n"
    )
}

#[test]
fn analyse_w308_tp_1329_bareword_my_unknown_method() {
    // TP — the ticket's repro: `my nosuchmethod` must fire W308, anchored on
    // the method word alone.
    let src = cls_1329("        my nosuchmethod\n");
    let diags = dispatch_diags_both_paths(&src);
    assert_eq!(
        diags.len(),
        1,
        "exactly one dispatch diagnostic expected; got {diags:?}"
    );
    assert_eq!(diags[0].0, "W308");
    assert_eq!(
        diags[0].1, "nosuchmethod",
        "W308 must underline the method word alone; got {diags:?}"
    );
}

#[test]
fn analyse_w308_tn_1329_bareword_my_real_method_silent() {
    // TN — `my animTick` dispatches a method the enclosing class declares.
    let src = cls_1329("        my animTick\n");
    assert!(
        dispatch_codes(&src).is_empty(),
        "`my animTick` must be silent; got {:?}",
        dispatch_diags_both_paths(&src),
    );
}

#[test]
fn analyse_w308_fp_1329_my_reaches_unexported_object_builtins() {
    // FP guard — `my` bypasses export filtering, so it reaches `oo::object`'s
    // unexported members. tclsh 9.0.4 and 8.6.16 both list them as
    // `<cloned> destroy eval unknown variable varname`, and `my variable v` is
    // the single most common line in idiomatic TclOO. None may draw W308.
    let src = cls_1329(
        "        my variable v\n        set v [my varname v]\n        my eval {set q 1}\n",
    );
    assert!(
        dispatch_codes(&src).is_empty(),
        "`my` must reach oo::object's unexported members; got {:?}",
        dispatch_diags_both_paths(&src),
    );
}

#[test]
fn analyse_w308_tp_1329_object_command_cannot_reach_unexported_builtins() {
    // The other side of that rule, so the fix cannot be "allow `variable`
    // everywhere": the object's own command really does *not* expose it
    // (tclsh 9.0.4: `$obj varname v` -> `unknown method "varname"`), so this
    // stays a true positive.
    let src =
        "oo::class create C {\n    method m {} { return 1 }\n}\nC create obj\nobj varname v\n";
    assert_eq!(
        dispatch_codes(src),
        vec!["W308".to_string()],
        "`$obj varname` is a real error — only `my` reaches it",
    );
}

#[test]
fn analyse_w308_fp_1329_my_outside_any_class_body() {
    // FP guard — no enclosing class means no receiver to check against.
    // A `my` at top level, or in a plain proc, must abstain rather than
    // invent a class.
    for src in [
        "my nosuchmethod\n",
        "proc p {} {\n    my nosuchmethod\n}\n",
        "namespace eval ns {\n    my nosuchmethod\n}\n",
    ] {
        assert!(
            dispatch_codes(src).is_empty(),
            "`my` outside a class body must abstain; got {:?} for {src:?}",
            dispatch_diags_both_paths(src),
        );
    }
}

#[test]
fn analyse_w308_fp_1329_inherited_and_forwarded_and_unknown() {
    // FP guards for the three ways a method can exist without being declared
    // on the dispatching class itself.
    let inherited = "oo::class create B1329 {\n    method inherited {} {}\n}\n\
         oo::class create D1329 {\n    superclass B1329\n    method go {} {\n        \
         my inherited\n    }\n}\n";
    let forwarded = "oo::class create F1329 {\n    forward fwd my other\n    \
         method other {} {}\n    method go {} {\n        my fwd\n    }\n}\n";
    let unknown_handler = "oo::class create U1329 {\n    method unknown {args} {}\n    \
         method go {} {\n        my anything\n    }\n}\n";
    for src in [inherited, forwarded, unknown_handler] {
        assert!(
            dispatch_codes(src).is_empty(),
            "a resolvable-by-other-means method must not draw W308; got {:?} for {src:?}",
            dispatch_diags_both_paths(src),
        );
    }
}

#[test]
fn analyse_w308_fp_1329_unprovable_method_set_abstains() {
    // FP guard — a mixin or superclass outside the local index makes the
    // method set a lower bound, never a complete one. W308 must abstain.
    let mixin = "oo::class create M1329 {\n    mixin ::ext::Unindexed\n    \
         method go {} {\n        my whatever\n    }\n}\n";
    let superclass = "oo::class create S1329 {\n    superclass ::ext::Unindexed\n    \
         method go {} {\n        my whatever\n    }\n}\n";
    for src in [mixin, superclass] {
        assert!(
            dispatch_codes(src).is_empty(),
            "an unprovable method set must abstain; got {:?} for {src:?}",
            dispatch_diags_both_paths(src),
        );
    }
}

#[test]
fn analyse_w308_fp_1329_computed_method_word_abstains() {
    // FP guard — the dispatched name is chosen at run time, so no static
    // method set can contradict it.
    let src = cls_1329("        set m animTick\n        my $m\n        my get$m\n");
    assert!(
        dispatch_codes(&src).is_empty(),
        "a computed method word must abstain; got {:?}",
        dispatch_diags_both_paths(&src),
    );
}

#[test]
fn analyse_w308_fp_1329_disturbed_self_dispatch_keyword_abstains() {
    // FP guard — if the document shadows, renames or deletes `my`, a bare
    // `my` may not be TclOO's dispatcher at all. Order-independent: the
    // `rename` written *after* the class body still governs, because the
    // method body only runs later.
    let shadowed = format!(
        "proc my {{args}} {{}}\n{}",
        cls_1329("        my nosuchmethod\n")
    );
    let renamed = format!("{}rename my mine\n", cls_1329("        my nosuchmethod\n"));
    let aliased = format!(
        "{}interp alias {{}} my {{}} puts\n",
        cls_1329("        my nosuchmethod\n")
    );
    for src in [shadowed, renamed, aliased] {
        assert!(
            dispatch_codes(&src).is_empty(),
            "a disturbed `my` must abstain; got {:?} for {src:?}",
            dispatch_diags_both_paths(&src),
        );
    }
}

#[test]
fn analyse_w308_1329_next_and_self_name_no_method() {
    // `next` / `nextto` re-invoke the *currently executing* method and `self`
    // introspects — the registry classifies neither as SelfDispatch, so no
    // word of theirs is validated as a method name.
    let src = "oo::class create N1329 {\n    method go {} {\n        next\n        \
         self class\n        self namespace\n    }\n}\n";
    assert!(
        dispatch_codes(src).is_empty(),
        "`next` / `self` name no method; got {:?}",
        dispatch_diags_both_paths(src),
    );
}

#[test]
fn analyse_w308_1329_dialect_gate_no_tcloo_before_86() {
    // `my` is TCL86_PLUS registry data, so an 8.4 / 8.5 document has no
    // self-dispatch keyword at all and the site is never recorded.
    let src = cls_1329("        my nosuchmethod\n");
    for dialect in ["tcl8.4", "tcl8.5"] {
        let r = crate::analyser::Analyser::new().analyse(&src, dialect);
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
            "no TclOO under {dialect}, so no W308; got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn analyse_w308_1330_cmd_site_spans_are_exact() {
    // Issue #1330 — every `CmdCommandSite`-anchored diagnostic, with its
    // exact span. All three were wrong before the fix: the two head-anchored
    // ones (E001, W307) stopped one byte short of the closing `]`, and the
    // method-anchored one (W308) was reported at the body fragment's own
    // offsets on the per-item path.
    let dog = "oo::class create Dog {\n    method bark {} { return {} }\n}\n";

    // W308, method-word anchored, inside a proc body (the per-item shape).
    let in_proc = format!("{dog}proc handler {{}} {{\n    [Dog new] badmethod\n}}\n");
    assert_eq!(
        dispatch_diags_both_paths(&in_proc)
            .into_iter()
            .map(|(c, t, _)| (c, t))
            .collect::<Vec<_>>(),
        vec![("W308".to_string(), "badmethod".to_string())],
    );

    // E001, head anchored: the whole `[Dog new]`, closing bracket included.
    let bare = format!("{dog}[Dog new]\n");
    assert_eq!(
        dispatch_diags_both_paths(&bare)
            .into_iter()
            .map(|(c, t, _)| (c, t))
            .collect::<Vec<_>>(),
        vec![("E001".to_string(), "[Dog new]".to_string())],
    );

    // W307, head anchored, same rule.
    assert_eq!(
        dispatch_diags_both_paths("[getcmd] doit\n")
            .into_iter()
            .map(|(c, t, _)| (c, t))
            .collect::<Vec<_>>(),
        vec![("W307".to_string(), "[getcmd]".to_string())],
    );
}

#[test]
fn analyse_w308_1330_quick_fix_span_matches_the_diagnostic() {
    // The "did you mean" fix rewrites `method_span`. Un-rebased, it would
    // splice the replacement into unrelated bytes elsewhere in the file —
    // a silent document corruption, not just a misplaced squiggle. Pin the
    // fix span to the diagnostic's own on the per-item path.
    let src = "oo::class create Dog {\n    method bark {} { return {} }\n}\n\
         proc handler {} {\n    [Dog new] bakr\n}\n";
    let r = crate::analyser::Analyser::new().analyse_per_item(src, "tcl");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W308)
        .unwrap_or_else(|| panic!("W308 expected; got {:?}", r.diagnostics));
    let fix = d
        .fixes
        .first()
        .unwrap_or_else(|| panic!("a `did you mean` fix expected; got {d:?}"));
    assert_eq!(fix.span, d.span, "fix must rewrite the word it points at");
    assert_eq!(
        &src[fix.span.start() as usize..fix.span.end() as usize],
        "bakr"
    );
    assert_eq!(fix.new_text, "bark");
}

#[test]
fn w144_core_subcommand_lifecycle_uses_registry_safe_fix() {
    let src = "interp slaves\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let result = crate::analyser::Analyser::new().analyse(src, dialect);
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W144)
            .unwrap_or_else(|| panic!("{dialect}: expected W144, got {:?}", result.diagnostics));
        assert_eq!(&src[diagnostic.span.as_range()], "slaves");
        assert!(
            diagnostic.message.contains("Tcl 8.6") || diagnostic.message.contains("Tcl 9.0"),
            "{dialect}: W144 must use the active Tcl-core axis: {diagnostic:?}"
        );
        assert_eq!(diagnostic.fixes.len(), 1, "{dialect}: {diagnostic:?}");
        let fix = &diagnostic.fixes[0];
        assert_eq!(&src[fix.span.as_range()], "slaves");
        assert_eq!(fix.new_text, "children");
        assert_eq!(
            fix.safety,
            crate::irules_checks::FixSafety::SemanticsEquivalent
        );
    }

    let legacy = crate::analyser::Analyser::new().analyse(src, "tcl8.5");
    assert!(
        !legacy
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W144),
        "slaves predates the Tcl 8.6 preferred spelling: {:?}",
        legacy.diagnostics
    );
}
