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

use crate::interprocedural::test_body_identity::fixture_id;

use super::*;
use crate::cfg::Terminator;
use tcl_lexer::Span;

#[test]
fn caller_ssa_read_key_uses_typed_activation_identity() {
    // Implementation contract: naming.variable.invocation-caller-ssa-frame
    // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
    let context = ResolveContext::default().in_frame(
        &crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "caller".to_owned(),
        },
    );
    let caller = VariableCellKey::Activation {
        identity: "caller".to_owned(),
        simple: "shared".into(),
    }
    .with_lifetime(11)
    .with_index("member");
    let foreign = VariableCellKey::Activation {
        identity: "callee".to_owned(),
        simple: "local".into(),
    };
    assert_eq!(
        invocation_read_key_in_context(caller.clone(), &context),
        Some(caller)
    );
    assert!(invocation_read_key_in_context(foreign.clone(), &context).is_none());
    assert!(
        invocation_read_key_in_context(
            foreign.clone(),
            &ResolveContext::default()
                .in_frame(&crate::var_resolve::VariableExecutionFrame::Global)
        )
        .is_none()
    );
    let written = VariableCellKey::Authored(foreign.compatibility_name());
    assert_eq!(
        invocation_read_key_in_context(written.clone(), &context),
        Some(written)
    );
    let shared = VariableCellKey::Namespace {
        identity: crate::command_binding::SourceNamespaceKey::authored("::"),
        simple: "shared".into(),
    };
    assert_eq!(
        invocation_read_key_in_context(shared.clone(), &context),
        Some(shared)
    );
}

#[test]
fn invocation_read_projection_keeps_physical_inventory_and_root_alias_reads() {
    // Implementation contract: naming.variable.invocation-caller-ssa-frame
    // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, foreign) in [
        ("proc child {} {set local 1; set copy $local}; child", true),
        (
            "set shared 1; proc child {} {upvar #0 shared alias; set copy $alias}; child",
            false,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let ssa = &unit.top_level.ssa;
        let mut seen = false;
        for block in ssa.blocks.values() {
            for statement in &block.statements {
                let Some(tokens) = statement.statement.tokens() else {
                    continue;
                };
                for place in crate::place_bridge::invocation_execution_read_places(tokens, registry)
                {
                    let Some(key) = crate::var_resolve::canonical_binding_value_key(&place) else {
                        continue;
                    };
                    if key.activation_identity().is_some() != foreign {
                        continue;
                    }
                    seen = true;
                    let used = statement
                        .uses
                        .keys()
                        .any(|symbol| ssa.cell_key(*symbol) == Some(&key));
                    assert_eq!(used, !foreign, "{source}/{key:?}");
                }
            }
        }
        assert!(
            seen,
            "original physical read inventory missing for {source}"
        );
    }
}

#[test]
fn word_read_requires_original_spelling_as_well_as_site() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "set constructor list; set x [$constructor a b]; set out $x",
        registry,
        false,
        "tcl8.6",
    );
    let ssa = &unit.top_level.ssa;
    let mut reached = false;
    for (&block, data) in &ssa.blocks {
        for index in 0..data.statements.len() {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for original in tokens.words().iter().filter(|word| {
                word.sole_variable_substitution()
                    .is_some_and(|(spelling, _)| spelling == "$x")
            }) {
                assert!(view.read_word(original).is_some());
                assert!(view.read_word_produces_value(original, registry));
                assert!(view.read_word_place(original, registry).is_some());
                assert_eq!(
                    view.read_word_representation(original, registry),
                    Some(tcl_syntax::value::ValueRepresentation::List)
                );
                let (_, source) = original.sole_variable_substitution().unwrap();
                let changed = WordExpr::Variable {
                    spelling: "$z".into(),
                    source: source.clone(),
                };
                assert!(view.read_word(&changed).is_none());
                assert!(!view.read_word_produces_value(&changed, registry));
                assert!(view.read_word_place(&changed, registry).is_none());
                assert!(view.read_word_contents(&changed, registry).is_none());
                assert!(view.read_word_representation(&changed, registry).is_none());
                assert!(
                    view.read_word_representation_alternatives(&changed, registry)
                        .is_none()
                );
                assert!(
                    !view
                        .read_word_representation_advice(&changed, registry)
                        .already_numeric
                );
                reached = true;
            }
        }
    }
    assert!(reached, "original physical read must be present");
}

