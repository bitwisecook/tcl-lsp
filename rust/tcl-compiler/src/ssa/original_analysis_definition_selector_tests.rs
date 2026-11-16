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

fn unit(
    source: &str,
) -> (
    crate::compilation_unit::CompilationUnit,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
) {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (unit, context)
}

fn assert_foreign_word_geometry(
    unit: &crate::compilation_unit::CompilationUnit,
    source: &str,
    block: BlockId,
    index: usize,
    context: &tcl_registry::model::ContextRegistry,
) {
    let (other, _) = self::unit(source);
    let originals = other
        .top_level
        .ssa
        .point_contexts
        .as_ref()
        .unwrap()
        .source_token_inventory()
        .values()
        .filter_map(|tokens| {
            tokens
                .words()
                .first()
                .map(|word| (word.source().span.start(), tokens.clone()))
        })
        .collect();
    let mut foreign = unit.clone();
    assert!(
        foreign
            .top_level
            .ssa
            .point_contexts
            .as_mut()
            .unwrap()
            .restore_source_tokens(&originals)
    );
    assert!(
        original_analysis_words_for_statement(
            &foreign,
            &foreign.top_level,
            block,
            index,
            context.commands()
        )
        .is_none()
    );
}

#[test]
fn original_analysis_word_geometry_keeps_current_input_vector_and_owner() {
    let source = "set value 42\n";
    let (unit, context) = unit(source);
    let function = &unit.top_level;
    let (block, index) = function
        .ssa
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements.iter().enumerate().find_map(|(index, _)| {
                SsaSourceView::at_statement(&function.ssa, block, index)
                    .source_tokens()
                    .filter(|tokens| tokens.synthetic.is_none())
                    .map(|_| (block, index))
            })
        })
        .expect("genuine original source operation");
    let original =
        original_analysis_words_for_statement(&unit, function, block, index, context.commands())
            .expect("genuine current original word geometry");
    assert_eq!(
        original
            .words()
            .iter()
            .map(tcl_lexer::NativeWord::bytes)
            .collect::<Vec<_>>(),
        vec![b"set".as_slice(), b"value".as_slice(), b"42".as_slice()]
    );
    assert_eq!(
        original.config(),
        unit.ir_module
            .source_metadata_input
            .as_ref()
            .unwrap()
            .lexer_config()
    );
    assert!(
        original_analysis_words_for_statement(
            &unit,
            function,
            block,
            usize::MAX,
            context.commands()
        )
        .is_none()
    );
    assert!(
        original_analysis_words_for_statement(
            &unit,
            function,
            block,
            index,
            &CommandRegistry::build_default()
        )
        .is_none()
    );
    let mut missing = unit.clone();
    missing.top_level.source_metadata_input = None;
    assert!(
        original_analysis_words_for_statement(
            &missing,
            &missing.top_level,
            block,
            index,
            context.commands()
        )
        .is_none()
    );
    let mut stale = unit.clone();
    stale.source.push(' ');
    assert!(
        original_analysis_words_for_statement(
            &stale,
            &stale.top_level,
            block,
            index,
            context.commands()
        )
        .is_none()
    );
    assert_foreign_word_geometry(&unit, source, block, index, &context);
}

#[test]
fn definition_receiver_retains_literal_sigil_and_complete_braced_word() {
    let source = "set {$x} 1; incr {$x}; set x 2; incr x";
    let (unit, context) = unit(source);
    let plan = tcl_lexer::native_script_words_in(
        unit.ir_module.source.clone(),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        unit.ir_module.lexer_config,
    )
    .unwrap();
    let left = original_analysis_definition_at_unit(
        &unit,
        plan.commands[1].words[1].span(),
        context.commands(),
    )
    .unwrap();
    let right = original_analysis_definition_at_unit(
        &unit,
        plan.commands[3].words[1].span(),
        context.commands(),
    )
    .unwrap();
    assert_eq!(left.source_name_bytes(), b"$x");
    assert_eq!(right.source_name_bytes(), b"x");
    assert_ne!(left.symbol(), right.symbol());
    assert_eq!(source.get(left.span().as_range()), Some("{$x}"));
    assert!(
        original_analysis_definition_at_unit(
            &unit,
            tcl_lexer::Span::new(left.span().start(), left.span().end() - 1),
            context.commands()
        )
        .is_none()
    );
}

#[test]
fn original_analysis_definitions_preserve_occurrence_versions_and_current_body_owner() {
    let source = "set value TOP\nproc p {} {set value 123; set value TEXT; return $value}\n";
    let (unit, context) = unit(source);
    let definitions = source
        .match_indices("set value")
        .map(|(offset, _)| {
            let span = tcl_lexer::Span::new(
                u32::try_from(offset).expect("fixture source offset fits u32") + 4,
                u32::try_from(offset).expect("fixture source offset fits u32") + 9,
            );
            original_analysis_definition_at_unit(&unit, span, context.commands())
                .expect("exact genuine source definition")
        })
        .collect::<Vec<_>>();
    assert_eq!(definitions.len(), 3);
    assert!(std::ptr::eq(
        definitions[0].function(),
        std::ptr::from_ref(&unit.top_level)
    ));
    assert!(std::ptr::eq(
        definitions[1].function(),
        definitions[2].function()
    ));
    assert!(!std::ptr::eq(
        definitions[0].function(),
        definitions[1].function()
    ));
    assert_eq!(definitions[1].symbol(), definitions[2].symbol());
    assert_ne!(definitions[1].version(), definitions[2].version());
    let span = definitions[1].span();
    assert!(
        original_analysis_definition_at_unit(
            &unit,
            tcl_lexer::Span::new(span.start() + 1, span.end()),
            context.commands()
        )
        .is_none()
    );
    let body = definitions[1].function().body_id.unwrap();
    for formals in [false, true] {
        let mut changed = unit.clone();
        let procedure = changed
            .ir_module
            .procedures
            .get_mut(&body)
            .or_else(|| changed.ir_module.body_units.get_mut(&body))
            .unwrap();
        if formals {
            procedure.params.push("added".into());
        } else {
            procedure.body_source = Some("return OTHER".into());
        }
        assert!(original_analysis_definition_at_unit(&changed, span, context.commands()).is_none());
    }
    let mut missing = unit.clone();
    missing
        .procedures
        .get_mut(&body)
        .or_else(|| missing.body_units.get_mut(&body))
        .unwrap()
        .source_metadata_input = None;
    assert!(original_analysis_definition_at_unit(&missing, span, context.commands()).is_none());
    let mut stale = unit.clone();
    stale.source.push(' ');
    assert!(original_analysis_definition_at_unit(&stale, span, context.commands()).is_none());
}

