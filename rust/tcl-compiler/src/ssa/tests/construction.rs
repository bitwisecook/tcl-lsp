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

//! `build_ssa` end to end: linear code, diamonds, loops, setter reads, the complexity guard and destruction predecessors.

use super::*;

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
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
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
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
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
