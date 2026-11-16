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

//! The solver over hand-built SSA: constants, phi joins, folded-type refinement and parameters.

use super::*;

/// A definition the static typing knows nothing of takes the type its
/// evaluation proved — the stated intrep, else the one the route
/// constructed — and a known static type stands, element facts and all.
#[test]
fn a_folded_type_refines_only_what_the_static_typing_leaves_unknown() {
    use crate::value_transfer::FoldedType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let stated = FoldedType {
        intrep: Some(TclType::String),
        shape: None,
        representation: RepresentationEvidence::Unknown,
    };
    let built = FoldedType {
        intrep: None,
        shape: None,
        representation: RepresentationEvidence::Constructed(TclType::ByteArray),
    };
    assert_eq!(
        refined_by_folded_type(TypeLattice::overdefined(), Some(&stated)),
        TypeLattice::of(TclType::String)
    );
    assert_eq!(
        refined_by_folded_type(TypeLattice::unknown(), Some(&built)),
        TypeLattice::of(TclType::ByteArray)
    );
    assert_eq!(
        refined_by_folded_type(TypeLattice::of(TclType::List), Some(&stated)),
        TypeLattice::of(TclType::List),
        "a known static type stands"
    );
    assert_eq!(
        refined_by_folded_type(TypeLattice::overdefined(), None),
        TypeLattice::overdefined()
    );
}

fn empty_sccp(f: &Function, blocks: &[&str]) -> SccpResult {
    SccpResult {
        required_math_invocations: Vec::new(),
        required_expression_preparations: Vec::new(),
        explanations: Vec::new(),
        route_tally: crate::value_transfer::RouteTally::default(),
        folded_types: HashMap::new(),
        preserved: HashMap::new(),
        raised: HashSet::default(),
        template_plans: Vec::new(),
        selections: Vec::new(),
        existence: HashMap::new(),
        existence_reads: HashMap::new(),
        existence_exits: HashMap::new(),
        existence_entries: HashMap::new(),
        refinements: Vec::new(),
        refinements_at: HashMap::new(),
        value_entries: HashMap::new(),
        query_places: Vec::new(),
        existence_guards: Vec::new(),
        loop_enumerations: Vec::new(),
        reads_module: false,
        completion: crate::sccp::RunCompletion::default(),
        values: HashMap::new(),
        executable_blocks: blocks
            .iter()
            .map(|n| f.block_id(n).expect("interned block"))
            .collect(),
        executable_edges: HashSet::default(),
        constant_branches: Vec::new(),
    }
}

fn assign_const(name: &str, value: &str) -> Statement {
    Statement::AssignConst {
        span: Span::new(0, 0),
        name: name.to_owned(),
        value: value.to_owned(),
        name_braced: false,
        value_span: None,
    }
}

