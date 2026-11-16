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
use crate::cfg::{Block, Function, Terminator};
use crate::expr_ast::ExprNode;
use tcl_lexer::Span;

fn logical_role_metadata_fixture() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    logical_metadata_fixture("info exists selected")
}

fn logical_metadata_fixture(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut info = registry.get("info").unwrap().clone();
    info.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(info);
    let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input)
        .expect("genuine positive supplied Logical input");
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
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
    (context, unit)
}

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

fn original_operand_metadata_fixture() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::environment_ingress::RetainedNativeUnit,
) {
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = baseline.commands().project_for_profile(profile);
    let mut setter = registry.get("set").unwrap().clone();
    setter.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(setter);
    let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set selected VALUE",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        Arc::clone(&context),
    );
    (
        context,
        crate::environment_ingress::RetainedNativeUnit::new(unit, owner),
    )
}

fn original_setter_definition_point(
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

#[test]
fn destruction_predecessor_keeps_substituted_operand_use_in_def_use() {
    for substituted in [false, true] {
        let cfg = Function::new("::top", "entry");
        let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        let symbol = ssa.intern_var("x");
        let mut statement = SsaStatement {
            statement: Statement::Call {
                span: Span::new(0, 8),
                command: "unset".into(),
                canonical_command: Some("::unset".into()),
                args: vec![if substituted { "$x" } else { "x" }.into()],
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            },
            uses: if substituted {
                HashMap::from([(symbol, 1)])
            } else {
                HashMap::new()
            },
            defs: HashMap::from([(symbol, 2)]),
            may_defs: HashSet::from([symbol]),
            destruction_defs: HashSet::from([symbol]),
            quoted_uses: HashSet::new(),
            name_only_uses: HashSet::new(),
        };
        record_destruction_predecessor(
            symbol,
            1,
            &mut statement.uses,
            &mut statement.name_only_uses,
        );
        ssa.blocks.insert(
            cfg.entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![statement],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        let chains =
            crate::def_use::build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        let uses = chains.uses_of(ssa.var_symbol("x").unwrap(), 1);
        assert_eq!(uses.len(), 1);
        assert_eq!(
            uses[0].class,
            if substituted {
                UseClass::Substituted
            } else {
                UseClass::Name
            }
        );
        assert_eq!(
            uses[0].kind,
            if substituted {
                crate::def_use::UseKind::Operand
            } else {
                crate::def_use::UseKind::VariableName
            }
        );
    }
}

/// Intern `name` into `func` and insert a fresh block for it, returning
/// the [`BlockId`]. The shared test idiom for building a CFG by hand.
fn block(func: &mut Function, name: &str) -> BlockId {
    let id = func.intern_block(name);
    func.blocks.insert(id, Block::new(name));
    id
}

fn make_goto(target: BlockId) -> Terminator {
    Terminator::Goto { target, span: None }
}

fn make_branch(cond: &str, t: BlockId, f: BlockId) -> Terminator {
    Terminator::Branch {
        condition: ExprNode::Raw { text: cond.into() },
        true_target: t,
        false_target: f,
        span: None,
        condition_base: None,
    }
}

fn make_return() -> Terminator {
    Terminator::Return {
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        braced: false,
    }
}

fn call(span: Span, command: &str, args: &[&str], defs: &[&str]) -> Statement {
    Statement::Call {
        span,
        command: command.into(),
        canonical_command: None,
        args: args.iter().map(|arg| (*arg).into()).collect(),
        defs: defs.iter().map(|name| (*name).into()).collect(),
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

/// `(func, [entry, then, else, end])` for a diamond CFG:
/// entry → branch → then/else → end → return.
fn diamond_cfg() -> Function {
    let mut func = Function::new("::test", "entry");
    let entry = func.entry;
    let then = block(&mut func, "then");
    let els = block(&mut func, "else");
    let end = block(&mut func, "end");
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_branch("$x", then, els));
    func.blocks.get_mut(&then).unwrap().terminator = Some(make_goto(end));
    func.blocks.get_mut(&els).unwrap().terminator = Some(make_goto(end));
    func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
    func
}

/// Build a loop CFG: entry → header → branch → body → header / end
fn loop_cfg() -> Function {
    let mut func = Function::new("::test", "entry");
    let entry = func.entry;
    let header = block(&mut func, "header");
    let body = block(&mut func, "body");
    let end = block(&mut func, "end");
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(header));
    func.blocks.get_mut(&header).unwrap().terminator = Some(make_branch("$i < 10", body, end));
    func.blocks.get_mut(&body).unwrap().terminator = Some(make_goto(header));
    func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
    func
}

/// Resolve a block name to its id in `func` (test convenience).
fn id_of(func: &Function, name: &str) -> BlockId {
    func.block_id(name).expect("block name interned")
}

// Data structure tests

#[test]
fn phi_construction() {
    let phi = Phi {
        name: Symbol(0),
        version: 3,
        incoming: HashMap::from([(BlockId(1), 1), (BlockId(2), 2)]),
    };
    assert_eq!(phi.name, Symbol(0));
    assert_eq!(phi.version, 3);
    assert_eq!(phi.incoming.len(), 2);
}

#[test]
fn ssa_statement_construction() {
    let stmt = SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        uses: HashMap::new(),
        defs: HashMap::from([(Symbol(0), 1)]),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    assert_eq!(stmt.defs[&Symbol(0)], 1);
    assert_eq!(stmt.uses.len(), 0);
}

#[test]
fn ssa_block_construction() {
    let block = SsaBlock {
        name: "entry".into(),
        phis: vec![],
        statements: vec![],
        entry_versions: HashMap::new(),
        exit_versions: HashMap::from([(Symbol(0), 1)]),
    };
    assert_eq!(block.name, "entry");
    assert_eq!(block.phis, [] as [crate::ssa::Phi; 0]);
}

#[test]
fn array_relationships_use_physical_kinds_instead_of_key_text() {
    let mut elements = ArrayElems::default();
    elements.note(crate::place::scalar("literal(parenthesis)", "::", false));
    assert!(elements.elements.is_empty());
    let element = crate::place::array_elem(
        "a",
        crate::place::Index::literal("key"),
        "::N(parenthesis)",
        false,
    );
    let element_key = crate::var_resolve::canonical_binding_value_key(&element).unwrap();
    let mut root = element.clone();
    root.kind = crate::place::PlaceKind::ArrayWhole;
    root.index = None;
    let root_key = crate::var_resolve::canonical_binding_value_key(&root).unwrap();
    elements.note(element);
    assert_eq!(elements.root_for_element(&element_key), Some(&root_key));
    assert_eq!(
        expand_defs(std::slice::from_ref(&element_key), &elements),
        vec![element_key, root_key]
    );
}

#[test]
fn array_refresh_preserves_parentheses_in_its_namespace_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "namespace eval {::N(parenthesis)} {proc p {} {variable a; set a(key) OLD; set a(key) NEW; return $a(key)}}",
        registry,
        false,
    );
    let function = unit.function("::N(parenthesis)::p").unwrap();
    let root = function
        .ssa
        .cell_symbol("::N(parenthesis)::a")
        .expect("qualified physical root");
    assert!(function.ssa.is_array_root_refresh_symbol(root));
    assert_eq!(function.ssa.cell_symbol("::N"), None);
}

#[test]
fn scalar_recreation_does_not_inherit_array_refresh_classification() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {} {set a(key) OLD; unset a; set a 5; return $a}";
    let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
    let function = unit.function("::f").unwrap();
    let mut refreshes = HashSet::new();
    let mut scalar_stores = HashSet::new();
    for statement in function
        .ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
    {
        for (&symbol, &version) in &statement.defs {
            if function.ssa.is_array_root_refresh_version(symbol, version) {
                assert!(!function.ssa.is_array_root_refresh_symbol(symbol));
                if source
                    .get(statement.statement.span().start() as usize..)
                    .is_some_and(|written| written.starts_with("set a(key) OLD"))
                {
                    refreshes.insert(statement.statement.span().start());
                }
            }
            if source
                .get(statement.statement.span().start() as usize..)
                .is_some_and(|written| written.starts_with("set a 5"))
            {
                scalar_stores.insert(statement.statement.span().start());
                assert!(!function.ssa.is_array_root_refresh_version(symbol, version));
            }
        }
    }
    // The element store refreshes its aggregate root, and recreation
    // writes a scalar. Destruction may carry a separate invalidation.
    assert_eq!(refreshes.len(), 1);
    assert_eq!(scalar_stores.len(), 1);
}

#[test]
fn synthetic_array_root_refresh_is_not_dictionary_value_contents() {
    for profile in ["tcl8.6", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {} {set a(key) OLD; set a(key) NEW; return $a(key)}",
            registry,
            false,
        );
        let function = unit.function("::f").unwrap();
        let refreshes: Vec<_> = function
            .ssa
            .cell_names()
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let symbol = Symbol(u32::try_from(index).unwrap());
                function
                    .ssa
                    .is_array_root_refresh_symbol(symbol)
                    .then_some(name)
            })
            .collect();
        if profile == "tcl8.6" {
            assert_eq!(refreshes.len(), 1, "{refreshes:?}");
        } else {
            assert!(refreshes.is_empty(), "{refreshes:?}");
        }
    }
}

#[test]
fn ssa_function_construction() {
    let func = SsaFunction {
        name: "::test".into(),
        entry: BlockId(0),
        blocks: HashMap::new(),
        value_clobbers: HashMap::new(),
        idom: HashMap::new(),
        dominance_frontier: HashMap::new(),
        dominator_tree: HashMap::new(),
        block_names: vec!["entry".into()],
        var_names: Vec::new(),
        cell_names: Vec::new(),
        cell_keys: Vec::new(),
        cell_indices: Vec::new(),
        source_variables: FxHashMap::default(),
        source_may_effects: HashMap::new(),
        original_script_points: HashMap::new(),
        source_topology: HashMap::new(),
        var_to_symbol: rustc_hash::FxHashMap::default(),
        cell_to_symbol: rustc_hash::FxHashMap::default(),
        point_symbols: HashMap::new(),
        point_contexts: None,
        read_references: HashMap::new(),
        source_read_references: HashMap::new(),
        array_root_refresh_symbols: HashSet::new(),
        array_root_refresh_versions: HashSet::new(),
        unseen_call_versions: HashSet::new(),
    };
    assert_eq!(func.name, "::test");
}

#[test]
fn var_interner_assigns_first_seen_order_symbols() {
    let mut func = SsaFunction::trivial("::test", BlockId(0), vec!["entry".into()]);
    let x = func.intern_var("x");
    let y = func.intern_var("y");
    assert_eq!(x, Symbol(0));
    assert_eq!(y, Symbol(1));
    // Re-interning a known name returns the existing symbol.
    assert_eq!(func.intern_var("x"), x);
    assert_eq!(func.var_symbol("y"), Some(y));
    assert_eq!(func.var_symbol("missing"), None);
    assert_eq!(func.var_name(x), "x");
    assert_eq!(func.var_names(), ["x".to_string(), "y".to_string()]);
}

// defs_of tests

#[test]
fn defs_of_assign_const() {
    let stmt = Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    assert_eq!(defs_of(&stmt), vec!["x"]);
}

