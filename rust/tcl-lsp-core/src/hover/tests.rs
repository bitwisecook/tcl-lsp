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

mod builtin_commands;
mod class_members;
mod classes;
mod embedded_operands;
mod format_specifiers;
mod pattern_syntax;
mod procedures;
mod variables;

fn assert_foreign_factory_method_source_parent(
    source: &str,
    analysis: &AnalysisResult,
    headers: &[(
        u64,
        tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders,
    )],
) {
    use tcl_compiler::registry_invocation::source_structure::original_foreign_factory_bodies;
    let bodies = original_foreign_factory_bodies(source, analysis, 2, headers);
    let [body] = bodies.as_slice() else {
        panic!("unique foreign body");
    };
    let class = body.class_syntax().unwrap();
    let members = class.callable_syntax().unwrap();
    let [member] = members.as_slice() else {
        panic!("one written method");
    };
    assert_eq!(member.parent().body().caller_document(), 2);
    assert_eq!(member.parent().body().declaring_document(), 1);
    assert_eq!(member.name(), "go");
}

#[test]
fn foreign_factory_method_hover_keeps_the_source_parent() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders;
    let library =
        Analyser::new().analyse("oo::class create Meta {superclass ::oo::class}", "tcl9.0");
    let header = OriginalProjectFactoryHeaders::capture(&library).unwrap();
    let source = "Meta create Made { method go {} {return 1} }";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let caller_header = OriginalProjectFactoryHeaders::capture(&analysis).unwrap();
    let headers = [(1, header.clone()), (2, caller_header.clone())];
    assert_foreign_factory_method_source_parent(source, &analysis, &headers);
    let position = u32::try_from(source.find("go").unwrap()).unwrap();
    assert!(super::may_have_foreign_factory_hover(
        source, 0, position, &analysis
    ));
    assert!(!super::may_have_foreign_factory_hover(
        source, 0, 0, &analysis
    ));
    let hover =
        super::conditional_foreign_factory_hover(source, 0, position, &analysis, 2, &headers)
            .unwrap();
    assert!(hover.value.contains("Conditional source method of `Made`"));
    assert_eq!(
        super::conditional_foreign_factory_hover(
            source,
            0,
            position,
            &analysis,
            2,
            &[
                (1, header.clone()),
                (3, header.clone()),
                (2, caller_header.clone())
            ]
        ),
        None
    );
    let blocker = Analyser::new().analyse("proc Meta args {}", "tcl9.0");
    let blocker = OriginalProjectFactoryHeaders::capture(&blocker).unwrap();
    assert_eq!(
        super::conditional_foreign_factory_hover(
            source,
            0,
            position,
            &analysis,
            2,
            &[
                (1, header.clone()),
                (3, blocker),
                (2, caller_header.clone())
            ]
        ),
        None
    );
    for blocked in [
        "interp alias {} Meta {} other; Meta create Made {method go {} {}}",
        "proc Meta args {}; Meta create Made {method go {} {}}",
        // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
        "set ::auto_index(unknown_effect) {source lib.tcl}; unknown_effect; Meta create Made {method go {} {}}",
        "Meta create [set name] {method go {} {}}",
        "Meta create {*}$args",
    ] {
        let blocked_analysis = Analyser::new().analyse(blocked, "tcl9.0");
        let blocked_header = OriginalProjectFactoryHeaders::capture(&blocked_analysis)
            .unwrap_or_else(|| caller_header.clone());
        let position = u32::try_from(blocked.find("go").unwrap_or(0)).unwrap();
        assert_eq!(
            super::conditional_foreign_factory_hover(
                blocked,
                0,
                position,
                &blocked_analysis,
                2,
                &[(1, header.clone()), (2, blocked_header)]
            ),
            None,
            "{blocked}"
        );
    }
    assert_eq!(
        super::conditional_foreign_factory_hover(
            &format!("{source} "),
            0,
            position,
            &analysis,
            2,
            &headers
        ),
        None
    );
}

