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

fn refusals(
    source: &str,
    analysis: &AnalysisResult,
) -> (Option<RenameRefusal>, Option<RenameRefusal>) {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let index = LineIndex::new(source);
    (
        method_rename_hazard(
            source,
            profile,
            analysis,
            MethodRenameTarget {
                family: &[],
                method: "speak",
                is_classmethod: false,
                new_name: "renamed",
            },
            &index,
        ),
        namespace_variable_rename_hazard(source, profile, analysis, "::ns::value", &index),
    )
}

#[test]
fn complete_empty_and_literal_nonmember_sources_have_no_census_refusal() {
    for source in ["", "set inert 1", "if {1} {}"] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let (method, variable) = refusals(source, &analysis);
        assert!(method.is_none(), "{source}: {method:?}");
        assert!(variable.is_none(), "{source}: {variable:?}");
    }
}

#[test]
fn missing_body_and_parse_phases_cannot_be_a_negative_hazard_census() {
    for source in [
        "set before 1; $handler {set hidden 2}; set after 3",
        "if {1} $body",
        "if {1} {set unfinished",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let (method, variable) = refusals(source, &analysis);
        assert!(
            method.unwrap().reason.contains("potential source coverage"),
            "{source}"
        );
        assert!(
            variable
                .unwrap()
                .reason
                .contains("potential source coverage"),
            "{source}"
        );
    }
}

#[test]
fn positive_dispatch_hazard_survives_incomplete_source_inventory() {
    let source = "$other speak; if {1} $body";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let (method, _) = refusals(source, &analysis);
    assert!(method.unwrap().reason.contains("class is not tracked"));
}

#[test]
fn stale_source_input_and_configuration_withdraw_hazard_census() {
    let source = "set inert 1";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let mut changed = analysis.clone();
    changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    let mut foreign = analysis.clone();
    foreign.resolved_input = Analyser::new().analyse(source, "tcl9.0").resolved_input;
    for (text, current) in [
        ("set other 1", &analysis),
        (source, &missing),
        (source, &changed),
        (source, &foreign),
    ] {
        let (method, variable) = refusals(text, current);
        assert!(method.is_some());
        assert!(variable.is_some());
    }
}

#[test]
fn incomplete_census_retains_real_positive_command_visits() {
    let source = "set before 1; if {1} $body; set after 3";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut seen_before = false;
    let mut seen_after = false;
    let coverage = walk_document(source, profile, &analysis, &mut |command| {
        seen_before |=
            command.name() == "set" && command.args().first().is_some_and(|word| word == "before");
        seen_after |=
            command.name() == "set" && command.args().first().is_some_and(|word| word == "after");
    });
    assert!(seen_before);
    assert!(seen_after);
    assert_eq!(
        coverage,
        crate::executable_regions::PotentialSourceVisit::Incomplete
    );
}

#[test]
fn empty_body_does_not_bypass_document_phase_validation() {
    let source = "set inert 1";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.body_lexer_config = None;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut called = false;
    let coverage = walk_body(source, profile, &analysis, Span::new(0, 0), &mut |_| {
        called = true;
    });
    assert_eq!(
        coverage,
        crate::executable_regions::PotentialSourceVisit::Unavailable
    );
    assert!(!called);
}
