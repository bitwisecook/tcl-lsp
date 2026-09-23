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

//! Contract tests for the value-transfer declarations
//! (`docs/design/compiler/value-transfers.md`).
//!
//! Three things are pinned: the derivation agrees with the descriptor it
//! derives from and derives from nothing else; the three declaration
//! states resolve innermost-first at every scope; and the set of specs
//! carrying each route, swept over every loadable dialect and the shipped
//! `.tclspec` packs, so a route cannot appear, vanish, or move without this
//! file changing beside it.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use tcl_registry::forms::CommandForm;
use tcl_registry::invocation_words::{InvocationWord, InvocationWords};
use tcl_registry::native_lowering::{CellUpdate, NativeLowering};
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_registry::types::VarWriteTyping;
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, Axis, BodyRegion, Budget, CommandSemantics, ConstOps,
    ConstValue, DeclarationScope, DeclineReason, DerivedSemantics, EvalAnswer, EvalRoute,
    EvaluationState, EvaluatorOwner, ExactValue, ExactValueOrUnavailable, FactDomain, FactView,
    InvocationLayout, LiftedAnswer, NativeEvalId, Needs, NoRouteReason, NumericValue, OperandId,
    OperandView, PlaceRef, PlanAnswer, ResolvedInvocationView, ResolvedSemantics,
    SemanticsDeclaration, SemanticsOrigin, StoreOutcome, TargetId, TransferAnswer, ValueIdentity,
    WordStructure, evaluate_lifted, resolve_semantics,
};
use tcl_registry::value_transfer::{BindingIdentity, ExistenceOutcome, IterableKind};
use tcl_registry::{ArgRole, CommandRegistry, InvocationWordKind, Traits};
use tcl_syntax::value::ValueOps as _;

const LOADABLE_DIALECTS: &[&str] = &[
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "f5-irules",
    "f5-iapps",
    "expect",
    "bpf",
];

fn full_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::build_default();
    for name in LOADABLE_DIALECTS {
        let profile = tcl_dialect::DialectProfile::find(name).expect("a compiled-in dialect name");
        for &layer in profile.base_layers {
            reg.load_surface(layer);
        }
    }
    let packs = tcl_spectcl::bundled::load_from(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs"),
    );
    assert!(
        !packs.is_empty(),
        "the shipped EDA loadables must be present"
    );
    for pack in &packs.packs {
        for command in &pack.commands {
            reg.insert(command.spec.clone());
        }
    }
    reg
}

/// A test view: literal operands with the given roles, and a table of
/// facts the driver would have proven.
struct TestInputs<'a> {
    view: ResolvedInvocationView<'a>,
    operands: BTreeMap<usize, FactView>,
    places: BTreeMap<usize, Result<PlaceRef, DeclineReason>>,
    prior: BTreeMap<String, FactView>,
    context: AnalysisContext,
}

impl<'a> TestInputs<'a> {
    fn new(command: &'a str, operands: Vec<OperandView<'a>>) -> Self {
        Self {
            view: ResolvedInvocationView {
                canonical_command: command,
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands,
            },
            operands: BTreeMap::new(),
            places: BTreeMap::new(),
            prior: BTreeMap::new(),
            context: AnalysisContext::detached(None),
        }
    }
}

fn literal(text: &str, role: Option<ArgRole>) -> OperandView<'_> {
    OperandView {
        text,
        kind: InvocationWordKind::Literal,
        role,
    }
}

impl AnalysisInputs for TestInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        assert_eq!(domain, FactDomain::ExactValue);
        self.operands.get(&id.0).cloned().unwrap_or_else(|| {
            self.view
                .operand(id)
                .map_or(FactView::Top(DeclineReason::NotExact), |operand| {
                    FactView::Exact(ExactValue::from_literal(operand.text), None)
                })
        })
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        self.places.get(&id.0).cloned().unwrap_or_else(|| {
            self.view
                .operand(id)
                .map(|operand| PlaceRef::scalar(operand.text))
                .ok_or(DeclineReason::NotExact)
        })
    }

    fn variable(&self, name: &str, _domain: FactDomain) -> FactView {
        self.prior
            .get(name)
            .cloned()
            .unwrap_or(FactView::Top(DeclineReason::NotExact))
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        self.variable(&place.name, domain)
    }

    fn word_structure(&self, _id: OperandId) -> Result<WordStructure, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn nested(&self, _script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        EvalAnswer::Declined(DeclineReason::Unsupported)
    }

    fn math_function(&self, _name: &str) -> Result<BindingIdentity, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn context(&self) -> &AnalysisContext {
        &self.context
    }
}

