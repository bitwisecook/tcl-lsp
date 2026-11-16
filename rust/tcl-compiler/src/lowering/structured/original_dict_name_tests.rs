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

use crate::ir::Statement;

fn module(source: &str) -> crate::ir::Module {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input),
        source,
    )
}

#[test]
fn selected_dictionary_writes_keep_original_literal_variable_name_data() {
    for (word, name) in [
        ("{}", ""),
        ("{$literal}", "$literal"),
        ("{${braced}}", "${braced}"),
        ("{arr(key)}", "arr(key)"),
    ] {
        let module = module(&format!("dict set {word} key VALUE"));
        let Statement::Call {
            defs,
            reads_own_defs,
            ..
        } = &module.top_level.statements[0]
        else {
            panic!("original selected dictionary call")
        };
        assert_eq!(defs, &[name.to_owned()]);
        assert!(*reads_own_defs);
    }
}

#[test]
fn dictionary_prefix_and_expansion_keep_original_slot_ownership() {
    for (source, written_operand) in [
        (
            "interp alias {} write {} dict set; write {$literal} key VALUE",
            "{$literal}",
        ),
        ("dict set {*}{d key} VALUE", "{*}{d key}"),
        ("dict set {*}$words", "{*}$words"),
    ] {
        let module = module(source);
        let statement = module.top_level.statements.last().unwrap();
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let input = module.source_metadata_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let context = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )
        .unwrap();
        assert!(
            crate::registry_invocation::original_structured_invocation_with_metadata_context(
                registry, context, tokens
            )
            .is_none(),
            "shifted original operands cannot borrow direct source structure: {source}"
        );
        assert!(matches!(
            statement,
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
        assert!(
            tokens.words().iter().any(|word| {
                word.original_written_span(module.source.bytes())
                    .is_some_and(|span| {
                        module.source.bytes().get(span.as_range())
                            == Some(written_operand.as_bytes())
                    })
            }),
            "original shifted source operand must be retained: {written_operand}"
        );
    }
    let module = module("dict get {key VALUE} key");
    let Statement::Call {
        defs,
        reads_own_defs,
        safe_on_uninit,
        ..
    } = &module.top_level.statements[0]
    else {
        panic!("original nonmutating dictionary dispatch")
    };
    assert_eq!(defs.len(), 0);
    assert!(!reads_own_defs && !safe_on_uninit);
}

#[test]
fn dynamic_dictionary_targets_keep_original_dispatch_without_static_defs() {
    let module = module("dict set $receiver key VALUE");
    let statement = &module.top_level.statements[0];
    match statement {
        Statement::Call { defs, .. } => assert_eq!(defs.len(), 0),
        Statement::Barrier { .. } => {}
        _ => panic!("original dynamic dictionary dispatch"),
    }
    let tokens = module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let span = tokens.words()[2]
        .original_written_span(module.source.bytes())
        .unwrap();
    assert_eq!(
        module.source.bytes().get(span.as_range()),
        Some(b"$receiver".as_slice())
    );
}