#[test]
fn expression_read_requires_original_spelling_as_well_as_extent() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc p {x} {expr {$x + 1}}",
        registry,
        false,
        "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut reached = false;
    for (&block, data) in &ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in tokens
                .variable_accesses
                .iter()
                .filter(|read| read.original_spelling == "$x")
            {
                let original = crate::expr_ast::ExprNode::Var {
                    text: "$x".into(),
                    name: "x".into(),
                    start: 0,
                    end: 1,
                };
                let base = Some(access.source.span.start());
                assert!(
                    view.read_expression_variable_place(&original, base, registry)
                        .is_some()
                );
                let changed = crate::expr_ast::ExprNode::Var {
                    text: "$z".into(),
                    name: "z".into(),
                    start: 0,
                    end: 1,
                };
                assert!(
                    view.read_expression_variable_place(&changed, base, registry)
                        .is_none()
                );
                assert!(
                    view.read_expression_representation(&changed, base, None, registry)
                        .is_none()
                );
                assert!(
                    view.read_expression_representation_alternatives(
                        &changed, base, None, registry
                    )
                    .is_none()
                );
                assert!(
                    view.read_expression_variable_place(&original, None, registry)
                        .is_none()
                );
                reached = true;
            }
        }
    }
    assert!(reached, "the original expression read must be retained");
}

#[test]
fn potential_missing_read_retains_closed_presence_alternatives() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc p {flag} {if {$flag} {set x 1}; set out $x}",
        registry,
        false,
        "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut reached = false;
    for (&block, data) in &ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in tokens
                .variable_accesses
                .iter()
                .filter(|read| read.original_spelling == "$x")
            {
                assert!(
                    view.read_contents_presence_alternatives_at(&access.source, "$x", registry)
                        .is_some_and(SsaReadPresenceAlternatives::may_be_undefined),
                    "source={:?}; residual={:?}; presence={:?}",
                    access.source,
                    access.context_residual(),
                    access
                        .context_alternatives()
                        .iter()
                        .map(|context| {
                            let place = crate::var_resolve::resolve_substitution_access(
                                "$x",
                                context,
                                registry,
                                tcl_registry::TraceOperation::Read,
                            );
                            (place.observed, context.contents_presence(&place))
                        })
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    view.read_contents_presence_at(&access.source, "$x", registry),
                    Some(crate::var_resolve::ContentsPresence::Unknown)
                );
                assert_eq!(
                    view.read_completion_at(&access.source, "$x", registry),
                    Some(crate::var_resolve::VariableReadCompletion::Unknown)
                );
                assert!(
                    view.read_reference(&access.source, "$x")
                        .is_none_or(|read| read.version.is_none())
                );
                reached = true;
            }
        }
    }
    assert!(reached);
    assert!(
        SsaSourceView::unpositioned(ssa)
            .read_contents_presence_alternatives_at(
                &SourceSite::source(Span::new(0, 2)),
                "$x",
                registry,
            )
            .is_none()
    );
}

#[test]
fn missing_read_completion_does_not_require_a_value_version() {
    use crate::var_resolve::VariableReadCompletion;

    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc p {known} {set copy $known; set copy $missing}",
        registry,
        false,
        "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut missing = false;
    let mut incoming = false;
    for (&block, data) in &ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in &tokens.variable_accesses {
                let completion =
                    view.read_completion_at(&access.source, &access.original_spelling, registry);
                match access.original_spelling.as_str() {
                    "$missing" => {
                        assert_eq!(completion, Some(VariableReadCompletion::Error));
                        assert_eq!(
                            view.read_contents_presence_at(
                                &access.source,
                                &access.original_spelling,
                                registry,
                            ),
                            Some(crate::var_resolve::ContentsPresence::Undefined)
                        );
                        assert!(
                            view.read_reference(&access.source, &access.original_spelling)
                                .is_none_or(|read| read.version.is_none())
                        );
                        missing = true;
                    }
                    "$known" => {
                        assert_eq!(completion, Some(VariableReadCompletion::Unknown));
                        incoming = true;
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(
        missing && incoming,
        "both actual read contexts are retained"
    );
    assert!(SsaSourceView::unpositioned(ssa).source_tokens().is_none());
}

#[test]
fn retained_word_read_success_is_separate_from_an_address_or_version() {
    for (profile, body, expected) in [
        ("tcl8.6", "set x 1; set copy $x", true),
        ("tcl8.6", "set copy $missing", false),
        ("tcl8.6", "array set a {k VALUE}; set copy $a", false),
        ("tcl8.6", "set a(k) VALUE; set copy $a(k)", true),
        ("jim", "set d odd; set copy $d(k)", false),
    ] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let source = format!("proc p {{}} {{{body}}}");
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            &source, registry, false, profile,
        );
        let ssa = &unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body")
            .ssa;
        let mut reached = false;
        for (&block, data) in &ssa.blocks {
            for index in 0..data.statements.len() {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for word in tokens.words() {
                    if word.sole_variable_substitution().is_some() {
                        assert_eq!(
                            view.read_word_produces_value(word, registry),
                            expected,
                            "{profile}: {body}"
                        );
                        reached = true;
                    }
                }
            }
        }
        assert!(reached, "actual original read retained: {profile}: {body}");
    }
}

#[test]
fn scalar_read_of_an_array_is_not_a_missing_contents_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc p {} {array set a {k VALUE}; puts $a}",
        registry,
        false,
        "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut reached = false;
    for (&block, data) in &ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in tokens
                .variable_accesses
                .iter()
                .filter(|read| read.original_spelling == "$a")
            {
                assert_eq!(
                    view.read_completion_at(&access.source, "$a", registry),
                    Some(crate::var_resolve::VariableReadCompletion::Error)
                );
                assert_eq!(
                    view.read_contents_presence_at(&access.source, "$a", registry),
                    Some(crate::var_resolve::ContentsPresence::Defined)
                );
                assert!(
                    view.read_reference(&access.source, "$a")
                        .is_none_or(|read| read.version.is_none()),
                    "a rejected scalar read cannot supply an SSA value"
                );
                reached = true;
            }
        }
    }
    assert!(reached, "the failing scalar read is retained");
}