/// `CellReadModifyWrite(u)` on a spec ⇒ the resolved value transfer is the
/// cell update `u` on the same target, with the descriptor's absent-cell
/// rule, for the shipped specs and for a synthetic one.
#[test]
fn a_cell_read_modify_write_descriptor_derives_the_same_cell_update() {
    let reg = CommandRegistry::build_default();
    for (name, update) in [
        ("incr", CellUpdate::Increment),
        ("append", CellUpdate::Append),
        ("lappend", CellUpdate::ListAppend),
    ] {
        let spec = reg.get(name).expect(name);
        assert_eq!(
            spec.native_lowering,
            Some(NativeLowering::CellReadModifyWrite(update))
        );
        let resolved = resolve_semantics(spec, None, None);
        let ResolvedSemantics::Derived(DerivedSemantics::CellUpdate(cell)) = resolved else {
            panic!("{name}: expected a derived cell update, got {resolved:?}");
        };
        assert_eq!(cell.update, update, "{name}");
        assert_eq!(cell.creates_absent, spec.safe_on_uninit, "{name}");
        assert_eq!(resolved.origin(), SemanticsOrigin::Derived);
        let inputs = TestInputs::new(name, vec![literal("v", Some(ArgRole::VarWrite))]);
        match cell.structure(&inputs) {
            PlanAnswer::CellReadModifyWrite {
                target,
                operation,
                amount,
                creates_absent,
            } => {
                assert_eq!(target, TargetId(OperandId(0)));
                assert_eq!(operation, update);
                assert_eq!(amount, None);
                assert_eq!(creates_absent, spec.safe_on_uninit);
            }
            other => panic!("{name}: {other:?}"),
        }
    }
}

/// Descriptor availability and enabled evaluation are separate columns:
/// the increment has a registry-owned direct route; append and list-append
/// carry the descriptor and no route.
#[test]
fn every_cell_update_has_a_registry_owned_route() {
    let reg = CommandRegistry::build_default();
    let route = |name: &str| resolve_semantics(reg.get(name).expect(name), None, None).route();
    for (name, id) in [
        ("incr", NativeEvalId::CellIncrement),
        ("append", NativeEvalId::CellAppend),
        ("lappend", NativeEvalId::CellListAppend),
    ] {
        assert_eq!(route(name), Some(EvalRoute::Direct { id }), "{name}");
        assert_eq!(id.owner(), EvaluatorOwner::Registry, "{name}");
    }
    assert_eq!(NativeEvalId::StringRange.owner(), EvaluatorOwner::Registry);
}

/// The targets whose incoming value an evaluation reads: the cell update's
/// target by default, and none for a route that reads no storage.
#[test]
fn incoming_targets_default_to_the_cell_update_target() {
    use tcl_registry::value_transfer::builtins::STRING_RANGE;
    let reg = CommandRegistry::build_default();
    let incr = resolve_semantics(reg.get("incr").expect("incr"), None, None);
    let incr = incr.semantics().expect("the derived cell update");
    let inputs = TestInputs::new("incr", vec![literal("n", Some(ArgRole::VarWrite))]);
    assert_eq!(incr.incoming_targets(&inputs), [TargetId(OperandId(0))]);
    let inputs = TestInputs::new(
        "string",
        vec![
            literal("range", None),
            literal("abc", None),
            literal("0", None),
            literal("1", None),
        ],
    );
    assert!(STRING_RANGE.incoming_targets(&inputs).is_empty());
}

/// `ElementsOf` states a type relationship and `LOOP_LIST_HEADER` a CFG
/// shape; neither states iteration, so a spec carrying both and declaring
/// nothing derives nothing.
#[test]
fn elements_of_and_loop_list_header_derive_nothing() {
    let spec = CommandSpec {
        name: "vendor_each",
        traits: Traits::LOOP_LIST_HEADER | Traits::HAS_LOOP_BODY,
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        var_write_typing: VarWriteTyping::ElementsOf { container_arg: 1 },
        ..CommandSpec::DEFAULT
    };
    let resolved = resolve_semantics(&spec, None, None);
    assert!(
        matches!(resolved, ResolvedSemantics::None),
        "iteration must be declared, never derived: {resolved:?}"
    );
    // The shipped loops declare it explicitly.
    let reg = CommandRegistry::build_default();
    for name in ["foreach", "lmap"] {
        let resolved = resolve_semantics(reg.get(name).expect(name), None, None);
        assert_eq!(
            resolved.origin(),
            SemanticsOrigin::Declared(DeclarationScope::Command),
            "{name}"
        );
    }
    // `dict for` carries the header shape and declares nothing.
    let dict = reg.get("dict").expect("dict");
    let sub = dict.subcommand("for").expect("dict for");
    assert!(sub.loop_list_header);
    assert!(matches!(
        resolve_semantics(dict, Some(sub), None),
        ResolvedSemantics::None
    ));
}

