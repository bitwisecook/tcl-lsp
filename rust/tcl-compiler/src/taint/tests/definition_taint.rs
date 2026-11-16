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

//! What a statement gives its definitions: `incr`, alias prefixes, destructions, constants, sources, summaries and transforms.

use super::*;

#[test]
fn incr_taint_uses_the_selected_definition_name_without_reparsing_data() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let cfg = crate::cfg::Function::new("model", "entry");
    let mut ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
    let literal = ssa.intern_var("$literal");
    let stripped = ssa.intern_var("literal");
    let uses = HashMap::from([(literal, 1), (stripped, 1)]);
    let taints = HashMap::from([
        ((literal, 1), TaintLattice::tainted()),
        ((stripped, 1), TaintLattice::clean()),
    ]);
    let ctx = TaintCtx {
        registry,
        ssa: &ssa,
        interproc: None,
        known_procs: None,
        dialect: registry.profile(),
        source: TaintSourceContext::standalone(registry, registry.profile()),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };
    let statement = Statement::Incr {
        span: Span::new(0, 0),
        name: "$literal".into(),
        name_braced: true,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    assert!(evaluate_taint_def(&statement, literal, &uses, &taints, ctx, &ssa).is_tainted());
    assert!(
        evaluate_taint_def(&statement, literal, &HashMap::new(), &taints, ctx, &ssa).is_tainted()
    );
}

#[test]
fn unrepresented_variable_contents_have_no_clean_taint_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "set represented SAFE",
        registry,
        false,
    );
    let symbol = unit.top_level.ssa.var_symbol("represented").unwrap();
    let uses = HashMap::from([(symbol, 0)]);
    let taints = HashMap::<ValueKey, TaintLattice>::new();
    assert!(var_taint("unrepresented", &uses, &taints, &unit.top_level.ssa).is_tainted());
    assert!(!var_taint("represented", &uses, &taints, &unit.top_level.ssa).is_tainted());
    assert!(var_taint("represented", &HashMap::new(), &taints, &unit.top_level.ssa).is_tainted());
    assert!(unit.top_level.ssa.var_symbol("unrepresented").is_none());
}

#[test]
fn alias_prefix_values_and_braced_literals_do_not_resubstitute_variables() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for source in [
        "interp alias {} collect {} list {$attacker}\nset value [collect]",
        "set value [list {$attacker}]",
    ] {
        let module = crate::lowering::lower_to_ir(source, registry);
        let (value, tokens) = module
            .top_level
            .statements
            .iter()
            .rev()
            .find_map(|statement| {
                if let Statement::AssignValue {
                    value,
                    tokens: Some(tokens),
                    ..
                } = statement
                {
                    Some((value, tokens))
                } else {
                    None
                }
            })
            .expect("assignment carrier");
        let cfg = crate::cfg::Function::new("::top", "entry");
        let mut ssa =
            crate::ssa::SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        let attacker = ssa.intern_var("attacker");
        let ctx = TaintCtx {
            registry,
            ssa: &ssa,
            interproc: None,
            known_procs: None,
            dialect: registry.profile(),
            source: TaintSourceContext::standalone(registry, registry.profile()),
            taint_summaries: None,
            instance_classes: None,
            source_position: Some(tokens.argv[0].start()),
            source_tokens: Some(tokens),
            word_offset: tokens.argv.last().map(|span| span.start()),
            source_word: tokens.word_exprs.last(),
        };
        let uses = HashMap::from([(attacker, 1)]);
        let taints = HashMap::from([((attacker, 1), TaintLattice::tainted())]);
        assert!(
            !word_taint(value, &uses, &taints, ctx).is_tainted(),
            "{source}"
        );
    }
}

#[test]
fn destruction_versions_preserve_predecessor_taint_without_sanitising() {
    use crate::cfg::Function;
    use crate::ssa::{SsaBlock, SsaFunction};
    let registry = CommandRegistry::build_default();
    let cfg = Function::new("::top", "entry");
    let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
    let symbol = ssa.intern_var("payload");
    let ctx = TaintCtx {
        registry: &registry,
        ssa: &ssa,
        interproc: None,
        known_procs: None,
        dialect: None,
        source: TaintSourceContext::standalone(&registry, None),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };
    for conditional in [false, true] {
        for predecessor in [
            TaintLattice::clean(),
            TaintLattice::tainted(),
            TaintLattice::tainted().with(TaintColour::CRLF_FREE),
        ] {
            let mut statement = ssa_stmt(
                call_stmt("unset", &["payload"]),
                HashMap::from([(symbol, 1)]),
                HashMap::from([(symbol, 2)]),
            );
            statement.destruction_defs.insert(symbol);
            if conditional {
                statement.may_defs.insert(symbol);
            }
            let block = SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![statement],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            };
            let mut taints = HashMap::from([((symbol, 1), predecessor)]);
            propagate_statement_taints(&mut taints, cfg.entry, &block, ctx, &ssa, None);
            assert_eq!(
                taints[&(symbol, 2)],
                predecessor,
                "conditional={conditional}: unset supplies neither cleaning nor new mitigation"
            );
            let uses = HashMap::from([(symbol, 2)]);
            assert_eq!(
                word_taint("$payload", &uses, &taints, ctx),
                predecessor,
                "the downstream script operand keeps its possible predecessor influence"
            );
        }
    }
}

