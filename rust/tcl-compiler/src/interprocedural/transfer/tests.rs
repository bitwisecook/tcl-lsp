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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

use super::*;
use crate::compilation_unit::CompilationUnit;

fn name(outcomes: &[ExistenceOutcome]) -> ParamRole {
    ParamRole::Name {
        level: FrameLevel::Relative(1),
        outcomes: outcomes.to_vec(),
    }
}

const BIND: ExistenceOutcome = ExistenceOutcome::Bind(BindingKind::Scalar);

/// The transfer summaries the build of `source` computes under the
/// `environment` profile.
fn summaries(source: &str, environment: &str) -> HashMap<String, TransferSummary> {
    let (_, unit) = crate::interprocedural::logical_completion_unit(source, environment);
    fixture_transfer_labels(&unit.ir_module, unit.transfers.0)
}

fn fixture_transfer_labels(
    module: &crate::ir::Module,
    rows: HashMap<crate::ir::ProcedureId, TransferSummary>,
) -> HashMap<String, TransferSummary> {
    let mut shown = HashMap::new();
    for (id, summary) in rows {
        let body = module
            .procedures
            .get(&id)
            .expect("actual transfer fixture body");
        assert!(
            shown.insert(body.qualified_name.clone(), summary).is_none(),
            "authored transfer fixture labels must be unique"
        );
    }
    shown
}

/// The summaries of `source` with a cycle given at most `rounds` rounds.
fn summaries_within(source: &str, rounds: RecursionLimit) -> HashMap<String, TransferSummary> {
    let (context_owner, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
    let registry = context_owner.commands();
    let context =
        crate::value_transfer::AnalysisContextKey::for_module(&unit.command_mutations, registry);
    let summaries = ModuleProcedures::with_rounds(
        ModuleInputs {
            ir: &unit.ir_module,
            cfg: &unit.cfg_module,
            registry,
            mutations: &unit.command_mutations,
            projection: &unit.caller_scope.proc_binding_trust,
            trace: crate::compilation_unit::ModuleTraceFacts::of(&unit.ir_module),
            config: unit.ir_module.lexer_config,
            analysis_context: &context,
        },
        rounds,
    )
    .into_summaries()
    .0;
    fixture_transfer_labels(&unit.ir_module, summaries)
}

const CALLEES: &str = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n\
        proc reset {name} {upvar 1 $name v; unset v}\n\
        proc twice {name} {upvar 1 $name w; bump w; bump w}\n\
        proc renew {name} {upvar 1 $name x; reset x; bump x}\n\
        proc clear {name} {upvar 1 $name y; bump y; reset y}\n\
        proc maybe {name c} {upvar 1 $name v; if {$c} {set v 1}}\n\
        proc ensure {name} {upvar 1 $name v; if {![info exists v]} {set v 0}}\n\
        proc peek {name} {upvar 1 $name v; return $v}\n";

/// A summary composes its callees' outcomes in the order the body
/// applies them: `twice`'s place is the two `bump` updates, a bind;
/// `reset` then `bump` binds and `bump` then `reset` unbinds. Each
/// procedure's two runs — the place bound on entry, then unbound — tell
/// a may-write (`maybe`) from a write whatever the prior (`ensure`), and
/// a place the body only reads is preserved. The composition is the
/// same under every release: `incr` of an unset variable raises in 8.4
/// and creates it from 8.5, and either way a normal completion leaves
/// the place bound.
#[test]
fn summaries_compose_through_two_callees() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let summaries = summaries(CALLEES, environment);
        let params = |qname: &str| -> Vec<ParamRole> {
            summaries
                .get(qname)
                .unwrap_or_else(|| panic!("{environment}: {qname} has a summary"))
                .params
                .clone()
        };
        assert_eq!(
            params("::bump"),
            [name(&[BIND]), ParamRole::Value],
            "{environment}"
        );
        assert_eq!(
            params("::reset"),
            [name(&[ExistenceOutcome::Unbind])],
            "{environment}"
        );
        assert_eq!(params("::twice"), [name(&[BIND])], "{environment}");
        assert_eq!(params("::renew"), [name(&[BIND])], "{environment}");
        assert_eq!(
            params("::clear"),
            [name(&[ExistenceOutcome::Unbind])],
            "{environment}"
        );
        assert_eq!(
            params("::maybe"),
            [
                name(&[ExistenceOutcome::MayBind(BindingKind::Either)]),
                ParamRole::Value
            ],
            "{environment}"
        );
        assert_eq!(
            params("::ensure"),
            [name(&[ExistenceOutcome::Bind(BindingKind::Either)])],
            "{environment}"
        );
        assert_eq!(
            params("::peek"),
            [name(&[ExistenceOutcome::Preserve])],
            "{environment}"
        );
        let twice = &summaries["::twice"];
        assert_eq!(
            twice
                .evidence
                .bindings
                .iter()
                .map(|binding| binding.name.as_str())
                .collect::<Vec<_>>(),
            ["::bump"],
            "{environment}: `twice` rests on `bump`"
        );
        assert_eq!(
            twice.completion,
            CompletionCodeDomain::Exact(NORMAL_OR_ERROR),
            "{environment}"
        );
        assert!(twice.globals.is_empty(), "{environment}");
    }
}