/// The three states resolve innermost-first: a form's abstention hides a
/// subcommand's declaration, a subcommand's abstention hides the
/// command's, and an abstention anywhere also stops the derivation.
#[test]
fn abstention_exists_at_command_subcommand_and_form_scope() {
    static DECLARED: tcl_registry::value_transfer::builtins::DirectRoute =
        tcl_registry::value_transfer::builtins::DirectRoute {
            id: NativeEvalId::ListLength,
            result_type: tcl_registry::TclType::Int,
        };
    let declined_form = CommandForm {
        name: "declined",
        semantics: SemanticsDeclaration::Declined,
        ..CommandForm::DEFAULT
    };
    let inherited_form = CommandForm {
        name: "inherited",
        ..CommandForm::DEFAULT
    };
    let declared_sub = SubCommand {
        name: "declared",
        semantics: SemanticsDeclaration::Declared(&DECLARED),
        ..SubCommand::DEFAULT
    };
    let declined_sub = SubCommand {
        name: "declined",
        semantics: SemanticsDeclaration::Declined,
        ..SubCommand::DEFAULT
    };
    let inherited_sub = SubCommand {
        name: "inherited",
        ..SubCommand::DEFAULT
    };
    let derived_command = CommandSpec {
        name: "bump",
        native_lowering: Some(NativeLowering::CellReadModifyWrite(CellUpdate::Increment)),
        ..CommandSpec::DEFAULT
    };
    let declined_command = CommandSpec {
        name: "bump",
        semantics: SemanticsDeclaration::Declined,
        native_lowering: Some(NativeLowering::CellReadModifyWrite(CellUpdate::Increment)),
        ..CommandSpec::DEFAULT
    };

    // Form scope wins over a declared subcommand.
    assert_eq!(
        resolve_semantics(&derived_command, Some(&declared_sub), Some(&declined_form)).origin(),
        SemanticsOrigin::Declined(DeclarationScope::Form)
    );
    // A subcommand's declaration wins over the command's derivation.
    assert_eq!(
        resolve_semantics(&derived_command, Some(&declared_sub), Some(&inherited_form)).origin(),
        SemanticsOrigin::Declared(DeclarationScope::Subcommand)
    );
    // A subcommand's abstention stops the command's derivation.
    assert_eq!(
        resolve_semantics(&derived_command, Some(&declined_sub), None).origin(),
        SemanticsOrigin::Declined(DeclarationScope::Subcommand)
    );
    // An inheriting form falls through to the command's derivation.
    assert_eq!(
        resolve_semantics(&derived_command, None, Some(&inherited_form)).origin(),
        SemanticsOrigin::Derived
    );
    // A resolved subcommand selects an operation of its own, so the
    // command-level descriptor — which states the whole command's
    // operation — derives nothing for it.
    assert_eq!(
        resolve_semantics(
            &derived_command,
            Some(&inherited_sub),
            Some(&inherited_form)
        )
        .origin(),
        SemanticsOrigin::None
    );
    // The command's own abstention stops its derivation.
    assert_eq!(
        resolve_semantics(&declined_command, None, None).origin(),
        SemanticsOrigin::Declined(DeclarationScope::Command)
    );
}

/// The increment route over `n` holding `old`, stepped by `amount`, under
/// `dialect`'s profile — `None` is a profile naming no release.
fn evaluate_increment(
    cell: &dyn CommandSemantics,
    dialect: Option<&str>,
    old: FactView,
    amount: Option<&'static str>,
) -> EvalAnswer {
    let mut operands = vec![literal("n", Some(ArgRole::VarWrite))];
    if let Some(amount) = amount {
        operands.push(literal(amount, None));
    }
    let mut inputs = TestInputs::new("incr", operands);
    inputs.prior.insert("n".to_owned(), old);
    inputs.context = AnalysisContext::detached(dialect.and_then(tcl_dialect::DialectProfile::find));
    cell.evaluate(&inputs, &mut Budget::evaluation())
}

/// The value an evaluated increment answers, or the reason it declined.
fn increment_result(answer: EvalAnswer) -> Result<ExactValue, DeclineReason> {
    match answer {
        EvalAnswer::Evaluated(outcome) => match outcome.result {
            ExactValueOrUnavailable::Exact(value) => Ok(value),
            ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
        },
        EvalAnswer::Declined(reason) => Err(reason),
        EvalAnswer::Pending => panic!("pending"),
    }
}

/// What the increment route answers: the integer `ConstOps` built, with the
/// representation it constructed as evidence.
fn built_int(i: i64) -> ExactValue {
    ExactValue {
        representation: tcl_registry::value_transfer::RepresentationEvidence::Constructed(
            tcl_registry::TclType::Int,
        ),
        ..ExactValue::int(i)
    }
}