#[test]
fn foreign_factory_hover_uses_the_manufacturer_selected_body_ordinal() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders;
    let producer = concat!(
        "oo::class create Meta {superclass oo::class; ",
        "self method create {name extra body} {next $name $body}}",
    );
    let library = Analyser::new().analyse(producer, "tcl9.0");
    let source = "Meta create Made ignored {method go {} {return 1}}";
    let caller = Analyser::new().analyse(source, "tcl9.0");
    let headers = [
        (1, OriginalProjectFactoryHeaders::capture(&library).unwrap()),
        (2, OriginalProjectFactoryHeaders::capture(&caller).unwrap()),
    ];
    let position = u32::try_from(source.find("go").unwrap()).unwrap();
    let hover = super::conditional_foreign_factory_hover(source, 0, position, &caller, 2, &headers)
        .unwrap();
    assert!(hover.value.contains("Conditional source method of `Made`"));
}

use tcl_dialect::model::{Family, SurfaceLayer};

use super::*;
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn analyse_for(source: &str, environment: &str) -> AnalysisResult {
    let mut analyser = Analyser::new();
    let input = analyser
        .prepare_analysis_input(environment)
        .expect("test environment has a complete analysis input");
    analyser = analyser.with_resolved_input(input);
    analyser.analyse(source, environment).clone()
}

#[test]
fn standalone_reference_cursor_uses_explicit_selected_grammar_and_utf16_geometry() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    let source = "puts café🙂 ${a{b}c} ${cash$name} ${scalar(open)tail}";
    let prefix = "puts café🙂 ";
    let character = u32::try_from(prefix.encode_utf16().count() + 2).unwrap();
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        let config = tcl_lexer::LexerConfig {
            braced_var: style,
            ..Default::default()
        };
        assert_eq!(
            find_var_at_position_with_config(source, 0, character, config).as_deref(),
            Some(if style.nests() { "a{b}c" } else { "a{b" }),
        );
        let dollar = u32::try_from(
            source[..source.find("$name").unwrap()]
                .encode_utf16()
                .count(),
        )
        .unwrap();
        assert_eq!(
            find_var_at_position_with_config(source, 0, dollar, config).as_deref(),
            Some("cash$name")
        );
        let tail = u32::try_from(
            source[..source.find("scalar(open)").unwrap()]
                .encode_utf16()
                .count(),
        )
        .unwrap();
        assert_eq!(
            find_var_at_position_with_config(source, 0, tail, config).as_deref(),
            Some("scalar(open)tail")
        );
    }
}

#[test]
fn qualified_colon_variable_hover_keeps_the_empty_cell_separate() {
    let source = "set {} EMPTY; set : COLON; puts ${}; puts $:";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, dialect);
        let hover = qualified_variable_hover(&analysis, ":::").unwrap();
        assert!(hover.value.contains("`:::`"), "{dialect}: {}", hover.value);
        assert!(!hover.value.contains("`::`"), "{dialect}: {}", hover.value);
    }
}

fn position_of(source: &str, needle: &str) -> (u32, u32) {
    let offset = source.find(needle).expect("test cursor text");
    let before = &source[..offset];
    let line = u32::try_from(before.bytes().filter(|b| *b == b'\n').count()).unwrap();
    let column = u32::try_from(
        before
            .rsplit_once('\n')
            .map_or(before, |(_, tail)| tail)
            .len(),
    )
    .unwrap();
    (line, column)
}

#[test]
fn find_word_span_returns_none_at_eol() {
    // Position one past the line's last char yields None.
    let src = "proc foo {} {}\n";
    let line = src.split('\n').next().unwrap();
    let len = u32::try_from(line.chars().count()).expect("len fits u32");
    assert!(find_word_span_at_position(src, 0, len).is_none());
}

#[test]
fn smoke_find_word_span_extracts_word_under_cursor() {
    // Cursor on the 'r' of `proc`.
    let src = "proc greet {} {}\n";
    let (word, start, end) = find_word_span_at_position(src, 0, 1).unwrap();
    assert_eq!(word, "proc");
    assert_eq!(start, 0);
    assert_eq!(end, 4);
}