#[test]
fn caught_missing_catalogue_command_does_not_hide_unknown_fallback_mutations() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
    let source = "set a old\ncatch {lassign {new second} a b} m\nputs $a\n";
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        source,
        registry,
        false,
        registry.profile().unwrap(),
    );
    let function = &cu.top_level;
    let point = function
        .cfg
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    matches!(statement, Statement::Call {command, ..} if command == "puts")
                        .then_some((block, index))
                })
        })
        .unwrap();
    let view = SsaSourceView::at_statement(&function.ssa, point.0, point.1);
    let read = view.read_spelling("a");
    let contexts = function.ssa.point_contexts.as_ref().unwrap();
    let context = contexts.before_statement(point.0, point.1);
    assert!(
        read.is_none_or(|read| read.version.is_none()),
        "read={read:?} constants={:?} dynamic_bindings={} dynamic_traces={} source_reads={} point_names={:?}",
        context.constant_values,
        context.dynamic_bindings,
        context.dynamic_traces,
        contexts.source_reads_at(point.0, point.1).len(),
        view.source_symbols().collect::<Vec<_>>(),
    );
}

#[test]
fn caught_error_only_fallback_preserves_the_reaching_cell_contents() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
    let source = "proc unknown {args} {return -code error absent}\nset a old\ncatch {lassign {new second} a b} m\nputs $a\n";
    let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
        source,
        registry,
        false,
        registry.profile().unwrap(),
    );
    let function = &cu.top_level;
    let read = function
        .cfg
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    matches!(statement, Statement::Call {command, ..} if command == "puts").then(
                        || {
                            SsaSourceView::at_statement(&function.ssa, block, index)
                                .read_spelling("a")
                        },
                    )
                })
        })
        .flatten();
    assert!(
        read.is_some_and(|read| read.version.is_some()),
        "read={read:?}"
    );
}

#[test]
fn scalar_clobber_keeps_contents_lineage_without_creating_a_store() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut function = crate::cfg::Function::new("::f", "entry");
    function
        .blocks
        .get_mut(&function.entry)
        .unwrap()
        .statements
        .push(assignment("x"));
    let mut ssa = build_ssa(&function, &registry);
    let symbol = *ssa.blocks[&function.entry].statements[0]
        .defs
        .keys()
        .next()
        .unwrap();
    let prior = ssa.blocks[&function.entry].statements[0].defs[&symbol];
    let fresh = prior + 1;
    ssa.value_clobbers.insert(
        function.entry,
        HashMap::from([(1, HashMap::from([(symbol, (prior, fresh))]))]),
    );
    let origins = represented_contents_origins(&ssa.blocks, &ssa.value_clobbers);
    assert_eq!(origins[&(symbol, fresh)], ContentsOrigin::WrittenAt(0));
    assert_eq!(ssa.binding_version(symbol, fresh), prior);
    assert!(
        !ssa.blocks[&function.entry]
            .statements
            .iter()
            .any(|statement| { statement.defs.get(&symbol) == Some(&fresh) })
    );
    // Destruction invalidates contents even though its predecessor has a
    // real store. A later scalar clobber cannot revive that old value.
    let statement = &mut ssa.blocks.get_mut(&function.entry).unwrap().statements[0];
    statement.destruction_defs.insert(symbol);
    let origins = represented_contents_origins(&ssa.blocks, &ssa.value_clobbers);
    assert_eq!(origins[&(symbol, fresh)], ContentsOrigin::Unknown);
}