/// The increment reads its numerals under the target's release, as the
/// adapter does, and a profile naming no release declines wherever the
/// releases differ; the evidence names the route and the release.
#[test]
fn the_increment_route_reads_numerals_under_the_target_release() {
    let cell = resolve_semantics(
        CommandRegistry::build_default().get("incr").expect("incr"),
        None,
        None,
    );
    let cell = cell.semantics().expect("derived");
    let exact = |i: i64| FactView::Exact(ExactValue::int(i), None);
    let text = |t: &str| FactView::Exact(ExactValue::text(t), None);

    // A leading zero reads as octal up to 8.6 and decimal from 9.0.
    for (dialect, want) in [("tcl8.6", 9), ("tcl8.4", 9), ("tcl9.0", 11)] {
        assert_eq!(
            increment_result(evaluate_increment(cell, Some(dialect), text("010"), None)),
            Ok(built_int(want)),
            "{dialect}"
        );
    }
    for dialect in [None, Some("f5-irules")] {
        assert_eq!(
            increment_result(evaluate_increment(cell, dialect, text("010"), None)),
            Err(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar)),
            "{dialect:?}"
        );
    }
    // A whitespace-padded step is an integer in every release (tclsh 8.4,
    // 8.6, 9.0: `set x 1; incr x " 5"` is 6).
    assert_eq!(
        increment_result(evaluate_increment(cell, None, exact(1), Some(" 5"))),
        Ok(built_int(6))
    );
    // Past the wide boundary 8.5 onward widens; 8.4 prints a value the
    // model does not compute; a profile naming no release cannot say.
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let value = increment_result(evaluate_increment(
            cell,
            Some(dialect),
            exact(i64::MAX),
            None,
        ))
        .expect(dialect);
        assert_eq!(value.bytes, b"9223372036854775808", "{dialect}");
        assert_eq!(value.numeric, None, "{dialect}");
    }
    assert_eq!(
        increment_result(evaluate_increment(
            cell,
            Some("tcl8.4"),
            exact(i64::MAX),
            None
        )),
        Err(DeclineReason::WrongRepresentation)
    );
    assert_eq!(
        increment_result(evaluate_increment(cell, None, exact(i64::MAX), None)),
        Err(DeclineReason::ReleaseAmbiguous(Axis::IntTower))
    );
    // The evidence names the route and the release the answer depended on.
    match evaluate_increment(cell, Some("tcl8.6"), exact(1), None) {
        EvalAnswer::Evaluated(outcome) => {
            assert_eq!(
                outcome.evidence.release,
                Some(tcl_dialect::TclVersion::V8_6)
            );
            assert_eq!(
                outcome.evidence.route.map(|r| r.route),
                Some(EvalRoute::Direct {
                    id: NativeEvalId::CellIncrement
                })
            );
        }
        other => panic!("{other:?}"),
    }
}