#[test]
fn defs_of_dynamic_target_name_has_no_static_def() {
    // `set $p 1` / `set ${p} 1` write the variable *named by* `$p`, not
    // `p` itself — an opaque place, so there is no static def.
    for name in ["$p", "${p}", "a$b", "[gen]"] {
        let stmt = Statement::AssignValue {
            span: Span::new(0, 10),
            name: name.into(),
            name_braced: false,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(
            defs_of(&stmt).len(),
            0,
            "dynamic target {name:?} must not be a static def"
        );
    }
    // A constant-keyed array element defs its own per-element variable
    // (the rename walk adds the base def alongside); a dynamic key
    // stays on the base.
    let arr = Statement::AssignValue {
        span: Span::new(0, 10),
        name: "arr(idx)".into(),
        name_braced: false,
        value: "1".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    assert_eq!(defs_of(&arr), vec!["arr(idx)"]);
    let dyn_key = Statement::AssignValue {
        span: Span::new(0, 10),
        name: "arr($i)".into(),
        name_braced: false,
        value: "1".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    assert_eq!(defs_of(&dyn_key), vec!["arr"]);
    // Braces suppress substitution: the key is the literal text `$i`.
    let braced = Statement::AssignValue {
        span: Span::new(0, 10),
        name: "arr($i)".into(),
        name_braced: true,
        value: "1".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    assert_eq!(defs_of(&braced), vec!["arr($i)"]);
}

#[test]
fn is_dynamic_write_target_classifies_names() {
    assert!(is_dynamic_write_target("$p", false));
    assert!(is_dynamic_write_target("${p}", false));
    assert!(is_dynamic_write_target("a$b", false));
    assert!(is_dynamic_write_target("[gen]", false));
    assert!(!is_dynamic_write_target("p", false));
    assert!(!is_dynamic_write_target("arr(idx)", false));
    assert!(!is_dynamic_write_target("ns::var", false));
}

#[test]
fn brace_quoted_write_target_is_not_dynamic() {
    // `set {$n} 1` names the literal variable `$n`; the braces suppress
    // every substitution, so nothing about the target is computed.
    // tclsh 9.0.4 / 8.6.14 (identical):
    //   set {$n} v; info exists {$n} → 1 ; info exists n → 0
    assert!(!is_dynamic_write_target("$n", true));
    assert!(!is_dynamic_write_target("${n}", true));
    assert!(!is_dynamic_write_target("[gen]", true));
    assert!(!is_dynamic_write_target("$a(k)", true));
    // TN control: the same spellings *unbraced* are still dynamic.
    assert!(is_dynamic_write_target("$n", false));
    assert!(is_dynamic_write_target("[gen]", false));
}

#[test]
fn defs_of_incr() {
    let stmt = Statement::Incr {
        span: Span::new(0, 10),
        name: "i".into(),
        name_braced: false,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    assert_eq!(defs_of(&stmt), vec!["i"]);
}

#[test]
fn defs_of_call_with_defs() {
    let stmt = Statement::Call {
        span: Span::new(0, 20),
        command: "lappend".into(),
        canonical_command: None,
        args: vec!["list".into(), "item".into()],
        defs: vec!["list".into()],
        reads: vec![],
        reads_own_defs: true,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    assert_eq!(defs_of(&stmt), vec!["list"]);
}

#[test]
fn defs_of_call_no_defs() {
    let stmt = Statement::Call {
        span: Span::new(0, 10),
        command: "puts".into(),
        canonical_command: None,
        args: vec!["hello".into()],
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    assert_eq!(defs_of(&stmt), [] as [std::string::String; 0]);
}

#[test]
fn defs_of_return() {
    let stmt = Statement::Return {
        tokens: None,
        span: Span::new(0, 10),
        value: Some("1".into()),
        value_word: None,
        expr: None,
        expr_base: None,
        command_binding: None,
        braced: false,
    };
    assert_eq!(defs_of(&stmt), [] as [std::string::String; 0]);
}

#[test]
fn defs_of_barrier_trace() {
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "trace".into(),
        command: "trace".into(),
        canonical_command: None,
        args: vec!["add".into(), "variable".into(), "$x".into()],
        tokens: None,
    };
    assert_eq!(defs_of(&stmt), vec!["x"]);
}

#[test]
fn defs_of_barrier_dict_for() {
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "dict for".into(),
        command: "dict::for".into(),
        canonical_command: None,
        args: vec!["k v".into(), "$d".into()],
        tokens: None,
    };
    assert_eq!(defs_of(&stmt), vec!["k", "v"]);
}

/// Loop-header barriers resolve via the registry's `loop_list_header`
/// flag — the ensemble-rewritten spellings work, and a user proc that
/// merely ENDS in `::for` does not have its first argument misread as
/// loop variables (the old suffix match did exactly that).
#[test]
fn defs_of_barrier_loop_header_via_registry() {
    let reg = CommandRegistry::build_default();
    let barrier = |command: &str| Statement::Barrier {
        span: Span::new(0, 30),
        reason: "loop".into(),
        command: command.into(),
        canonical_command: None,
        args: vec!["k v".into(), "$d".into()],
        tokens: None,
    };
    for cmd in ["::tcl::dict::for", "dict::for", "::tcl::dict::map"] {
        assert_eq!(
            defs_of_with_registry(&barrier(cmd), Some(&reg)),
            vec!["k", "v"],
            "{cmd} is a registry loop-list-header subcommand"
        );
    }
    assert_eq!(
        defs_of_with_registry(&barrier("::tcl::array::for"), Some(&reg)),
        vec!["k", "v"],
        "array for shares the loop-list-header shape"
    );
    assert!(
        !defs_of_with_registry(&barrier("my::for"), Some(&reg))
            .iter()
            .any(|d| d == "k" || d == "v"),
        "a user proc ending in ::for must not be misread as a loop header"
    );
}

#[test]
fn defs_of_barrier_loop_header_uses_tcl_list_grammar() {
    let reg = CommandRegistry::build_default();
    let barrier = |var_list: &str| Statement::Barrier {
        span: Span::new(0, 30),
        reason: "loop".into(),
        command: "::tcl::dict::for".into(),
        canonical_command: None,
        args: vec![var_list.into(), "$d".into()],
        tokens: None,
    };

    assert_eq!(
        defs_of_with_registry(
            &barrier(r#"{one name} "two name" three\ name plain"#),
            Some(&reg),
        ),
        vec!["one name", "two name", "three name", "plain"],
    );
    assert_eq!(
        defs_of_with_registry(&barrier("{unterminated"), Some(&reg)).len(),
        0,
        "a malformed var-list fails before defining loop variables"
    );
}

/// `tcltest::test` body indices come from the spec's arg-role resolver
/// (option-keyed `-setup`/`-body`/`-cleanup` values plus the legacy
/// positional body) via the generic `ArgRole::Body` walk, not a
/// `command == "test"` special case.
#[test]
fn structural_body_indices_tcltest_via_registry() {
    use tcl_registry::ArgRole;
    // This is a role-metadata query, not proof of a loaded source command.
    let mut reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    let unprovided = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    reg.insert_ambient_package("tcltest", "2.5.8");
    // Option form: -setup and -body values are Body roles; -result's is
    // not. Both the qualified and the exported bare spelling resolve.
    let option_form = [
        "n",
        "d",
        "-setup",
        "{set x 1}",
        "-body",
        "{incr x}",
        "-result",
        "2",
    ];
    let got = reg.arg_indices_for_role("tcltest::test", &option_form, ArgRole::Body);
    assert_eq!(got, vec![3, 5], "option-form bodies via the option model");
    assert_eq!(
        unprovided.arg_indices_for_role("tcltest::test", &option_form, ArgRole::Body),
        vec![3, 5],
        "package assistance advertises roles without proving a loaded handler"
    );
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "tcltest::test n d -body {set package_only 1}",
        &unprovided,
        false,
    );
    assert!(
        unit.top_level.ssa.blocks.values().all(|block| block
            .statements
            .iter()
            .all(|statement| statement.defs.is_empty())),
        "unprovided package role inventory cannot donate physical definitions"
    );
    // The bare `test` spelling is NOT a registry key: it resolves only
    // through the lowering's namespace-import canonicalisation (which
    // stamps `canonical_command` — threaded into the SSA lookups). A
    // user proc that merely happens to be called `test` must not pick
    // up tcltest body semantics, which the old string match caused.
    assert_eq!(
        reg.arg_indices_for_role("test", &option_form, ArgRole::Body)
            .len(),
        0
    );
    // Legacy positional form: body is the penultimate argument.
    let legacy = ["n", "d", "{incr x}", "1"];
    let got = reg.arg_indices_for_role("tcltest::test", &legacy, ArgRole::Body);
    assert_eq!(got, vec![2], "legacy positional body");
    // Option form with NO body options marks nothing (and must not fall
    // back to the positional branch: `-result` is not a body).
    let no_body = ["n", "d", "-result", "2"];
    let got = reg.arg_indices_for_role("tcltest::test", &no_body, ArgRole::Body);
    assert_eq!(got.len(), 0, "no body options → no body roles: {got:?}");
    // The generic structural walk consumes the same roles (braced-token
    // filtering applies on real token streams; `None` tokens filter all,
    // so only the empty expectation is assertable here).
    let no_body_args: Vec<String> = no_body.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        structural_body_indices("tcltest::test", &no_body_args, None, &reg).len(),
        0
    );
}

/// `trace add variable` defs route through the registry's
/// `ArgRole::VarWrite` query rather than a string match.
#[test]
fn defs_of_barrier_trace_via_registry() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "trace".into(),
        command: "trace".into(),
        canonical_command: None,
        args: vec!["add".into(), "variable".into(), "$x".into()],
        tokens: None,
    };
    assert_eq!(defs_of_with_registry(&stmt, Some(&reg)), vec!["x"]);
}

/// `trace add execution` does NOT define a variable — the
/// command name being traced is not a `VarWrite` target.
#[test]
fn defs_of_barrier_trace_add_execution_no_def() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "trace".into(),
        command: "trace".into(),
        canonical_command: None,
        args: vec!["add".into(), "execution".into(), "foo".into()],
        tokens: None,
    };
    assert_eq!(
        defs_of_with_registry(&stmt, Some(&reg)),
        [] as [std::string::String; 0]
    );
}

/// `global x y z` produces NO defs from the registry path
/// (`var_scoping` handles the per-arg list).  Without the
/// `CREATES_DYNAMIC_BARRIER` skip, the role-driven walk would
/// only mark `x` (partial defs) and miss `y` / `z`.
#[test]
fn defs_of_barrier_global_vararg_no_partial_defs() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "global".into(),
        command: "global".into(),
        canonical_command: None,
        args: vec!["x".into(), "y".into(), "z".into()],
        tokens: None,
    };
    assert_eq!(
        defs_of_with_registry(&stmt, Some(&reg)),
        [] as [std::string::String; 0]
    );
}

/// `variable a b c` — same vararg-list shape as
/// `global`, same skip.
#[test]
fn defs_of_barrier_variable_vararg_no_partial_defs() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "variable".into(),
        command: "variable".into(),
        canonical_command: None,
        args: vec!["a".into(), "b".into(), "c".into()],
        tokens: None,
    };
    assert_eq!(
        defs_of_with_registry(&stmt, Some(&reg)),
        [] as [std::string::String; 0]
    );
}

// Dominator tests

#[test]
fn dominators_linear() {
    // entry → b1 → b2 → return
    let mut func = Function::new("::test", "entry");
    let entry = func.entry;
    let b1 = block(&mut func, "b1");
    let b2 = block(&mut func, "b2");
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
    func.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
    func.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

    let dom = compute_dominators(&func);
    assert_eq!(dom[&entry], HashSet::from([entry]));
    assert_eq!(dom[&b1], HashSet::from([entry, b1]));
    assert_eq!(dom[&b2], HashSet::from([entry, b1, b2]));
}

#[test]
fn dominators_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);

    // entry dominates everything
    for id in func.blocks.keys() {
        assert!(dom[id].contains(&entry));
    }
    // then and else are not dominated by each other
    assert!(!dom[&then].contains(&els));
    assert!(!dom[&els].contains(&then));
    // end is dominated by entry but not by then or else
    assert!(dom[&end].contains(&entry));
    assert!(!dom[&end].contains(&then));
    assert!(!dom[&end].contains(&els));
}

#[test]
fn idom_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);

    assert_eq!(idom[&entry], None);
    assert_eq!(idom[&then], Some(entry));
    assert_eq!(idom[&els], Some(entry));
    assert_eq!(idom[&end], Some(entry));
}

#[test]
fn idom_loop() {
    let func = loop_cfg();
    let entry = func.entry;
    let (header, body, end) = (
        id_of(&func, "header"),
        id_of(&func, "body"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);

    assert_eq!(idom[&entry], None);
    assert_eq!(idom[&header], Some(entry));
    assert_eq!(idom[&body], Some(header));
    assert_eq!(idom[&end], Some(header));
}

#[test]
fn compute_idom_fast_matches_reference() {
    // The production CHK path (`compute_idom_fast`) must produce
    // exactly the same immediate dominators as the set-based
    // reference (`compute_idom` over `compute_dominators`) across
    // linear / diamond / loop shapes.
    let mut linear = Function::new("::test", "entry");
    let entry = linear.entry;
    let b1 = block(&mut linear, "b1");
    let b2 = block(&mut linear, "b2");
    linear.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
    linear.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
    linear.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

    for func in [linear, diamond_cfg(), loop_cfg()] {
        let reference = compute_idom(&func, &compute_dominators(&func));
        let fast = compute_idom_fast(&func);
        assert_eq!(fast, reference, "CHK idom diverged for {:?}", func.entry);
    }
}

#[test]
fn compute_idom_fast_handles_long_chain_without_blowup() {
    // Generated dispatch procedures form long diamond chains; dominator
    // computation must remain bounded on this shape.
    let mut func = Function::new("::big", "b0");
    let n = 4000;
    for i in 0..n {
        let cur = func.intern_block(format!("b{i}"));
        func.blocks
            .entry(cur)
            .or_insert_with(|| Block::new(format!("b{i}")));
        let then = block(&mut func, &format!("t{i}"));
        let next = block(&mut func, &format!("b{}", i + 1));
        func.blocks.get_mut(&then).unwrap().terminator = Some(make_return());
        func.blocks.get_mut(&cur).unwrap().terminator = Some(make_branch("c", then, next));
    }
    let bn = id_of(&func, &format!("b{n}"));
    func.blocks.get_mut(&bn).unwrap().terminator = Some(make_return());
    let idom = compute_idom_fast(&func);
    // Each chain block's idom is the previous chain block.
    assert_eq!(idom[&id_of(&func, "b1")], Some(id_of(&func, "b0")));
    assert_eq!(idom[&bn], Some(id_of(&func, &format!("b{}", n - 1))));
    assert_eq!(idom[&id_of(&func, "b0")], None);
}

#[test]
fn dominance_frontier_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);

    // then and else have "end" in their dominance frontier
    assert!(df[&then].contains(&end));
    assert!(df[&els].contains(&end));
    // entry has no dominance frontier
    assert_eq!(df[&entry].len(), 0);
}

#[test]
fn dominance_frontier_loop() {
    let func = loop_cfg();
    let entry = func.entry;
    let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);

    // body has "header" in its dominance frontier (back edge)
    assert!(df[&body].contains(&header));
    // entry strictly dominates header, so header is NOT in entry's DF
    assert_eq!(
        df[&entry].len(),
        0,
        "entry's DF should be empty; got {:?}",
        df[&entry]
    );
}

#[test]
fn dom_tree_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let tree = build_dom_tree(&idom);

    // entry's children include then, else, end (all directly dominated)
    let entry_children = &tree[&entry];
    assert!(entry_children.contains(&els));
    assert!(entry_children.contains(&end));
    assert!(entry_children.contains(&then));
}

// Phi placement tests

#[test]
fn phi_vars_diamond_with_defs() {
    // x defined in both then and else → phi needed at end
    let mut func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    func.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(10, 20),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    func.blocks
        .get_mut(&els)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(30, 40),
            name: "x".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        });

    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    let x = crate::var_resolve::canonical_literal_variable_key(
        "x",
        &ResolveContext::for_function(&func.name),
        &CommandRegistry::build_default(),
    )
    .unwrap();
    assert!(phi[&end].contains(&x), "x should need a phi at 'end'");
    assert!(
        !phi[&entry].contains(&x),
        "x should not need a phi at entry"
    );
}

#[test]
fn phi_vars_single_def_no_phi() {
    // x defined only in entry → no phi needed anywhere
    let mut func = diamond_cfg();
    let entry = func.entry;
    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });

    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    let x = crate::var_resolve::canonical_literal_variable_key(
        "x",
        &ResolveContext::for_function(&func.name),
        &CommandRegistry::build_default(),
    )
    .unwrap();
    for vars in phi.values() {
        assert!(!vars.contains(&x), "x should not need a phi anywhere");
    }
}

#[test]
fn phi_vars_loop_def() {
    // i defined in entry and body → phi at header
    let mut func = loop_cfg();
    let entry = func.entry;
    let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 10),
            name: "i".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        });
    func.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(Statement::Incr {
            span: Span::new(30, 40),
            name: "i".into(),
            name_braced: false,
            amount: None,
            amount_braced: false,
            safe_on_uninit: false,
        });

    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    let i = crate::var_resolve::canonical_literal_variable_key(
        "i",
        &ResolveContext::for_function(&func.name),
        &CommandRegistry::build_default(),
    )
    .unwrap();
    assert!(phi[&header].contains(&i), "i should need a phi at 'header'");
}

#[test]
fn phi_vars_no_defs_no_phis() {
    let func = diamond_cfg();
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    for vars in phi.values() {
        assert_eq!(vars.len(), 0, "no defs → no phis");
    }
}

// uses_of tests

fn default_registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

