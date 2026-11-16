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

//! Cell updates: read-modify-write descriptors, absent cells, the cell-write route, keyed dict updates and the increment route.

use super::*;

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

/// A cell update of a place the inputs prove unbound runs over no prior
/// value only where every release the target names creates the cell
/// (`docs/design/compiler/value-transfers.md` § *Existence*, the release
/// rule): `incr fresh` is 1 and `incr fresh 2` is 2 from 8.5 and raises
/// `can't read "fresh": no such variable` under 8.4, so the route answers
/// that error under 8.4 and declines under the `tcl` profile that spans
/// both; `append` and `lappend` create the cell in every release (tclsh 8.4
/// to 9.1).
#[test]
fn an_absent_cell_is_created_where_every_release_creates_it() {
    let reg = CommandRegistry::build_default();
    let run = |name: &'static str, words: &[&'static str], dialect: &str| {
        let semantics = resolve_semantics(reg.get(name).expect(name), None, None);
        let semantics = semantics.semantics().expect("a cell update");
        let mut operands = vec![literal("v", Some(ArgRole::VarWrite))];
        operands.extend(words.iter().map(|word| literal(word, None)));
        let mut inputs = TestInputs::new(name, operands);
        inputs.prior.insert("v".to_owned(), absent());
        inputs.context = AnalysisContext::detached(tcl_dialect::DialectProfile::find(dialect));
        evaluated(semantics.evaluate(&inputs, &mut Budget::evaluation())).map(|outcome| {
            if let Some(label) = error_label(&outcome) {
                return label;
            }
            assert_eq!(
                outcome.ordered_stores.len(),
                1,
                "{name} {words:?}: the one write"
            );
            let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
                panic!("unavailable");
            };
            String::from_utf8(result.bytes.clone()).expect("text")
        })
    };
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert_eq!(run("incr", &[], dialect), Ok("1".to_owned()), "{dialect}");
        assert_eq!(
            run("incr", &["2"], dialect),
            Ok("2".to_owned()),
            "{dialect}"
        );
    }
    // Under 8.4 the program raises, and the route says so: the cell is
    // absent, so no store ran. A profile naming no release cannot say which
    // of the two it is, and declines.
    assert_eq!(
        run("incr", &[], "tcl8.4"),
        Ok("error after 0".to_owned()),
        "tcl8.4"
    );
    assert_eq!(
        run("incr", &[], "tcl"),
        Err(DeclineReason::UnboundPlace),
        "tcl"
    );
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
        assert_eq!(
            run("append", &["foo"], dialect),
            Ok("foo".to_owned()),
            "{dialect}"
        );
        assert_eq!(
            run("lappend", &["foo"], dialect),
            Ok("foo".to_owned()),
            "{dialect}"
        );
    }
}

/// 8.4's `incr` of an absent place raises `can't read "x": no such variable`
/// with no `-errorcode` (tclsh 8.4.20), after no store; an element's message
/// says whether its array exists, which the fact does not, so only its code
/// is proven.
#[test]
fn an_increment_of_an_absent_place_under_8_4_is_the_commands_error() {
    let t = target;
    let message = |text: &str| Some(text.to_owned());
    let run = |name: &str| {
        completed(
            "incr",
            None,
            &[t(name)],
            &[(name, absent())],
            Some("tcl8.4"),
        )
    };
    assert_eq!(
        raised(&run("fresh")),
        Some((
            0,
            message("can't read \"fresh\": no such variable"),
            message("NONE")
        ))
    );
    assert_eq!(raised(&run("a(1)")), Some((0, None, message("NONE"))));
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
    assert_eq!(STRING_RANGE.incoming_targets(&inputs).len(), 0);
}

