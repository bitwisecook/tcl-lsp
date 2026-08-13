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

//! Conf-wrapped iRules tests.
//!
//! Covers the structural (range / boolean / extracted-string) assertions for
//! two areas:
//!
//! * conf-wrapped iRules analysis (`ltm rule` / `gtm rule` mode): conf-wrap
//!   detection, embedded-rule body extraction, rule ranges, multiple rules,
//!   nested braces, gtm vs ltm.
//! * BIG-IP embedded iRule extraction ranges: offset → brace-range mapping
//!   for embedded iRule navigation.
//!
//! The functions under test live in [`tcl_bigip::rule_extract`]
//! (`is_conf_wrapped_irules`, `find_embedded_rules`, `find_rule_at_offset`,
//! `replace_rule_body`, `EmbeddedRule`).
//!
//! GAPs (behaviours this tree does not model — see the `// GAP:` comments
//! inline): the LSP *analysis* layer (`analyse_conf_wrapped`, the
//! `IRULE5006/5007/1001` depth/context diagnostics, `shift_position` /
//! `_shift_range`, the merged scope tree, cross-rule proc sharing,
//! `get_document_symbols` rule containers, and
//! `find_enclosing_when_event(embedded_rules=…)`) is not modelled here:
//! `tcl-lsp-core/src/irules_context.rs` documents that the conf-wrapped
//! `embedded_rules` mode is deliberately not modelled (the raw iRule body
//! is analysed directly). Those assertions are skipped here; the
//! rule-extraction facts they depend on (rule count, names, body offsets)
//! are covered via `find_embedded_rules`.

use tcl_bigip::rule_extract::{
    find_embedded_rules, find_rule_at_offset, is_conf_wrapped_irules, replace_rule_body,
};

// -- Detection ---------------------------------------------------------------

/// A bare `when … { }` is not conf-wrapped.
#[test]
fn standalone_irule_not_detected() {
    let src = "when HTTP_REQUEST {\n    log local0. \"hi\"\n}";
    assert!(!is_conf_wrapped_irules(src));
}

#[test]
fn ltm_rule_detected() {
    let src =
        "ltm rule /Common/foo {\n    when HTTP_REQUEST {\n        log local0. \"hi\"\n    }\n}";
    assert!(is_conf_wrapped_irules(src));
}

#[test]
fn gtm_rule_detected() {
    let src =
        "gtm rule /Common/bar {\n    when DNS_REQUEST {\n        log local0. \"hi\"\n    }\n}";
    assert!(is_conf_wrapped_irules(src));
}

#[test]
fn multiple_rules_detected() {
    let src = "ltm rule /Common/a {\n    when RULE_INIT { }\n}\n\
               ltm rule /Common/b {\n    when HTTP_REQUEST { }\n}\n";
    assert!(is_conf_wrapped_irules(src));
}

// GAP: dialect detection exercises `detect_dialect_from_source`, which maps a
// conf-wrapped source to the "f5-irules" dialect and lets an explicit
// `# tcl-dialect:` directive override it. That dialect-selection layer is not
// part of `rule_extract`; only the underlying `is_conf_wrapped_irules` predicate
// is covered (asserted above).

// -- Embedded-rule extraction range ------------------------------------------

/// The stanza range starts at the `ltm rule` header (line 0, col 0) and ends
/// at the closing `}`.
#[test]
fn find_embedded_rules_range_maps_to_braces() {
    let source = "ltm rule /Common/my_irule {\n\
                  \x20   when HTTP_REQUEST {\n\
                  \x20       pool /Common/web_pool\n\
                  \x20   }\n\
                  }\n";
    let rules = find_embedded_rules(source);
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];

    assert_eq!(rule.range.start.line, 0);
    assert_eq!(rule.range.start.character, 0);
    assert_eq!(
        &source[rule.range.start.offset as usize..rule.range.start.offset as usize + 8],
        "ltm rule"
    );

    // The range end offset points AT the closing brace.
    assert_eq!(
        source.as_bytes()[rule.range.end.offset as usize] as char,
        '}'
    );
    assert_eq!(rule.range.end.line, 4);
    assert_eq!(rule.range.end.character, 0);
}