#[test]
fn scalar_copy_cycles_defer_uncomputed_results_without_losing_the_store() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut function = crate::cfg::Function::new("::f", "entry");
    let mut store = assignment("x");
    let Statement::AssignConst { span, .. } = &mut store else {
        unreachable!();
    };
    *span = Span::new(5, 6);
    function
        .blocks
        .get_mut(&function.entry)
        .unwrap()
        .statements
        .push(store);
    let ssa = build_ssa(&function, &registry);
    let entry = ssa.blocks[&function.entry].clone();
    let (&symbol, &written) = entry.statements[0].defs.iter().next().unwrap();
    let empty = |name: &str| SsaBlock {
        name: name.to_owned(),
        phis: Vec::new(),
        statements: Vec::new(),
        entry_versions: HashMap::new(),
        exit_versions: HashMap::new(),
    };
    // Four unique definitions cycle through an analysis-only copy:
    // x1 = store(5), x3 = copy(x2), x2 = join(x1, x4, x4),
    // x4 = join(x1, x3). Both phis have an independent store edge.
    let mut blocks = HashMap::from([(function.entry, entry)]);
    for id in 1..8 {
        blocks.insert(BlockId(id), empty("retained"));
    }
    // Evaluate the header phi before the backedge phi under the actual
    // map walk so a premature Unknown would circulate around the cycle.
    let order: Vec<_> = blocks
        .keys()
        .copied()
        .filter(|id| *id != function.entry)
        .collect();
    let (header, backedge) = (order[0], order[1]);
    blocks.get_mut(&header).unwrap().phis.push(Phi {
        name: symbol,
        version: written + 1,
        incoming: HashMap::from([
            (BlockId(0), written),
            (BlockId(4), written + 3),
            (BlockId(6), written + 3),
        ]),
    });
    blocks.get_mut(&backedge).unwrap().phis.push(Phi {
        name: symbol,
        version: written + 3,
        incoming: HashMap::from([(BlockId(0), written), (BlockId(1), written + 2)]),
    });
    let copies = HashMap::from([(
        header,
        HashMap::from([(0, HashMap::from([(symbol, (written + 1, written + 2))]))]),
    )]);
    let origins = represented_contents_origins(&blocks, &copies);
    for version in written..=written + 3 {
        assert_eq!(origins[&(symbol, version)], ContentsOrigin::WrittenAt(5));
    }
    // An actually absent predecessor remains unknown, independently of
    // the deferred derived keys above.
    blocks.get_mut(&backedge).unwrap().phis[0]
        .incoming
        .insert(BlockId(1), written + 4);
    let origins = represented_contents_origins(&blocks, &copies);
    assert_eq!(origins[&(symbol, written + 3)], ContentsOrigin::Unknown);
    assert_eq!(origins[&(symbol, written + 1)], ContentsOrigin::Unknown);
    blocks.get_mut(&backedge).unwrap().phis[0]
        .incoming
        .insert(BlockId(1), written + 2);
    let copies = HashMap::from([(
        header,
        HashMap::from([(0, HashMap::from([(symbol, (written + 4, written + 2))]))]),
    )]);
    let origins = represented_contents_origins(&blocks, &copies);
    assert_eq!(origins[&(symbol, written + 2)], ContentsOrigin::Unknown);
    assert_eq!(origins[&(symbol, written + 3)], ContentsOrigin::Unknown);
}

#[test]
fn computed_value_store_keeps_its_exact_following_read_origin() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {} {\n set b [binary format a* hello]\n set u [string toupper $b]\n}";
    let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
    let function = unit.function("::f").unwrap();
    let origins = represented_contents_origins(&function.ssa.blocks, &function.ssa.value_clobbers);
    let mut checked = 0;
    for (&block, body) in &function.ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&function.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in &tokens.variable_accesses {
                if access.original_spelling != "$b" && access.original_spelling != "${b}" {
                    continue;
                }
                let target = source_read_place(access, registry);
                let read = view.read_reference(&access.source, &access.original_spelling);
                let origin = access
                    .variable_context
                    .read_contents_origin(&target, registry);
                assert!(
                    read.is_some_and(|read| read.version.is_some()),
                    "read={read:?} source_origin={origin:?} target={target:?} uses={:?} represented={origins:?} statement_span={:?}",
                    statement.uses,
                    statement.statement.span()
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1);
}

