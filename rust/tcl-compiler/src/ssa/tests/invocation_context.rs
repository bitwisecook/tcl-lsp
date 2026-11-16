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

//! Supplied metadata against standalone ownership: role scans, definition subjects and operand symbols under the actual availability and grammar.

use super::*;

#[test]
fn supplied_ssa_availability_without_source_cannot_reopen_standalone_ownership() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Lexical software advice only, independent of Native frames or stores.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let static_context = tcl_registry::model::semantic::SemanticContext::for_profile(profile);
    let standalone = SsaInvocationContext::supplied(registry, Some(static_context.into()));
    let supplied = SsaInvocationContext::supplied(registry, Some(context.as_ref().into()));
    assert!(standalone.standalone);
    assert!(!supplied.standalone);
    assert!(supplied.metadata.unwrap().matches_registry(registry));
    assert!(supplied.metadata.unwrap().source_analysis_input().is_none());
    let owns = |selection| {
        braced_word_owns_name(&BracedWordSite {
            braced: true,
            evaluated_in_frame: false,
            described: false,
            word: "set x 1; puts $x",
            name: "x",
            selection,
            tokens: None,
            written_argument: 0,
            config,
        })
    };
    assert!(owns(standalone));
    assert!(!owns(supplied));
    assert!(!owns(SsaInvocationContext::supplied(registry, None)));
}

#[test]
fn original_ssa_body_ownership_exclusion_keeps_literal_roots_and_opaque_residuals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The bridge consumes conditional body names, never actual read exclusion.
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use crate::registry_invocation::InvocationMetadataContext;
    use std::sync::Arc;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    for (body, expected) in [
        ("set {$literal} VALUE; puts ${$literal}", true),
        ("set {$literal} VALUE; operation", false),
    ] {
        let source = format!("catch {{{body}}}");
        let unit = CompilationUnit::build_with_analysis_input(
            &source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(
                script.statements.last().expect("original installer"),
            )
            .expect("original whole installer words");
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module);
        let owns = |name, selection| {
            braced_word_owns_name(&BracedWordSite {
                braced: true,
                evaluated_in_frame: false,
                described: false,
                word: body,
                name,
                selection,
                tokens: Some(tokens),
                written_argument: 0,
                config,
            })
        };
        let selection = SsaInvocationContext::supplied(registry, metadata);
        assert_eq!(owns("$literal", selection), expected, "{body}");
        assert!(
            !owns("literal", selection),
            "the literal sigil is part of the name"
        );
        assert!(!owns(
            "$literal",
            SsaInvocationContext::supplied(registry, None)
        ));
    }
}

#[test]
fn supplied_ssa_role_scans_keep_actual_availability_grammar_and_missing_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Selected source roles are independent of Native NamePolicy and actual execution.
    use crate::registry_invocation::InvocationMetadataContext;
    use std::sync::Arc;
    let (context, unit) = logical_role_metadata_fixture();
    let registry = context.commands();
    let function = &unit.top_level;
    let input = function.source_metadata_input().unwrap();
    let statement = function
        .cfg
        .blocks
        .values()
        .flat_map(|body| &body.statements)
        .find(|statement| statement.tokens().is_some())
        .unwrap();
    let scan = |metadata| {
        let mut scanner = VarReferenceScanner::with_config(
            VarScanOptions {
                include_var_read_roles: false,
                ..Default::default()
            },
            function.source_lexer_config(),
        );
        uses_of_classified_in_context(
            statement,
            &mut scanner,
            SsaInvocationContext::supplied(registry, metadata),
        )
    };
    let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
    assert!(metadata.is_some());
    assert!(
        function
            .ssa
            .point_contexts
            .as_ref()
            .unwrap()
            .before_statement(function.cfg.entry, 0)
            .execution_name_policy
            .is_none()
    );
    assert_eq!(scan(metadata), vec![("selected".into(), UseClass::Name)]);
    assert_eq!(
        scan(None),
        [] as [(std::string::String, crate::ssa::UseClass); 0]
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), registry));
    let older_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        older,
        input.lexer_config(),
    );
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    let mut grammar = input.lexer_config();
    grammar.strict_quoting = !grammar.strict_quoting;
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&context),
        grammar,
    );
    for withheld in [&older_input, &foreign, &changed] {
        assert_eq!(
            scan(InvocationMetadataContext::for_analysis_input(
                registry, withheld
            )),
            [] as [(std::string::String, crate::ssa::UseClass); 0]
        );
    }
}

