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

use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
use crate::ir::{CommandTokens, Statement};
use std::sync::Arc;
use tcl_registry::model::ContextRegistry;

fn original_unit(
    source: &str,
    environment: &str,
) -> (
    Arc<ContextRegistry>,
    crate::environment_ingress::RetainedNativeUnit,
) {
    let environment = tcl_registry::model::ingress::resolve_known_environment(environment)
        .expect("actual selected source environment");
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    assert_eq!(native.execution_point, environment.point());
    let entry = crate::command_binding::SourceAnalysisEntry {
        metadata_context:
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &input,
            )),
        native_entry: Some(Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (
        context,
        crate::environment_ingress::RetainedNativeUnit::new(unit, owner),
    )
}

fn original_parent(unit: &CompilationUnit) -> &CommandTokens {
    let top = &unit.ir_module.top_level;
    top.retained_source_tokens_for_statement(&top.statements[0])
        .expect("actual structured parent source vector")
}

#[test]
fn original_structured_body_caller_keeps_parent_ordinal_and_namespace() {
    // naming.source.original-point-operand-projection
    // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
    // Actual software lowering and selected source context; no external
    // provider execution or proof that a conditional body entered.
    for source in [
        "if {$condition} {set first café} else {set second two}",
        "if {$condition} {set first \\\n café} else {set second \\\n two}",
    ] {
        for environment in ["tcl8.4", "tcl9.0"] {
            let (context, unit) = original_unit(source, environment);
            let module = &unit.ir_module;
            let parent = original_parent(&unit);
            let binding = parent.source_binding.as_ref().unwrap();
            let input = module.source_metadata_input.as_ref().unwrap();
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
                context.commands(),
                module,
            )
            .unwrap();
            let Statement::If {
                clauses, else_body, ..
            } = &module.top_level.statements[0]
            else {
                panic!("actual original source retains its structured If");
            };
            assert_eq!(clauses.len(), 1);
            let bodies = [&clauses[0].body, else_body.as_ref().unwrap()];
            for (body, word) in bodies.into_iter().zip([2, 4]) {
                let expected = binding
                    .original_body_source_for_word(
                        parent,
                        parent.words()[word].source().span,
                        input.lexer_config(),
                    )
                    .unwrap();
                assert_eq!(body.executed_source.as_deref(), Some(&expected));
                if let crate::command_binding::SourceOriginKind::Derived {
                    parent: actual_parent,
                    arguments,
                    source: actual_source,
                    ..
                } = expected.origin.kind()
                {
                    assert_eq!(actual_parent, binding.invocation_site().unwrap());
                    assert_eq!(arguments.as_slice(), &[word - 1]);
                    assert_eq!(actual_source, &expected.text);
                    assert_eq!(expected.base(), 0);
                } else {
                    assert!(!source.contains('\\'));
                }
                let namespace = binding
                    .original_body_namespace_context(
                        context.commands(),
                        metadata,
                        parent,
                        &expected,
                    )
                    .expect("authentic selected immediate-body namespace");
                assert_eq!(body.namespace_context.as_deref(), Some(&namespace));
                assert_ne!(body.statements, [] as [crate::ir::Statement; 0]);
                assert!(body.statements.iter().all(|statement| !matches!(
                    statement, Statement::Barrier { reason, .. }
                        if reason == "original body namespace context is unavailable"
                )));
                let child = body
                    .retained_source_tokens_for_statement(&body.statements[0])
                    .expect("lowered child keeps its original source tokens");
                assert_eq!(child.argv_texts.first().map(String::as_str), Some("set"));
                assert_eq!(child.words().len(), 3);
                assert_eq!(
                    child
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_lexer_config_for_tokens(child),
                    Some(input.lexer_config()),
                );
            }
        }
    }
}

#[test]
fn original_structured_body_word_caller_refuses_full_extent_and_changed_parent() {
    // naming.source.original-point-operand-projection
    // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
    let source = "if {$condition} {set first café} else {set second two}";
    let (_, unit) = original_unit(source, "tcl9.0");
    let parent = original_parent(&unit);
    let config = unit.ir_module.lexer_config;
    let binding = parent.source_binding.as_ref().unwrap();
    let span = parent.words()[2].source().span;
    let whole = tcl_lexer::word_span_at(source, span);
    assert_ne!(
        span, whole,
        "nonempty original braced word has distinct extents"
    );
    assert!(
        binding
            .original_body_source_for_word(parent, span, config)
            .is_some()
    );
    assert!(
        binding
            .original_body_source_for_word(parent, whole, config)
            .is_none()
    );
    let changed = tcl_lexer::Span::new(span.start().checked_add(1).unwrap(), span.end());
    assert!(
        binding
            .original_body_source_for_word(parent, changed, config)
            .is_none()
    );
    let mut changed_vector = parent.clone();
    changed_vector.argv_texts[2] = "set first changed".into();
    assert!(
        binding
            .original_body_source_for_word(&changed_vector, span, config)
            .is_none()
    );
    let (_, foreign) = original_unit(
        "if {$condition} {set first élan} else {set second two}",
        "tcl9.0",
    );
    let mut changed_source = parent.clone();
    changed_source.source_binding = original_parent(&foreign).source_binding.clone();
    assert!(
        changed_source
            .source_binding
            .as_ref()
            .unwrap()
            .original_body_source_for_word(&changed_source, span, config)
            .is_none()
    );
    let mut changed_config = config;
    changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
    assert!(
        binding
            .original_body_source_for_word(parent, span, changed_config)
            .is_none()
    );
}