#[test]
fn lowered_literal_read_names_keep_empty_and_dollar_data() {
    // Public native literal-name cases: experiments/name-handling/results/frame-local-name-geometry.
    let (context, unit) = logical_metadata_fixture("info exists {}; info exists {$name}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .expect("actual retained Logical source input");
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    let reads: Vec<_> = unit
        .ir_module
        .top_level
        .statements
        .iter()
        .map(|statement| uses_of_classified_in_context(statement, &mut scanner, selection))
        .collect();
    assert!(
        reads[0].contains(&(String::new(), UseClass::Name)),
        "{reads:?}"
    );
    assert!(
        reads[1].contains(&("$name".to_owned(), UseClass::Name)),
        "{reads:?}"
    );
    assert!(
        !reads[1].iter().any(|(name, _)| name == "name"),
        "{reads:?}"
    );
}

#[test]
fn readonly_logical_roles_retain_source_reads_without_normal_or_native_ownership() {
    let (context, unit) = logical_metadata_fixture("info exists {}; info exists {$name}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    for statement in &unit.ir_module.top_level.statements {
        let tokens = statement.tokens().unwrap();
        assert!(selection.logical_structure(tokens).is_some());
        assert!(selection.normal(tokens).is_none());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .variable_context
                .interpreter
                .is_none()
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .variable_context
                .execution
                .is_none()
        );
    }
}

#[test]
fn readonly_logical_roles_keep_unknown_read_hazards_after_unclosed_commands() {
    let (context, unit) =
        logical_metadata_fixture("info exists {}; opaque_reader; info exists {$name}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let statement = unit.ir_module.top_level.statements.last().unwrap();
    assert!(
        selection
            .logical_structure(statement.tokens().unwrap())
            .is_none()
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    assert!(
        uses_of_classified_in_context(statement, &mut scanner, selection)
            .iter()
            .any(|(name, _)| name == "name")
    );
}

#[test]
fn quiet_logical_scalar_mutations_keep_original_names_without_numeric_or_normal_results() {
    let (context, unit) =
        logical_metadata_fixture("incr {}; incr {$counter}; info exists {$counter}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    for (statement, expected) in unit
        .ir_module
        .top_level
        .statements
        .iter()
        .zip(["", "$counter", "$counter"])
    {
        let tokens = unit
            .ir_module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .expect("original consumed or direct source words");
        assert!(selection.logical_structure(tokens).is_some());
        assert!(selection.normal(tokens).is_none());
        let variables = &tokens.source_binding.as_ref().unwrap().variable_context;
        assert!(variables.interpreter.is_none());
        assert!(variables.execution.is_none());
        assert!(variables.literal_value(expected, registry).is_none());
        let reads =
            uses_of_classified_with_source(statement, &mut scanner, selection, Some(tokens));
        assert!(
            reads.contains(&(expected.to_owned(), UseClass::Name)),
            "{reads:?}"
        );
        assert!(
            !reads.iter().any(|(name, _)| name == "counter"),
            "{reads:?}"
        );
    }
}

#[test]
fn dynamic_scalar_targets_keep_later_unknown_read_hazards() {
    let (context, unit) = logical_metadata_fixture("incr {}; incr $name; incr {$counter}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let statement = unit.ir_module.top_level.statements.last().unwrap();
    assert!(
        selection
            .logical_structure(statement.tokens().unwrap())
            .is_none()
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    assert!(
        uses_of_classified_in_context(statement, &mut scanner, selection)
            .iter()
            .any(|(name, _)| name == "counter")
    );
}

#[test]
fn quiet_logical_scalar_mutation_withdraws_prior_literal_contents() {
    let (context, unit) =
        logical_metadata_fixture("set counter 9; incr counter; info exists counter");
    let registry = context.commands();
    let statements = &unit.ir_module.top_level.statements;
    let before_update = &unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(&statements[1])
        .expect("original scalar update source words")
        .source_binding
        .as_ref()
        .unwrap()
        .variable_context;
    assert_eq!(before_update.literal_value("counter", registry), Some("9"));
    let after_update = &unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(&statements[2])
        .expect("original post-update source words")
        .source_binding
        .as_ref()
        .unwrap()
        .variable_context;
    assert!(after_update.literal_value("counter", registry).is_none());
    assert!(!after_update.dynamic_bindings);
    assert!(!after_update.dynamic_traces);
}

#[test]
fn empty_dictionary_scope_source_phases_keep_original_names_and_unknown_alias_contents() {
    let (context, unit) = logical_metadata_fixture(
        "set protected VALUE; dict with {} {}; dict with {$dict} {# comment only\n}; info exists {$dict}",
    );
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let script = &unit.ir_module.top_level;
    let before = script
        .retained_source_tokens_for_statement(&script.statements[1])
        .unwrap();
    assert_eq!(
        before
            .source_binding
            .as_ref()
            .unwrap()
            .variable_context
            .literal_value("protected", registry),
        Some("VALUE")
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    for (statement, expected) in script.statements[1..].iter().zip(["", "$dict", "$dict"]) {
        let tokens = script
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        assert!(selection.logical_structure(tokens).is_some());
        assert!(selection.normal(tokens).is_none());
        let reads = uses_of_classified_in_context(statement, &mut scanner, selection);
        assert!(
            reads.contains(&(expected.to_owned(), UseClass::Name)),
            "{reads:?}"
        );
        assert!(!reads.iter().any(|(name, _)| name == "dict"));
    }
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let variables = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert!(variables.literal_value("protected", registry).is_none());
    assert!(!variables.dynamic_bindings);
    assert!(!variables.dynamic_traces);
    assert!(variables.interpreter.is_none());
    assert!(variables.execution.is_none());
}

fn dictionary_effect_fixture(
    access: tcl_registry::world_effect::StaticEffectAccess,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    use std::sync::Arc;
    use tcl_registry::world_effect::{
        CallbackEffect, StaticEffectFootprint, WorldEffectDescriptor,
    };
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut info = registry.get("info").unwrap().clone();
    info.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(info);
    let mut dictionary = registry.get("dict").unwrap().clone();
    let mut subcommands = dictionary.subcommands.to_vec();
    subcommands
        .iter_mut()
        .find(|subcommand| subcommand.name == "with")
        .unwrap()
        .world_effects = Some(WorldEffectDescriptor {
        static_footprint: StaticEffectFootprint {
            accesses: Box::leak(vec![access].into_boxed_slice()),
            callback: CallbackEffect::NONE,
        },
        ..WorldEffectDescriptor::EMPTY
    });
    dictionary.subcommands = Box::leak(subcommands.into_boxed_slice());
    registry.insert(dictionary);
    let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "dict with {} {}; info exists {$name}",
        crate::compilation_unit::UnitBuildOptions {
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
    (context, unit)
}

#[test]
fn dictionary_source_phases_refuse_unrepresented_explicit_writes() {
    use tcl_registry::world_effect::{
        EffectAccessMode, StaticEffectAccess, StaticInterpreterScope, StaticNamespaceScope,
        StaticSubjectScope, WorldStateDomain,
    };
    for (domain, mode, namespace, subject) in [
        (
            WorldStateDomain::VariableTraces,
            EffectAccessMode::Write,
            StaticNamespaceScope::Current,
            StaticSubjectScope::Wildcard,
        ),
        (
            WorldStateDomain::VariableTraces,
            EffectAccessMode::ReadWrite,
            StaticNamespaceScope::Current,
            StaticSubjectScope::Wildcard,
        ),
        (
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
            StaticNamespaceScope::Named("::foreign"),
            StaticSubjectScope::Named("extra"),
        ),
        (
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
            StaticNamespaceScope::Any,
            StaticSubjectScope::Wildcard,
        ),
        (
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
            StaticNamespaceScope::Current,
            StaticSubjectScope::Wildcard,
        ),
    ] {
        let (context, unit) = dictionary_effect_fixture(StaticEffectAccess::new(
            domain,
            mode,
            StaticInterpreterScope::Current,
            namespace,
            subject,
        ));
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let selection = SsaInvocationContext::supplied(registry, Some(metadata));
        let script = &unit.ir_module.top_level;
        let before = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        assert!(
            selection.logical_structure(before).is_some(),
            "{domain:?} {namespace:?} {subject:?}"
        );
        let after = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(
            selection.logical_structure(after).is_none(),
            "{domain:?} {namespace:?} {subject:?}"
        );
        assert!(selection.normal(after).is_none());
    }
}

#[test]
fn dictionary_scope_bodies_and_variable_observers_keep_lookup_hazards() {
    for source in [
        "dict with {} {opaque_body}; info exists {$name}",
        "trace add variable protected write observer; dict with {} {}; info exists {$name}",
        "dict with $target {}; info exists {$name}",
    ] {
        let (context, unit) = logical_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let selection = SsaInvocationContext::supplied(registry, Some(metadata));
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(selection.logical_structure(tokens).is_none(), "{source}");
        assert!(selection.normal(tokens).is_none(), "{source}");
    }
}

#[test]
fn original_source_may_definitions_keep_source_identity_separate_from_locations() {
    let (_, unit) = crate::interprocedural::logical_completion_unit(
        "proc p {} {global g; set g 1}\np",
        "tcl8.6",
    );
    let function = &unit.top_level;
    let sources: Vec<_> = function.ssa.source_variable_symbols().collect();
    assert!(
        !sources.is_empty(),
        "actual retained hidden write must issue source geometry"
    );
    assert_eq!(
        function.ssa.cell_keys().len() + sources.len(),
        function.ssa.var_names().len()
    );
    for (symbol, source) in sources {
        assert!(function.ssa.cell_key(symbol).is_none());
        assert!(function.ssa.cell_name(symbol).is_none());
        assert!(function.ssa.variable_symbols().any(|held| held == symbol));
        assert!(function.ssa.cell_symbols().all(|(held, _)| held != symbol));
        assert_eq!(
            function
                .def_use
                .source_variable_symbol(&function.ssa, symbol),
            Some(source)
        );
        assert!(function.ssa.source_variable_snapshot(symbol).is_some());
        let mut reached = false;
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if let Some(&version) = statement.defs.get(&symbol) {
                    reached = true;
                    assert!(statement.may_defs.contains(&symbol));
                    assert!(function.ssa.source_may_requires_unknown_at(block, index));
                    assert_eq!(
                        function.sccp.values.get(&(symbol, version)),
                        Some(&crate::analyses::LatticeValue::Overdefined)
                    );
                    assert_eq!(
                        function.sccp.existence.get(&(symbol, version)),
                        Some(&tcl_registry::value_transfer::Existence::MayBound)
                    );
                    let mut stale = function.ssa.clone();
                    let statement =
                        &mut stale.blocks.get_mut(&block).unwrap().statements[index].statement;
                    let Statement::Call { command, .. } = statement else {
                        panic!("actual original call");
                    };
                    command.push_str("-report-changed");
                    assert!(stale.source_variable_snapshot(symbol).is_none());
                    assert!(
                        function
                            .def_use
                            .source_variable_symbol(&stale, symbol)
                            .is_none()
                    );
                    assert!(stale.source_may_requires_unknown_at(block, index));
                }
            }
        }
        assert!(
            reached,
            "source identity must retain its actual definition point/version"
        );
    }
}

#[test]
fn original_source_may_clobber_receipts_refuse_removed_or_changed_maps() {
    let (_, unit) = crate::interprocedural::logical_completion_unit(
        "proc p {} {global g; set g 1}\nset local before\np\nreturn [unknown_read]",
        "tcl8.6",
    );
    let ssa = &unit.top_level.ssa;
    let mut reached = false;
    for (&(block, index), effect) in &ssa.source_may_effects {
        if effect
            .original
            .as_ref()
            .is_none_or(|original| original.writes().is_empty())
        {
            continue;
        }
        reached = true;
        assert!(
            ssa.source_may_clobber_is_current_at(block, index),
            "actual live location must have a producer-issued marker/clobber relation"
        );
        let held = effect.clobber.as_ref().unwrap();
        assert!(!held.versions.is_empty());
        assert!(is_unseen_call_marker(
            &ssa.blocks[&block].statements[held.marker_index].statement
        ));
        let mut stale = ssa.clone();
        stale
            .value_clobbers
            .get_mut(&block)
            .unwrap()
            .remove(&held.marker_index);
        assert!(is_unseen_call_marker(
            &stale.blocks[&block].statements[held.marker_index].statement
        ));
        assert!(!stale.source_may_clobber_is_current_at(block, index));
        assert!(stale.source_may_requires_unknown_at(block, index));
        let mut stale = ssa.clone();
        stale
            .value_clobbers
            .get_mut(&block)
            .unwrap()
            .get_mut(&held.marker_index)
            .unwrap()
            .clear();
        assert!(
            !stale.source_may_clobber_is_current_at(block, index),
            "empty clobber inventory is not closed coverage"
        );
        let mut stale = ssa.clone();
        let versions = stale
            .value_clobbers
            .get_mut(&block)
            .unwrap()
            .get_mut(&held.marker_index)
            .unwrap();
        let (&symbol, &pair) = versions.iter().next().unwrap();
        versions.insert(symbol, (pair.0, pair.1 + 1));
        assert!(!stale.source_may_clobber_is_current_at(block, index));
    }
    assert!(
        reached,
        "genuine call source-MAY producer must be exercised"
    );
}

fn assert_source_may_identity(
    function: &crate::compilation_unit::FunctionUnit,
    symbol: Symbol,
    variable: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
) {
    assert!(function.ssa.cell_key(symbol).is_none());
    assert_eq!(
        function
            .def_use
            .source_variable_symbol(&function.ssa, symbol),
        Some(variable)
    );
}

fn source_may_cfg_has_cycle(function: &crate::compilation_unit::FunctionUnit) -> bool {
    let mut cyclic = false;
    for &start in function.cfg.blocks.keys() {
        let mut pending = function.cfg.block_successors(start);
        let mut seen = HashSet::new();
        while let Some(next) = pending.pop() {
            if next == start {
                cyclic = true;
                break;
            }
            if seen.insert(next) {
                pending.extend(function.cfg.block_successors(next));
            }
        }
    }
    cyclic
}

fn assert_source_may_stale_graphs(
    function: &crate::compilation_unit::FunctionUnit,
    symbol: Symbol,
    phis: &HashMap<Version, (BlockId, &Phi)>,
) {
    let (&version, &(block, _)) = phis.iter().next().unwrap();
    let mut stale = function.ssa.clone();
    let phi = stale
        .blocks
        .get_mut(&block)
        .unwrap()
        .phis
        .iter_mut()
        .find(|phi| phi.name == symbol && phi.version == version)
        .unwrap();
    phi.incoming.clear();
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "same-source changed phi graph cannot reuse old chains"
    );
    let mut stale = function.ssa.clone();
    let (_, body) = stale
        .blocks
        .iter_mut()
        .find(|(_, body)| {
            body.statements
                .iter()
                .any(|statement| statement.uses.contains_key(&symbol))
        })
        .unwrap();
    let statement = body
        .statements
        .iter_mut()
        .find(|statement| statement.uses.contains_key(&symbol))
        .unwrap();
    statement.uses.remove(&symbol);
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "same-source changed source use cannot reuse old chains"
    );
    let mut stale = function.ssa.clone();
    stale.source_topology.get_mut(&block).unwrap().terminator = None;
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "changed retained source topology cannot reuse old chains"
    );
    let mut stale = function.ssa.clone();
    stale.idom.clear();
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "changed SSA topology cannot reuse old chains"
    );
}

fn assert_original_source_may_joins(source: &str, minimum_calls: usize, has_loop: bool) {
    let (_, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
    let function = &unit.top_level;
    let sources: Vec<_> = function.ssa.source_variable_symbols().collect();
    assert!(
        !sources.is_empty(),
        "actual hidden writes must issue source identities"
    );
    let cyclic = source_may_cfg_has_cycle(function);
    assert_eq!(
        cyclic, has_loop,
        "control must retain its actual CFG branch/backedge geometry"
    );
    for (symbol, variable) in sources {
        assert_source_may_identity(function, symbol, variable);
        let definitions: HashSet<_> = function
            .ssa
            .blocks
            .values()
            .flat_map(|body| {
                body.statements
                    .iter()
                    .filter_map(|statement| statement.defs.get(&symbol).copied())
            })
            .collect();
        assert!(
            definitions.len() >= minimum_calls,
            "each genuine call point must retain its own source MAY version"
        );
        let mut phis = HashMap::new();
        for (&block, body) in &function.ssa.blocks {
            for phi in body.phis.iter().filter(|phi| phi.name == symbol) {
                assert!(!phi.incoming.is_empty());
                for (&predecessor, &version) in &phi.incoming {
                    assert!(function.cfg.block_successors(predecessor).contains(&block));
                    assert!(
                        function
                            .def_use
                            .uses_of(symbol, version)
                            .iter()
                            .any(|use_site| {
                                use_site.kind == crate::def_use::UseKind::PhiIncoming
                                    && use_site.phi_version == phi.version
                                    && use_site.class == UseClass::Quoted
                            })
                    );
                }
                phis.insert(phi.version, (block, phi));
            }
        }
        assert!(
            !phis.is_empty(),
            "source definitions must use the existing SSA join algorithm"
        );
        let mut observed = false;
        for (&block, body) in &function.ssa.blocks {
            if !matches!(
                &function.cfg.blocks[&block].terminator,
                Some(cfg::Terminator::Return { .. })
            ) {
                continue;
            }
            let Some(&exit) = body.exit_versions.get(&symbol) else {
                continue;
            };
            if exit == 0 {
                continue;
            }
            let mut pending = vec![exit];
            let mut reaching = HashSet::new();
            while let Some(version) = pending.pop() {
                if !reaching.insert(version) {
                    continue;
                }
                if let Some((_, phi)) = phis.get(&version) {
                    pending.extend(phi.incoming.values().copied());
                }
            }
            if !definitions.is_subset(&reaching) {
                continue;
            }
            observed = true;
            assert!(
                function
                    .def_use
                    .uses_of(symbol, exit)
                    .iter()
                    .any(|use_site| {
                        use_site.kind == crate::def_use::UseKind::Terminator
                            && use_site.block == body.name
                            && use_site.class == UseClass::Quoted
                    })
            );
        }
        assert!(
            observed,
            "the actual post-join observer must retain every reaching source definition"
        );
        assert_source_may_stale_graphs(function, symbol, &phis);
    }
}

#[test]
fn original_source_may_branch_versions_reach_post_join_observers() {
    assert_original_source_may_joins(
        "proc p {} {global g; set g 1}\nif {$condition} {p} else {p}\nreturn [unknown_read]",
        2,
        false,
    );
}

#[test]
fn original_source_may_loop_versions_reach_post_join_observers() {
    assert_original_source_may_joins(
        "proc p {} {global g; set g 1}\nwhile {$condition} {p}\nreturn [unknown_read]",
        1,
        true,
    );
}

#[test]
fn conditional_logical_store_symbols_keep_original_empty_and_dollar_names() {
    let (context, unit) = logical_metadata_fixture("set {$literal} 9; set {} 8");
    let registry = context.commands();
    let function = &unit.top_level;
    let points = function.ssa.point_contexts.as_ref().unwrap();
    let mut found = HashSet::new();
    for (&block, data) in &function.cfg.blocks {
        for (index, _) in data.statements.iter().enumerate() {
            let Some(store) = points.conditional_logical_scalar_store_at(block, index, registry)
            else {
                continue;
            };
            let place = store.definition_place(registry).unwrap();
            let key = crate::var_resolve::canonical_binding_value_key(&place).unwrap();
            let source = function.ssa.source_symbols_at(block, index).unwrap();
            let symbol = source
                .get(store.source_name())
                .expect("original conditional store name");
            assert_eq!(function.ssa.cell_key(*symbol), Some(&key));
            assert!(place.cell.as_ref().unwrap().interpreter.is_none());
            assert!(place.cell.as_ref().unwrap().execution.is_none());
            assert!(!source.contains_key("literal"));
            let tokens = points.source_tokens_at(block, index).unwrap();
            let metadata = function
                .invocation_metadata_context_for_module(registry, &unit.ir_module)
                .unwrap();
            assert!(
                SsaInvocationContext::supplied(registry, Some(metadata))
                    .normal(tokens)
                    .is_none()
            );
            found.insert(store.source_name().to_owned());
        }
    }
    assert_eq!(found, HashSet::from(["$literal".to_owned(), String::new()]));
}

#[test]
fn conditional_store_symbol_owner_refuses_after_dynamic_target_and_unknown_command() {
    let (context, unit) =
        logical_metadata_fixture("set {$literal} 9; set $target 8; opaque_writer; set {$later} 7");
    let registry = context.commands();
    let function = &unit.top_level;
    let points = function.ssa.point_contexts.as_ref().unwrap();
    let mut checked = 0;
    for (&block, data) in &function.cfg.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let Some(tokens) = statement.tokens() else {
                continue;
            };
            if tokens
                .argv_texts
                .get(1)
                .is_some_and(|name| name == "$later")
            {
                checked += 1;
                assert!(
                    points
                        .conditional_logical_scalar_store_at(block, index, registry)
                        .is_none()
                );
                assert!(
                    !function
                        .ssa
                        .source_symbols_at(block, index)
                        .unwrap()
                        .contains_key("$later")
                );
            }
        }
    }
    assert_eq!(checked, 1);
}

