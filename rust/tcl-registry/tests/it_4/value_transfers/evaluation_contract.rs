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

//! The evaluation contract: lifting over finite inputs, admitted axes, outcome validation and ordered state.

use super::*;

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

/// The correlated limit holds for a keyed update: the dictionary the
/// variable holds is one input, so a finite prior evaluates per member,
/// and a finite key beside it is a second distinct value, which declines
/// as correlated — the lattice holds no pairing of a dictionary with a
/// key. The same identity read as the value is one distinct value.
#[test]
fn a_keyed_update_lifts_its_dictionary_as_one_finite_input() {
    use tcl_registry::value_transfer::keyed_update::{DICT_INCR, DICT_SET};
    let dictionaries = |identity: u64| {
        FactView::Finite(
            vec![ExactValue::text("a 1"), ExactValue::text("a 2")],
            Some(ValueIdentity(identity)),
        )
    };
    let results = |answer: LiftedAnswer| match answer {
        LiftedAnswer::Evaluated(outcomes) => Ok(outcomes
            .into_iter()
            .map(|outcome| match outcome.result {
                ExactValueOrUnavailable::Exact(value) => {
                    String::from_utf8(value.bytes).expect("text")
                }
                ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
            })
            .collect::<Vec<_>>()),
        LiftedAnswer::Declined(reason) => Err(reason),
        LiftedAnswer::Pending => panic!("pending"),
    };

    // One finite input: the dictionary's prior.
    let mut inputs = TestInputs::new(
        "::tcl::dict::incr",
        vec![literal("d", Some(ArgRole::VarWrite)), literal("a", None)],
    );
    inputs.prior.insert("d".to_owned(), dictionaries(1));
    assert_eq!(
        results(evaluate_lifted(
            &DICT_INCR,
            &inputs,
            &mut Budget::evaluation(),
            32
        )),
        Ok(vec!["a 2".to_owned(), "a 3".to_owned()])
    );
    // Two distinct finite inputs: the dictionary and the key.
    let mut inputs = TestInputs::new(
        "::tcl::dict::incr",
        vec![literal("d", Some(ArgRole::VarWrite)), literal("$k", None)],
    );
    inputs.prior.insert("d".to_owned(), dictionaries(1));
    inputs.operands.insert(
        1,
        FactView::Finite(
            vec![ExactValue::text("a"), ExactValue::text("b")],
            Some(ValueIdentity(2)),
        ),
    );
    assert_eq!(
        results(evaluate_lifted(
            &DICT_INCR,
            &inputs,
            &mut Budget::evaluation(),
            32
        )),
        Err(DeclineReason::CorrelatedSets)
    );
    // The dictionary's own identity read again as the value is one
    // distinct value: `dict set d b $d` nests each member under `b`.
    let mut inputs = TestInputs::new(
        "::tcl::dict::set",
        vec![
            literal("d", Some(ArgRole::VarWrite)),
            literal("b", None),
            literal("$d", None),
        ],
    );
    inputs.prior.insert("d".to_owned(), dictionaries(1));
    inputs.operands.insert(2, dictionaries(1));
    assert_eq!(
        results(evaluate_lifted(
            &DICT_SET,
            &inputs,
            &mut Budget::evaluation(),
            32
        )),
        Ok(vec!["a 1 b {a 1}".to_owned(), "a 2 b {a 2}".to_owned()])
    );
}