/// An offset inside the block resolves to the enclosing rule.
#[test]
fn find_rule_at_offset_inside_block() {
    let source = "ltm rule /Common/a {\n    when HTTP_REQUEST { log local0. \"a\" }\n}\n";
    let offset = source.find("HTTP_REQUEST").expect("event present");
    let rule = find_rule_at_offset(source, offset);
    assert!(rule.is_some());
    assert_eq!(rule.unwrap().full_path, "/Common/a");
}

// -- Analysis: rule count + names --------------------------------------------
// `analyse_conf_wrapped` returns `(result, rules)`; the *rules* half is exactly
// `find_embedded_rules`. The diagnostics half (IRULE5006/5007 etc.) is a GAP.

/// One rule named `test`.
#[test]
fn single_rule_extracted() {
    let src = "ltm rule /Common/test {\n\
               \x20   when HTTP_REQUEST {\n\
               \x20       log local0. \"hello\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "test");
    // GAP: the IRULE5006/IRULE5007 depth-diagnostic assertions require
    // `analyse_conf_wrapped`; not modelled in Rust (no conf-wrapped analysis).
}

/// Two rules, ordered.
#[test]
fn multiple_rules_all_extracted() {
    let src = "ltm rule /Common/rule_a {\n\
               \x20   when RULE_INIT {\n\
               \x20       set static::app_debug 1\n\
               \x20   }\n\
               }\n\
               ltm rule /Common/rule_b {\n\
               \x20   when HTTP_REQUEST {\n\
               \x20       log local0. \"request\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].name, "rule_a");
    assert_eq!(rules[1].name, "rule_b");
}

/// A `gtm rule` is extracted by name.
#[test]
fn gtm_rule_extracted() {
    let src = "gtm rule /Common/dns_handler {\n\
               \x20   when DNS_REQUEST {\n\
               \x20       log local0. \"dns\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "dns_handler");
}

/// Multiple `when` blocks still extract as one rule, and both events live in
/// the body.
#[test]
fn multiple_when_blocks_in_one_rule() {
    let src = "ltm rule /Common/multi {\n\
               \x20   when RULE_INIT {\n\
               \x20       set static::app_debug 1\n\
               \x20   }\n\
               \x20   when HTTP_REQUEST {\n\
               \x20       log local0. \"request\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    assert!(rules[0].body.contains("RULE_INIT"));
    assert!(rules[0].body.contains("HTTP_REQUEST"));
}

// -- Range / offset facts ----------------------------------------------------
// GAP: shift_position / _shift_range (line/char/offset rebasing of a sub-range
// into file coordinates) and the IRULE* diagnostics are part of the analysis
// layer and not modelled here. The *structural* offset facts they guard — that
// a rule's body offsets are well-formed and absolute, and that the second
// rule's body sits after the first — are covered here via EmbeddedRule.

/// Every rule's body offsets are within `[0, len(src)]` and well-ordered.
#[test]
fn body_offsets_within_source_bounds() {
    let src = "ltm rule /Common/test {\n\
               \x20   when HTTP_REQUEST {\n\
               \x20       HTTP::respond 200 content \"ok\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    for r in &rules {
        assert!(r.body_start_offset <= src.len());
        assert!(r.body_end_offset <= src.len());
        assert!(r.body_start_offset <= r.body_end_offset);
        assert!(r.block_start_offset <= r.body_start_offset);
        assert!(r.body_end_offset < r.block_end_offset);
        assert!(r.range.start.offset as usize <= src.len());
        assert!(r.range.end.offset as usize <= src.len());
    }
}

/// The second rule's body starts on a later line and its body offsets follow
/// the first rule's.
#[test]
fn second_rule_body_offsets_follow_first() {
    let src = "ltm rule /Common/first {\n\
               \x20   when RULE_INIT { }\n\
               }\n\
               ltm rule /Common/second {\n\
               \x20   when CLIENT_ACCEPTED {\n\
               \x20       HTTP::respond 200 content \"ok\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 2);
    // The second rule body starts strictly after the first rule body ends.
    assert!(rules[1].body_start_offset > rules[0].body_end_offset);
    // `HTTP::respond` (line 5 in the file) is inside the second rule's body.
    let resp = src.find("HTTP::respond").expect("present");
    assert!(resp >= rules[1].body_start_offset);
    assert!(resp < rules[1].body_end_offset);
    // GAP: that an IRULE1001 diagnostic *fires* on line 5 with that offset
    // requires the dialect-scoped analyser; not modelled here.
}