/// The registry-owned increment: read the proven old value, add the exact
/// step, return the new value and one write of it to the target — and
/// decline, never guess, on a pending, non-integer, or set-valued input.
#[test]
fn the_increment_route_runs_the_shared_core_under_the_target_semantics() {
    let cell = resolve_semantics(
        CommandRegistry::build_default().get("incr").expect("incr"),
        None,
        None,
    );
    let cell = cell.semantics().expect("derived");
    let evaluate =
        |old: FactView, amount: Option<&'static str>| evaluate_increment(cell, None, old, amount);
    let exact = |i: i64| FactView::Exact(ExactValue::int(i), None);

    match evaluate(exact(5), None) {
        EvalAnswer::Evaluated(outcome) => {
            assert_eq!(outcome.result, ExactValueOrUnavailable::Exact(built_int(6)));
            assert_eq!(
                outcome.ordered_stores,
                vec![StoreOutcome::Write {
                    target: TargetId(OperandId(0)),
                    value: built_int(6)
                }]
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        evaluate(exact(3), Some("10")),
        EvalAnswer::Evaluated(outcome) if outcome.result == ExactValueOrUnavailable::Exact(built_int(13))
    ));
    assert!(matches!(
        evaluate(exact(10), Some("-2")),
        EvalAnswer::Evaluated(outcome) if outcome.result == ExactValueOrUnavailable::Exact(built_int(8))
    ));
    assert_eq!(evaluate(FactView::Pending, None), EvalAnswer::Pending);
    assert_eq!(
        evaluate(FactView::Top(DeclineReason::NotExact), None),
        EvalAnswer::Declined(DeclineReason::NotExact)
    );
    assert_eq!(
        evaluate(FactView::Exact(ExactValue::text("abc"), None), None),
        EvalAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    assert_eq!(
        evaluate(exact(1), Some("2.5")),
        EvalAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    // A finite set that reaches the evaluator is one the lift did not pin.
    assert_eq!(
        evaluate(
            FactView::Finite(
                vec![ExactValue::int(1), ExactValue::int(2)],
                Some(ValueIdentity(7))
            ),
            None
        ),
        EvalAnswer::Declined(DeclineReason::CorrelatedSets)
    );
    // The type transfer names the result and the target as integers.
    let inputs = TestInputs::new("incr", vec![literal("n", Some(ArgRole::VarWrite))]);
    match cell.transfer(FactDomain::Type, &inputs, &mut Budget::unbounded()) {
        TransferAnswer::Type(facts) => {
            assert_eq!(facts.result, Some(tcl_registry::TclType::Int));
            assert_eq!(
                facts.per_target,
                vec![(TargetId(OperandId(0)), tcl_registry::TclType::Int)]
            );
        }
        other => panic!("{other:?}"),
    }
}

/// The unbind derived from `DESTROYS_VARIABLE`: every resolvable
/// variable-writing operand is unbound on the normal path, and nothing
/// else is touched.
#[test]
fn destroys_variable_derives_an_unbind_transfer() {
    let reg = CommandRegistry::build_default();
    let unset = reg.get("unset").expect("unset");
    let resolved = resolve_semantics(unset, None, None);
    assert!(matches!(
        resolved,
        ResolvedSemantics::Derived(DerivedSemantics::Unbind(_))
    ));
    let inputs = TestInputs::new(
        "unset",
        vec![
            literal("-nocomplain", Some(ArgRole::Option)),
            literal("p", Some(ArgRole::VarWrite)),
            literal("q", Some(ArgRole::VarWrite)),
        ],
    );
    let semantics = resolved.semantics().expect("derived");
    match semantics.transfer(FactDomain::Existence, &inputs, &mut Budget::unbounded()) {
        TransferAnswer::Existence(transfer) => {
            assert_eq!(transfer.paths.len(), 1);
            assert_eq!(
                transfer.paths[0].outcomes,
                vec![
                    (TargetId(OperandId(1)), ExistenceOutcome::Unbind),
                    (TargetId(OperandId(2)), ExistenceOutcome::Unbind),
                ]
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        semantics.transfer(FactDomain::Type, &inputs, &mut Budget::unbounded()),
        TransferAnswer::Generic
    );
}

/// The synthetic loop header projects to the declared iteration protocol:
/// one list iterable at operand 0, the header's binders in order.
#[test]
fn the_loop_header_projects_to_the_declared_iteration_plan() {
    let reg = CommandRegistry::build_default();
    let binders = vec!["x".to_owned()];
    for name in ["foreach", "lmap"] {
        let resolved = resolve_semantics(reg.get(name).expect(name), None, None);
        let semantics = resolved.semantics().expect("declared");
        let mut inputs = TestInputs::new(name, vec![literal("a b c", None)]);
        inputs.view.layout = InvocationLayout::LoopHeader { binders: &binders };
        match semantics.structure(&inputs) {
            PlanAnswer::Iterate(plan) => {
                assert_eq!(plan.binders.len(), 1);
                assert_eq!(plan.iterable, IterableKind::List(OperandId(0)));
                assert!(plan.body.is_none());
            }
            other => panic!("{name}: {other:?}"),
        }
        // The source layout's plan is not yet described.
        let source = TestInputs::new(name, vec![literal("x", None), literal("a b c", None)]);
        assert!(matches!(
            semantics.structure(&source),
            PlanAnswer::Declined(DeclineReason::Unsupported)
        ));
    }
}

/// The value axis is a projection of the invocation resolver: resolving
/// `string length` selects the subcommand's declaration, and resolving a
/// head with no declaration and no derivable descriptor answers none.
#[test]
fn the_resolver_projects_the_declaration_state() {
    let reg = CommandRegistry::build_default();
    let words = [
        InvocationWord::Literal("length"),
        InvocationWord::Literal("abc"),
    ];
    let resolved = reg
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("string"), &words),
            None,
        )
        .resolved()
        .expect("string length resolves");
    assert_eq!(
        resolved.semantics.value.origin(),
        SemanticsOrigin::Declared(DeclarationScope::Subcommand)
    );
    assert_eq!(
        resolved.semantics.value.route(),
        Some(EvalRoute::Direct {
            id: NativeEvalId::StringLength
        })
    );
    let puts = reg
        .resolve_invocation("puts", &["hello"], None)
        .expect("puts resolves");
    assert!(matches!(puts.semantics.value, ResolvedSemantics::None));
    assert_eq!(puts.semantics.value.route(), None);
}

/// The pinned-set gate: the specs carrying each route, over every loadable
/// dialect and the shipped packs. A route cannot appear, vanish, or move
/// without this list changing beside it.
#[test]
fn route_stamps_match_the_pinned_set() {
    let reg = full_registry();
    let mut actual: BTreeSet<(String, &'static str)> = BTreeSet::new();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            if let Some(route) = resolve_semantics(spec, None, None).route() {
                actual.insert((spec.name.to_owned(), route_label(route)));
            }
            for sub in spec.subcommands {
                if let SemanticsDeclaration::Declared(semantics) = sub.semantics {
                    actual.insert((
                        format!("{} {}", spec.name, sub.name),
                        route_label(semantics.route()),
                    ));
                }
            }
        }
    }
    let expected: BTreeSet<(String, &'static str)> = [
        ("append", "direct:cell-append"),
        ("expr", "expression:tcl.expr"),
        ("foreach", "none:unauthored"),
        ("format", "direct:format-template"),
        ("incr", "direct:cell-increment"),
        ("lappend", "direct:cell-list-append"),
        ("list", "direct:list-of-args"),
        ("llength", "direct:list-length"),
        ("lmap", "none:unauthored"),
        ("string length", "direct:string-length"),
        ("string range", "direct:string-range"),
        ("unset", "none:unauthored"),
    ]
    .into_iter()
    .map(|(name, route)| (name.to_owned(), route))
    .collect();
    let missing: Vec<_> = expected.difference(&actual).collect();
    let extra: Vec<_> = actual.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "route stamps drifted from the pinned set\nmissing: {missing:?}\nextra: {extra:?}"
    );
}