fn assignment(name: &str) -> Statement {
    Statement::AssignConst {
        span: Span::new(0, 1),
        name: name.to_owned(),
        name_braced: false,
        value: "10".to_owned(),
        value_span: None,
    }
}

fn variable_return(name: &str) -> Terminator {
    Terminator::Return {
        tokens: None,
        value: Some(format!("${name}")),
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        braced: false,
    }
}

#[test]
fn relative_qualified_write_and_absolute_read_share_symbol() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut cfg = cfg::Function::new("::N::p", "entry");
    let block = cfg.blocks.get_mut(&cfg.entry).expect("entry");
    block.statements.push(assignment("R::x"));
    block.terminator = Some(variable_return("::N::R::x"));
    let mut entry = ResolveContext::for_function(&cfg.name);
    entry.known_namespaces.insert("::N::R".to_owned());
    entry.namespace_cells.present.insert("::N::R::x".to_owned());
    let ssa = build_ssa_with_context(
        &cfg,
        registry,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        entry,
    );
    let relative = ssa
        .var_symbol_at(cfg.entry, 0, "R::x")
        .expect("bound relative access");
    assert_eq!(Some(relative), ssa.var_symbol("::N::R::x"));
    assert!(
        ssa.blocks[&cfg.entry].statements[0]
            .defs
            .contains_key(&relative)
    );
}

#[test]
fn top_level_global_and_bare_spelling_share_identity_and_keep_display_name() {
    let registry = CommandRegistry::build_default();
    let mut cfg = cfg::Function::new("::top", "entry");
    let block = cfg.blocks.get_mut(&cfg.entry).expect("entry");
    block.statements.push(assignment("g"));
    block.terminator = Some(variable_return("::g"));
    let ssa = build_ssa(&cfg, &registry);
    let symbol = ssa.var_symbol("g").expect("source name");
    assert_eq!(Some(symbol), ssa.var_symbol("::g"));
    assert_eq!(ssa.var_name(symbol), "g");
}
#[test]
fn scalar_alias_write_after_unset_reaches_both_spellings() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source = "proc p {} {set x OLD; upvar 0 x y; unset x; set y NEW; puts $x; puts $y}; p";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl9.0",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut written = None;
    let mut read = Vec::new();
    for (&block, body) in &ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            match &statement.statement {
                operation
                    if source
                        .get(operation.span().start() as usize..operation.span().end() as usize)
                        .is_some_and(|text| text.starts_with("set y NEW")) =>
                {
                    written = ssa.var_symbol_at(block, index, "y").and_then(|symbol| {
                        statement
                            .defs
                            .get(&symbol)
                            .map(|&version| (symbol, version))
                    });
                }
                Statement::Call { args, .. }
                    if args.iter().any(|arg| {
                        arg == "${x}" || arg == "${y}" || arg == "$x" || arg == "$y"
                    }) =>
                {
                    read.push((block, index, statement.uses.clone()));
                }
                _ => {}
            }
        }
    }
    let (symbol, version) =
        written.unwrap_or_else(|| panic!("alias store contents definition: {}", ssa.outline()));
    assert_eq!(read.len(), 2, "both puts reads: {}", ssa.outline());
    assert!(
        read.iter()
            .all(|(_, _, uses)| uses.get(&symbol) == Some(&version)),
        "scalar alias store must reach both spellings: {}",
        ssa.outline()
    );
}
#[test]
fn exact_reads_keep_alias_retargeting_within_one_word_separate() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc p {} {set x OLD; set y NEW; upvar 0 x a; puts \"$a[upvar 0 y a]$a\"}; p";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut pairs = Vec::new();
    for (&block, body) in &ssa.blocks {
        for index in 0..body.statements.len() {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            let reads: Vec<_> = tokens
                .variable_accesses
                .iter()
                .filter(|access| access.original_spelling == "$a")
                .filter_map(|access| view.read_reference(&access.source, &access.original_spelling))
                .collect();
            if reads.len() == 2 {
                assert!(view.is_positioned());
                assert!(view.read_spelling("a").is_none());
                assert!(
                    view.symbol("a").is_none(),
                    "one source name cannot select two cells"
                );
                pairs.push(reads);
            }
        }
    }
    assert_eq!(pairs.len(), 1, "{}", ssa.outline());
    assert_ne!(pairs[0][0].symbol, pairs[0][1].symbol);
    assert!(pairs[0].iter().all(|read| read.version.is_some()));
}