fn make_ssa_stmt(ssa: &mut SsaFunction, stmt: Statement, defs: &[(&str, u32)]) -> SsaStatement {
    SsaStatement {
        statement: stmt,
        uses: HashMap::new(),
        defs: defs.iter().map(|&(n, v)| (ssa.intern_var(n), v)).collect(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn integer_literal_infers_int() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    f.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(assign_const("x", "42"));

    let sccp = empty_sccp(&f, &["entry"]);
    let mut ssa = SsaFunction::trivial("::top", entry, f.block_names().to_vec());
    let stmt = make_ssa_stmt(&mut ssa, assign_const("x", "42"), &[("x", 1)]);
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let x = ssa.var_symbol("x").unwrap();
    let types = propagate_types(
        &f,
        &ssa,
        &sccp,
        &registry(),
        &HashSet::new(),
        &HashSet::new(),
        crate::compilation_unit::ModuleTraceFacts::none(),
    );
    assert_eq!(types.get(&(x, 1)), Some(&TypeLattice::of(TclType::Int)));
}

#[test]
fn phi_joins_types_from_executable_preds() {
    // A minimal two-block CFG: entry → exit.
    // The phi in exit merges version 1 (INT) from entry.
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let exit = cfg.intern_block("exit");
    cfg.blocks.insert(exit, Block::new("exit"));
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(crate::cfg::Terminator::Goto {
        target: exit,
        span: None,
    });

    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let x = ssa.intern_var("x");
    let phi = Phi {
        name: x,
        version: 2,
        incoming: [(entry, 1u32)].into_iter().collect(),
    };
    let entry_stmt = make_ssa_stmt(&mut ssa, assign_const("x", "10"), &[("x", 1)]);
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![entry_stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    ssa.blocks.insert(
        exit,
        SsaBlock {
            name: "exit".into(),
            phis: vec![phi],
            statements: Vec::new(),
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let mut sccp = empty_sccp(&cfg, &["entry", "exit"]);
    sccp.executable_edges.insert((entry, exit));

    let types = propagate_types(
        &cfg,
        &ssa,
        &sccp,
        &registry(),
        &HashSet::new(),
        &HashSet::new(),
        crate::compilation_unit::ModuleTraceFacts::none(),
    );
    // x@1 (entry) should be Int.
    assert_eq!(types.get(&(x, 1)), Some(&TypeLattice::of(TclType::Int)));
    // x@2 (phi in exit) should propagate Int from entry.
    assert_eq!(types.get(&(x, 2)), Some(&TypeLattice::of(TclType::Int)));
}

#[test]
fn conditionally_assigned_param_merges_to_overdefined() {
    use crate::compilation_unit::CompilationUnit;
    // A parameter assigned in only one arm must NOT be
    // typed from that arm alone. The merge phi joins the live-in
    // (caller-supplied, unknown → OVERDEFINED) with the assigned-arm Int,
    // so the merged type is OVERDEFINED — never Known Int.
    let cu = CompilationUnit::build_for(
        "proc ::p {c x} { if {$c} { set x 5 }\n return $x }",
        &registry(),
        false,
    );
    let fu = cu.function("::p").unwrap();
    // (TP) The merge phi for `x` is OVERDEFINED.
    let has_overdefined = fu
        .types
        .iter()
        .any(|((n, _), t)| fu.ssa.var_name(*n) == "x" && t.kind() == TypeKind::Overdefined);
    assert!(
        has_overdefined,
        "conditionally-assigned param 'x' should merge to OVERDEFINED: {:?}",
        fu.types
    );
    // (FP guard) The assigned arm (`set x 5`) is *still* typed Known Int —
    // the fix widens only the merge, not the definite assignment. Exactly
    // one `x` version is Known Int (the assigned arm) and one is
    // OVERDEFINED (the phi); pre-fix, the phi would also be Known Int.
    let known_int_count = fu
        .types
        .iter()
        .filter(|((n, _), t)| {
            fu.ssa.var_name(*n) == "x" && matches!(t.tcl_type(), Some(TclType::Int))
        })
        .count();
    assert_eq!(
        known_int_count, 1,
        "only the assigned arm of 'x' should be Known Int (not the phi): {:?}",
        fu.types
    );
}

#[test]
fn unconditionally_assigned_param_still_typed() {
    use crate::compilation_unit::CompilationUnit;
    // (FP guard): an *unconditionally* reassigned local is
    // still typed from its definition — the version-0 join only applies
    // when a live-in genuinely reaches the merge. Here `y` is set on every
    // path, so its post-assignment type stays Known Int.
    let cu = CompilationUnit::build_for(
        "proc ::q {c} { set y 5\n if {$c} { set y 7 } else { set y 9 }\n return $y }",
        &registry(),
        false,
    );
    let fu = cu.function("::q").unwrap();
    let all_int = fu
        .types
        .iter()
        .filter(|((n, _), _)| fu.ssa.var_name(*n) == "y")
        .all(|(_, t)| matches!(t.tcl_type(), Some(TclType::Int)) || t.kind() == TypeKind::Unknown);
    assert!(
        all_int,
        "unconditionally-assigned 'y' should stay Int on every path: {:?}",
        fu.types
    );
}