#[test]
fn find_word_span_stops_at_dollar_sign() {
    // `$var` — `$` is in `_WORD_DELIMS`, so a cursor inside
    // `var` should yield just `var`.
    let src = "set x $var\n";
    let (word, start, end) = find_word_span_at_position(src, 0, 8).unwrap();
    assert_eq!(word, "var");
    assert_eq!(start, 7);
    assert_eq!(end, 10);
}

#[test]
fn smoke_find_var_at_position_recognises_dollar_ref() {
    // Cursor inside `$var`.
    let src = "set x $var\n";
    assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
}

#[test]
fn find_var_at_position_returns_none_for_bare_word() {
    let src = "set x 1\n";
    assert!(find_var_at_position(src, 0, 4).is_none());
}

#[test]
fn find_var_at_position_resolves_second_var_of_concatenation() {
    // `set z $x$y` — a cursor on `y` must resolve `y`, not
    // walk left across the `$` to the first variable `x`.
    let src = "set z $x$y\n";
    // Columns: s(0)e(1)t(2) (3)z(4) (5)$(6)x(7)$(8)y(9)
    assert_eq!(find_var_at_position(src, 0, 9), Some("y".to_owned())); // on `y`
    assert_eq!(find_var_at_position(src, 0, 7), Some("x".to_owned())); // on `x`
    // Three-way concatenation resolves the middle var too.
    let src3 = "set w $a$b$c\n";
    assert_eq!(find_var_at_position(src3, 0, 9), Some("b".to_owned()));
}

#[test]
fn find_var_at_position_stops_at_lone_colon() {
    // `$host:$port` — a lone `:` is not part of the variable name (Tcl
    // substitutes only `$host`); it must not be pulled in as `host:`
    // (issue 183).
    let src = "puts $host:$port\n";
    // Columns: p0 u1 t2 s3 (4) $5 h6 o7 s8 t9 :10 $11 ...
    assert_eq!(find_var_at_position(src, 0, 7), Some("host".to_owned()));
    // The `::` qualifier IS part of the name, and a full colon run stays.
    let src2 = "puts $a::b:$c\n";
    assert_eq!(find_var_at_position(src2, 0, 7), Some("a::b".to_owned()));
    let src3 = "puts $a:::b\n";
    assert_eq!(find_var_at_position(src3, 0, 7), Some("a:::b".to_owned()));
}

#[test]
fn find_var_at_position_recognises_braced_form() {
    // Cursor on the `r` inside `${var}`.  The braced form
    // should still resolve to `"var"` so rename and hover
    // can find the symbol.
    let src = "set x ${var}\n";
    assert_eq!(find_var_at_position(src, 0, 9), Some("var".to_owned()));
    // Cursor immediately after the opening `${` (start of name).
    assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
    // Cursor on the closing `}` itself — the inner name is
    // still resolvable as long as the cursor sits inside the
    // braces inclusive of the close brace.
    assert_eq!(find_var_at_position(src, 0, 11), Some("var".to_owned()));
}

#[test]
fn hover_on_unknown_word_returns_none() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    // Cursor on "hello" — not a proc / class / var, so None.
    // (`puts` is a builtin and isn't in `all_procs` either.)
    assert!(hover(src, 0, 6, &analysis, None).is_none());
}

#[test]
fn hover_returns_none_for_out_of_range_line() {
    let src = "proc foo {} {}\n";
    let analysis = analyse(src);
    assert!(hover(src, 99, 0, &analysis, None).is_none());
}