/// A procedure that reaches code the module cannot see, or a frame its
/// summary cannot name, has no summary, and neither has a caller of it.
#[test]
fn a_barrier_has_no_summary() {
    let source = "proc opaque {name} {upvar 1 $name v; mystery; incr v}\n\
            proc far {name} {upvar 2 $name v; incr v}\n\
            proc outer {name} {upvar 1 $name v; opaque v}\n\
            proc global_write {} {set ::counter 5}\n";
    let summaries = summaries(source, "tcl8.6");
    for barrier in ["::opaque", "::far", "::outer"] {
        assert!(!summaries.contains_key(barrier), "{barrier}: {summaries:?}");
    }
    let global_write = &summaries["::global_write"];
    assert_eq!(
        global_write.globals,
        [(
            PlaceRef::scalar("::counter"),
            ExistenceOutcome::MayBind(BindingKind::Either)
        )]
    );
}

/// A call that omits a `Name` parameter names the place the parameter's
/// default spells, as Tcl binds the default; a word count the callee's
/// parameters reject is a call that never completes; and a default that
/// names an element is a barrier, as an element argument is.
#[test]
fn a_call_names_its_omitted_parameters_default() {
    let source = "proc bumpd {{name n} {by 1}} {upvar 1 $name v; incr v $by}\n\
            proc cell {{name a(1)}} {upvar 1 $name v; incr v}\n";
    let (context_owner, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
    let registry = context_owner.commands();
    let module = ModuleProcedures::of_unit(&unit, registry);
    let bind = step_of(&[BIND]);
    assert_eq!(
        module.call_transfer(&fixture_id(&unit.ir_module.procedures, "::bumpd"), &[]),
        Some(CallTransfer::Places(vec![("n".to_owned(), bind)]))
    );
    assert_eq!(
        module.call_transfer(
            &fixture_id(&unit.ir_module.procedures, "::bumpd"),
            &[Some("m"), None]
        ),
        Some(CallTransfer::Places(vec![("m".to_owned(), bind)]))
    );
    assert_eq!(
        module.call_transfer(
            &fixture_id(&unit.ir_module.procedures, "::bumpd"),
            &[Some("m"), Some("1"), Some("x")]
        ),
        Some(CallTransfer::Never)
    );
    assert_eq!(
        module.call_transfer(&fixture_id(&unit.ir_module.procedures, "::cell"), &[]),
        None
    );
}

/// A `Name` argument naming a place outside the caller's frame — a
/// qualified name, or a local the caller links to a namespace's
/// variable by `upvar #0`, `global` or `variable` — is a write the
/// caller's summary states among its globals: with the callee's outcome
/// where every completion of the caller makes the call, may-bound where
/// only some do, and may-unbound too where the callee may unbind it. A
/// local linked to one of two places names a place the summary cannot,
/// so that caller has no summary. tclsh 8.4 to 9.1 print 2 for `puts
/// [c]`.
#[test]
fn conditional_source_outer_places_do_not_issue_physical_steps() {
    let (context, unit) =
        crate::interprocedural::logical_completion_unit("proc p {} {global g; set g 1}", "tcl8.6");
    let id = fixture_id(&unit.ir_module.procedures, "::p");
    let summary = &unit.transfers.0[&id];
    assert_eq!(summary.global_sources.len(), 1);
    assert_eq!(
        summary.global_sources[0]
            .0
            .symbol()
            .checked_c_namespace_key()
            .as_deref(),
        Some("::g")
    );
    let procedures = ModuleProcedures::of_unit(&unit, context.commands());
    assert!(procedures.outer_steps(&id).is_none());
}

#[test]
fn a_name_argument_naming_an_outer_place_is_a_global_write() {
    let source = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n\
            set g 1\n\
            proc c {} {bump ::g; return $::g}\n\
            proc d {} {upvar #0 g x; bump x}\n\
            proc e {} {global g; bump g}\n\
            proc h {c} {if {$c} {bump ::g}}\n\
            proc f {c} {if {$c} {upvar #0 g x} else {upvar #0 k x}; bump x}\n\
            namespace eval ns {variable v 1; proc w {} {variable v; bump v}}\n\
            proc reset {name} {upvar 1 $name v; unset v}\n\
            proc r {} {reset ::q}\n\
            proc s {c} {if {$c} {reset ::q}}\n";
    let (context_owner, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
    let registry = context_owner.commands();
    let module = ModuleProcedures::of_unit(&unit, registry);
    let g = PlaceRef::scalar("::g".to_owned());
    for caller in ["::c", "::d", "::e"] {
        assert_eq!(
            unit.transfers.0[&fixture_id(&unit.ir_module.procedures, caller)].globals,
            vec![(g.clone(), BIND)],
            "{caller}"
        );
        assert!(
            !module.keeps_to_its_frame(&fixture_id(&unit.ir_module.procedures, caller)),
            "{caller}"
        );
    }
    assert_eq!(
        unit.transfers.0[&fixture_id(&unit.ir_module.procedures, "::h")].globals,
        vec![(g, ExistenceOutcome::MayBind(BindingKind::Either))]
    );
    assert_eq!(
        unit.transfers.0[&fixture_id(&unit.ir_module.procedures, "::ns::w")].globals,
        vec![(PlaceRef::scalar("::ns::v".to_owned()), BIND)]
    );
    let q = PlaceRef::scalar("::q".to_owned());
    assert_eq!(
        unit.transfers.0[&fixture_id(&unit.ir_module.procedures, "::r")].globals,
        vec![(q.clone(), ExistenceOutcome::Unbind)]
    );
    assert_eq!(
        unit.transfers.0[&fixture_id(&unit.ir_module.procedures, "::s")].globals,
        vec![
            (q.clone(), ExistenceOutcome::Unbind),
            (q, ExistenceOutcome::MayBind(BindingKind::Either))
        ]
    );
    assert!(
        !fixture_id_if_present(&unit.ir_module.procedures, "::f")
            .is_some_and(|id| unit.transfers.0.contains_key(&id))
    );
    assert!(
        fixture_id_if_present(&unit.ir_module.procedures, "::bump")
            .is_some_and(|id| unit.transfers.0.contains_key(&id))
    );
}

/// A release unit summarises each procedure from its body as its own
/// declared frame runs it: a write to a qualified name and one through
/// `global` are may-binds of the global place, a procedure that names no
/// outer place has a summary, and a parameter a body links with `upvar` and
/// then increments is bound. A procedure the module calls, walked in its
/// callers' activations too, is summarised from its body lowered again in
/// its declared frame, and its callers' summaries rest on it.
#[test]
fn release_units_summarise_their_procedures() {
    let source = "proc g {} {set ::counter 5}\n\
            proc h {} {global k; set k 1}\n\
            proc a {} {set x 0; incr x; return $x}\n\
            proc bump {name} {upvar 1 $name v; incr v}\n\
            proc callee {} {return 1}\n\
            proc caller {} {callee}\n\
            proc user {} {set y 0; bump y}\n";
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let context = tcl_registry::model::ingress::static_context_for(environment);
        let unit =
            CompilationUnit::build_for_dialect(source, context.commands(), false, environment);
        let summaries = fixture_transfer_labels(&unit.ir_module, unit.transfers.0.clone());
        let may = ExistenceOutcome::MayBind(BindingKind::Either);
        assert_eq!(
            summaries["::g"].globals,
            [(PlaceRef::scalar("::counter"), may)],
            "{environment}"
        );
        assert_eq!(
            summaries["::h"].globals,
            [(PlaceRef::scalar("::k"), may)],
            "{environment}"
        );
        assert!(summaries["::a"].globals.is_empty(), "{environment}");
        assert!(summaries["::callee"].globals.is_empty(), "{environment}");
        assert_eq!(
            summaries["::caller"]
                .evidence
                .bindings
                .iter()
                .map(|binding| binding.name.as_str())
                .collect::<Vec<_>>(),
            ["::callee"],
            "{environment}"
        );
        assert!(summaries.contains_key("::user"), "{environment}");
        assert_eq!(summaries["::bump"].params, [name(&[BIND])], "{environment}");
    }
}

/// A write through a local linked to a caller's place binds the place, and
/// the summary says so; a later `unset` through the link may leave it
/// unbound. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 print `2 1 1 2 0`
/// for `set n 1; bump n; grow g; drop d; puts "$n [info exists g] $g [info
/// exists d]"`.
#[test]
fn a_write_through_a_caller_link_binds_the_place() {
    let source = "proc bump {name} {upvar 1 $name v; incr v}\n\
            proc grow {name} {upvar 1 $name v; set v 1; lappend v 2}\n\
            proc drop {name} {upvar 1 $name v; set v 1; unset v}\n";
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let context = tcl_registry::model::ingress::static_context_for(environment);
        let unit =
            CompilationUnit::build_for_dialect(source, context.commands(), false, environment);
        let summaries = fixture_transfer_labels(&unit.ir_module, unit.transfers.0.clone());
        assert_eq!(summaries["::bump"].params, [name(&[BIND])], "{environment}");
        assert_eq!(summaries["::grow"].params, [name(&[BIND])], "{environment}");
        let ParamRole::Name { outcomes, .. } = &summaries["::drop"].params[0] else {
            panic!("{environment}: drop links its parameter");
        };
        assert!(
            outcomes.contains(&ExistenceOutcome::Unbind),
            "{environment}: {outcomes:?}"
        );
    }
}

/// A module that shadows a builtin keeps the summary of every procedure
/// whose body never reaches the shadowed command: `bump` spells no
/// `incr`, so it keeps its own, while `user`, which lowers the rebound
/// `incr` to a typed statement, has none. A rebinding whose subject the
/// scan cannot name may have moved any command, so it keeps the
/// module-wide answer. With `proc c {} {set x 0; bump x; return $x}`
/// added, tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 print `1 0` for
/// `puts "[c] [user]"`.
#[test]
fn a_shadowed_builtin_keeps_the_summaries_that_never_reach_it() {
    let shadowed = "proc incr {args} {return 0}\n\
            proc bump {name} {upvar 1 $name v; set v [expr {$v + 1}]}\n\
            proc user {} {set x 0; incr x; return $x}\n";
    let computed = "rename $old {}\n\
            proc bump {name} {upvar 1 $name v; set v [expr {$v + 1}]}\n";
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let context = tcl_registry::model::ingress::static_context_for(environment);
        let build = |source: &str| {
            let unit =
                CompilationUnit::build_for_dialect(source, context.commands(), false, environment);
            fixture_transfer_labels(&unit.ir_module, unit.transfers.0.clone())
        };
        let kept = build(shadowed);
        assert!(kept.contains_key("::bump"), "{environment}: {kept:?}");
        assert!(!kept.contains_key("::user"), "{environment}: {kept:?}");
        assert!(
            build(computed).is_empty(),
            "{environment}: an unnameable subject keeps every summary out"
        );
    }
}

const CYCLE: &str =
    "proc up {name n} {upvar 1 $name v; if {$n > 0} {incr v; up v [expr {$n - 1}]}}\n";

/// A cycle is solved from its procedures never completing, round by
/// round: `up`, which passes its place to itself, settles in two rounds
/// on a may-write of the place. Given one round, the cycle does not
/// settle, and every place it names is may-bound afterwards whatever it
/// held, with any completion and a computed result.
#[test]
fn a_cycle_that_does_not_converge_is_may_bind() {
    let settled = summaries_within(
        CYCLE,
        crate::interprocedural::MAX_INTERPROCEDURAL_WALK_DEPTH,
    );
    let summary = &settled["::up"];
    assert_eq!(
        summary.params,
        [
            name(&[ExistenceOutcome::MayBind(BindingKind::Either)]),
            ParamRole::Value
        ],
        "`up` settles"
    );
    assert_eq!(
        summary.completion,
        CompletionCodeDomain::Exact(NORMAL_OR_ERROR)
    );
    let unsettled = summaries_within(CYCLE, RecursionLimit(1));
    let summary = &unsettled["::up"];
    assert_eq!(
        summary.params,
        [name(&ANY_OUTCOME), ParamRole::Value],
        "`up` does not settle"
    );
    assert_eq!(summary.completion, CompletionCodeDomain::Any);
    assert_eq!(summary.result, ReturnKind::Other);
    assert_eq!(
        step_of(&ANY_OUTCOME).apply(Existence::Bound(BindingKind::Scalar)),
        Existence::MayBound
    );
}

/// The revision every summary rides on moves with a `rename` or a
/// redefinition anywhere in the module, and with nothing else.
#[test]
fn a_rename_or_a_redefinition_moves_the_revision() {
    let revision = |source: &str| {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for(source, &registry, false);
        revision_of(&unit.ir_module, &unit.command_mutations)
    };
    let base =
        "proc bump {name} {upvar 1 $name v; incr v}\nproc p {} {set n 1; bump n; return $n}\n";
    let same = revision(base);
    assert_eq!(same, revision(base));
    assert_ne!(same, revision(&format!("{base}rename bump bump2\n")));
    assert_ne!(same, revision(&format!("{base}proc bump {{name}} {{}}\n")));
    assert_ne!(same, revision(&base.replace("incr v", "incr v 2")));
}

#[test]
fn transfer_formals_keep_selected_tcl_and_jim_binders_separate() {
    // naming.interprocedural.original-transfer-summary-source-context
    // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
    // Selected software formal binding, not native local slots or activation.
    let source = "proc optional {{a A} b} {return 1}\nproc middle {a args b} {return 1}\nproc linked {&v} {return 1}";
    for environment in ["tcl8.6", "jim"] {
        let (context, unit) = crate::interprocedural::logical_completion_unit(source, environment);
        let module = ModuleProcedures::of_unit(&unit, context.commands());
        let grammar = module
            .formals(&fixture_id(&unit.ir_module.procedures, "::optional"))
            .unwrap()
            .1;
        if grammar == tcl_dialect::ParameterGrammar::Jim {
            let values = module
                .parameter_values(
                    &fixture_id(&unit.ir_module.procedures, "::optional"),
                    &[Some(ExactValue::from_literal("B"))],
                )
                .unwrap();
            assert_eq!(
                values,
                [
                    Some(ExactValue::from_literal("A")),
                    Some(ExactValue::from_literal("B"))
                ]
            );
            let arguments: Vec<_> = ["A", "B", "C", "D"]
                .into_iter()
                .map(|value| Some(ExactValue::from_literal(value)))
                .collect();
            assert_eq!(
                module
                    .parameter_values(
                        &fixture_id(&unit.ir_module.procedures, "::middle"),
                        &arguments
                    )
                    .unwrap(),
                [
                    Some(ExactValue::from_literal("A")),
                    Some(ExactValue::from_literal("B C")),
                    Some(ExactValue::from_literal("D"))
                ]
            );
            assert!(
                module
                    .parameter_values(
                        &fixture_id(&unit.ir_module.procedures, "::linked"),
                        &[Some(ExactValue::from_literal("caller"))]
                    )
                    .is_none()
            );
            assert!(
                !fixture_id_if_present(&unit.ir_module.procedures, "::linked")
                    .is_some_and(|id| unit.transfers.0.contains_key(&id))
            );
        } else {
            assert_eq!(grammar, tcl_dialect::ParameterGrammar::Tcl);
            assert!(
                module
                    .parameter_values(
                        &fixture_id(&unit.ir_module.procedures, "::optional"),
                        &[Some(ExactValue::from_literal("B"))]
                    )
                    .is_none()
            );
            assert_eq!(
                module
                    .parameter_values(
                        &fixture_id(&unit.ir_module.procedures, "::linked"),
                        &[Some(ExactValue::from_literal("value"))]
                    )
                    .unwrap(),
                [Some(ExactValue::from_literal("value"))]
            );
        }
    }
}

#[test]
fn transfer_unused_formals_use_actual_variable_roots_and_metadata() {
    // naming.interprocedural.original-transfer-summary-source-context
    // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
    // Conditional SSA use inventory, not native frame or replacement proof.
    let source = "proc p {{a b} {$tag} é unused} {return \"${a b}${$tag}${é}\"}";
    let result = summaries(source, "tcl8.6");
    assert_eq!(
        result["::p"].params,
        [
            ParamRole::Value,
            ParamRole::Value,
            ParamRole::Value,
            ParamRole::Unused
        ]
    );
    let array = summaries("proc p {a(k)} {return 1}", "tcl8.6");
    assert!(
        !array.contains_key("::p"),
        "formal syntax supplies no scalar activation closure"
    );
}