#[test]
fn original_literal_empty_reads_and_mutations_keep_name_data() {
    // Native cases: experiments/name-handling/results/ssa-empty-read-write-names.
    let source = "info exists {}; info exists {$name}; incr {}; incr {$counter}; dict with {} {}; dict with {$dict} {}";
    let (context, unit) = logical_metadata_fixture(source);
    let registry = context.commands();
    let function = &unit.top_level;
    let metadata = function
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .expect("actual retained Logical source input");
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let mut scanner =
        VarReferenceScanner::with_config(VarScanOptions::default(), function.source_lexer_config());
    let statements = &unit.ir_module.top_level.statements;
    assert_eq!(statements.len(), 6);
    for (statement, expected) in statements
        .iter()
        .zip(["", "$name", "", "$counter", "", "$dict"])
    {
        let reads = uses_of_classified_in_context(statement, &mut scanner, selection);
        assert!(
            reads.contains(&(expected.to_owned(), UseClass::Name)),
            "{expected:?}: {reads:?}"
        );
        if let Some(unprefixed) = expected.strip_prefix('$') {
            assert!(!reads.iter().any(|(name, _)| name == unprefixed));
        }
    }
}

#[test]
fn original_loop_name_lists_keep_empty_and_dollar_elements() {
    // Native foreach cases: experiments/name-handling/results/ssa-empty-read-write-names.
    let source = "foreach {{} {$name}} $values {}";
    let (context, unit) = logical_metadata_fixture(source);
    let registry = context.commands();
    let function = &unit.top_level;
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .expect("actual original loop words");
    let metadata = function
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .expect("actual retained Logical source input");
    let selected = selected_barrier_defs(
        SsaInvocationContext::supplied(registry, Some(metadata)),
        tokens,
    );
    assert!(selected.contains(&String::new()), "{selected:?}");
    assert!(selected.contains(&"$name".to_owned()), "{selected:?}");
    assert!(!selected.contains(&"name".to_owned()), "{selected:?}");
    assert_eq!(
        selected_barrier_defs(SsaInvocationContext::supplied(registry, None), tokens),
        [] as [std::string::String; 0]
    );
}

#[test]
fn uses_of_assign_const() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert_eq!(uses.len(), 0, "constant assignment reads nothing");
}

#[test]
fn uses_of_assign_value_with_var() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: false,
        element_qualified: false,
    });
    let stmt = Statement::AssignValue {
        span: Span::new(0, 15),
        name: "y".into(),
        name_braced: false,
        value: "$x".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert!(uses.contains(&"x".to_string()), "should read $x");
    assert!(
        !uses.contains(&"y".to_string()),
        "should not read $y (it's defined)"
    );
}

#[test]
fn uses_of_incr() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::Incr {
        span: Span::new(0, 10),
        name: "i".into(),
        name_braced: false,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    // incr reads and writes — reads_own_def
    assert!(uses.contains(&"i".to_string()), "incr reads the variable");
}

#[test]
fn uses_of_return_with_value() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::Return {
        tokens: None,
        span: Span::new(0, 15),
        value: Some("$result".into()),
        value_word: None,
        expr: None,
        expr_base: None,
        command_binding: None,
        braced: false,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert!(uses.contains(&"result".to_string()));
}

#[test]
fn uses_of_expr_eval() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::ExprEval {
        span: Span::new(0, 20),
        command_binding: tcl_runtime_api::CommandBindingIdentity::new("expr", "expr"),
        expr: ExprNode::Binary {
            op: crate::expr_ast::BinOp::Add,
            left: Box::new(ExprNode::Var {
                text: "$a".into(),
                name: "a".into(),
                start: 0,
                end: 2,
            }),
            right: Box::new(ExprNode::Var {
                text: "$b".into(),
                name: "b".into(),
                start: 5,
                end: 7,
            }),
        },
        expr_base: None,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert!(uses.contains(&"a".to_string()));
    assert!(uses.contains(&"b".to_string()));
}

// UseClass classification

/// A `Call` whose argument words are `args`, with per-word token kinds
/// derived from the word text: `{…}` lexes to `Str` (brace-quoted, the one
/// form Tcl leaves wholly unsubstituted), everything else to `Esc`.
fn call_with_words(command: &str, args: &[&str]) -> Statement {
    let source = std::iter::once(command)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let segment = crate::segmenter::segment_commands(&source).remove(0);
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(&source),
        tcl_lexer::LexerConfig::default(),
        &segment,
    );
    Statement::Call {
        span: segment.span,
        command: command.to_owned(),
        canonical_command: None,
        args: segment.texts.into_iter().skip(1).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    }
}

fn classify(stmt: &Statement, reg: &CommandRegistry, name: &str) -> Option<UseClass> {
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    uses_of_classified(stmt, &mut scanner, reg)
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, class)| class)
}