/// `set name value` writes the value byte for byte and returns it; `set
/// name` returns what the place holds, passes a pending prior through, and
/// declines an unbound one, since reading an absent variable is an error.
#[test]
fn the_cell_write_route_writes_and_reads_the_exact_value() {
    use tcl_registry::value_transfer::cell_write::CELL_WRITE;
    let write = TestInputs::new(
        "set",
        vec![literal("v", Some(ArgRole::VarWrite)), literal(" a ", None)],
    );
    let outcome =
        evaluated(CELL_WRITE.evaluate(&write, &mut Budget::evaluation())).expect("a write");
    assert_eq!(
        outcome.result,
        ExactValueOrUnavailable::Exact(ExactValue::from_literal(" a "))
    );
    assert_eq!(
        outcome.ordered_stores,
        [StoreOutcome::Write {
            target: TargetId(OperandId(0)),
            value: ExactValue::from_literal(" a "),
        }]
    );
    assert_eq!(CELL_WRITE.incoming_targets(&write).len(), 0);

    let read_inputs = |prior: FactView| {
        let mut inputs = TestInputs::new("set", vec![literal("v", Some(ArgRole::VarRead))]);
        inputs.prior.insert("v".to_owned(), prior);
        inputs
    };
    let read =
        |prior: FactView| CELL_WRITE.evaluate(&read_inputs(prior), &mut Budget::evaluation());
    let seven = ExactValue::from_literal("7");
    let outcome = evaluated(read(FactView::Exact(seven.clone(), None))).expect("a read");
    assert_eq!(outcome.result, ExactValueOrUnavailable::Exact(seven));
    assert_eq!(outcome.ordered_stores.len(), 0);
    assert_eq!(read(FactView::Pending), EvalAnswer::Pending);
    assert_eq!(
        read(FactView::Top(DeclineReason::UnboundPlace)),
        EvalAnswer::Declined(DeclineReason::UnboundPlace)
    );
    assert_eq!(
        CELL_WRITE.incoming_targets(&read_inputs(FactView::Pending)),
        [TargetId(OperandId(0))]
    );
}

