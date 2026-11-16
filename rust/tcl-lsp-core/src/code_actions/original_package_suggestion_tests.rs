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

fn at(line: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: 2,
        end_line: line,
        end_character: 2,
    }
}

#[test]
// Implementation contract: naming.core.original-package-suggestion-subjects
// docs/design/analysis/name-resolution-proofs/original-package-suggestion-subjects.md
fn original_package_hint_uses_written_units_and_issuer_after_reporting_clear() {
    let source = "http::f\\uD800\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let registry = tcl_registry::CommandRegistry::build_default();
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "other::display".into();
    }
    for diagnostic in &mut analysis.diagnostics {
        diagnostic.message = "counterfactual display".into();
    }
    let actions = package_require_actions(source, at(0), &registry, Some(&analysis), &[]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].edits[0].new_text, "package require http\n");
    assert_eq!(
        package_require_actions(
            &format!("# displaced\n{source}"),
            at(1),
            &registry,
            Some(&analysis),
            &[]
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let context = [ContextDiagnostic {
        data: None,
        code: "W123".into(),
        message: "Unknown http::name".into(),
        range: at(0),
    }];
    for diagnostic in &mut analysis.diagnostics {
        diagnostic.subject = None;
    }
    assert_eq!(
        package_require_actions(source, at(0), &registry, Some(&analysis), &context),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
// Implementation contract: naming.core.original-package-suggestion-subjects
// docs/design/analysis/name-resolution-proofs/original-package-suggestion-subjects.md
fn original_package_hint_requires_own_lookup_and_canonical_package_key() {
    let source = "http::missing\n";
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(
        package_require_actions(source, at(0), &registry, Some(&analysis), &[]).len(),
        1
    );
    let invocation = analysis
        .command_invocations
        .iter_mut()
        .find(|invocation| {
            invocation
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"http::missing")
        })
        .unwrap();
    invocation.original_lookup = None;
    assert_eq!(
        package_require_actions(source, at(0), &registry, Some(&analysis), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let source = "package require h\\u0074tp\nhttp::missing\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let requirement = analysis.package_requires.first_mut().unwrap();
    assert!(
        requirement
            .original_name
            .as_ref()
            .unwrap()
            .key()
            .matches_ascii("http")
    );
    requirement.name = "unrelated-report".into();
    assert_eq!(
        package_require_actions(source, at(1), &registry, Some(&analysis), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
}
#[test]
fn original_package_action_uses_genuine_provision_and_requirement_operands() {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    for (source, available) in [
        ("package provide http\n", false),
        ("package provide http 1\n", true),
        ("package require -exact h\\u0074tp 1\n", true),
        ("set text {package require http}\n", false),
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        for requirement in &mut analysis.package_requires {
            requirement.name = "wrong".to_owned();
        }
        for provision in &mut analysis.package_provides {
            provision.name = "wrong".to_owned();
        }
        assert_eq!(
            super::original_source_package_already_named(source, &analysis, "http"),
            Some(available),
            "{source}"
        );
    }
}

#[test]
fn original_logical_package_action_requires_its_positive_vector_and_current_input() {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "logical-package-source-action",
        &[],
        "Logical package advice",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let source = "http::missing\n";
    let mut analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, profile.name);
    assert!(
        tcl_compiler::registry_invocation::source_structure::original_logical_source_words_at(
            source, &analysis, 0
        )
        .is_some()
    );
    for invocation in &mut analysis.command_invocations {
        invocation.name = "wrong::report".to_owned();
    }
    let actions = package_require_actions(source, at(0), context.commands(), Some(&analysis), &[]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].edits[0].new_text, "package require http\n");
    let mut config = input.lexer_config();
    config.leading_bom = match config.leading_bom {
        tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
    };
    analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        config,
    ));
    assert_eq!(
        package_require_actions(source, at(0), context.commands(), Some(&analysis), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let native = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        tcl_compiler::registry_invocation::source_structure::original_logical_source_words_at(
            source, &native, 0
        )
        .is_none()
    );
}

fn selected_namespace_package_input() -> tcl_compiler::analyser::ResolvedAnalysisInput {
    use std::sync::Arc;
    let mut store = tcl_registry::CommandRegistry::build_default();
    store.insert(tcl_registry::CommandSpec {
        name: "tools::fetch",
        surface: Some(tcl_dialect::model::SpecSurface::TCL86),
        required_package: Some("tools-runtime"),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let store = Arc::new(store);
    let context = Arc::new(
        tcl_registry::model::ingress::resolve_environment("tcl8.6")
            .default_context_registry()
            .with_command_store(Arc::clone(&store)),
    );
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "logical-selected-package-owner",
        &[],
        "Logical package source",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        config,
    )
}

#[test]
fn original_logical_package_hint_uses_selected_namespace_owner_not_package_spelling() {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    use std::sync::Arc;
    let input = selected_namespace_package_input();
    let context = input.context_registry();
    let store = Arc::clone(context.commands());
    let profile = input.analyser_profile();
    let config = input.lexer_config();
    let source = "tools::missing\n";
    let analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, profile.name);
    let actions = package_require_actions(source, at(0), &store, Some(&analysis), &[]);
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].title, "Add 'package require tools-runtime'");
    assert_eq!(
        actions[0].edits[0].new_text,
        "package require tools-runtime\n"
    );
    assert_eq!(
        package_require_actions("tools::changed\n", at(0), &store, Some(&analysis), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let argument_source = "tools::missing operand\n";
    let argument_analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(argument_source, profile.name);
    assert_eq!(
        package_require_actions(
            argument_source,
            at(18),
            &store,
            Some(&argument_analysis),
            &[]
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let mut wrong_config = config;
    wrong_config.leading_bom = match config.leading_bom {
        tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
    };
    let mut changed = analysis.clone();
    changed.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        wrong_config,
    ));
    assert_eq!(
        package_require_actions(source, at(0), &store, Some(&changed), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn original_package_hint_keeps_literal_position_case_and_existing_require_guards() {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    use std::sync::Arc;
    let input = selected_namespace_package_input();
    let context = input.context_registry();
    let store = Arc::clone(context.commands());
    let profile = input.analyser_profile();
    for source in [
        "Tools::missing\n",
        "puts {tools::missing}\n",
        "package require tools-runtime\ntools::missing\n",
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let offset = u32::try_from(source.rfind("tools::missing").unwrap_or(0)).unwrap();
        let position = LineIndex::new(source).position_at_utf16(offset, source);
        let range = LspRange {
            start_line: position.line,
            start_character: position.character.get(),
            end_line: position.line,
            end_character: position.character.get(),
        };
        assert!(
            package_require_actions(source, range, &store, Some(&analysis), &[]).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn original_package_hint_declines_conflicting_unowned_and_foreign_context_rows() {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    use std::sync::Arc;
    let input = selected_namespace_package_input();
    let context = input.context_registry();
    let store = Arc::clone(context.commands());
    let profile = input.analyser_profile();
    let config = input.lexer_config();
    let mut conflicting = tcl_registry::CommandRegistry::build_default();
    for (name, package) in [
        ("tools::fetch", Some("tools-runtime")),
        ("tools::other", Some("foreign-provider")),
        ("unowned::fetch", Some("own-provider")),
        ("unowned::other", None),
    ] {
        conflicting.insert(tcl_registry::CommandSpec {
            name,
            surface: Some(tcl_dialect::model::SpecSurface::TCL86),
            required_package: package,
            ..tcl_registry::CommandSpec::DEFAULT
        });
    }
    let conflicting = Arc::new(conflicting);
    let conflict_context = Arc::new(context.with_command_store(Arc::clone(&conflicting)));
    for source in ["tools::missing\n", "unowned::missing\n"] {
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&conflict_context),
            config,
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert_eq!(
            package_require_actions(source, at(0), &conflicting, Some(&analysis), &[]),
            [] as [crate::code_actions::CodeAction; 0]
        );
    }
    let foreign = Arc::new(
        tcl_registry::model::ingress::resolve_environment("tcl8.4")
            .default_context_registry()
            .with_command_store(Arc::clone(&store)),
    );
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, foreign, config);
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse("tools::missing\n", profile.name);
    assert_eq!(
        package_require_actions("tools::missing\n", at(0), &store, Some(&analysis), &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
}