/// A `proc` declared in a rule body lies within that rule's body offsets, on a
/// later line.
#[test]
fn proc_in_rule_within_body() {
    let src = "ltm rule /Common/with_proc {\n\
               \x20   proc helper { x } { return $x }\n\
               \x20   when HTTP_REQUEST { }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    let proc_off = src.find("proc helper").expect("present");
    assert!(proc_off >= rules[0].body_start_offset);
    assert!(proc_off < rules[0].body_end_offset);
    // GAP: the analyser's `all_procs` map and its file-coordinate name_range
    // (proc.name_range.start.line == 1) come from `analyse_conf_wrapped`; not
    // modelled here.
}

// -- find_enclosing_when_event with embedded_rules ---------------------------
// GAP: `find_enclosing_when_event(src, line, embedded_rules=rules)` would scope
// the cursor's enclosing `when` event to the rule body it falls in.
// `tcl_lsp_core::irules_context::find_enclosing_when_event(src, line, dialect)`
// deliberately does NOT take `embedded_rules` (documented: the conf-wrapped
// mode is not modelled). The structural precondition those cases rely on —
// which rule body a given line falls inside — is covered here via the
// EmbeddedRule line ranges.

/// A cursor line maps to the correct rule body by line range.
#[test]
fn cursor_line_maps_to_rule_body() {
    let src = "ltm rule /Common/a {\n\
               \x20   when HTTP_REQUEST {\n\
               \x20       log local0. \"hi\"\n\
               \x20   }\n\
               }\n\
               ltm rule /Common/b {\n\
               \x20   when DNS_REQUEST {\n\
               \x20       log local0. \"dns\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 2);

    let rule_for_line = |line: u32| -> Option<&str> {
        rules
            .iter()
            .find(|r| line >= r.range.start.line && line <= r.range.end.line)
            .map(|r| r.name.as_str())
    };
    // Line 2 (`log local0. "hi"`) is in rule `a`; line 7 is in rule `b`.
    assert_eq!(rule_for_line(2), Some("a"));
    assert_eq!(rule_for_line(7), Some("b"));
    // GAP: that find_enclosing_when_event returns "HTTP_REQUEST" / "DNS_REQUEST"
    // for those lines under the embedded_rules scoping is not modelled here.
}

/// A line past the closing brace of the only rule falls in no rule body.
#[test]
fn cursor_outside_all_rules() {
    let src = "ltm rule /Common/a {\n    when HTTP_REQUEST { }\n}\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    // Line 2 is the closing brace line; the rule range ends there, line 3+ is
    // outside. The block end offset is the byte after `}`.
    let line2_after_brace = src.len(); // trailing newline region
    assert!(find_rule_at_offset(src, line2_after_brace).is_none());
}

// -- Brace scanning robustness -----------------------------------------------

/// Braces inside a double-quoted string do not terminate the rule body.
#[test]
fn braces_in_quoted_string() {
    let src = "ltm rule /Common/quoted {\n\
               \x20   when HTTP_REQUEST {\n\
               \x20       set x \"value with { braces }\"\n\
               \x20       log local0. $x\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "quoted");
    // The body contains the full when block including the quoted string.
    assert!(rules[0].body.contains("braces"));
    // And the body ends at the real closing brace: it still contains the
    // trailing `log` line after the quoted-brace string.
    assert!(rules[0].body.contains("log local0. $x"));
}

/// Escaped quotes inside strings are handled.
#[test]
fn escaped_quote_in_string() {
    let src = "ltm rule /Common/escaped {\n\
               \x20   when HTTP_REQUEST {\n\
               \x20       set x \"value \\\"with\\\" braces { }\"\n\
               \x20   }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "escaped");
}