#[test]
fn const_assignment_is_not_tainted() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    let stmt = Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "hello".into(),
        value_span: None,
    };
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(stmt.clone());
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let x = ssa.intern_var("x");
    let ssa_stmt = ssa_stmt(stmt, HashMap::new(), [(x, 1u32)].into_iter().collect());
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    assert!(
        taints.get(&(x, 1)).is_none_or(|t| !t.is_tainted()),
        "constant assignment should not be tainted"
    );
}

/// Actual F5 getter properties survive an optional diagnostic dialect
/// filter; an unloaded catalogue spelling supplies no security properties.
#[test]
fn irules_http_uri_is_a_dialect_agnostic_source() {
    use crate::compilation_unit::CompilationUnit;

    let profile = tcl_dialect::DialectProfile::irules();
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    let registry = registry.project_for_profile(profile);
    for dialect in [None, Some(profile)] {
        let cu = CompilationUnit::build_for_profile("set u [HTTP::uri]", &registry, false, profile)
            .with_interprocedural(&registry, dialect);
        let fu = cu.function("::top").unwrap();
        let u = fu
            .taints
            .iter()
            .find(|((n, _), _)| fu.ssa.var_name(*n) == "u")
            .map(|(_, t)| *t);
        assert!(
            u.is_some_and(TaintLattice::is_tainted),
            "HTTP::uri should be a taint source (dialect={dialect:?})",
        );
        assert!(
            u.is_some_and(|t| t.colours.contains(TaintColour::PATH_PREFIXED)),
            "HTTP::uri getter should carry PATH_PREFIXED (dialect={dialect:?})",
        );
    }
    let plain = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for("set u [HTTP::uri]", &plain, false);
    assert!(
        unit.top_level
            .taints
            .values()
            .all(|taint| { !taint.colours.contains(TaintColour::PATH_PREFIXED) }),
        "an unloaded F5 name must not donate getter safety properties"
    );
}

/// Inter-procedural: `proc id {x} { return $x }` + tainted actual
/// should taint the call's result because the passthrough parameter
/// forwards the tainted value through the proc.
#[test]
fn interprocedural_passthrough_transfers_taint() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc ::id {x} { return $x }\n\
             set tainted [gets stdin]\n\
             set out [::id $tainted]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let fu = cu.function("::top").unwrap();
    assert!(
        fu.taints
            .iter()
            .any(|((n, _), t)| fu.ssa.var_name(*n) == "out" && t.is_tainted()),
        "expected 'out' to be tainted via passthrough proc: {:?}",
        fu.taints,
    );
}

#[test]
fn file_join_transform_reaches_the_live_cfg_taint_lattice() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let colour_for = |source: &str, name: &str| {
        let cu = CompilationUnit::build_for(source, &registry, false);
        let fu = cu.function("::top").expect("top-level CFG built");
        fu.taints
            .iter()
            .find(|((symbol, _), _)| fu.ssa.var_name(*symbol) == name)
            .map(|(_, taint)| *taint)
            .expect("assignment has a live SSA taint value")
    };

    let joined = colour_for(
        "set raw [gets stdin]\nset joined [file join /base $raw]",
        "joined",
    );
    assert!(joined.is_tainted(), "the source taint must reach file join");
    assert!(
        joined.colours.contains(TaintColour::PATH_JOINED),
        "file join's registry transform must reach the CFG taint lattice: {joined:?}"
    );

    // Mutation control: replacing the transform-bearing `file join` with
    // a same-shape non-transform command removes the only PATH_JOINED
    // producer. If `word_taint_at` stopped applying registry transforms,
    // this and the live vector above would become indistinguishable.
    let without_transform = colour_for(
        "set raw [gets stdin]\nset joined [file tail $raw]",
        "joined",
    );
    assert!(without_transform.is_tainted());
    assert!(
        !without_transform.colours.contains(TaintColour::PATH_JOINED),
        "the mutation control must not inherit file join's transform: {without_transform:?}"
    );
}