/// A braced word at a role the callee does **not** evaluate in this frame
/// is a `Quoted` use: still recorded (liveness must assume it may be
/// evaluated later) but not a read here.
/// tclsh-proof: tclsh8.6.14 — `puts {$y}` prints `$y` with `y` undefined.
#[test]
fn uses_of_classified_braced_data_word_is_quoted() {
    let reg = default_registry()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let stmt = call_with_words("puts", &["{$y}"]);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// The same name in an unbraced word is a definite read.
#[test]
fn uses_of_classified_unbraced_word_is_substituted() {
    let reg = default_registry();
    let stmt = call_with_words("puts", &["$y"]);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A braced `Expr`-role word really does substitute — `expr` evaluates it
/// against the caller's variables — so it stays `Substituted`.
#[test]
fn uses_of_classified_braced_expr_word_is_substituted() {
    let reg = default_registry();
    let stmt = call_with_words("expr", &["{$a + $b}"]);
    assert_eq!(classify(&stmt, &reg, "a"), Some(UseClass::Substituted));
    assert_eq!(classify(&stmt, &reg, "b"), Some(UseClass::Substituted));
}

/// A command the registry does not describe carries no role information,
/// so its braced word is *unclassified*: it may be a script that runs in
/// this frame. A name the word sets itself is that script's own local —
/// the un-hooked definer shape — so it is `Quoted`.
#[test]
fn uses_of_classified_unknown_definer_body_local_is_quoted() {
    let reg = default_registry();
    let stmt = call_with_words(
        "mydefiner",
        &["::foo::bar", "{optlist}", "{set y 1; return $y}"],
    );
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// TN control — a name the unclassified word does **not** set stays a
/// read: a wrapper handing the word to an `uplevel`-ing worker really does
/// evaluate it in this frame, and tclsh errors on the unset name.
#[test]
fn uses_of_classified_unknown_command_free_read_stays_substituted() {
    let reg = default_registry();
    let stmt = call_with_words("wrapper", &["myf", "{puts $myf}"]);
    assert_eq!(classify(&stmt, &reg, "myf"), Some(UseClass::Substituted));
}

#[test]
fn source_ownership_classification_uses_the_retained_lexical_config() {
    // naming.variable.ssa-retained-lexical-config
    // docs/design/analysis/name-resolution-proofs/variable-ssa-retained-lexical-config.md
    let registry = default_registry();
    let tcl = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let jim = tcl_lexer::LexerConfig {
        base_offset: 87,
        base_line: 4,
        base_col: 12,
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_dialect("jim")
    };
    // These unchanged, brace-balanced source words exercise command
    // separators and list parsing. They are lexical classification controls,
    // not a proof that the unknown wrapper executes either body.
    for (body, tcl_class, jim_class) in [
        (
            "{set\u{000b}x 1; puts $x}",
            UseClass::Quoted,
            UseClass::Substituted,
        ),
        (
            "{foreach \"{x}y\" {1 2} {}; puts $x}",
            UseClass::Substituted,
            UseClass::Quoted,
        ),
    ] {
        let statement = call_with_words("wrapper", &[body]);
        for (config, expected) in [(tcl, tcl_class), (jim, jim_class)] {
            let mut scanner = VarReferenceScanner::with_config(VarScanOptions::default(), config);
            let class = uses_of_classified(&statement, &mut scanner, &registry)
                .into_iter()
                .find(|(name, _)| name == "x")
                .map(|(_, class)| class);
            assert_eq!(class, Some(expected), "{body:?}, {config:?}");
            assert_eq!(scanner.lexer_config(), config.nested().normalized());
        }
    }
}

#[test]
fn nested_source_expression_roles_use_the_retained_lexical_config() {
    // naming.variable.ssa-retained-lexical-config
    // docs/design/analysis/name-resolution-proofs/variable-ssa-retained-lexical-config.md
    let registry = default_registry();
    let statement = call_with_words("puts", &["[expr\u{000b} {$nested}]"]);
    for (config, expected) in [
        (tcl_lexer::LexerConfig::for_dialect("tcl9.0"), true),
        (tcl_lexer::LexerConfig::for_dialect("jim"), false),
    ] {
        // Jim's actual head is `expr` followed by a vertical-tab byte.
        // Registry source roles cannot replace that head with `expr`.
        let mut scanner = VarReferenceScanner::with_config(VarScanOptions::default(), config);
        let names = uses_of_classified(&statement, &mut scanner, &registry);
        assert_eq!(
            names
                .iter()
                .any(|(name, class)| name == "nested" && *class == UseClass::Substituted),
            expected,
            "{config:?}: {names:?}",
        );
    }
}

/// A name reached both ways in one statement is a definite read.
#[test]
fn uses_of_classified_substituted_wins_over_quoted() {
    let reg = default_registry();
    let stmt = call_with_words("puts", &["{$y}", "$y"]);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A braced `return` value is literal.
/// tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]` prints
/// `$y` with `y` undefined.
#[test]
fn uses_of_classified_braced_return_value_is_quoted() {
    let reg = default_registry();
    let stmt = Statement::Return {
        tokens: None,
        span: Span::new(0, 15),
        value: Some("$y".into()),
        value_word: None,
        expr: None,
        expr_base: None,
        command_binding: None,
        braced: true,
    };
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// Build a one-arm opaque `switch` around `body`, with `pattern` as the
/// arm's pattern and the canonical braced arm block.
fn opaque_switch(subject: &str, pattern: &str, body: crate::ir::Script) -> Statement {
    Statement::Switch {
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        command: "switch".into(),
        span: Span::new(0, 40),
        subject: subject.into(),
        subject_span: Span::new(0, 1),
        arms: vec![crate::ir::SwitchArm {
            pattern: pattern.into(),
            pattern_braced: true,
            pattern_span: Span::new(0, 1),
            body: Some(body),
            body_span: Some(Span::new(0, 1)),
            fallthrough: false,
        }],
        default_body: None,
        default_span: None,
        mode: crate::ir::SwitchMode::Glob,
        nocase: false,
        raw_args: Vec::new(),
        patterns_braced: true,
    }
}

/// An arm body is the one script that reaches SSA
/// un-lowered, and its reads must arrive classified exactly as the same
/// word would be outside the arm. A braced data word is `Quoted`.
/// tclsh-proof: tclsh 9.0.4 — `proc f {z} { switch -glob $z { a* { puts
/// {$b} } } }; f abc` prints `$b` with `b` undefined.
#[test]
fn uses_of_classified_opaque_switch_arm_braced_data_word_is_quoted() {
    let reg = default_registry()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut body = crate::ir::Script::new();
    body.statements.push(call_with_words("puts", &["{$y}"]));
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    // The subject is a genuine read of this frame.
    assert_eq!(classify(&stmt, &reg, "z"), Some(UseClass::Substituted));
}

/// TP control — the substituting spelling of the same arm word stays a
/// definite read, so the classification is threaded, not dropped.
#[test]
fn uses_of_classified_opaque_switch_arm_unbraced_word_is_substituted() {
    let reg = default_registry();
    let mut body = crate::ir::Script::new();
    body.statements.push(call_with_words("puts", &["$y"]));
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A `Statement::Foreach` inside an opaque arm is walked by
/// `reads_in_stmt` rather than lowered, so `ForeachIterator::list_braced`
/// is the extra fact that walk needs: a braced value word is
/// literal list text, recorded `Quoted` so liveness still honours it.
#[test]
fn uses_of_classified_opaque_switch_arm_braced_foreach_list_is_quoted() {
    let reg = default_registry();
    let mut body = crate::ir::Script::new();
    body.statements.push(Statement::Foreach {
        span: Span::new(0, 20),
        iterators: vec![crate::ir::ForeachIterator {
            vars: vec!["n".into()],
            list_arg: "a $y c".into(),
            list_braced: true,
        }],
        body: crate::ir::Script::new(),
        body_span: Span::new(0, 1),
        is_lmap: false,
        raw_args: Vec::new(),
        is_dict_iteration: false,
        is_array_iteration: false,
        raw_tokens: None,
    });
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// TP control for the loop value word — the substituting spelling is a
/// definite read even inside an opaque arm.
#[test]
fn uses_of_classified_opaque_switch_arm_substituted_foreach_list_is_substituted() {
    let reg = default_registry();
    let mut body = crate::ir::Script::new();
    body.statements.push(Statement::Foreach {
        span: Span::new(0, 20),
        iterators: vec![crate::ir::ForeachIterator {
            vars: vec!["n".into()],
            list_arg: "a $y c".into(),
            list_braced: false,
        }],
        body: crate::ir::Script::new(),
        body_span: Span::new(0, 1),
        is_lmap: false,
        raw_args: Vec::new(),
        is_dict_iteration: false,
        is_array_iteration: false,
        raw_tokens: None,
    });
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A pattern from the canonical braced arm block is a
/// literal list element; supplied as separate words it substitutes.
#[test]
fn uses_of_classified_switch_pattern_class_follows_patterns_braced() {
    let reg = default_registry();
    let braced = opaque_switch("$z", "$p*", crate::ir::Script::new());
    assert_eq!(classify(&braced, &reg, "p"), Some(UseClass::Quoted));
    let mut worded = opaque_switch("$z", "$p*", crate::ir::Script::new());
    if let Statement::Switch {
        patterns_braced, ..
    } = &mut worded
    {
        *patterns_braced = false;
    }
    assert_eq!(classify(&worded, &reg, "p"), Some(UseClass::Substituted));
}

// build_ssa tests

#[test]
fn build_ssa_linear() {
    let reg = default_registry();
    // entry: set x 1; set y $x; return
    let mut func = Function::new("::test", "entry");
    let entry = func.entry;
    func.blocks.get_mut(&entry).unwrap().statements = vec![
        Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        Statement::AssignValue {
            span: Span::new(8, 16),
            name: "y".into(),
            name_braced: false,
            value: "$x".into(),
            value_needs_backsubst: false,
            tokens: None,
        },
    ];
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

    let ssa = build_ssa(&func, &reg);
    assert_eq!(ssa.name, "::test");
    assert_eq!(ssa.entry, entry);

    let sx = ssa.var_symbol("x").expect("x interned");
    let sy = ssa.var_symbol("y").expect("y interned");
    let entry_blk = &ssa.blocks[&entry];
    // First statement (set x 1): defs x=1
    assert_eq!(entry_blk.statements[0].defs.get(&sx), Some(&1));
    // Second statement (set y $x): uses x=1, defs y=1
    assert_eq!(entry_blk.statements[1].uses.get(&sx), Some(&1));
    assert_eq!(entry_blk.statements[1].defs.get(&sy), Some(&1));
}

fn logical_option_metadata_fixture(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
        tcl_registry::hover::OptionSpec {
            name: "-global",
            value: tcl_registry::hover::OptionValue::global_var_name(),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-local",
            value: tcl_registry::hover::OptionValue::var_name(),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-new",
            value: tcl_registry::hover::OptionValue::global_var_name(),
            surface: Some(tcl_dialect::model::SpecSurface::TCL90_PLUS),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
    ];
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    registry.insert(tcl_registry::CommandSpec {
        name: "scope-owner",
        options: OPTIONS,
        arity: tcl_registry::Arity::any(),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
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
    (context, unit)
}

#[test]
fn selected_option_writes_keep_source_schema_scope_and_alias_ordinals() {
    use tcl_registry::VariableScope::{CurrentFrame, Global};
    for (source, expected) in [
        (
            "scope-owner -g value -local local",
            vec![(1, 1, Global), (3, 3, CurrentFrame)],
        ),
        (
            "interp alias {} scoped {} scope-owner -global; scoped value",
            vec![(1, 0, Global)],
        ),
        (
            "interp alias {} fixed {} scope-owner -global captured; fixed",
            vec![],
        ),
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let writes = original_command_option_writes(tokens, metadata, registry)
            .expect("authentic selected original source options");
        assert_eq!(
            writes
                .iter()
                .map(|write| (
                    write.effective_argument,
                    write.written_argument,
                    write.scope
                ))
                .collect::<Vec<_>>(),
            expected,
            "{source}"
        );
        assert!(
            SsaInvocationContext::supplied(registry, Some(metadata))
                .normal(tokens)
                .is_none(),
            "readonly option metadata cannot establish a Native handler"
        );
        let mut changed = tokens.clone();
        changed.argv_texts[0] = "scope-owner".into();
        changed.argv_texts.push("foreign".into());
        assert!(original_command_option_writes(&changed, metadata, registry).is_none());
    }
    for source in [
        "scope-owner $option value",
        "scope-owner {*}$options",
        "scope-owner -new value",
        "proc scope-owner args {}; scope-owner -global value",
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        assert!(
            original_command_option_writes(tokens, metadata, context.commands()).is_none(),
            "unknown/unavailable/replaced option selection: {source}"
        );
    }
}

#[test]
fn selected_taint_option_writes_require_taint_specific_registry_facts() {
    for (source, expected) in [
        ("entry .e -textvariable value", vec![(2, 2)]),
        ("scope-owner -global value", vec![]),
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let writes = original_command_taint_option_writes(tokens, metadata, registry)
            .expect("complete selected source command schema");
        assert_eq!(
            writes
                .iter()
                .map(|write| (write.effective_argument, write.written_argument))
                .collect::<Vec<_>>(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn selected_instance_option_writes_keep_genuine_factory_and_written_receiver() {
    for (source, expected) in [
        ("entry .e; .e configure -textvariable value", vec![(2, 2)]),
        (
            "entry .e; interp alias {} configureEntry {} .e configure -textvariable; configureEntry value",
            vec![(2, 0)],
        ),
        (
            "entry .e; interp alias {} fixed {} .e configure -textvariable captured; fixed",
            vec![],
        ),
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let writes = original_instance_option_writes(tokens, metadata, registry)
            .expect("authentic original selected Registry instance descriptor");
        assert_eq!(
            writes
                .iter()
                .map(|write| (write.effective_argument, write.written_argument))
                .collect::<Vec<_>>(),
            expected,
            "{source}"
        );
        assert!(
            writes
                .iter()
                .all(|write| write.scope == tcl_registry::VariableScope::Global)
        );
        assert!(
            SsaInvocationContext::supplied(registry, Some(metadata))
                .normal(tokens)
                .is_none()
        );
        assert!(
            unit.top_level
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .all(|statement| statement.defs.is_empty()),
            "source option schema supplies no physical store: {source}"
        );
        let mut detached = tokens.clone();
        detached.source_binding = None;
        assert!(original_instance_option_writes(&detached, metadata, registry).is_none());
    }
    for source in [
        "entry .e; proc .e args {}; .e configure -textvariable value",
        "label .e; .e configure -textvariable value",
        "set w [entry .e]; unknown; $w configure -textvariable value",
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        assert!(
            original_instance_option_writes(tokens, metadata, context.commands()).is_none(),
            "{source}"
        );
    }
    let (context, unit) = original_operand_metadata_fixture();
    let (block, index, _) = original_setter_definition_point(&unit.top_level);
    let tokens = SsaSourceView::at_statement(&unit.top_level.ssa, block, index)
        .source_tokens()
        .unwrap();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
        .unwrap();
    assert!(
        SsaInvocationContext::supplied(context.commands(), Some(metadata))
            .normal(tokens)
            .is_some(),
        "the independent actual Native setter/store eligibility remains available"
    );
}

#[test]
fn linked_option_original_values_preserve_literal_dollars_and_element_roots() {
    let (context, unit) = logical_metadata_fixture(
        "info exists {$literal}; info exists {items(key)}; info exists {(key)}; info exists {}",
    );
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    for (statement, expected) in unit.ir_module.top_level.statements.iter().zip([
        Some("::$literal"),
        Some("::items"),
        Some("::"),
        None,
    ]) {
        let tokens = statement.tokens().unwrap();
        assert_eq!(
            linked_option_definition_name(tokens, 1, true, metadata, registry).as_deref(),
            expected,
        );
        assert!(original_global_root_frame(tokens, metadata, registry).is_none());
        let mut changed = tokens.clone();
        changed.argv_texts[2] = "different_name".to_owned();
        assert!(linked_option_definition_name(&changed, 1, true, metadata, registry).is_none());
        let mut detached = tokens.clone();
        detached.source_binding = None;
        assert!(linked_option_definition_name(&detached, 1, true, metadata, registry).is_none());
    }
}

#[test]
fn logical_linked_option_unicode_nul_retains_complete_qualified_name_data() {
    for (source, expected) in [
        (
            "info exists \"original_scope::literal\\u0000tail\"",
            "::original_scope::literal\0tail",
        ),
        (
            "info exists \"original_scope::items\\u0000tail(key)\"",
            "::original_scope::items\0tail",
        ),
    ] {
        let (context, unit) = logical_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit.ir_module.top_level.statements[0].tokens().unwrap();
        assert_eq!(
            linked_option_definition_name(tokens, 1, true, metadata, registry).as_deref(),
            Some(expected)
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        let input = binding.original_written_name_input(tokens, 2).unwrap();
        let key = input.original_word_key().unwrap();
        assert!(key.bytes().contains(&0));
        assert_ne!(
            key.policy()
                .recipe()
                .combined_variable_input(key.bytes())
                .root()
                .selected(),
            key.bytes()
        );
        assert!(
            SsaInvocationContext::supplied(registry, Some(metadata))
                .normal(tokens)
                .is_none()
        );
    }
}

#[test]
fn top_label_and_option_definition_cannot_supply_global_frame_ownership() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let mut function = Function::new("::top", "entry");
    let block = function.blocks.get_mut(&function.entry).unwrap();
    block.statements.push(call(
        Span::new(0, 34),
        "entry",
        &[".e", "-textvariable", "value"],
        &["::value"],
    ));
    assert!(!mirror_top_level_global_option_defs(
        &mut function,
        registry
    ));
    let Statement::Call { defs, .. } = &function.blocks[&function.entry].statements[0] else {
        panic!("original option statement");
    };
    assert_eq!(defs, &["::value".to_owned()]);
}

#[test]
fn catalogue_widget_option_roles_do_not_donate_physical_store_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let arguments = [".e", "-textvariable", "value"];
    let option = registry
        .get("entry")
        .unwrap()
        .options
        .iter()
        .find(|option| option.name == "-textvariable")
        .unwrap();
    assert!(
        matches!(option.value, tcl_registry::hover::OptionValue::Takes(argument)
            if argument.role == tcl_registry::ArgRole::VarWrite)
    );
    assert_eq!(
        registry.option_variable_scope("entry", &arguments, 2, registry.own_surface_query()),
        Some(tcl_registry::VariableScope::Global)
    );
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "entry .e -textvariable value",
        registry,
        false,
    );
    assert!(
        unit.top_level
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
}

#[test]
fn catalogue_instance_options_do_not_manufacture_live_widget_handlers() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let invocation = registry
        .resolve_instance_invocation(
            "entry",
            ".e",
            &["configure", "-textvariable", "value"],
            registry.own_surface_query(),
        )
        .expect("catalogue instance assistance");
    assert!(
        invocation
            .semantics
            .traits
            .contains(tcl_registry::Traits::CONFIGURES_INSTANCE_OPTIONS)
    );
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "entry .e; .e configure -textvariable value",
        registry,
        false,
    );
    assert!(
        unit.top_level
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
}

#[test]
fn build_ssa_top_level_ordinary_global_set_has_no_bare_alias() {
    let reg = default_registry();
    let mut func = Function::new("::top", "entry");
    let entry = func.entry;
    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 13),
            name: "::value".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

    let ssa = build_ssa(&func, &reg);
    let absolute = ssa.var_symbol("::value").expect("global spelling interned");
    assert_eq!(
        ssa.blocks[&entry].statements[0].defs.get(&absolute),
        Some(&1)
    );
    assert_eq!(ssa.var_symbol("value"), None);
}

#[test]
fn qualified_catalogue_option_keeps_scope_assistance_without_a_store() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let arguments = [".e", "-textvariable", "::ns::value"];
    assert_eq!(
        registry.option_variable_scope("entry", &arguments, 2, registry.own_surface_query()),
        Some(tcl_registry::VariableScope::Global)
    );
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "namespace eval ns {}; entry .e -textvariable ::ns::value",
        registry,
        false,
    );
    assert!(
        unit.top_level
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
    assert_eq!(unit.top_level.ssa.var_symbol("value"), None);
}

#[test]
fn build_ssa_global_option_without_absolute_def_has_no_bare_alias() {
    let reg = default_registry();
    let mut func = Function::new("::top", "entry");
    let entry = func.entry;
    func.blocks.get_mut(&entry).unwrap().statements.push(call(
        Span::new(0, 34),
        "entry",
        &[".e", "-textvariable", "value"],
        &[],
    ));
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

    let ssa = build_ssa(&func, &reg);
    assert_eq!(ssa.var_symbol("::value"), None);
    assert_eq!(ssa.var_symbol("value"), None);
}

#[test]
fn procedure_widget_catalogue_does_not_write_global_or_local_values() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "proc make_entry {} {entry .e -textvariable value; .e configure -textvariable value}",
        registry,
        false,
    );
    let function = unit.function("::make_entry").unwrap();
    assert!(
        function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
}

#[test]
fn build_ssa_diamond_phi() {
    let reg = default_registry();
    // entry → branch on $x → then: set x 1 → end
    //                       → else: set x 2 → end
    let mut func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );

    // Define x in entry first so it's used in the condition.
    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        });

    func.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(10, 18),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    func.blocks
        .get_mut(&els)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(20, 28),
            name: "x".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        });
    // Read `x` after the join so it is upward-exposed at `end` — under
    // semi-pruned SSA a phi is placed only for a name with a downstream
    // reader (a dead phi for an unread merge is correctly dropped).
    func.blocks
        .get_mut(&end)
        .unwrap()
        .statements
        .push(Statement::Call {
            span: Span::new(30, 38),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["$x".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        });

    let ssa = build_ssa(&func, &reg);

    // x is defined in both then and else and read at end, so the end block
    // should have a phi for x.
    let sx = ssa.var_symbol("x").expect("x interned");
    let end_block = &ssa.blocks[&end];
    assert!(
        end_block.phis.iter().any(|phi| phi.name == sx),
        "end block should have a phi for x"
    );

    // The phi should have incoming edges from then and else.
    if let Some(phi) = end_block.phis.iter().find(|p| p.name == sx) {
        assert!(
            phi.incoming.contains_key(&then),
            "phi should have incoming from then"
        );
        assert!(
            phi.incoming.contains_key(&els),
            "phi should have incoming from else"
        );
    }
}

#[test]
fn exact_jim_config_places_and_versions_phi_for_expr_sugar_return() {
    let reg = default_registry();
    let mut func = diamond_cfg();
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    for (block, value) in [(then, "1"), (els, "2")] {
        func.blocks
            .get_mut(&block)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(10, 20),
                name: "a".into(),
                name_braced: false,
                value: value.into(),
                value_span: None,
            });
    }
    func.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
        tokens: None,
        value: Some("$($a)".into()),
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        braced: false,
    });

    // Jim is an environment grammar without a catalogue profile. Its
    // `$()` expression sugar must therefore come from the exact config,
    // not the default registry's profile. Deliberately include file/offset
    // state: SSA re-scans detached words in local nested coordinates.
    let jim = tcl_lexer::LexerConfig {
        base_offset: 91,
        base_line: 7,
        base_col: 13,
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_dialect("jim")
    };
    let ssa = build_ssa_with_config(&func, &reg, jim);
    let a = ssa.var_symbol("a").expect("Jim return read interns a");
    let phi = ssa.blocks[&end]
        .phis
        .iter()
        .find(|phi| phi.name == a)
        .expect("Jim-only return read keeps the merge phi live");
    assert!(phi.incoming.contains_key(&then));
    assert!(phi.incoming.contains_key(&els));

    let def_use = crate::def_use::build_def_use_chains(&ssa, Some(&func), jim);
    let chain = def_use
        .chain_for(ssa.var_symbol("a").unwrap(), phi.version)
        .expect("the phi definition has a chain");
    assert!(
        chain
            .uses
            .iter()
            .any(|use_site| use_site.kind == crate::def_use::UseKind::Terminator),
        "the Jim return must read the merged SSA version: {chain:?}"
    );

    let tcl = build_ssa_with_config(&func, &reg, tcl_lexer::LexerConfig::for_dialect("tcl9.0"));
    assert!(
        tcl.var_symbol("a")
            .is_none_or(|symbol| { !tcl.blocks[&end].phis.iter().any(|phi| phi.name == symbol) }),
        "Tcl must not reinterpret Jim's `$()` expression sugar as a read"
    );
}

#[test]
fn build_ssa_loop() {
    let reg = default_registry();
    // entry: set i 0 → header: branch $i<10 → body: incr i → header
    //                                        → end: return
    let mut func = loop_cfg();
    let entry = func.entry;
    let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));

    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 8),
            name: "i".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        });
    func.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(Statement::Incr {
            span: Span::new(20, 28),
            name: "i".into(),
            name_braced: false,
            amount: None,
            amount_braced: false,
            safe_on_uninit: false,
        });

    let ssa = build_ssa(&func, &reg);

    // header should have a phi for i (from entry and body).
    let si = ssa.var_symbol("i").expect("i interned");
    let header_blk = &ssa.blocks[&header];
    assert!(
        header_blk.phis.iter().any(|p| p.name == si),
        "header should have a phi for i"
    );
}