#[test]
fn named_source_body_tokens_require_the_actual_header_body_and_attachment() {
    let source = "proc p {} {set value TEXT; return $value}\n";
    let (unit, context) = unit(source);
    let retained = unit.ir_module.retained_source_bindings.as_deref().unwrap();
    let actual_bodies = unit
        .ir_module
        .original_source_procedure_bodies()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let [body] = actual_bodies.as_slice() else {
        panic!("one actual Module-issued original declaration in this fixture");
    };
    let body = *body;
    let function = unit.function_for_body(Some(body)).unwrap();
    let tokens = function
        .ssa
        .point_contexts
        .as_ref()
        .unwrap()
        .source_token_inventory()
        .values()
        .map(|tokens| {
            let mut original = tokens.clone();
            crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
            original
        })
        .collect::<Vec<_>>();
    assert_eq!(tokens.len(), 2);
    for original in &tokens {
        assert!(retained.owns_original_body_tokens(
            &unit.ir_module,
            body,
            original,
            context.commands()
        ));
        let mut missing = original.clone();
        missing.source_binding = None;
        assert!(!retained.owns_original_body_tokens(
            &unit.ir_module,
            body,
            &missing,
            context.commands()
        ));
    }
    let (other, _) = self::unit(source);
    let actual_bodies = other
        .ir_module
        .original_source_procedure_bodies()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let [foreign_body] = actual_bodies.as_slice() else {
        panic!("one actual Module-issued original declaration in this fixture");
    };
    let foreign_body = *foreign_body;
    assert!(!retained.owns_original_body_tokens(
        &unit.ir_module,
        foreign_body,
        &tokens[0],
        context.commands()
    ));
    let foreign_function = other.function_for_body(Some(foreign_body)).unwrap();
    for original in foreign_function
        .ssa
        .point_contexts
        .as_ref()
        .unwrap()
        .source_token_inventory()
        .values()
    {
        let mut original = original.clone();
        crate::lattice_rebase::rebase_command_tokens(&mut original, foreign_function.base_offset);
        assert!(!retained.owns_original_body_tokens(
            &unit.ir_module,
            body,
            &original,
            context.commands()
        ));
    }
    for formals in [false, true] {
        let mut changed = unit.ir_module.clone();
        let procedure = changed.procedures.get_mut(&body).unwrap();
        if formals {
            procedure.params.push("added".into());
        } else {
            procedure.body_source = Some("return OTHER".into());
        }
        assert!(!retained.owns_original_body_tokens(
            &changed,
            body,
            &tokens[0],
            context.commands()
        ));
    }
}

#[test]
fn logical_card_body_tokens_refuse_a_different_current_declaration() {
    let source =
        "proc p {} {set value TEXT; return $value}\nproc q {} {set value TEXT; return $value}\n";
    let (unit, context) = unit(source);
    let retained = unit.ir_module.retained_source_bindings.as_deref().unwrap();
    let mut bodies = unit
        .ir_module
        .original_source_procedure_bodies()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    bodies.sort_unstable();
    let [first, second] = bodies.as_slice() else {
        panic!("two real current declarations");
    };
    for (actual, other) in [(*first, *second), (*second, *first)] {
        let function = unit.function_for_body(Some(actual)).unwrap();
        for tokens in function
            .ssa
            .point_contexts
            .as_ref()
            .unwrap()
            .source_token_inventory()
            .values()
        {
            let mut original = tokens.clone();
            crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
            assert!(
                original
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .has_original_logical_source_advice()
            );
            assert!(retained.owns_original_body_tokens(
                &unit.ir_module,
                actual,
                &original,
                context.commands()
            ));
            assert!(!retained.owns_original_body_tokens(
                &unit.ir_module,
                other,
                &original,
                context.commands()
            ));
        }
    }
}

#[test]
fn readonly_source_reads_withhold_unrepresented_contents_versions() {
    let source = "proc p {} {set value TEXT; return $value}\n";
    let (unit, context) = unit(source);
    let offset =
        u32::try_from(source.find("$value").unwrap()).expect("fixture source offset fits u32");
    let read = original_source_read_at_unit(
        &unit,
        tcl_lexer::Span::new(offset, offset + 6),
        context.commands(),
    )
    .expect("genuine readonly source read occurrence");
    assert!(read.function().body_id.is_some());
    assert_eq!(read.version(), None);
    assert!(
        read.function()
            .ssa
            .source_read_references
            .values()
            .any(|reference| reference.symbol == read.symbol())
    );
}