#[test]
fn incoming_slot_projection_keeps_distinct_call_activations() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc p {n} {return [expr {$n + 0}]}; p 20; p 21";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let node = tcl_syntax::expr::parser::parse_expr("$n", Some("tcl8.6"));
    let base = Some(u32::try_from(source.find("$n").unwrap()).unwrap());
    let mut proofs = Vec::new();
    for (&block, data) in &ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(ssa, block, index);
            if let Some(proof) = view.read_expression_incoming_slot(&node, base, "n", registry) {
                proofs.push(proof);
            }
        }
    }
    assert!(
        !proofs.is_empty(),
        "the reached ordinary formal has an input proof"
    );
    assert!(proofs.iter().all(|proof| proof.slot == "n"));
    assert!(
        proofs.iter().any(|proof| proof.cells.len() >= 2),
        "separate callers retain separate physical owners: {proofs:?}"
    );
}

#[test]
fn incoming_slot_projection_rejects_a_formal_rebound_to_global_storage() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "set n 100; proc p {n} {unset n; global n; return [expr {$n + 0}]}; p 20; p 21";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let node = tcl_syntax::expr::parser::parse_expr("$n", Some("tcl8.6"));
    let base = Some(u32::try_from(source.find("$n").unwrap()).unwrap());
    let mut reached = false;
    for (&block, data) in &ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(ssa, block, index);
            reached |= view.expression_variable_access(&node, base).is_some();
            assert!(
                view.read_expression_incoming_slot(&node, base, "n", registry)
                    .is_none()
            );
        }
    }
    assert!(
        reached,
        "the global read is retained but is not a formal input"
    );
}

fn iteration_read_summary(
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> String {
    let mut original = Vec::new();
    for (&block, data) in &function.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&function.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in tokens
                .variable_accesses
                .iter()
                .filter(|access| access.original_spelling == "$l")
            {
                let place = source_read_place(access, registry);
                original.push((
                    block,
                    index,
                    tokens.synthetic,
                    &statement.defs,
                    &statement.uses,
                    access.source.span,
                    place.clone(),
                    access
                        .variable_context
                        .read_contents_origin(&place, registry),
                    view.read_reference(&access.source, &access.original_spelling),
                ));
            }
        }
    }
    format!("{original:?}")
}

#[test]
fn entered_iteration_store_provides_the_body_read_version() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    // The stock outer List closes iteration callbacks without normalising
    // its unknown element before the retained original body read.
    let source = "proc p {item} {set l [list $item]; foreach x $l {set y [expr {$x + 1}]}}";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut reads = Vec::new();
    let mut inventory = Vec::new();
    for (&block, data) in &ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            inventory.push((
                block,
                index,
                tokens.argv_texts.first().cloned(),
                tokens.source_binding.as_ref().map(|binding| {
                    (
                        binding.unknown,
                        binding.may_be_absent,
                        binding
                            .targets
                            .iter()
                            .map(|target| target.command.as_str())
                            .collect::<Vec<_>>(),
                    )
                }),
                tokens
                    .nested_bindings
                    .iter()
                    .map(|(site, binding)| {
                        (
                            site,
                            binding.unknown,
                            binding.may_be_absent,
                            binding
                                .targets
                                .iter()
                                .map(|target| target.command.as_str())
                                .collect::<Vec<_>>(),
                            binding.compiled_execution_residual,
                        )
                    })
                    .collect::<Vec<_>>(),
                tokens
                    .variable_accesses
                    .iter()
                    .map(|access| (&access.source, &access.original_spelling))
                    .collect::<Vec<_>>(),
            ));
            for access in tokens
                .variable_accesses
                .iter()
                .filter(|access| source_read_name(access) == "x")
            {
                let place = source_read_place(access, registry);
                let origin = access
                    .variable_context
                    .read_contents_origin(&place, registry);
                let read = view.read_reference(&access.source, &access.original_spelling);
                assert!(
                    read.is_some_and(|read| read.version.is_some_and(|version| version > 0)),
                    "entered iterator value has a represented store: place={place:?}, origin={origin:?}, read={read:?}"
                );
                reads.push(read.unwrap());
            }
        }
    }
    assert!(
        !reads.is_empty(),
        "the reached expression read is retained: {inventory:?}"
    );
}