#[test]
fn aliased_set_preserves_aliased_nested_expr_read_in_call_ir_for_ssa() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let reg = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let lower = |source: &str| {
        let mut lowerer = crate::lowering::Lowerer::new(&reg);
        lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        });
        crate::lowering::lower_to_ir_with(lowerer, source)
    };
    let module = lower(
        "set x 1\ninterp alias {} myset {} set\ninterp alias {} e {} expr\nmyset y [e {$x+1}]\n",
    );
    let call = module
        .top_level
        .statements
        .last()
        .expect("aliased set call");
    assert!(
        defs_of_with_registry(call, Some(&reg)).contains(&"y".to_owned()),
        "aliased set must define y: {call:?}"
    );
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    assert!(
        uses_of(call, &mut scanner, &reg).contains(&"x".to_owned()),
        "SSA must consume the proved aliased expression read"
    );
    let failed =
        lower("interp alias {} myset {} set\ninterp alias {} e {} expr\nmyset y [e {$x+1}]\n");
    assert!(
        defs_of_with_registry(failed.top_level.statements.last().unwrap(), Some(&reg)).is_empty(),
        "an argument read failure cannot define the setter's output"
    );
}

#[test]
fn setter_source_reads_use_selected_child_expression_owners() {
    // naming.variable.selected-setter-expression-reads
    // docs/design/analysis/name-resolution-proofs/variable-selected-setter-expression-reads.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        for (source, read, expected) in [
            (
                "set input 3; interp alias {} setter {} set; setter output [expr {$input + 1}]",
                "input",
                true,
            ),
            (
                "proc expr args {return VALUE}; interp alias {} setter {} set; setter output [expr {$missing}]",
                "missing",
                false,
            ),
            (
                "set input 3; interp alias {} calc {} expr; interp alias {} setter {} set; setter output [calc {$input + 1}]",
                "input",
                true,
            ),
        ] {
            let mut lowerer = crate::lowering::Lowerer::new(&registry);
            lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            });
            let module = crate::lowering::lower_to_ir_with(lowerer, source);
            let call = module
                .top_level
                .statements
                .last()
                .expect("actual setter call");
            let tokens = module
                .top_level
                .retained_source_tokens_for_statement(call)
                .unwrap_or_else(|| {
                    panic!("{dialect}: original aliased setter carrier missing: {call:?}")
                });
            assert!(tokens.source_binding.is_some(), "{dialect}: {source}");
            assert!(
                defs_of_with_registry(call, Some(&registry)).contains(&"output".to_owned()),
                "{dialect}: actual setter definition is an independent positive: {call:?}",
            );
            let mut scanner = VarReferenceScanner::with_config(
                VarScanOptions::default(),
                module.native_lexer_config(),
            );
            let names = uses_of(call, &mut scanner, &registry);
            assert_eq!(
                names.iter().any(|name| name == read),
                expected,
                "{dialect}: {source}: {names:?}"
            );
        }
    }
}

#[test]
fn set_value_reads_preserves_scanner_element_qualification() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        element_qualified: true,
        ..VarScanOptions::default()
    });
    assert!(
        set_value_reads(
            "[::expr {$array(key) + 1}]",
            None,
            &mut scanner,
            SsaInvocationContext::standalone(&reg)
        )
        .contains("array(key)")
    );
}

#[test]
fn build_ssa_empty_function() {
    let reg = default_registry();
    let mut func = Function::new("::empty", "entry");
    let entry = func.entry;
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

    let ssa = build_ssa(&func, &reg);
    assert_eq!(ssa.blocks.len(), 1);
    assert_eq!(ssa.blocks[&entry].phis, [] as [crate::ssa::Phi; 0]);
    assert_eq!(
        ssa.blocks[&entry].statements,
        [] as [crate::ssa::SsaStatement; 0]
    );
}

#[test]
fn complexity_guard_skips_oversized_ssa() {
    let reg = default_registry();
    // A small function is below the ceiling and analysed normally.
    let small = Function::new("::small", "entry");
    assert!(!is_complexity_guarded(&small));

    // A function above the block ceiling is guarded: `build_ssa` returns a
    // trivial SSA without running the O(blocks·vars) dominator + phi walk
    // that would cost seconds on a pathological generated body.
    let mut big = Function::new("::big", "b0");
    let b0 = big.entry;
    for i in 0..=COMPLEXITY_GUARD_BLOCKS {
        let id = big.intern_block(format!("b{i}"));
        big.blocks.insert(id, Block::new(format!("b{i}")));
    }
    assert!(big.blocks.len() > COMPLEXITY_GUARD_BLOCKS);
    assert!(is_complexity_guarded(&big));

    let ssa = build_ssa(&big, &reg);
    assert_eq!(ssa.blocks.len(), 0, "guarded SSA must be trivial");
    assert_eq!(ssa.name, "::big");
    assert_eq!(ssa.entry, b0);
}