/// The cross-document renderer answers by qualified name and
/// produces the *same* body the in-document path does, for both a proc and
/// a class — one renderer, so a call-site hover in another file can never
/// drift from the declaration's own.
#[test]
fn qualified_symbol_hover_matches_the_in_document_rendering_1018() {
    let src = "namespace eval ::math {}\nnamespace eval ::geo {}\nproc ::math::zzqfrobnicate {val args} { return $val }\noo::class create ::geo::Plane {\n    method fly {} {}\n}\n";
    let analysis = analyse(src);

    let proc_hover =
        qualified_symbol_hover(&analysis, "::math::zzqfrobnicate").expect("proc hover");
    assert_eq!(
        proc_hover.value,
        proc_hover_text(&analysis.all_procs["::math::zzqfrobnicate"]),
    );
    assert!(proc_hover.value.contains("val"), "{proc_hover:?}");

    let class_hover = qualified_symbol_hover(&analysis, "::geo::Plane").expect("class hover");
    assert_eq!(
        class_hover.value,
        class_hover_text(&analysis, &analysis.all_classes["::geo::Plane"]),
    );

    // A name neither map holds resolves to nothing — the fallback never
    // invents a symbol.
    assert!(qualified_symbol_hover(&analysis, "::math::nosuchthing").is_none());
    // The lookup is by *qualified* name only; a bare tail is not a match,
    // so a same-tailed proc in an unrelated namespace cannot be picked up.
    assert!(qualified_symbol_hover(&analysis, "zzqfrobnicate").is_none());
}

/// `pos_of` — (line, character) of the `occurrence`-th `needle`.
fn pos_of(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
    let mut start = 0;
    for _ in 0..occurrence {
        let idx = src[start..].find(needle).expect("needle not found") + start;
        start = idx + 1;
    }
    let idx = start - 1;
    let prefix = &src[..idx];
    let line = u32::try_from(prefix.matches('\n').count()).unwrap();
    let col = u32::try_from(idx - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
    (line, col)
}

// IP address hover

#[test]
fn ip_hover_classifies_private_ipv4() {
    let t = ip_address_hover_text("10.0.0.1").expect("hover");
    assert!(t.contains("IPv4 address"), "{t}");
    assert!(t.contains("Private (RFC 1918)"), "{t}");
}

#[test]
fn ip_hover_classifies_loopback() {
    let t = ip_address_hover_text("127.0.0.1").expect("hover");
    assert!(t.contains("Loopback"), "{t}");
}

#[test]
fn ip_hover_classifies_public_ipv4() {
    let t = ip_address_hover_text("8.8.8.8").expect("hover");
    assert!(t.contains("Public"), "{t}");
}

#[test]
fn ip_hover_renders_cidr_prefix() {
    let t = ip_address_hover_text("10.0.0.0/8").expect("hover");
    assert!(t.contains("CIDR network: `10.0.0.0/8`"), "{t}");
}

#[test]
fn ip_hover_classifies_ipv6_loopback() {
    let t = ip_address_hover_text("::1").expect("hover");
    assert!(t.contains("IPv6 address"), "{t}");
    assert!(t.contains("Loopback"), "{t}");
}

#[test]
fn ip_hover_detects_ipv4_mapped_ipv6() {
    let t = ip_address_hover_text("::ffff:192.0.2.1").expect("hover");
    assert!(t.contains("IPv4-mapped"), "{t}");
}

#[test]
fn ip_hover_rejects_non_ip_strings() {
    assert!(ip_address_hover_text("hello").is_none());
    assert!(ip_address_hover_text("256.256.256.256").is_none());
    assert!(ip_address_hover_text("not.an.ip.address").is_none());
}

#[test]
fn hover_fires_for_ip_address_word() {
    let src = "set host 10.0.0.1\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor on `10.0.0.1`.
    let h = hover(src, 0, 11, &analysis, None).expect("hover");
    assert!(h.value.contains("IPv4 address"), "{}", h.value);
}

fn retained_registry_text(source: &str, cursor: u32, environment: &str) -> Option<String> {
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-registry-cards";
    let profile = profile.intern();
    let context =
        tcl_registry::model::ingress::resolve_environment(environment).default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "presentation-only");
    original_registry::hover(source, &analysis, cursor).map(|hover| hover.value)
}
