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
use tcl_registry::{Arity, CommandSpec};

fn logical_analysis(source: &str, registry: tcl_registry::CommandRegistry) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "package-census-logical",
        &[],
        "Logical package census",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .with_command_store(std::sync::Arc::new(registry)),
    );
    Analyser::new()
        .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}

#[test]
fn package_census_uses_original_inventory_and_actual_overlay_operation() {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "need",
        arity: Arity::exact(1),
        analyser_hook: Some(tcl_registry::hooks::AnalyserHookId::PackageRequire),
        arg_roles: &[(0, tcl_registry::ArgRole::Name)],
        ..CommandSpec::DEFAULT
    });
    let source = "need h\\u0074tp\n";
    let mut analysis = logical_analysis(source, registry);
    assert!(analysis.allows_lexical_declaration_advice());
    analysis.command_invocations.clear();
    analysis.package_requires.clear();
    analysis.package_provides.clear();
    assert_eq!(
        original_source_package_already_named(source, &analysis, "http"),
        Some(true)
    );

    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "package",
        surface: Some(tcl_dialect::model::SpecSurface::TCL86),
        ..CommandSpec::DEFAULT
    });
    let source = "package require http\n";
    let analysis = logical_analysis(source, registry);
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    assert!(
        context
            .context()
            .resolve_spec(context.commands(), "package")
            .unwrap()
            .subcommands
            .is_empty()
    );
    assert_eq!(
        original_source_package_already_named(source, &analysis, "http"),
        Some(false)
    );
}

#[test]
fn package_action_census_cannot_lose_a_genuine_reference_with_report_rows() {
    let source = "http::missing\npackage provide http 1\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis
        .command_invocations
        .retain(|invocation| invocation.range.start() == 0);
    analysis.package_requires.clear();
    analysis.package_provides.clear();
    assert_eq!(
        original_source_package_already_named(source, &analysis, "http"),
        Some(true)
    );
    let range = LspRange {
        start_line: 0,
        start_character: 2,
        end_line: 0,
        end_character: 2,
    };
    let registry = tcl_registry::CommandRegistry::build_default();
    assert_eq!(
        package_require_actions(source, range, &registry, Some(&analysis), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn package_census_retains_positive_named_sources_and_incomplete_regions() {
    for (source, expected) in [
        ("package require {http}\n", Some(true)),
        ("package provide \"http\" 1\n", Some(true)),
        ("proc pending {} {package require http}\n", Some(true)),
        ("set data {package require http}\n", Some(false)),
        ("package provide http\n", Some(false)),
        ("package require $name\n", None),
        ("package $operation http\n", None),
        ("package require\n", None),
        ("set broken {unterminated\n", None),
        ("proc package args {}; package require http\n", None),
        ("set head package; $head require http\n", None),
        ("package require http; $head arbitrary\n", Some(true)),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            original_source_package_already_named(source, &analysis, "http"),
            expected,
            "{source}"
        );
    }
}

#[test]
fn package_census_preserves_genuine_absent_head_advice_and_current_input() {
    let source = "http::missing\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let registry = tcl_registry::CommandRegistry::build_default();
    let range = LspRange {
        start_line: 0,
        start_character: 2,
        end_line: 0,
        end_character: 2,
    };
    assert_eq!(
        original_source_package_already_named(source, &analysis, "http"),
        Some(false)
    );
    assert_eq!(
        package_require_actions(source, range, &registry, Some(&analysis), &[]).len(),
        1
    );
    assert_eq!(
        original_source_package_already_named("# changed\nhttp::missing\n", &analysis, "http"),
        None
    );
    analysis.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    assert_eq!(
        original_source_package_already_named(source, &analysis, "http"),
        None
    );
    assert_eq!(
        package_require_actions(source, range, &registry, Some(&analysis), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn selected_syntax_coverage_keeps_package_and_call_closure_distinct() {
    use crate::source_structure::{SourceSyntaxCoverage, SourceSyntaxCoverageApplicability};
    for (source, expected) in [
        (
            "set data {package require http}\n",
            SourceSyntaxCoverageApplicability::Complete,
        ),
        (
            "package require $name\n",
            SourceSyntaxCoverageApplicability::Complete,
        ),
        (
            "http::missing\n",
            SourceSyntaxCoverageApplicability::Incomplete,
        ),
        (
            "set head package; $head require http\n",
            SourceSyntaxCoverageApplicability::Incomplete,
        ),
        (
            "set broken {unterminated\n",
            SourceSyntaxCoverageApplicability::Incomplete,
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let coverage = SourceSyntaxCoverage::capture(source, &analysis).unwrap();
        assert_eq!(
            coverage.applicability(source, &analysis),
            Some(expected),
            "{source}"
        );
        assert!(coverage.commands(source, &analysis).is_some());
        assert_eq!(
            coverage.applicability(&format!("#{source}"), &analysis),
            None
        );
        assert!(
            coverage
                .commands(&format!("#{source}"), &analysis)
                .is_none()
        );
    }
}