/// `collapsed_extra_defs` recurses once per nested
/// `If`/`While`/`For`/`Foreach`/`Catch`/`Try`/`Switch` body, with no
/// depth cap of its own. Transitively
/// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass,
/// so this is defence-in-depth / consistency with every other
/// full-tree walker in this crate, not a currently-reproducible crash.
/// 2000 levels is comfortably past this new cap; the assertion is that
/// the call returns at all, not what it returns.
#[test]
fn deeply_nested_if_survives_collapsed_extra_defs() {
    use crate::ir::{IfClause, Script};

    const DEPTH: usize = 2000;
    let leaf = Statement::AssignConst {
        span: Span::new(0, 0),
        name: "leaf".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    let mut script = Script::from_statements(vec![leaf]);
    for _ in 0..DEPTH {
        script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 0),
            clauses: vec![IfClause {
                condition: ExprNode::Raw { text: "1".into() },
                condition_span: Span::new(0, 0),
                body: script,
                body_span: Span::new(0, 0),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
    }

    let reg = CommandRegistry::build_default();
    let extra = collapsed_extra_defs(&script, &reg, 0);
    // Nothing but the leaf `AssignConst` here is a def source visible to
    // this helper (it only recovers `for`/condition-command-sub defs) —
    // the assertion is that this returns at all without overflowing the
    // stack, not what it returns.
    let _ = extra;
}

/// The classified uses of every `Call` in a one-statement proc, lowered
/// from source so the statement carries the real word snapshot.
fn classified_call_uses(body: &str) -> Vec<(String, UseClass)> {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = format!("proc f {{tainted}} {{\n {body}\n}}");
    let cu = crate::compilation_unit::CompilationUnit::build_for(&src, &reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let mut out = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            if matches!(stmt, Statement::Call { .. }) {
                out.extend(uses_of_classified(stmt, &mut scanner, &reg));
            }
        }
    }
    out
}

/// A braced word of a *nested* command is read exactly as the same word of
/// a direct call is, when the registry says the callee evaluates it in
/// this frame. Only a pair of braces separated `[expr $tainted]` (whose
/// read the enclosing word's own scan already saw) from
/// `[expr {$tainted}]`, which would otherwise be invisible to every
/// consumer of the use map.
#[test]
fn braced_word_of_a_nested_substitution_is_read_in_an_evaluated_role() {
    let uses = classified_call_uses("puts [expr {$tainted}]");
    assert!(
        uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
        "an `Expr` role's braced word substitutes in this frame: {uses:?}"
    );
}

#[test]
fn conditional_nested_expression_reads_keep_original_tokens_without_entry() {
    let registry = default_registry();
    let source = "proc f {tainted} {puts [expr {$tainted + 1}]}";
    let unit = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
    let function = unit.function("::f").unwrap();
    let child = function
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(Statement::tokens)
        .flat_map(|tokens| {
            crate::word_subst::lifted_calls(Some(tokens), unit.ir_module.lexer_config)
        })
        .find(|call| call.command == "expr")
        .and_then(|call| call.tokens)
        .expect("original nested expression");
    let binding = child.source_binding.as_ref().unwrap();
    assert_eq!(
        binding.runtime_reachability(),
        crate::command_binding::SourceRuntimeReachability::Conditional,
    );
    assert!(binding.proved_execution_target().is_none());
    let scanner = VarReferenceScanner::new(VarScanOptions::default());
    let mut reads = BTreeSet::new();
    scan_selected_expression_roles(
        &child,
        &scanner,
        SsaInvocationContext::standalone(&registry),
        &mut reads,
    );
    assert_eq!(reads, BTreeSet::from(["tainted".to_owned()]));

    let mut foreign = child.clone();
    foreign.argv_texts[1] = "$other + 1".to_owned();
    foreign.word_exprs[1] = WordExpr::BracedLiteral {
        text: "$other + 1".to_owned(),
        source: foreign.word_exprs[1].source().clone(),
    };
    reads.clear();
    scan_selected_expression_roles(
        &foreign,
        &scanner,
        SsaInvocationContext::standalone(&registry),
        &mut reads,
    );
    assert!(
        reads.is_empty(),
        "foreign source cannot borrow May-read topology"
    );
}

/// …and a braced word in a role the callee does *not* evaluate stays the
/// literal it is. `list` builds a value; `{$tainted}` is one of its
/// elements, not a script.
#[test]
fn braced_word_of_a_nested_substitution_stays_literal_in_a_data_role() {
    let uses = classified_call_uses("puts [list {$tainted}]");
    assert!(
        !uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
        "`list` does not evaluate its arguments in the calling frame: {uses:?}"
    );
}

/// A nested command's **body** must not be read as a frame use, even
/// though a body written on a *statement* is. The difference is the
/// lowering: a statement's body becomes its own CFG block, so `lmap`'s
/// loop variable is defined where the body's reads are seen, while a
/// nested substitution gets no block at all — so its body's reads arrive
/// with none of its own definitions and `x` looks read-before-set.
#[test]
fn body_word_of_a_nested_substitution_is_not_a_frame_read() {
    let uses = classified_call_uses("puts [join [lmap x $tainted {string toupper $x}] ,]");
    assert!(
        !uses.iter().any(|(name, _)| name == "x"),
        "`lmap` binds its own loop variable; the frame never reads it: {uses:?}"
    );
}

/// A script a substitution runs in this frame — `catch`'s — shares the
/// frame's names, so `inner` is the frame's: its write is the effect
/// call's. The walk cannot order the script's read of it after the
/// script's own write, so the effect call reads it by name beside that
/// write, and nothing the host substitutes names it.
#[test]
fn a_protected_scripts_local_is_read_by_name_beside_its_write() {
    let uses = classified_call_uses("puts [catch {set inner 1; expr {$inner + 1}}]");
    let inner: Vec<_> = uses.iter().filter(|(name, _)| name == "inner").collect();
    assert_eq!(inner, [&("inner".to_owned(), UseClass::Name)], "{uses:?}");
}

/// The classified uses of every `AssignValue` in a one-statement proc —
/// the assignment twin of [`classified_call_uses`].
fn classified_assign_uses(body: &str) -> Vec<(String, UseClass)> {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = format!("proc f {{tainted}} {{\n {body}\n}}");
    let cu = crate::compilation_unit::CompilationUnit::build_for(&src, &reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let mut out = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            if matches!(stmt, Statement::AssignValue { .. }) {
                out.extend(uses_of_classified(stmt, &mut scanner, &reg));
            }
        }
    }
    out
}

/// An assignment's value word substitutes by exactly the rules a call's
/// word does, so the braced body of a nested `[expr …]`
/// is a read there too. Without it the expression's operands are absent
/// from `uses` and every consumer of that map — the expr shimmer detector
/// included — is blind to `set r [list [expr {$tainted + 1}]]`.
#[test]
fn braced_word_of_a_nested_substitution_is_read_in_an_assignment_value() {
    let uses = classified_assign_uses("set r [list [expr {$tainted + 1}]]");
    assert!(
        uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
        "an `Expr` role's braced word substitutes in this frame: {uses:?}"
    );
}

/// …and the body exclusion holds on the assignment path too — this is the
/// spelling the repository's own corpus actually contains.
#[test]
fn body_word_of_a_nested_substitution_in_an_assignment_is_not_a_frame_read() {
    let uses = classified_assign_uses("set r [join [lmap x $tainted {string toupper $x}] ,]");
    assert!(
        !uses.iter().any(|(name, _)| name == "x"),
        "`lmap` binds its own loop variable; the frame never reads it: {uses:?}"
    );
}

/// The SSA of `source`'s function `name`, built as a unit builds it.
fn ssa_of_function(source: &str, name: &str) -> SsaFunction {
    // A registry naming no release reads an authored `proc` as a
    // declaration only, so its body has no statements; these are Tcl 8.6.
    let reg = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let cu = crate::compilation_unit::CompilationUnit::build_for(source, &reg, false);
    cu.function(name).expect("function analysed").ssa.clone()
}

/// The SSA statements of `ssa` whose statement satisfies `wanted`.
fn statements_where(ssa: &SsaFunction, wanted: impl Fn(&Statement) -> bool) -> Vec<&SsaStatement> {
    ssa.blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter(|statement| wanted(&statement.statement))
        .collect()
}

/// The definition point the CFG builder puts ahead of a statement for
/// what the statement's `[…]` substitutions write.
fn is_effect_call(stmt: &Statement) -> bool {
    stmt.word_effects_host().is_some()
}

/// A statement's word effects are paired with the statement whose words
/// they are by construction: the statement right after the definition
/// point, whatever its span, and no other statement has a host. A control
/// statement's words have no statement of their own, so their definition
/// point has none.
#[test]
fn word_effects_stand_ahead_of_the_statement_whose_words_they_are() {
    let effect_block = |source: &str, name: &str| {
        let ssa = ssa_of_function(source, name);
        let block = ssa
            .blocks
            .values()
            .find(|block| {
                block
                    .statements
                    .iter()
                    .any(|info| is_effect_call(&info.statement))
            })
            .expect("a block with an effect call");
        let call = block
            .statements
            .iter()
            .position(|info| is_effect_call(&info.statement))
            .expect("the effect call");
        (block.statements.clone(), call)
    };
    for (source, name) in [
        (
            "proc p {} {set x 1; set r [expr {$x + [incr x]}]; return $x}\n",
            "::p",
        ),
        ("set x 1\nset r [expr {[incr x] + [foo]}]\n", "::top"),
        ("set x 1\nexpr {$x + [incr x]}\n", "::top"),
    ] {
        let (statements, call) = effect_block(source, name);
        let host = word_effects_host(&statements, call).expect("the call has a host");
        assert_eq!(host, call + 1, "{source}");
        assert!(
            !is_synthetic_statement(&statements[host].statement),
            "{source}"
        );
        assert_eq!(
            statements[host].statement.span(),
            statements[call].statement.span(),
            "{source}"
        );
        for other in (0..statements.len()).filter(|&index| index != call) {
            assert_eq!(word_effects_host(&statements, other), None, "{source}");
        }
    }
    let (mut statements, call) = effect_block(
        "proc p {} {set x 1; set r [expr {$x + [incr x]}]; return $x}\n",
        "::p",
    );
    let host = word_effects_host(&statements, call).expect("the call has a host");
    let Statement::AssignExpr { span, .. } = &mut statements[host].statement else {
        panic!("an assignment of an expression hosts the call");
    };
    *span = tcl_lexer::Span::new(span.start() + 1, span.end());
    assert_eq!(word_effects_host(&statements, call), Some(host));
    let (statements, call) = effect_block(
        "proc p {} {set x 1; return [expr {$x + [incr x]}]}\n",
        "::p",
    );
    assert_eq!(word_effects_host(&statements, call), None);
}

/// The words of a statement whose `[…]` substitutions write a place read it
/// before, between and after the write. The effect call ahead of the
/// statement reads the version before the write, by name, so the store
/// feeding the word that runs first stays live, and the statement's own use
/// names the version after the write by name as well: no word of it is an
/// operand something may forward.
#[test]
fn an_effect_call_reads_the_version_before_the_write_its_statement_reads() {
    for (body, reads_it) in [
        ("set r [expr {$x + [set x 10] + $x}]", true),
        ("expr {$x + [set x 10] + $x}", true),
        ("set r \"$x [set x 10] $x\"", true),
        ("incr r [expr {$x + [set x 10]}]", false),
        ("set r [expr {[string length $x] + [set x 10]}]", false),
        ("set r [expr {[expr {$x + 1}] + [set x 10]}]", false),
    ] {
        let source = format!("proc p {{}} {{\n set x 1\n set r 5\n {body}\n puts $x\n}}\n");
        let ssa = ssa_of_function(&source, "::p");
        let x = ssa.var_symbol("x").expect("x");
        let call = statements_where(&ssa, is_effect_call)[0];
        assert_eq!(call.defs[&x], 2, "{body}");
        assert_eq!(call.uses.get(&x), Some(&1), "{body}");
        assert!(call.name_only_uses.contains(&x), "{body}");
        if reads_it {
            let host = statements_where(&ssa, |stmt| {
                matches!(
                    stmt,
                    Statement::AssignExpr { .. }
                        | Statement::ExprEval { .. }
                        | Statement::AssignValue { .. }
                ) && stmt.span() == call.statement.span()
            })[0];
            assert_eq!(host.uses.get(&x), Some(&2), "{body}");
            assert!(host.name_only_uses.contains(&x), "{body}");
        }
    }
}

/// A place no word of the statement reads is not read by the call that
/// carries its write, so the store the write overwrites stays dead: the
/// call's read is for the words, not for every place it defines.
#[test]
fn an_effect_call_leaves_a_place_its_statement_never_reads_alone() {
    for body in [
        "set r [expr {[set x 10] + 1}]",
        "set r \"[set x 10]\"",
        "set r [expr {[gets $fd x] + 1}]",
    ] {
        let source = format!("proc p {{fd}} {{\n set x 1\n {body}\n puts $x\n}}\n");
        let ssa = ssa_of_function(&source, "::p");
        let x = ssa.var_symbol("x").expect("x");
        let call = statements_where(&ssa, is_effect_call)[0];
        assert_eq!(call.defs[&x], 2, "{body}");
        assert_eq!(call.uses.get(&x), None, "{body}");
    }
}

/// A call that embeds the write holds it among its own definitions, and its
/// words read the place before, between and after it. It reads the version
/// before the write by name, and none of its words is an operand to forward;
/// a place the call names itself, in the position its command reads or
/// writes, stays what it was.
#[test]
fn a_call_reads_a_place_its_embedded_command_writes_by_name() {
    for (command, body) in [
        ("puts", "puts [expr {$x + [set x 10] + $x}]"),
        ("puts", "puts \"$x [incr x] $x\""),
        ("foo", "foo $x [incr x] $x"),
        ("lappend", "lappend l [incr x] $x"),
    ] {
        let source = format!("proc p {{}} {{\n set x 1\n set l {{}}\n {body}\n puts $x\n}}\n");
        let ssa = ssa_of_function(&source, "::p");
        let x = ssa.var_symbol("x").expect("x");
        let host = statements_where(
            &ssa,
            |stmt| matches!(stmt, Statement::Call { command: name, .. } if name == command),
        )[0];
        assert_eq!(host.uses.get(&x), Some(&1), "{body}");
        assert_eq!(host.defs[&x], 2, "{body}");
        assert!(host.name_only_uses.contains(&x), "{body}");
    }
    let ssa = ssa_of_function(
        "proc p {} {\n set l {}\n lappend l $l\n puts $l\n}\n",
        "::p",
    );
    let l = ssa.var_symbol("l").expect("l");
    let host = statements_where(
        &ssa,
        |stmt| matches!(stmt, Statement::Call { command, .. } if command == "lappend"),
    )[0];
    assert!(
        !host.name_only_uses.contains(&l),
        "the command's own target is read before it writes it: its word is an operand"
    );
}

/// A `switch` the flow graph keeps as one statement defines, as may-defs,
/// every name its arms write or bind — the default arm's, a fall-through
/// arm's, a nested command's, an `incr`'s, a `lappend`'s and a `global`
/// binding's — and uses the prior version of each, quoted, so liveness
/// keeps the store that feeds it and no read-before-set is invented.
#[test]
fn an_opaque_switch_defines_every_name_its_arms_write() {
    let ssa = ssa_of_function(
        "proc p {s} {\n set x 1\n set n 0\n\
             switch -glob -- $s {\n  a* - b* { set x 2 }\n  c* { if {[gets stdin line]} { incr n } }\n  default { lappend l 1; global g }\n }\n puts $x\n}\n",
        "::p",
    );
    let statement = ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find(|statement| matches!(statement.statement, Statement::Switch { .. }))
        .expect("the switch stays one statement");
    let mut written: Vec<&str> = statement
        .may_defs
        .iter()
        .map(|&symbol| ssa.var_name(symbol))
        .collect();
    written.sort_unstable();
    assert_eq!(written, ["g", "l", "line", "n", "x"]);
    for &symbol in &statement.may_defs {
        assert!(statement.defs.contains_key(&symbol));
        assert!(statement.quoted_uses.contains(&symbol));
    }
    // The read after the switch reads the version the switch defines.
    let x = ssa.var_symbol("x").expect("x");
    assert_eq!(statement.defs[&x], 2);
    assert_eq!(statement.uses[&x], 1);
}

