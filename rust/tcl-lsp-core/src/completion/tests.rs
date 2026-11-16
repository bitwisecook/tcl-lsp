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
use tcl_compiler::analyser::Analyser;
use tcl_dialect::model::{Family, SurfaceLayer};

mod argument_values;
mod commands;
mod methods;
mod variables;

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

/// [`analyse`] under Tcl 9.0 — for sources using 9.0-only class
/// machinery (`oo::configurable`, `property`), which the 8.6 analysis
/// the rest of this module uses does not model.
fn analyse9(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl9.0").clone()
}

#[test]
fn dollar_trigger_at_eol_returns_partial() {
    let (trigger, partial) = variable_trigger("set x $", 0, 7).unwrap();
    assert_eq!(trigger, '$');
    assert_eq!(partial, "");
}

#[test]
fn dollar_trigger_with_partial_name() {
    let (trigger, partial) = variable_trigger("set y $foo", 0, 10).unwrap();
    assert_eq!(trigger, '$');
    assert_eq!(partial, "foo");
}

#[test]
fn brace_trigger_recognised() {
    let (trigger, partial) = variable_trigger("set y ${ba", 0, 10).unwrap();
    assert_eq!(trigger, '{');
    assert_eq!(partial, "ba");
}

#[test]
fn space_resets_trigger_context() {
    // The `$` is followed by a space + new identifier — no
    // variable trigger active at the cursor.
    assert!(variable_trigger("set $ ab", 0, 8).is_none());
}

// iRules event-name completion
//
// When the cursor sits at word-index 1 of an event-handler
// command (the `when` iRules keyword carries
// `Traits::IS_EVENT_HANDLER`), the completion surface lists
// event names from the shared `EventRegistry`.

fn irules_registry() -> CommandRegistry {
    let mut r = CommandRegistry::build_default();
    r.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    r
}

/// iRule event snippet labels surfaced at a cursor.
fn irule_snippet_labels(
    src: &str,
    line: u32,
    character: u32,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<String> {
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(src, dialect.name);
    let registry = irules_registry();
    completions(
        src,
        line,
        character,
        &analysis,
        Some(&registry),
        None,
        dialect,
    )
    .into_iter()
    .filter(|i| i.label.starts_with("iRule"))
    .map(|i| i.label)
    .collect()
}

#[test]
fn irules_event_snippets_surface_at_top_level() {
    // `irule` partial at a top-level command position in the iRules
    // dialect, including canonicalised legacy aliases, offers every
    // event template.
    for dialect in ["f5-irules", "irules", "tcl-irule"] {
        let labels = irule_snippet_labels(
            "irule",
            0,
            5,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert!(
            labels.iter().any(|l| l == "iRule RULE_INIT"),
            "{dialect}: expected RULE_INIT; got {labels:?}",
        );
        assert!(
            labels.iter().any(|l| l == "iRule HTTP_REQUEST"),
            "{dialect}: expected HTTP_REQUEST; got {labels:?}",
        );
    }
}

#[test]
fn completion_builds_resolved_event_facts_once_per_request() {
    let before = crate::irules_context::expensive_build_count();
    let labels = irule_snippet_labels("irule", 0, 5, tcl_dialect::DialectProfile::irules());
    assert!(labels.iter().any(|label| label == "iRule RULE_INIT"));
    assert_eq!(
        crate::irules_context::expensive_build_count(),
        before + 1,
        "completion must reuse one precomputed identity/boundary inventory"
    );
}

#[test]
fn irules_snippets_decline_for_already_declared_event() {
    // `when RULE_INIT { }` already declared → the RULE_INIT template
    // drops out, but other event templates remain.
    let src = "when RULE_INIT {\n}\nirule";
    let labels = irule_snippet_labels(src, 2, 5, tcl_dialect::DialectProfile::irules());
    assert!(
        !labels.iter().any(|l| l == "iRule RULE_INIT"),
        "RULE_INIT already declared, should decline; got {labels:?}",
    );
    assert!(
        labels.iter().any(|l| l == "iRule HTTP_REQUEST"),
        "HTTP_REQUEST not declared, should remain; got {labels:?}",
    );
}

#[test]
fn irules_event_snippets_suppressed_inside_when_body() {
    // Cursor inside a `when HTTP_REQUEST { }` body → the top-level
    // guard suppresses every event template.
    let src = "when HTTP_REQUEST {\n    irule\n}\n";
    let labels = irule_snippet_labels(src, 1, 9, tcl_dialect::DialectProfile::irules());
    assert!(
        labels.is_empty(),
        "event templates must not offer inside a when block; got {labels:?}",
    );
}

#[test]
fn irules_snippets_hidden_in_plain_tcl_dialect() {
    // Same source, but the `tcl8.6` dialect carries no iRule
    // templates at all.
    let labels = irule_snippet_labels(
        "irule",
        0,
        5,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(labels.is_empty(), "got {labels:?}");
}

// The shared candidate filter (prefix path + fuzzy fallback).

#[test]
fn filter_candidates_prefix_path_keeps_input_order() {
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("alpha", vec!["delta", "alpha", "beta", "alphabet"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, vec!["alpha", "alphabet"]);
}

#[test]
fn filter_candidates_empty_partial_keeps_everything() {
    let FilteredCandidates { candidates, fuzzy } = filter_candidates("", vec!["b", "a"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, vec!["b", "a"]);
}

#[test]
fn filter_candidates_no_fallback_for_one_char_fragment() {
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("q", vec!["set", "puts"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, [] as [&str; 0]);
}

#[test]
fn filter_candidates_fuzzy_ranks_by_distance_then_name() {
    // No candidate prefix-matches `abcdef`; `zbcdef` is one edit
    // away, `abcdxy` two — distance outranks the name order.
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("abcdef", vec!["abcdxy", "zbcdef", "qqqqqq"], |n| *n);
    assert!(fuzzy);
    assert_eq!(candidates, vec!["zbcdef", "abcdxy"]);
}

#[test]
fn filter_candidates_fuzzy_cap_respected() {
    // Nine candidates all one edit from `az` — the fallback caps
    // at MAX_FUZZY_SUGGESTIONS, keeping the name-order head.
    let names = vec!["aa", "ab", "ac", "ad", "ae", "af", "ag", "ah", "ai"];
    let FilteredCandidates { candidates, fuzzy } = filter_candidates("az", names, |n| *n);
    assert!(fuzzy);
    assert_eq!(candidates.len(), MAX_FUZZY_SUGGESTIONS);
    assert_eq!(candidates[0], "aa");
    assert!(!candidates.contains(&"ai"), "cap should drop the tail");
}

#[test]
fn filter_candidates_fuzzy_respects_distance_budget() {
    // `xyzzy` is beyond the scaled edit budget of every candidate.
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("xyzzy", vec!["set", "puts"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, [] as [&str; 0]);
}