#[test]
fn iteration_input_reference_keeps_preloop_version_when_body_mutates_list() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc p {} {set l BEFORE; foreach x $l {set l AFTER}; return $x}; p";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body");
    let mut cached = Vec::new();
    for (&block, data) in &function.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            if statement.statement.tokens().is_none_or(|tokens| {
                !matches!(
                    tokens.synthetic,
                    Some(crate::ir::SyntheticMarker::IterationBindings(_))
                )
            }) {
                continue;
            }
            assert!(function.cfg.source_tokens_at(block, index).is_none());
            assert!(function.cfg.source_tokens_at(block, usize::MAX).is_none());
            let input = function.cfg.source_input_tokens_at(block, index).unwrap();
            let view = SsaSourceView::at_statement(&function.ssa, block, index);
            cached.push(view.read_word(&input.words()[1]).unwrap_or_else(|| {
                    let accesses: Vec<_> = input
                        .variable_accesses
                        .iter()
                        .map(|access| {
                            (
                                access.source.span,
                                &access.original_spelling,
                                source_read_place(access, registry),
                                view.read_reference(&access.source, &access.original_spelling),
                            )
                        })
                        .collect();
                    let original = iteration_read_summary(function, registry);
                    panic!(
                        "original captured input read: word={:?}, accesses={accesses:?}, evaluated={original:?}",
                        input.words()[1]
                    )
                }));
            assert!(
                !statement.uses.contains_key(&cached.last().unwrap().symbol),
                "cached list is not reevaluated"
            );
        }
    }
    assert_eq!(cached.len(), 1);
    for (&block, data) in &function.cfg.blocks {
        if matches!(data.terminator, Some(Terminator::Goto { .. })) {
            assert!(function.cfg.source_tokens_at(block, usize::MAX).is_none());
            assert_eq!(
                function
                    .ssa
                    .point_contexts
                    .as_ref()
                    .unwrap()
                    .source_reads_at(block, usize::MAX),
                [] as [crate::command_binding::SourceVariableAccess; 0]
            );
        }
    }
    let version = cached[0].version.expect("represented preloop definition");
    let (block, index, _) = function
        .ssa
        .blocks
        .iter()
        .flat_map(|(&block, data)| {
            data.statements
                .iter()
                .enumerate()
                .map(move |(index, statement)| (block, index, statement))
        })
        .find(|(_, _, statement)| statement.defs.get(&cached[0].symbol) == Some(&version))
        .unwrap();
    let source = SsaSourceView::at_statement(&function.ssa, block, index);
    assert!(
        matches!(source.source_tokens().and_then(|tokens| tokens.words().get(2)),
            Some(crate::ir::WordExpr::Literal { text, .. }) if text == "BEFORE")
    );
}

#[test]
fn terminator_read_consensus_declines_retargeted_source_spelling() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc p {} {set x OLD; set y NEW; upvar 0 x a; return \"$a[upvar 0 y a]$a\"}; p";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut pairs = Vec::new();
    for &block in ssa.blocks.keys() {
        let view = SsaSourceView::at_terminator(ssa, block);
        let Some(tokens) = view.source_tokens() else {
            continue;
        };
        let reads: Vec<_> = tokens
            .variable_accesses
            .iter()
            .filter(|access| access.original_spelling == "$a")
            .filter_map(|access| view.read_reference(&access.source, &access.original_spelling))
            .collect();
        if reads.len() == 2 {
            assert!(view.symbol("a").is_none());
            assert!(view.read_spelling("a").is_none());
            pairs.push(reads);
        }
    }
    assert_eq!(pairs.len(), 1);
    assert_ne!(pairs[0][0].symbol, pairs[0][1].symbol);
    assert!(pairs[0].iter().all(|read| read.version.is_some()));
}