/// The version each name holds where a call to a command the module cannot
/// see sits is recorded, in a procedure as at the top level; a version a
/// later definition makes is not.
#[test]
fn an_unseen_call_records_the_versions_live_where_it_sits() {
    let ssa = ssa_of_function("set x 1\nset y 2\nfoo\nset x 3\nputs $x$y\n", "::top");
    let x = ssa.var_symbol("x").expect("x");
    let y = ssa.var_symbol("y").expect("y");
    assert!(ssa.is_observed_by_unseen_call(x, 1));
    assert!(ssa.is_observed_by_unseen_call(y, 1));
    assert!(!ssa.is_observed_by_unseen_call(x, 2));
    assert!(ssa.name_is_observed_by_unseen_call("x", 1));
    assert!(!ssa.name_is_observed_by_unseen_call("nothing", 1));
    let ssa = ssa_of_function("proc p {} {\n set x 1\n foo\n puts $x\n}\n", "::p");
    assert!(ssa.name_is_observed_by_unseen_call("x", 1));
    // A call the module resolves records nothing.
    for (source, name) in [
        ("proc foo {} { puts hi }\nset x 1\nfoo\nputs $x\n", "::top"),
        ("set x 1\nputs $x\n", "::top"),
    ] {
        let ssa = ssa_of_function(source, name);
        assert!(!ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
    // A command a substitution runs, a condition's and an arm's count too.
    for source in [
        "set x 1\nputs [foo]\n",
        "set x 1\nwhile {[foo]} { puts hi }\n",
        "set x 1\nswitch -glob -- $s { q* { foo } }\n",
    ] {
        let ssa = ssa_of_function(source, "::top");
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
}

/// A sourced file runs in the frame of the call, so it may write or read
/// any name there: a top-level name, and a procedure's local too.
#[test]
fn a_sourced_file_records_the_versions_live_where_it_sits_in_a_procedure_too() {
    for (source, name) in [
        ("set x 1\nsource other.tcl\nputs $x\n", "::top"),
        ("set x 1\nputs [source other.tcl]\n", "::top"),
        (
            "proc p {} {\n set x 1\n source other.tcl\n puts $x\n}\n",
            "::p",
        ),
        (
            "proc p {s} {\n set x 1\n switch -glob -- $s { q* { source other.tcl } }\n puts $x\n}\n",
            "::p",
        ),
    ] {
        let ssa = ssa_of_function(source, name);
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
}

/// The statements of `function` in `source`, in block order.
fn ssa_statements(ssa: &SsaFunction) -> Vec<&SsaStatement> {
    ssa.blocks
        .values()
        .flat_map(|block| &block.statements)
        .collect()
}

/// A callee an arm of an opaque `switch` calls writes the caller's name
/// through `upvar`, as it would at a call the graph lowers: the statement
/// that follows the `switch` defines the name as a *may*-definition that
/// uses the version before it, so the value the name held when no arm runs
/// survives into the join.
#[test]
fn an_opaque_switch_arm_that_calls_a_writer_may_define_the_name_it_writes() {
    let ssa = ssa_of_function(
        "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {s} {\n set line 5\n\
             switch -glob -- $s { g* { zero line } }\n puts $line\n}\n",
        "::p",
    );
    let marker = ssa_statements(&ssa)
        .into_iter()
        .find(|statement| is_arm_writes_marker(&statement.statement))
        .expect("the switch is followed by the marker for what its callee writes");
    let line = ssa.var_symbol("line").expect("line");
    assert_eq!(marker.may_defs, HashSet::from([line]));
    assert_eq!(marker.defs[&line], 2);
    assert_eq!(marker.uses[&line], 1);
    assert!(marker.quoted_uses.contains(&line));
    // A callee that writes a global, not this frame's name, adds no
    // may-definition of a local, and an arm that calls nothing adds no
    // marker.
    for source in [
        "proc hit {} {global hits; incr hits}\nproc p {s} {\n set n 5\n\
             switch -glob -- $s { g* { hit } }\n puts $n\n}\n",
        "proc p {s} {\n set n 5\n switch -glob -- $s { g* { set m 1 } }\n puts $n\n}\n",
    ] {
        let ssa = ssa_of_function(source, "::p");
        let n = ssa.var_symbol("n").expect("n");
        assert!(
            ssa_statements(&ssa)
                .into_iter()
                .filter(|statement| is_arm_writes_marker(&statement.statement))
                .all(|statement| !statement.may_defs.contains(&n)),
            "{source}"
        );
    }
}

/// The body of an opaque `catch` stays inside the call, so what it writes is
/// a *may*-definition the marker ahead of the call states: it reads the
/// version before it, the solver joins the two, and the call itself defines
/// only its result variable.
#[test]
fn an_opaque_catch_may_define_the_names_its_body_writes() {
    let ssa = ssa_of_function(
        "set g 5\ncatch { if {[gets stdin] eq {q}} { set g 0 } } msg\nputs $g$msg\n",
        "::top",
    );
    let statements = ssa_statements(&ssa);
    let marker = statements
        .iter()
        .find(|statement| is_arm_writes_marker(&statement.statement))
        .expect("the catch body's writes are stated ahead of the call");
    let g = ssa.var_symbol("g").expect("g");
    let msg = ssa.var_symbol("msg").expect("msg");
    assert_eq!(marker.may_defs, HashSet::from([g]));
    assert_eq!(marker.defs[&g], 2);
    assert_eq!(marker.uses[&g], 1);
    assert!(marker.quoted_uses.contains(&g));
    let call = statements
            .iter()
            .find(|statement| {
                matches!(&statement.statement, Statement::Call { command, .. } if command == "catch")
            })
            .expect("the catch call");
    assert!(call.defs.contains_key(&msg));
    assert!(
        !call.defs.contains_key(&g),
        "the call defines no name its body writes"
    );
}

/// The call reads a name its body writes as the body's own: what the body
/// consumes of the frame is its reads less its writes, as an opaque
/// `switch`'s arms are read, so `catch {set i 0; puts $i}` reads nothing
/// of `i`, while a name the body only reads is still a read of the frame.
#[test]
fn an_opaque_catch_reads_its_own_writes_as_its_own() {
    let ssa = ssa_of_function("catch {set i 0; puts $i; puts $j}\n", "::top");
    let statements = ssa_statements(&ssa);
    let call = statements
            .iter()
            .find(|statement| {
                matches!(&statement.statement, Statement::Call { command, .. } if command == "catch")
            })
            .expect("the catch call");
    let i = ssa.var_symbol("i").expect("i");
    let j = ssa.var_symbol("j").expect("j");
    assert!(!call.uses.contains_key(&i), "{:?}", call.uses);
    assert_eq!(call.uses.get(&j), Some(&0), "{:?}", call.uses);
}

/// A call to a command the module cannot see reads its words before its
/// head runs, so the version each word reads is the one the call holds,
/// which is recorded — the code the call reaches may read it — and the code
/// may rewrite the name, so a read after the call is of the fresh version
/// the marker leaves, which is not, and whose binding is the version the
/// call held. A name only a braced word carries, which the code may
/// evaluate, is recorded as well.
#[test]
fn a_call_the_module_cannot_see_holds_the_versions_its_words_read() {
    for (source, names) in [
        ("set cmd puts\n$cmd hi\nputs $cmd\n", vec!["cmd"]),
        ("set a 1\nset b 2\n$cmd $a $b\nputs $a$b\n", vec!["a", "b"]),
        ("set a 1\nset b 2\nget $a $b\nputs $a$b\n", vec!["a", "b"]),
        ("set f x.tcl\nsource $f\nputs $f\n", vec!["f"]),
    ] {
        let ssa = ssa_of_function(source, "::top");
        for name in names {
            let symbol = ssa.var_symbol(name).expect("the name is an SSA symbol");
            assert!(
                ssa.name_is_observed_by_unseen_call(name, 1),
                "{source}: {name}#1"
            );
            assert!(
                !ssa.name_is_observed_by_unseen_call(name, 2),
                "{source}: {name}#2"
            );
            assert_eq!(ssa.binding_version(symbol, 2), 1, "{source}: {name}#2");
        }
    }
    for source in [
        "set cmd puts\nset x 1\n$cmd hi\nputs $x\n",
        "set x 1\nget {$x}\nputs $x\n",
        "set x 1\nputs [get $x]\nputs $x\n",
    ] {
        let ssa = ssa_of_function(source, "::top");
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
}

/// The marker beside such a call defines no name: each name live past it
/// takes a fresh version where it stands
/// ([`SsaFunction::value_clobbers`]), whose binding is the version before
/// it, and a loop that holds the call places the φ the fresh version
/// needs. A name no read after the marker reaches takes none.
#[test]
fn the_marker_beside_a_call_the_module_cannot_see_gives_live_names_fresh_versions() {
    let fresh = |ssa: &SsaFunction, symbol: Symbol| -> Vec<(Version, Version)> {
        ssa.value_clobbers
            .values()
            .flat_map(HashMap::values)
            .filter_map(|versions| versions.get(&symbol).copied())
            .collect()
    };
    let ssa = ssa_of_function("set a 1\nget $a\nputs $a\n", "::top");
    let marker = ssa_statements(&ssa)
        .into_iter()
        .find(|statement| is_unseen_call_marker(&statement.statement))
        .expect("the call is followed by its marker");
    assert!(marker.defs.is_empty() && marker.may_defs.is_empty());
    let a = ssa.var_symbol("a").expect("a");
    assert_eq!(fresh(&ssa, a), [(1, 2)]);
    assert_eq!(ssa.binding_version(a, 2), 1);

    let ssa = ssa_of_function(
        "set a 1\nwhile {[gets stdin line] >= 0} { get $a }\nputs $a\n",
        "::top",
    );
    assert!(
        ssa.blocks
            .values()
            .any(|block| block.phis.iter().any(|phi| ssa.var_name(phi.name) == "a")),
        "the loop joins the fresh version `a` takes where the call sits"
    );

    let ssa = ssa_of_function("set a 1\nget $a\nset a 2\nputs $a\n", "::top");
    let a = ssa.var_symbol("a").expect("a");
    assert_eq!(fresh(&ssa, a), [] as [(u32, u32); 0]);
}

/// A call to a command the module cannot see inside a `catch` body, or with
/// a computed head, records the versions live where it sits, as a call the
/// graph lowers does, in a procedure as at the top level; a call the module
/// resolves records nothing.
#[test]
fn a_catch_body_and_a_computed_head_record_the_versions_live_where_they_sit() {
    for source in [
        "set x 1\ncatch {foo}\nputs $x\n",
        "set x 1\ncatch {foo} msg\nputs $x\n",
        "set x 1\ncatch { if {1} { foo } }\nputs $x\n",
        "set x 1\ncatch { puts [foo] }\nputs $x\n",
        "set x 1\ncatch { source other.tcl }\nputs $x\n",
        "set x 1\n$cmd\nputs $x\n",
        "set x 1\n$cmd arg\nputs $x\n",
        "set x 1\nputs [$cmd]\n",
        "set x 1\nif {[$cmd]} { puts hi }\n",
        "set x 1\nif {[catch {foo}]} { puts hi }\n",
        "set x 1\nset rc [catch {foo}]\nputs $x\n",
        "set x 1\nputs [catch { puts [foo] }]\n",
        "set x 1\nif {[catch $script]} { puts hi }\n",
    ] {
        let ssa = ssa_of_function(source, "::top");
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
    for (source, name, observed) in [
        (
            "proc p {} {\n set x 1\n catch { if {1} { foo } }\n puts $x\n}\n",
            "::p",
            true,
        ),
        ("proc p {} {\n set x 1\n $cmd\n puts $x\n}\n", "::p", true),
        ("proc p {} {\n set x 1\n puts [$cmd]\n}\n", "::p", true),
        (
            "proc p {} {\n set x 1\n catch { if {1} { source other.tcl } }\n puts $x\n}\n",
            "::p",
            true,
        ),
        (
            "proc foo {} { puts hi }\nset x 1\ncatch { if {1} { foo } }\nputs $x\n",
            "::top",
            false,
        ),
    ] {
        let ssa = ssa_of_function(source, name);
        assert_eq!(
            ssa.name_is_observed_by_unseen_call("x", 1),
            observed,
            "{source}"
        );
    }
}

/// A command an arm runs that may write any name — `namespace eval`,
/// `dict with`, `eval $script`, an `uplevel` — is followed by the barrier
/// the graph puts after such a command it lowers; an arm that holds none
/// has no barrier after it.
#[test]
fn an_opaque_switch_arm_that_may_write_any_name_is_followed_by_a_barrier() {
    let barrier_after_switch = |source: &str, name: &str| {
        let ssa = ssa_of_function(source, name);
        let statements = ssa_statements(&ssa);
        statements.iter().any(|statement| {
            matches!(
                &statement.statement,
                Statement::Barrier { tokens: Some(tokens), .. }
                    if tokens.synthetic == Some(crate::ir::SyntheticMarker::CallerFrameOpaque)
            )
        })
    };
    for (source, name) in [
        (
            "set go 1\nswitch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\n",
            "::top",
        ),
        (
            "proc p {s} {\n set go 1\n switch -glob -- $s { q* { dict with d { set go 0 } } }\n}\n",
            "::p",
        ),
        (
            "proc p {s} {\n set go 1\n switch -glob -- $s { q* { eval $s } }\n}\n",
            "::p",
        ),
    ] {
        assert!(barrier_after_switch(source, name), "{source}");
    }
    assert!(!barrier_after_switch(
        "proc p {s} {\n set go 1\n switch -glob -- $s { q* { set go 0 } }\n}\n",
        "::p"
    ));
}

#[test]
fn supplied_switch_may_writes_keep_actual_context_and_unavailable_residual() {
    let (context, unit) = logical_metadata_fixture("switch -glob $subject {a {set arm ONE}}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let statement = unit
        .ir_module
        .top_level
        .statements
        .iter()
        .find(|statement| matches!(statement, Statement::Switch { .. }))
        .expect("actual retained opaque switch");
    assert!(switch_may_defs_in_context(statement, selection).contains(&"arm".to_owned()));
    let Statement::Switch { arms, .. } = statement else {
        unreachable!()
    };
    let body = arms.iter().find_map(|arm| arm.body.as_ref()).unwrap();
    let mut names = BTreeSet::new();
    assert!(!nested_writes_in_context(body, selection, &mut names));
    assert!(names.contains("arm"));
    assert!(nested_writes_in_context(
        body,
        SsaInvocationContext::supplied(registry, None),
        &mut BTreeSet::new(),
    ));
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let foreign_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        foreign_profile,
        input.unit_profile(),
        input.context_registry().clone(),
        input.lexer_config(),
    );
    let foreign_metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &foreign,
        )
        .expect("same actual Registry and lexer configuration");
    assert_eq!(foreign.lexer_config(), input.lexer_config());
    assert_ne!(foreign, *input);
    assert!(nested_writes_in_context(
        body,
        SsaInvocationContext::supplied(registry, Some(foreign_metadata)),
        &mut BTreeSet::new(),
    ));
}