fn route_label(route: EvalRoute) -> &'static str {
    match route {
        EvalRoute::Direct { id } => match id {
            NativeEvalId::CellIncrement => "direct:cell-increment",
            NativeEvalId::CellAppend => "direct:cell-append",
            NativeEvalId::CellListAppend => "direct:cell-list-append",
            NativeEvalId::StringRange => "direct:string-range",
            NativeEvalId::ListOfArgs => "direct:list-of-args",
            NativeEvalId::FormatTemplate => "direct:format-template",
            NativeEvalId::ListLength => "direct:list-length",
            NativeEvalId::StringLength => "direct:string-length",
        },
        EvalRoute::Expression { .. } => "expression:tcl.expr",
        EvalRoute::Implementation(_) => "implementation",
        EvalRoute::None { reason } => match reason {
            NoRouteReason::Unauthored => "none:unauthored",
            NoRouteReason::Declared => "none:declared",
            NoRouteReason::FormUnsupported => "none:form-unsupported",
            NoRouteReason::Callback => "none:callback",
        },
    }
}

fn evaluated(
    answer: EvalAnswer,
) -> Result<Box<tcl_registry::value_transfer::InvocationOutcome>, DeclineReason> {
    match answer {
        EvalAnswer::Evaluated(outcome) => Ok(outcome),
        EvalAnswer::Declined(reason) => Err(reason),
        EvalAnswer::Pending => panic!("pending"),
    }
}

/// `append` and `lappend` are the runtime adapters' value computations —
/// `var::append_bytes` and `var::lappend_value` — with the lattice write as
/// the store: byte-exact, list-rendered canonically, and a list append over
/// a value that is not a list is the program's error, never a value.
#[test]
fn append_and_list_append_run_the_shared_cores() {
    let reg = CommandRegistry::build_default();
    let evaluate = |command: &'static str, prior: FactView, values: &[&'static str]| {
        let cell = resolve_semantics(reg.get(command).expect(command), None, None);
        let cell = cell.semantics().expect("derived");
        let mut operands = vec![literal("v", Some(ArgRole::VarWrite))];
        operands.extend(values.iter().map(|value| literal(value, None)));
        let mut inputs = TestInputs::new(command, operands);
        inputs.prior.insert("v".to_owned(), prior);
        evaluated(cell.evaluate(&inputs, &mut Budget::evaluation()))
    };
    let text = |t: &str| FactView::Exact(ExactValue::text(t), None);

    let outcome = evaluate("append", text("foo"), &["bar", " baz"]).expect("appends");
    assert_eq!(
        outcome.result,
        ExactValueOrUnavailable::Exact(ExactValue {
            bytes: b"foobar baz".to_vec(),
            numeric: None,
            representation: tcl_registry::value_transfer::RepresentationEvidence::Constructed(
                tcl_registry::TclType::String
            ),
        })
    );
    assert_eq!(outcome.ordered_stores.len(), 1);
    assert_eq!(outcome.types.result, Some(tcl_registry::TclType::String));
    let padded = evaluate("append", text(" a "), &["b"]).expect("exact bytes");
    assert!(matches!(padded.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b" a b"));
    let numeric = evaluate("append", text("4"), &["2"]).expect("appends digits");
    assert!(
        matches!(numeric.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b"42" && v.numeric == Some(NumericValue::Int(42)))
    );

    let outcome = evaluate("lappend", text("a b"), &["c", "d e"]).expect("appends elements");
    assert!(
        matches!(outcome.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b"a b c {d e}")
    );
    assert_eq!(outcome.types.result, Some(tcl_registry::TclType::List));
    let outcome = evaluate("lappend", text(""), &["c"]).expect("appends to the empty list");
    assert!(matches!(outcome.result, ExactValueOrUnavailable::Exact(ref v) if v.bytes == b"c"));
    assert_eq!(
        evaluate("lappend", text("{"), &["v"]),
        Err(DeclineReason::WrongRepresentation),
        "`lappend` over `{{` raises `unmatched open brace in list`"
    );
    let mut inputs = TestInputs::new(
        "lappend",
        vec![literal("v", Some(ArgRole::VarWrite)), literal("x", None)],
    );
    inputs.prior.insert("v".to_owned(), FactView::Pending);
    let cell = resolve_semantics(reg.get("lappend").expect("lappend"), None, None);
    assert_eq!(
        cell.semantics()
            .expect("derived")
            .evaluate(&inputs, &mut Budget::evaluation()),
        EvalAnswer::Pending
    );
}