#[test]
fn reaching_binding_resolves_contents_at_a_goto_without_inventing_a_read() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "proc a {} {set j 0; for {set k 5} {$k > 0} {incr k -1} {incr j}}",
        registry,
        false,
        registry.profile().unwrap(),
    );
    let function = unit.function("::a").unwrap();
    let node = function.cfg.loop_nodes.values().next().unwrap();
    let view = SsaSourceView::at_terminator(&function.ssa, node.entry_block);
    assert!(view.source_symbols().next().is_none());
    let reaching = view.reaching_binding("j", registry).unwrap();
    let version = reaching.version.expect("the literal store is represented");
    assert_eq!(
        function.sccp.values.get(&(reaching.symbol, version)),
        Some(&crate::analyses::LatticeValue::Const(
            crate::analyses::ConstValue::Int(0)
        )),
    );
    assert!(view.reaching_binding("missing", registry).is_none());
    assert!(
        SsaSourceView::unpositioned(&function.ssa)
            .reaching_binding("j", registry)
            .is_none()
    );
    assert!(
        SsaSourceView::at_terminator(&function.ssa, BlockId(u32::MAX))
            .reaching_binding("j", registry)
            .is_none()
    );
}

#[test]
fn reaching_binding_declines_contents_after_an_unrepresented_mutation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "proc a {cmd} {set x OLD; $cmd}",
        registry,
        false,
        registry.profile().unwrap(),
    );
    let function = unit.function("::a").unwrap();
    let block = function
        .ssa
        .blocks
        .iter()
        .find(|(_, block)| {
            block.statements.iter().any(|statement| {
                statement
                    .statement
                    .tokens()
                    .and_then(|tokens| tokens.words().first())
                    .is_some_and(|head| matches!(head, crate::ir::WordExpr::Variable { .. }))
            })
        })
        .unwrap()
        .0;
    let view = SsaSourceView::at_terminator(&function.ssa, *block);
    assert!(
        view.reaching_binding("x", registry)
            .is_none_or(|reaching| reaching.version.is_none()),
        "unknown contents cannot borrow the prior store",
    );
}

#[test]
fn nested_contents_store_does_not_borrow_an_earlier_ssa_version() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc p {} {set x OLD; puts \"$x[set x NEW]$x\"}; p";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut pairs = Vec::new();
    for (&block, body) in &ssa.blocks {
        for index in 0..body.statements.len() {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            let reads: Vec<_> = tokens
                .variable_accesses
                .iter()
                .filter(|access| access.original_spelling == "$x")
                .filter_map(|access| view.read_reference(&access.source, &access.original_spelling))
                .collect();
            if reads.len() == 2 {
                assert!(view.read_spelling("x").is_none());
                pairs.push(reads);
            }
        }
    }
    assert_eq!(pairs.len(), 1, "{}", ssa.outline());
    assert_eq!(pairs[0][0].symbol, pairs[0][1].symbol);
    assert!(pairs[0][0].version.is_some());
    assert!(pairs[0][1].version.is_none());
}

#[test]
fn name_only_read_projection_requires_reached_consensus() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc p {} {set x OLD; puts \"$x$x\"}; p",
        registry,
        false,
        "tcl8.6",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut projected = Vec::new();
    for (&block, body) in &ssa.blocks {
        for index in 0..body.statements.len() {
            let view = SsaSourceView::at_statement(ssa, block, index);
            assert!(view.read_spelling("unreached").is_none());
            if let Some(read) = view.read_spelling("x") {
                assert!(read.version.is_some());
                projected.push(read);
            }
        }
    }
    assert_eq!(projected.len(), 1);
    assert!(!SsaSourceView::unpositioned(ssa).is_positioned());
    assert!(
        SsaSourceView::unpositioned(ssa)
            .read_spelling("x")
            .is_none()
    );
}

#[test]
fn computed_command_head_keeps_its_captured_read_inside_dictionary_iteration() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc demo {} {\n    set x set\n    set d [dict create a true b false c true]\n    dict for {key value} $d {\n        if {$value} {\n            $x a $key\n        }\n    }\n}\n",
        registry,
        false,
    );
    let ssa = &unit.function("::demo").unwrap().ssa;
    let mut inventory = Vec::new();
    let mut reads = Vec::new();
    for (&block, body) in &ssa.blocks {
        for index in 0..body.statements.len() {
            let view = SsaSourceView::at_statement(ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            inventory.push((
                tokens.words().first().map(WordExpr::legacy_text),
                tokens
                    .variable_accesses
                    .iter()
                    .map(|access| access.original_spelling.clone())
                    .collect::<Vec<_>>(),
            ));
            for access in tokens
                .variable_accesses
                .iter()
                .filter(|access| source_read_name(access) == "x")
            {
                reads.push(view.read_reference(&access.source, &access.original_spelling));
            }
        }
    }
    assert!(
        reads.iter().any(|read| {
            read.is_some_and(|read| read.version.is_some_and(|version| version > 0))
        }),
        "computed command-head reads retain their actual producer: reads={reads:?}, inventory={inventory:?}"
    );
}