/// Every core a registry-owned direct route calls reads only axes the
/// route admits: under an empty admissibility set each poisons the run,
/// except the byte append, which reads nothing release-dependent.
#[test]
fn the_cores_the_routes_call_read_only_admitted_axes() {
    use tcl_registry::value_transfer::builtins::{
        ListLengthSemantics, ListOfArgsSemantics, StringLengthSemantics,
    };
    use tcl_registry::value_transfer::keyed_update::{DICT_INCR, DICT_SET};
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
    let _ = tcl_cmd_core::var::logical_append_value(
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

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&context, &mut budget, Needs::LIST_RENDERING).expect("admits");
    let _ = ops.dict_pairs(&ConstValue::text("a 1"));
    assert_eq!(
        closed(ops),
        Some(DeclineReason::MalformedAnswer),
        "the dict cores' canonical pairs"
    );

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&context, &mut budget, Needs::NONE).expect("admits");
    let _ = tcl_cmd_core::list::llength(&mut ops, &ConstValue::text("a {b c}"));
    assert_eq!(closed(ops), None, "llength's core reads no axis");

    let mut budget = Budget::evaluation();
    let mut ops = ConstOps::admit(&context, &mut budget, Needs::NONE).expect("admits");
    let _ = tcl_cmd_core::string::length(&mut ops, &ConstValue::text("abc"));
    assert_eq!(
        closed(ops),
        Some(DeclineReason::MalformedAnswer),
        "string length's core"
    );

    assert_eq!(ListOfArgsSemantics::NEEDS, Needs::LIST_RENDERING);
    assert_eq!(ListLengthSemantics::NEEDS, Needs::NONE);
    assert_eq!(
        StringLengthSemantics::NEEDS,
        Needs::CHAR_MODEL | Needs::SOURCE_ENCODING
    );
    assert_eq!(DICT_SET.needs(), Needs::DICT_ORDER | Needs::LIST_RENDERING);
    assert_eq!(
        DICT_INCR.needs(),
        Needs::DICT_ORDER | Needs::LIST_RENDERING | Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER
    );
}

/// The driver's structural checks before anything publishes
/// (`validate_outcome`): a store names a declared target — a `VarWrite`
/// operand the driver passes, or the place a cell-update plan names — and
/// each target has one outcome at most; the type facts name only targets;
/// and an error completion runs no more stores than it lists. Two targets
/// spelling one variable (`lassign … a a`) are two outcomes here: the
/// driver composes them once they resolve to one place.
#[test]
fn validate_outcome_rejects_a_store_to_a_non_target() {
    use tcl_registry::value_transfer::{
        CompletionOutcome, DependencyEvidence, InvocationOutcome, TypeFacts, validate_outcome,
    };
    let target = |index| TargetId(OperandId(index));
    let write = |index| StoreOutcome::Write {
        target: target(index),
        value: ExactValue::text("v"),
    };
    let outcome = |stores: Vec<StoreOutcome>| InvocationOutcome {
        completion: CompletionOutcome::Normal,
        nested_writes: Vec::new(),
        result: ExactValueOrUnavailable::Exact(ExactValue::int(1)),
        ordered_stores: stores,
        types: TypeFacts::default(),
        evidence: DependencyEvidence::default(),
    };
    let declared = [target(2), target(3)];
    let plan = PlanAnswer::NoStructure;

    assert_eq!(
        validate_outcome(
            &plan,
            &declared,
            &outcome(vec![write(2), StoreOutcome::Preserve { target: target(3) }]),
        ),
        Ok(()),
        "a write and a preserve of two declared targets"
    );
    assert_eq!(
        validate_outcome(&plan, &declared, &outcome(vec![write(1)])),
        Err(DeclineReason::MalformedAnswer),
        "a store to an operand that is no declared target"
    );
    assert_eq!(
        validate_outcome(&plan, &declared, &outcome(vec![write(4)])),
        Err(DeclineReason::MalformedAnswer),
        "a store past the declared targets"
    );
    assert_eq!(
        validate_outcome(
            &plan,
            &declared,
            &outcome(vec![write(2), StoreOutcome::Preserve { target: target(2) }]),
        ),
        Err(DeclineReason::MalformedAnswer),
        "two outcomes for one target"
    );
    assert_eq!(
        validate_outcome(&plan, &[], &outcome(Vec::new())),
        Ok(()),
        "an outcome with no stores needs no target"
    );

    // A cell update's plan names its own target, whatever roles the
    // resolver gave the words.
    let cell = PlanAnswer::CellReadModifyWrite {
        target: target(0),
        operation: CellUpdate::Append,
        amount: Some(OperandId(1)),
        creates_absent: None,
    };
    assert_eq!(
        validate_outcome(&cell, &[], &outcome(vec![write(0)])),
        Ok(())
    );
    assert_eq!(
        validate_outcome(&cell, &[], &outcome(vec![write(1)])),
        Err(DeclineReason::MalformedAnswer)
    );

    // The type facts name declared targets only.
    let mut typed = outcome(vec![write(2)]);
    typed.types.per_target = vec![(target(5), tcl_registry::TclType::Int)];
    assert_eq!(
        validate_outcome(&plan, &declared, &typed),
        Err(DeclineReason::MalformedAnswer)
    );

    // An error completion lists how many stores ran before it.
    let mut failed = outcome(vec![write(2)]);
    failed.completion = CompletionOutcome::Error {
        written: 2,
        message: ExactValueOrUnavailable::Exact(ExactValue::text("boom")),
        error_code: ExactValueOrUnavailable::Exact(ExactValue::text("NONE")),
    };
    assert_eq!(
        validate_outcome(&plan, &declared, &failed),
        Err(DeclineReason::MalformedAnswer)
    );

    // The count runs across the writes its substitutions made and then its
    // own stores.
    failed.nested_writes = vec![(PlaceRef::scalar("n"), write(0))];
    assert_eq!(validate_outcome(&plan, &declared, &failed), Ok(()));
    failed.completion = CompletionOutcome::Error {
        written: 3,
        message: ExactValueOrUnavailable::Exact(ExactValue::text("boom")),
        error_code: ExactValueOrUnavailable::Exact(ExactValue::text("NONE")),
    };
    assert_eq!(
        validate_outcome(&plan, &declared, &failed),
        Err(DeclineReason::MalformedAnswer)
    );
}