/// `string range` on the direct route: the shared core, with the index
/// numerals pre-resolved under the target's grammar and a non-ASCII operand
/// admitted only where the target decodes source as UTF-8. The shipped
/// `const_fold` is the same evaluator.
#[test]
fn string_range_runs_the_shared_core_with_the_index_grammar() {
    use tcl_registry::value_transfer::builtins::STRING_RANGE;
    let evaluate = |dialect: Option<&str>, args: [&'static str; 3]| {
        let mut inputs = TestInputs::new(
            "string",
            vec![
                literal("range", None),
                literal(args[0], None),
                literal(args[1], None),
                literal(args[2], None),
            ],
        );
        inputs.context =
            AnalysisContext::detached(dialect.and_then(tcl_dialect::DialectProfile::find));
        evaluated(STRING_RANGE.evaluate(&inputs, &mut Budget::evaluation())).map(|outcome| {
            match outcome.result {
                ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).unwrap(),
                ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
            }
        })
    };
    assert_eq!(evaluate(None, ["hello", "1", "3"]).as_deref(), Ok("ell"));
    assert_eq!(evaluate(None, [" a ", "0", "end"]).as_deref(), Ok(" a "));
    assert_eq!(evaluate(None, ["abcdef", "-2", "2"]).as_deref(), Ok("abc"));
    assert_eq!(evaluate(None, ["abc", "end-1", "end"]).as_deref(), Ok("bc"));
    assert_eq!(evaluate(None, ["abc", "3", "1"]).as_deref(), Ok(""));
    // tclsh 8.4, 8.5, 8.6: `ijkl`; tclsh 9.0, 9.1: `kl`.
    assert_eq!(
        evaluate(Some("tcl8.6"), ["abcdefghijkl", "010", "end"]).as_deref(),
        Ok("ijkl")
    );
    assert_eq!(
        evaluate(Some("tcl9.0"), ["abcdefghijkl", "010", "end"]).as_deref(),
        Ok("kl")
    );
    assert_eq!(
        evaluate(None, ["abcdefghijkl", "010", "end"]),
        Err(DeclineReason::ReleaseAmbiguous(Axis::IndexGrammar))
    );
    assert_eq!(
        evaluate(None, ["abc", "x", "1"]),
        Err(DeclineReason::WrongRepresentation)
    );
    assert_eq!(
        evaluate(Some("tcl9.0"), ["café", "0", "2"]).as_deref(),
        Ok("caf")
    );
    assert_eq!(
        evaluate(Some("tcl9.0"), ["café", "3", "3"]).as_deref(),
        Ok("é")
    );
    for dialect in [None, Some("tcl8.6")] {
        assert_eq!(
            evaluate(dialect, ["café", "0", "2"]),
            Err(DeclineReason::ReleaseAmbiguous(Axis::SourceEncoding)),
            "{dialect:?}"
        );
    }

    let reg = CommandRegistry::build_default();
    let range = reg
        .get("string")
        .expect("string")
        .subcommand("range")
        .expect("range");
    assert!(matches!(
        range.semantics,
        SemanticsDeclaration::Declared(semantics) if semantics.route() == EvalRoute::Direct { id: NativeEvalId::StringRange }
    ));
    assert_eq!(
        range.run_const_fold(&["hello", "1", "3"], None).as_deref(),
        Some("ell")
    );
    assert_eq!(
        range
            .run_const_fold(
                &["abcdefghijkl", "010", "end"],
                Some(tcl_dialect::TclVersion::V9_0)
            )
            .as_deref(),
        Some("kl")
    );
    assert_eq!(
        range
            .run_const_fold(
                &["abcdefghijkl", "010", "end"],
                Some(tcl_dialect::TclVersion::V8_6)
            )
            .as_deref(),
        Some("ijkl")
    );
    assert_eq!(
        range.run_const_fold(&["abcdefghijkl", "010", "end"], None),
        None
    );
    assert_eq!(range.run_const_fold(&["café", "0", "2"], None), None);
}