#[test]
fn supplied_ssa_definition_and_lexical_reads_do_not_reconstruct_catalogue_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (context, unit) = logical_role_metadata_fixture();
    let registry = context.commands();
    let function = &unit.top_level;
    let statement = function
        .cfg
        .blocks
        .values()
        .flat_map(|body| &body.statements)
        .find(|statement| statement.tokens().is_some())
        .unwrap();
    let unavailable = SsaInvocationContext::supplied(registry, None);
    assert_eq!(
        defs_of_in_context(statement, Some(unavailable)),
        [] as [std::string::String; 0]
    );
    let tokens = statement.tokens().unwrap();
    assert!(unavailable.normal(tokens).is_none());
    assert!(unavailable.resolve(tokens).is_none());
    // Pure word substitution remains lexical even when command roles are withheld.
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: false,
            ..Default::default()
        },
        function.source_lexer_config(),
    );
    assert_eq!(
        scanner.scan_word("$lexical [info exists nominal]", registry),
        BTreeSet::from(["lexical".to_owned()])
    );
    let mut requested = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: true,
            ..Default::default()
        },
        function.source_lexer_config(),
    );
    assert_eq!(
        unavailable.word_reads(&mut requested, "$lexical [info exists nominal]"),
        BTreeSet::from(["lexical".to_owned()])
    );
    assert!(
        SsaInvocationContext::standalone(registry)
            .word_reads(&mut requested, "$lexical [info exists nominal]")
            .contains("nominal")
    );
    assert_eq!(
        unavailable.word_reads(&mut requested, "$lexical [info exists nominal]"),
        BTreeSet::from(["lexical".to_owned()])
    );
}

pub(super) fn original_setter_definition_point(
    function: &crate::compilation_unit::FunctionUnit,
) -> (BlockId, usize, Symbol) {
    function
        .ssa
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    statement
                        .defs
                        .keys()
                        .next()
                        .map(|&symbol| (block, index, symbol))
                })
        })
        .expect("genuine selected original setter definition")
}

#[test]
fn original_definition_subjects_keep_supplied_availability_and_missing_input_separate() {
    // naming.variable.original-ssa-definition-operand
    // docs/design/analysis/name-resolution-proofs/original-ssa-definition-operand.md
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::registry_invocation::InvocationMetadataContext;
    use std::sync::Arc;
    let (context, unit) = original_operand_metadata_fixture();
    let function = &unit.top_level;
    let (block, index, symbol) = original_setter_definition_point(function);
    let view = SsaSourceView::at_statement(&function.ssa, block, index);
    let registry = context.commands();
    let select =
        |metadata| view.original_definition_name_with_metadata_context(symbol, registry, metadata);
    let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
    let original = select(metadata).expect("actual original receiver under current availability");
    assert_eq!(original.original_name_input().bytes(), b"selected");
    assert_eq!(Some(original.cell()), function.ssa.cell_key(symbol));
    assert!(select(None).is_none());
    let input = function.source_metadata_input().unwrap();
    let older_context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(older_context.commands(), context.commands()));
    let older = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        older_context,
        input.lexer_config(),
    );
    assert!(
        select(InvocationMetadataContext::for_analysis_input(
            registry, &older
        ))
        .is_none()
    );
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    assert!(
        select(InvocationMetadataContext::for_analysis_input(
            registry, &foreign
        ))
        .is_none()
    );
    let mut changed_config = input.lexer_config();
    changed_config.strict_quoting = !changed_config.strict_quoting;
    assert_ne!(
        changed_config.normalized(),
        input.lexer_config().normalized()
    );
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&context),
        changed_config,
    );
    assert!(
        select(InvocationMetadataContext::for_analysis_input(
            registry, &changed
        ))
        .is_none()
    );
    let mut unavailable_module = unit.ir_module.clone();
    unavailable_module.source_metadata_input = None;
    assert!(
        select(function.invocation_metadata_context_for_module(registry, &unavailable_module))
            .is_none()
    );
}

#[test]
fn original_operand_symbols_keep_actual_availability_and_source_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::registry_invocation::InvocationMetadataContext;
    use std::sync::Arc;
    let (context, unit) = original_operand_metadata_fixture();
    let function = &unit.top_level;
    let input = function.source_metadata_input().unwrap();
    let points = function.ssa.point_contexts.as_ref().unwrap();
    let point = function
        .cfg
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    statement
                        .is_executable_invocation()
                        .then_some((block, index))
                })
        })
        .unwrap();
    let symbols = |metadata, config| {
        let metadata = supplied_ssa_metadata(context.commands(), config, metadata);
        let mut walk = RenameWalk::new(&function.cfg, config, points.clone(), metadata);
        let mut symbols = HashMap::new();
        walk.retain_native_operand_symbols(
            point,
            points.context_before(point.0, point.1).unwrap(),
            context.commands(),
            &mut symbols,
        );
        symbols
    };
    let metadata = InvocationMetadataContext::for_analysis_input(context.commands(), input);
    assert_eq!(symbols(metadata, function.source_lexer_config()).len(), 1);
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), context.commands()));
    let older_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        older,
        input.lexer_config(),
    );
    assert!(
        symbols(
            InvocationMetadataContext::for_analysis_input(context.commands(), &older_input,),
            function.source_lexer_config()
        )
        .is_empty()
    );
    assert!(symbols(None, function.source_lexer_config()).is_empty());
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    assert!(
        symbols(
            InvocationMetadataContext::for_analysis_input(context.commands(), &foreign,),
            function.source_lexer_config()
        )
        .is_empty()
    );
    let mut stale_config = function.source_lexer_config();
    stale_config.strict_quoting = !stale_config.strict_quoting;
    assert!(symbols(metadata, stale_config).is_empty());
}