/// The ordered evaluation state's read rule (`docs/design/compiler/
/// value-transfers.md` § `expr`): the last write naming a place decides what
/// a read of it holds, a preserve changes nothing, a write that only shares
/// storage with the place (an element of the array a read names, or the array
/// of the element it names) leaves it unknown, a may-write or an unbind
/// leaves no value, and a place no write reaches is the program point's.
#[test]
fn the_ordered_state_reads_its_own_writes_first() {
    use tcl_registry::value_transfer::{FactBounds, NestedPolicy, WrittenPlace, written_in};
    let target = TargetId(OperandId(0));
    let write = |value: &str| StoreOutcome::Write {
        target,
        value: ExactValue::text(value),
    };
    let mut state = EvaluationState::new(NestedPolicy::LocalWrites);
    assert_eq!(state.written("x"), WrittenPlace::Untouched);

    state.writes.push((PlaceRef::scalar("x"), write("1")));
    state.writes.push((PlaceRef::scalar("y"), write("9")));
    state.writes.push((PlaceRef::scalar("x"), write("2")));
    assert_eq!(
        state.written("x"),
        WrittenPlace::Exact(ExactValue::text("2")),
        "the last write decides"
    );
    assert_eq!(
        state.written("y"),
        WrittenPlace::Exact(ExactValue::text("9"))
    );
    assert_eq!(
        state.written("z"),
        WrittenPlace::Untouched,
        "a place no write reaches"
    );

    state
        .writes
        .push((PlaceRef::scalar("x"), StoreOutcome::Preserve { target }));
    assert_eq!(
        state.written("x"),
        WrittenPlace::Exact(ExactValue::text("2")),
        "a preserve changes nothing"
    );

    state.writes.push((
        PlaceRef::scalar("x"),
        StoreOutcome::MayWrite {
            target,
            facts: FactBounds {
                existence: tcl_registry::value_transfer::Existence::MayBound,
                intrep: None,
                shape: None,
                segments: None,
                taint: None,
            },
        },
    ));
    assert_eq!(state.written("x"), WrittenPlace::Unknown, "a may-write");
    state.writes.push((PlaceRef::scalar("x"), write("3")));
    assert_eq!(
        state.written("x"),
        WrittenPlace::Exact(ExactValue::text("3")),
        "a later write decides again"
    );
    state
        .writes
        .push((PlaceRef::scalar("x"), StoreOutcome::Unbind { target }));
    assert_eq!(state.written("x"), WrittenPlace::Unknown, "an unbind");

    // An element and its array share storage: a write to one leaves a read
    // of the other unknown, and a read of another element untouched.
    let element = |key: &str| PlaceRef {
        name: format!("a({key})"),
        kind: tcl_registry::value_transfer::PlaceKind::Element {
            base: "a".to_owned(),
            key: key.to_owned(),
        },
    };
    let writes = vec![(
        element("k"),
        StoreOutcome::WriteElement {
            target,
            key: "k".to_owned(),
            value: ExactValue::text("v"),
        },
    )];
    assert_eq!(
        written_in(&writes, "a(k)"),
        WrittenPlace::Exact(ExactValue::text("v"))
    );
    assert_eq!(written_in(&writes, "a"), WrittenPlace::Unknown);
    assert_eq!(written_in(&writes, "a(j)"), WrittenPlace::Untouched);
}