// -- Empty / edge cases ------------------------------------------------------

#[test]
fn empty_rule_body() {
    let src = "ltm rule /Common/empty {\n}\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "empty");
}

#[test]
fn no_rules_returns_empty() {
    let src = "# just a comment\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 0);
    assert!(!is_conf_wrapped_irules(src));
    // GAP: also asserting `len(result.diagnostics) == 0` would need the
    // analysis layer (and thus any diagnostics), which is not modelled here.
}

/// ltm and gtm rules extract together.
#[test]
fn mixed_ltm_gtm_rules() {
    let src = "ltm rule /Common/http_handler {\n\
               \x20   when HTTP_REQUEST { }\n\
               }\n\
               gtm rule /Common/dns_handler {\n\
               \x20   when DNS_REQUEST { }\n\
               }\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 2);
    let names: std::collections::HashSet<&str> = rules.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains("http_handler"));
    assert!(names.contains("dns_handler"));
}

// -- Additional direct ports of EmbeddedRule fields & helpers ----------------

/// `EmbeddedRule` header / `full_path` / body fields are populated as documented
/// (the `header` excludes the opening brace; the `body` excludes both braces).
#[test]
fn embedded_rule_fields_populated() {
    let source = "ltm rule /Common/my_irule {\n    when HTTP_REQUEST {\n        pool /Common/web_pool\n    }\n}\n";
    let rules = find_embedded_rules(source);
    assert_eq!(rules.len(), 1);
    let r = &rules[0];
    assert_eq!(r.full_path, "/Common/my_irule");
    assert_eq!(r.name, "my_irule");
    assert_eq!(r.header, "ltm rule /Common/my_irule");
    // body_start is the char right after `{`; body_end is the char right
    // before the closing `}`.
    assert_eq!(source.as_bytes()[r.block_start_offset] as char, 'l'); // "ltm"
    assert_eq!(source.as_bytes()[r.body_start_offset - 1] as char, '{');
    assert_eq!(source.as_bytes()[r.body_end_offset] as char, '}');
    assert_eq!(source.as_bytes()[r.block_end_offset - 1] as char, '}');
    assert!(r.body.starts_with("\n    when HTTP_REQUEST"));
}

/// `replace_rule_body` swaps the body bytes between the braces, leaving the
/// header and closing brace intact.
#[test]
fn replace_rule_body_swaps_body() {
    let source = "ltm rule /Common/a {\n    when HTTP_REQUEST { }\n}\n";
    let rules = find_embedded_rules(source);
    assert_eq!(rules.len(), 1);
    let out = replace_rule_body(source, &rules[0], "\n    when DNS_REQUEST { }\n");
    assert_eq!(out, "ltm rule /Common/a {\n    when DNS_REQUEST { }\n}\n");
    // Re-extracting the rewritten source yields the new body.
    let rules2 = find_embedded_rules(&out);
    assert_eq!(rules2.len(), 1);
    assert!(rules2[0].body.contains("DNS_REQUEST"));
}

/// `find_rule_at_offset` returns None when the offset is in no rule, and selects
/// the correct rule among several (block span is inclusive of both ends).
#[test]
fn find_rule_at_offset_selects_and_misses() {
    let src = "ltm rule /Common/a {\n    when RULE_INIT { }\n}\nltm rule /Common/b {\n    when HTTP_REQUEST { }\n}\n";
    let rules = find_embedded_rules(src);
    assert_eq!(rules.len(), 2);

    // Offset inside rule a.
    let in_a = src.find("RULE_INIT").unwrap();
    assert_eq!(
        find_rule_at_offset(src, in_a).unwrap().full_path,
        "/Common/a"
    );
    // Offset inside rule b.
    let in_b = src.find("HTTP_REQUEST").unwrap();
    assert_eq!(
        find_rule_at_offset(src, in_b).unwrap().full_path,
        "/Common/b"
    );

    // The header start of each rule is an inclusive lower bound.
    assert_eq!(
        find_rule_at_offset(src, rules[0].block_start_offset)
            .unwrap()
            .full_path,
        "/Common/a"
    );
}
