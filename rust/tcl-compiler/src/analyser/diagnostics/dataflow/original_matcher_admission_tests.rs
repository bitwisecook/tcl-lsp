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

fn output_names(source: &str, dialect: &str) -> Vec<String> {
    let analysis = Analyser::new().analyse(source, dialect);
    let config = analysis.body_lexer_config.unwrap();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let words = crate::registry_invocation::source_structure::source_registry_words(
        source,
        &analysis,
        commands.last().unwrap(),
    );
    words.map_or_else(Vec::new, |words| {
        selected_matcher_no_match_outputs(
            &words,
            &analysis.resolved_input.as_ref().unwrap().context_registry(),
        )
    })
}

#[test]
fn selected_original_matcher_advice_keeps_alias_arguments_and_known_provider_barriers() {
    // naming.diagnostics.original-matcher-output-retention
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
    // Selected conditional source protocol and decoded original argv only;
    // none of these Native analyses grants current output cells or reads.
    for dialect in [
        "tcl", "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim",
    ] {
        for source in [
            "regexp x y v",
            "scan abc %d v",
            "regexp x y {$v}",
            "interp alias {} matcher {} regexp x y; matcher v",
            "interp alias {} matcher {} regexp -nocase x; matcher Y v",
            "rename regexp matcher; matcher x y v",
            "interp alias {} matcher {} regexp x y; rename matcher moved; moved v",
        ] {
            let expected = if source.contains("{$v}") { "$v" } else { "v" };
            // Jim has no `interp alias` (jimsh 0.84: wrong # args: should be
            // "interp"), so no alias is made and nothing is written.
            let expected: &[&str] = if dialect == "jim" && source.starts_with("interp alias") {
                &[]
            } else {
                &[expected]
            };
            assert_eq!(output_names(source, dialect), expected, "{dialect}: {source}");
        }
        for source in [
            "namespace eval app {proc regexp args {}}; ::app::regexp x y v",
            "proc regexp args {}; regexp x y v",
            "proc scan args {}; scan abc %d v",
            "interp alias {} matcher {} regexp x y; rename regexp gone; proc regexp args {}; matcher v",
            "interp alias {} matcher {} regexp -inline x y; matcher v",
            "regexp -bogus x y v",
            "regexp -about x y v",
            "regexp -nocase x X v",
            "regexp -expanded {a b} ab v",
            "regexp -start end x y v",
            "regexp {$p} y v",
            "scan abc {%d %d} v",
            "scan abc %2d v",
            "scan abc %n v",
            "regexp é y v",
        ] {
            assert!(
                output_names(source, dialect).is_empty(),
                "{dialect}: {source}"
            );
        }
        // `%b` is a conversion from 8.6: tclsh8.6.18/9.0.4/9.1.0 `scan abc %b v`
        // and `scan abc %bad v` -> 0 with v unset; tclsh8.4.20/8.5.19 and
        // jimsh 0.84 raise bad scan conversion character.
        let binary_scan: &[&str] = if matches!(dialect, "tcl8.6" | "tcl9.0" | "tcl9.1") {
            &["v"]
        } else {
            &[]
        };
        for source in ["scan abc %b v", "scan abc %bad v"] {
            assert_eq!(output_names(source, dialect), binary_scan, "{dialect}: {source}");
        }
    }
}

#[test]
fn original_matcher_query_refuses_missing_foreign_and_stale_source_inputs() {
    // naming.diagnostics.original-matcher-output-retention
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
    let source = "regexp x y v";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let words = crate::registry_invocation::source_structure::source_registry_words(
        source, &analysis, &command,
    )
    .unwrap();
    assert_eq!(
        selected_matcher_no_match_outputs(
            &words,
            &analysis.resolved_input.as_ref().unwrap().context_registry()
        ),
        vec!["v"]
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert_eq!(
        selected_matcher_no_match_outputs(&words, foreign),
        [] as [std::string::String; 0]
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        crate::registry_invocation::source_structure::source_registry_words(
            source, &missing, &command
        )
        .is_none()
    );
    assert!(
        crate::registry_invocation::source_structure::source_registry_words(
            "regexp x z v",
            &analysis,
            &command
        )
        .is_none()
    );
    let mut changed = analysis.clone();
    // Jim is an environment, not a catalogued profile row: take its lexer
    // configuration from a Jim analysis.
    changed.body_lexer_config = Analyser::new().analyse(source, "jim").body_lexer_config;
    assert_ne!(changed.body_lexer_config, analysis.body_lexer_config);
    assert!(
        crate::registry_invocation::source_structure::source_registry_words(
            source, &changed, &command
        )
        .is_none()
    );
}

#[test]
fn logical_no_match_diagnostics_keep_prior_and_intervening_values_and_literal_names() {
    // naming.diagnostics.original-matcher-output-retention
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
    // Emitted conditional Logical diagnostics, not Native current contents.
    for (source, expected) in [
        ("proc f {} {regexp x y v; puts $v}", Some("$v")),
        ("proc f {} {scan abc %d v; puts $v}", Some("$v")),
        ("proc f {} {if {![regexp x y v]} {puts $v}}", Some("$v")),
        ("proc f {} {regexp x y {$v}; puts ${$v}}", Some("${$v}")),
        ("proc f {} {set v OLD; regexp x y v; puts $v}", None),
        ("proc f {v} {regexp x y v; puts $v}", None),
        ("proc f {} {regexp x y v; set v NEW; puts $v}", None),
        (
            "proc f {} {if {![regexp x y v]} {set v NEW; puts $v}}",
            None,
        ),
        (
            "namespace eval app {proc regexp {pattern input name} {upvar 1 $name output; set output CUSTOM; return 0}}; proc f {} {if {![::app::regexp x y v]} {puts $v}}",
            None,
        ),
        (
            "set marker PRELUDE\nproc f {} {if {![regexp x y v]} {puts $v}}",
            Some("$v"),
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        let spans = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .map(|diagnostic| &source[diagnostic.span.as_range()])
            .collect::<Vec<_>>();
        assert_eq!(spans, expected.into_iter().collect::<Vec<_>>(), "{source}");
    }
}