/// The five keyed updates run the shared dict cores, each answer the one
/// tclsh 8.5 to 9.1 give: order kept, duplicates canonicalised, a key path
/// walked level by level, an absent variable the empty dictionary, and a
/// malformed dictionary the program's error.
#[test]
fn keyed_updates_run_the_shared_dict_cores() {
    let set = |prior: FactView, words: &[&'static str]| keyed_update("set", prior, words, None);
    let first = set(absent(), &["a", "1"]).expect("dict set");
    let second = set(held(&first), &["b", "2"]).expect("dict set");
    assert_eq!(set(held(&second), &["a", "3"]).as_deref(), Ok("a 3 b 2"));
    assert_eq!(
        set(held("a {x 1}"), &["a", "y", "2"]).as_deref(),
        Ok("a {x 1 y 2}")
    );
    assert_eq!(
        set(held("b 2 a 1"), &["c", "3"]).as_deref(),
        Ok("b 2 a 1 c 3")
    );
    assert_eq!(set(held(" a  1 "), &["b", "2"]).as_deref(), Ok("a 1 b 2"));
    assert_eq!(set(held("a 1 a 2"), &["b", "3"]).as_deref(), Ok("a 2 b 3"));
    assert_eq!(
        set(held("a 1 b"), &["c", "3"]).as_deref(),
        Ok("error after 0")
    );
    assert_eq!(
        set(held("a b"), &["a", "c", "d"]).as_deref(),
        Ok("error after 0"),
        "an intermediate value that is not a dictionary"
    );

    let unset = |prior: FactView, words: &[&'static str]| keyed_update("unset", prior, words, None);
    assert_eq!(unset(held("a 1 b 2"), &["a"]).as_deref(), Ok("b 2"));
    assert_eq!(unset(absent(), &["a"]).as_deref(), Ok(""));
    assert_eq!(unset(held(" a  1 "), &["zz"]).as_deref(), Ok("a 1"));
    assert_eq!(unset(held("a {x 1}"), &["a", "x"]).as_deref(), Ok("a {}"));
    assert_eq!(
        unset(held("a 1"), &["x", "y"]),
        Err(DeclineReason::WrongRepresentation),
        "a missing intermediate key"
    );

    let incr = |prior: FactView, words: &[&'static str], dialect: Option<&str>| {
        keyed_update("incr", prior, words, dialect)
    };
    assert_eq!(incr(absent(), &["k"], None).as_deref(), Ok("k 1"));
    assert_eq!(incr(held("k 5"), &["k", "-7"], None).as_deref(), Ok("k -2"));
    assert_eq!(incr(held("k { 5 }"), &["k"], None).as_deref(), Ok("k 6"));
    assert_eq!(incr(absent(), &["k", " 5"], None).as_deref(), Ok("k { 5}"));
    assert_eq!(
        incr(absent(), &["k", "010"], Some("tcl8.6")).as_deref(),
        Ok("k 010")
    );
    assert_eq!(
        incr(held("k 010"), &["k"], Some("tcl8.6")).as_deref(),
        Ok("k 9")
    );
    assert_eq!(
        incr(held("k 010"), &["k"], Some("tcl9.0")).as_deref(),
        Ok("k 11")
    );
    assert_eq!(
        incr(held("k 010"), &["k"], None),
        Err(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar))
    );
    assert_eq!(
        incr(held("k abc"), &["k"], None).as_deref(),
        Ok("error after 0")
    );

    let append =
        |prior: FactView, words: &[&'static str]| keyed_update("append", prior, words, None);
    let foo = append(absent(), &["k", "foo"]).expect("dict append");
    assert_eq!(append(held(&foo), &["k", "bar"]).as_deref(), Ok("k foobar"));
    assert_eq!(
        append(held("k 1"), &["k", "2", "3"]).as_deref(),
        Ok("k 123")
    );
    assert_eq!(append(absent(), &["k"]).as_deref(), Ok("k {}"));

    let lappend =
        |prior: FactView, words: &[&'static str]| keyed_update("lappend", prior, words, None);
    assert_eq!(
        lappend(absent(), &["k", "a", "b c"]).as_deref(),
        Ok("k {a {b c}}")
    );
    assert_eq!(lappend(held("k v"), &["k"]).as_deref(), Ok("k v"));
    assert_eq!(
        lappend(held("k \\{"), &["k", "v"]).as_deref(),
        Ok("error after 0")
    );

    // A prior the solver cannot prove is never taken for an absent one.
    assert_eq!(
        keyed_update(
            "set",
            FactView::Top(DeclineReason::NotExact),
            &["a", "1"],
            None
        ),
        Err(DeclineReason::NotExact)
    );
}

/// `::tcl::dict::incr d k` answers as `dict incr d k`: the qualified spec
/// carries the subcommand's declaration, and the dictionary operand is
/// found by its role in either layout.
#[test]
fn the_qualified_dict_spellings_share_the_declaration() {
    let reg = CommandRegistry::build_default();
    let qualified = reg.get("::tcl::dict::incr").expect("::tcl::dict::incr");
    let resolved = resolve_semantics(qualified, None, None);
    let semantics = resolved.semantics().expect("the keyed update");
    assert_eq!(semantics.identity(), "keyed-update:incr");
    let mut inputs = TestInputs::new(
        "::tcl::dict::incr",
        vec![literal("d", Some(ArgRole::VarWrite)), literal("k", None)],
    );
    inputs.prior.insert("d".to_owned(), held("k 41"));
    let outcome =
        evaluated(semantics.evaluate(&inputs, &mut Budget::evaluation())).expect("evaluated");
    assert_eq!(
        outcome.ordered_stores,
        [StoreOutcome::Write {
            target: TargetId(OperandId(0)),
            value: match outcome.result.clone() {
                ExactValueOrUnavailable::Exact(value) => value,
                ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
            },
        }]
    );
    assert_eq!(
        keyed_update("incr", held("k 41"), &["k"], None).as_deref(),
        Ok("k 42")
    );
    assert_eq!(
        semantics.incoming_targets(&inputs),
        [TargetId(OperandId(0))]
    );
}

/// A route that reads its target's prior value declares the read where
/// every consumer asks for it: [`Traits::READS_BEFORE_WRITE`] on the scope
/// that carries the route. The dictionary's keyed updates had the route but
/// not the trait, so a spelling the lowering reaches by head —
/// `::tcl::dict::set`, an alias of `dict set` — recorded no read, and O109
/// deleted the store it read: `set d {a 1}; ::tcl::dict::set d k v` printed
/// `k v` where tclsh 8.5 to 9.1 print `a 1 k v`.
#[test]
fn a_route_that_reads_its_target_declares_the_read() {
    let reg = full_registry();
    let mut names: Vec<&str> = reg.command_names().collect();
    names.sort_unstable();
    let mut checked = BTreeSet::new();
    for name in names {
        let Some(spec) = reg.get(name) else {
            continue;
        };
        let scopes = std::iter::once(None).chain(spec.subcommands.iter().map(Some));
        for sub in scopes {
            let resolved = resolve_semantics(spec, sub, None);
            let Some(identity) = resolved.semantics().map(CommandSemantics::identity) else {
                continue;
            };
            if !(identity.starts_with("cell-update:") || identity.starts_with("keyed-update:")) {
                continue;
            }
            let traits = spec.traits | sub.map_or_else(Traits::empty, |sub| sub.traits);
            let label = sub.map_or_else(
                || spec.name.to_owned(),
                |sub| format!("{} {}", spec.name, sub.name),
            );
            assert!(
                traits.contains(Traits::READS_BEFORE_WRITE),
                "{label} reads its target through `{identity}` but does not declare the read"
            );
            checked.insert(label);
        }
    }
    for label in [
        "incr",
        "append",
        "lappend",
        "dict set",
        "dict unset",
        "dict incr",
        "dict append",
        "dict lappend",
        "::tcl::dict::set",
        "::tcl::dict::lappend",
    ] {
        assert!(
            checked.contains(label),
            "{label} was not checked: {checked:?}"
        );
    }
}

/// The increment route over `n` holding `old`, stepped by `amount`, under
/// `dialect`'s profile — `None` is a profile naming no release.
pub(super) fn evaluate_increment(
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
            // An error the command raises reads as `error after N`.
            ExactValueOrUnavailable::Unavailable(_) if error_label(&outcome).is_some() => {
                Ok(ExactValue::text(error_label(&outcome).expect("an error")))
            }
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

    // A leading zero reads as octal up to 8.6 and decimal from 9.0 (tclsh
    // 8.4 to 8.6: `set x 010; incr x` is 9; 9.0 and 9.1: 11). `f5-irules`
    // reads it as its 8.4 base does (ruling 8).
    for (dialect, want) in [
        ("tcl8.6", 9),
        ("tcl8.4", 9),
        ("tcl9.0", 11),
        ("f5-irules", 9),
    ] {
        assert_eq!(
            increment_result(evaluate_increment(cell, Some(dialect), text("010"), None)),
            Ok(built_int(want)),
            "{dialect}"
        );
    }
    assert_eq!(
        increment_result(evaluate_increment(cell, None, text("010"), None)),
        Err(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar)),
    );
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
    // A value or a step that is no integer is the command's error, raised
    // before any store; the message is the numeral's (`expected integer but
    // got "abc"`), whose spelling this route does not prove.
    assert_eq!(
        raised(&evaluate(
            FactView::Exact(ExactValue::text("abc"), None),
            None
        )),
        Some((0, None, None))
    );
    assert_eq!(
        raised(&evaluate(exact(1), Some("2.5"))),
        Some((0, None, None))
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