/// The correlated finite-set limit: exactly one distinct SSA value among an
/// invocation's inputs may be finite, and it is evaluated per member; two
/// distinct finite inputs decline as correlated, and two reads of one
/// identity are one distinct value.
#[test]
fn the_lift_evaluates_per_member_over_one_finite_input() {
    let reg = CommandRegistry::build_default();
    let cell = resolve_semantics(reg.get("incr").expect("incr"), None, None);
    let cell = cell.semantics().expect("derived");
    let set = |identity: u64, members: &[i64]| {
        FactView::Finite(
            members.iter().map(|i| ExactValue::int(*i)).collect(),
            Some(ValueIdentity(identity)),
        )
    };
    let results = |answer: LiftedAnswer| match answer {
        LiftedAnswer::Evaluated(outcomes) => Ok(outcomes
            .into_iter()
            .map(|outcome| match outcome.result {
                ExactValueOrUnavailable::Exact(value) => value.as_int().expect("an integer"),
                ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
            })
            .collect::<Vec<_>>()),
        LiftedAnswer::Declined(reason) => Err(reason),
        LiftedAnswer::Pending => panic!("pending"),
    };

    // One finite input: the prior value of the target.
    let mut inputs = TestInputs::new(
        "incr",
        vec![literal("x", Some(ArgRole::VarWrite)), literal("10", None)],
    );
    inputs.prior.insert("x".to_owned(), set(1, &[1, 2]));
    assert_eq!(
        results(evaluate_lifted(
            cell,
            &inputs,
            &mut Budget::evaluation(),
            32
        )),
        Ok(vec![11, 12])
    );
    // Two distinct finite inputs: the target's prior and the step.
    let mut inputs = TestInputs::new(
        "incr",
        vec![literal("x", Some(ArgRole::VarWrite)), literal("$a", None)],
    );
    inputs.prior.insert("x".to_owned(), set(1, &[1, 2]));
    inputs.operands.insert(1, set(2, &[1, 2]));
    assert_eq!(
        results(evaluate_lifted(
            cell,
            &inputs,
            &mut Budget::evaluation(),
            32
        )),
        Err(DeclineReason::CorrelatedSets)
    );
    // The same identity read twice is one distinct value: `incr x $x`.
    let mut inputs = TestInputs::new(
        "incr",
        vec![literal("x", Some(ArgRole::VarWrite)), literal("$x", None)],
    );
    inputs.prior.insert("x".to_owned(), set(1, &[1, 2]));
    inputs.operands.insert(1, set(1, &[1, 2]));
    assert_eq!(
        results(evaluate_lifted(
            cell,
            &inputs,
            &mut Budget::evaluation(),
            32
        )),
        Ok(vec![2, 4])
    );
    // The member cap is a precision limit.
    let mut inputs = TestInputs::new("incr", vec![literal("x", Some(ArgRole::VarWrite))]);
    inputs.prior.insert("x".to_owned(), set(1, &[1, 2, 3]));
    assert_eq!(
        results(evaluate_lifted(cell, &inputs, &mut Budget::evaluation(), 2)),
        Err(DeclineReason::TooManyMembers)
    );
    // No finite input evaluates once.
    let mut inputs = TestInputs::new("incr", vec![literal("x", Some(ArgRole::VarWrite))]);
    inputs
        .prior
        .insert("x".to_owned(), FactView::Exact(ExactValue::int(4), None));
    assert_eq!(
        results(evaluate_lifted(
            cell,
            &inputs,
            &mut Budget::evaluation(),
            32
        )),
        Ok(vec![5])
    );
}

/// Every core a registry-owned direct route calls reads only axes the
/// route admits: under an empty admissibility set each poisons the run,
/// except the byte append, which reads nothing release-dependent.
#[test]
fn the_cores_the_routes_call_read_only_admitted_axes() {
    let context = AnalysisContext::detached(None);
    let closed = |ops: ConstOps<'_>| ops.take(ConstValue::int(0)).err();

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&context, &mut budget, Needs::NONE).expect("admits");
    let _ = ops.int_add(Some(&ConstValue::int(1)), &ConstValue::int(1));
    assert_eq!(
        closed(ops),
        Some(DeclineReason::MalformedAnswer),
        "incr's core"
    );

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&context, &mut budget, Needs::NONE).expect("admits");
    let _ = tcl_cmd_core::var::lappend_value(
        &mut ops,
        Some(ConstValue::text("a")),
        &[ConstValue::text("b")],
    );
    assert_eq!(
        closed(ops),
        Some(DeclineReason::MalformedAnswer),
        "lappend's core"
    );

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&context, &mut budget, Needs::NONE).expect("admits");
    let _ = ops.index(&ConstValue::text("1"), 3);
    assert_eq!(
        closed(ops),
        Some(DeclineReason::MalformedAnswer),
        "string range's index"
    );

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&context, &mut budget, Needs::NONE).expect("admits");
    let _ = tcl_cmd_core::var::append_bytes(
        &mut ops,
        Some(ConstValue::text("a")),
        &[ConstValue::text("b")],
    );
    assert_eq!(closed(ops), None, "append's core reads no axis");

    let increment = resolve_semantics(
        CommandRegistry::build_default().get("incr").expect("incr"),
        None,
        None,
    );
    let DerivedSemantics::CellUpdate(cell) = (match increment {
        ResolvedSemantics::Derived(derived) => derived,
        other => panic!("{other:?}"),
    }) else {
        panic!("a cell update")
    };
    assert_eq!(cell.needs(), Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER);
    assert_eq!(
        tcl_registry::value_transfer::builtins::StringRangeSemantics::NEEDS,
        Needs::INDEX_GRAMMAR | Needs::CHAR_INDEXING | Needs::SOURCE_ENCODING
    );
}
