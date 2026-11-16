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

//! `catch`: the code its script observes, the writes it carries, what it stores and when it declines.

use super::*;

/// `catch` binds its result and options variables whatever the script's
/// completion, with values the transfer need not know: it lists one path
/// whose completion domain is any, binding each as a scalar, beside the
/// route that evaluates a closed script. Tcl 8.4's two-word form binds its
/// one variable, and a call that names none has nothing to bind.
#[test]
fn catch_binds_its_result_and_options_whatever_the_completion() {
    use tcl_registry::completion::CompletionCodeDomain;
    use tcl_registry::value_transfer::BindingKind;
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("catch").expect("catch"), None, None);
    let semantics = resolved.semantics().expect("declared");
    assert_eq!(
        semantics.route(),
        EvalRoute::Direct {
            id: NativeEvalId::CatchProtected
        }
    );
    let bound = |words: Vec<OperandView<'static>>| {
        let inputs = TestInputs::new("catch", words);
        match semantics.transfer(FactDomain::Existence, &inputs, &mut Budget::unbounded()) {
            TransferAnswer::Existence(transfer) => {
                assert_eq!(transfer.paths.len(), 1);
                assert_eq!(transfer.paths[0].completion, CompletionCodeDomain::Any);
                transfer.paths[0].outcomes.clone()
            }
            other => panic!("{other:?}"),
        }
    };
    let body = || literal("script", Some(ArgRole::Body));
    let scalar = |at: usize| {
        (
            TargetId(OperandId(at)),
            ExistenceOutcome::Bind(BindingKind::Scalar),
        )
    };
    assert_eq!(
        bound(vec![
            body(),
            literal("m", Some(ArgRole::VarWrite)),
            literal("o", Some(ArgRole::VarWrite))
        ]),
        [scalar(1), scalar(2)]
    );
    assert_eq!(
        bound(vec![body(), literal("m", Some(ArgRole::VarWrite))]),
        [scalar(1)]
    );
    let bare = TestInputs::new("catch", vec![body()]);
    assert_eq!(
        semantics.transfer(FactDomain::Existence, &bare, &mut Budget::unbounded()),
        TransferAnswer::Generic
    );
    assert_eq!(
        semantics.transfer(FactDomain::Type, &bare, &mut Budget::unbounded()),
        TransferAnswer::Generic
    );
}

/// A nested service whose script wrote `writes` — each a scalar set to a text
/// — and then ended as `completion` with `result`, an error counting every
/// write the script made.
fn script_ending(
    writes: &'static [(&'static str, &'static str)],
    completion: CompletionOutcome,
    result: ExactValueOrUnavailable,
) -> NestedService<'static> {
    Box::new(move |_script, state| {
        for &(name, value) in writes {
            state.writes.push((
                PlaceRef::scalar(name),
                StoreOutcome::Write {
                    target: TargetId(OperandId(0)),
                    value: ExactValue::text(value),
                },
            ));
        }
        let completion = match &completion {
            CompletionOutcome::Error {
                message,
                error_code,
                ..
            } => CompletionOutcome::Error {
                written: state.writes.len(),
                message: message.clone(),
                error_code: error_code.clone(),
            },
            other => other.clone(),
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion,
            result: result.clone(),
            nested_writes: Vec::new(),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence: DependencyEvidence::default(),
        }))
    })
}

/// The inputs of `catch` over `words` — the script, then the result and
/// options variables — with the nested service `service` and the existence
/// `facts` of the variables (unbound where none is given), under `dialect`.
fn catch_inputs(
    words: &[&'static str],
    facts: &[(&str, FactView)],
    dialect: Option<&str>,
    service: NestedService<'static>,
) -> TestInputs<'static> {
    let operands = words
        .iter()
        .enumerate()
        .map(|(at, text)| {
            literal(
                text,
                Some(if at == 0 {
                    ArgRole::Body
                } else {
                    ArgRole::VarWrite
                }),
            )
        })
        .collect();
    let mut inputs = TestInputs::new("catch", operands);
    if let Some(script) = words.first() {
        inputs.bodies.insert(
            0,
            BodyRegion {
                script: (*script).to_owned(),
                base_offset: 0,
                frame: tcl_registry::FrameLevel::Relative(0),
            },
        );
    }
    inputs.nested = Some(service);
    let unbound = FactView::Domain(DomainFact::Existence(Existence::Unbound));
    for name in words.iter().skip(1) {
        inputs.prior.insert((*name).to_owned(), unbound.clone());
    }
    for (name, fact) in facts {
        inputs.prior.insert((*name).to_owned(), fact.clone());
    }
    inputs.context = AnalysisContext::detached(
        dialect.map(|name| tcl_dialect::DialectProfile::find(name).expect(name)),
    );
    inputs
}

/// `catch`'s route evaluated over [`catch_inputs`].
fn caught(
    words: &[&'static str],
    facts: &[(&str, FactView)],
    dialect: Option<&str>,
    service: NestedService<'static>,
) -> EvalAnswer {
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("catch").expect("catch"), None, None);
    let semantics = resolved.semantics().expect("declared");
    let inputs = catch_inputs(words, facts, dialect, service);
    semantics.evaluate(&inputs, &mut Budget::evaluation())
}

/// What `catch`'s answer proves: the code it returns, the writes of the
/// script it carries, and each of its own stores — the text of an exact one,
/// the prefix of an unavailable dictionary's text, `-` where there is none.
fn caught_summary(answer: &EvalAnswer) -> (i64, Vec<String>, Vec<String>) {
    let EvalAnswer::Evaluated(outcome) = answer else {
        panic!("not evaluated: {answer:?}");
    };
    assert_eq!(outcome.completion, CompletionOutcome::Normal);
    let ExactValueOrUnavailable::Exact(code) = &outcome.result else {
        panic!("the code is certain: {:?}", outcome.result);
    };
    let writes = outcome
        .nested_writes
        .iter()
        .map(|(place, _)| place.name.clone())
        .collect();
    let stores = outcome
        .ordered_stores
        .iter()
        .map(|store| match store {
            StoreOutcome::Write { target, value } => format!(
                "write {} {}",
                (target.0).0,
                String::from_utf8_lossy(&value.bytes)
            ),
            StoreOutcome::WriteUnavailable { target, facts } => {
                let prefix = facts
                    .segments
                    .as_ref()
                    .and_then(|segments| segments.prefix.clone())
                    .map_or_else(
                        || "-".to_owned(),
                        |bytes| String::from_utf8_lossy(&bytes).into_owned(),
                    );
                format!("unavailable {} {:?} {prefix}", (target.0).0, facts.intrep)
            }
            other => panic!("{other:?}"),
        })
        .collect();
    (code.as_int().expect("an integer"), writes, stores)
}

/// `catch` completes normally whatever its script does, returning the code a
/// caller of the script observes and storing what the script returned into
/// its result variable, with the options dictionary beside it from 8.5 (the
/// table measured under tclsh 8.5 to 9.1):
///
/// | script | code | result variable | options start |
/// |---|---|---|---|
/// | `set v 1` | 0 | `1` | `-code 0 -level 0` |
/// | `return 5` | 2 | `5` | `-code 0 -level 1` |
/// | `break` | 3 | empty | `-code 3 -level 0` |
/// | `continue` | 4 | empty | `-code 4 -level 0` |
/// | `return -code 5 custom` | 2 | `custom` | `-code 5 -level 1` |
/// | `error boom` | 1 | `boom` | not stated |
///
/// An error's dictionary lists `-errorinfo` first when `error` is given an
/// info word, so none of its text is claimed, and no dictionary is an exact
/// value: `-errorinfo`, `-errorline` and `-errorstack` are the interpreter's,
/// and from 8.6 a success carries a stale `-errorcode` after `incr` or
/// `lappend` of an absent variable.
#[test]
fn catch_completes_with_the_code_its_script_observes() {
    use tcl_registry::completion::CompletionCode;
    let text = |text: &str| ExactValueOrUnavailable::exact_text(text);
    let code = |code, level, result: &str| CompletionOutcome::Code {
        code,
        level,
        result: text(result),
    };
    let table: [(CompletionOutcome, &str, i64, &str, Option<&str>); 6] = [
        (
            CompletionOutcome::Normal,
            "1",
            0,
            "1",
            Some("-code 0 -level 0"),
        ),
        (
            code(CompletionCode::Ok, 1, "5"),
            "5",
            2,
            "5",
            Some("-code 0 -level 1"),
        ),
        (
            code(CompletionCode::Break, 0, ""),
            "",
            3,
            "",
            Some("-code 3 -level 0"),
        ),
        (
            code(CompletionCode::Continue, 0, ""),
            "",
            4,
            "",
            Some("-code 4 -level 0"),
        ),
        (
            code(CompletionCode::Other(5), 1, "custom"),
            "custom",
            2,
            "custom",
            Some("-code 5 -level 1"),
        ),
        (
            CompletionOutcome::Error {
                written: 0,
                message: text("boom"),
                error_code: text("NONE"),
            },
            "",
            1,
            "boom",
            None,
        ),
    ];
    for dialect in [Some("tcl8.5"), Some("tcl8.6"), Some("tcl9.0")] {
        for (completion, result, observed, stored, options) in &table {
            let answer = caught(
                &["script", "m", "o"],
                &[],
                dialect,
                script_ending(&[], completion.clone(), text(result)),
            );
            let (reported, writes, stores) = caught_summary(&answer);
            assert_eq!(reported, *observed, "{dialect:?} {completion:?}");
            assert_eq!(writes.len(), 0);
            let dictionary = format!("unavailable 2 Some(Dict) {}", options.unwrap_or("-"));
            assert_eq!(stores, [format!("write 1 {stored}"), dictionary]);
        }
    }
}

/// The writes the script made are the command's own nested writes, ahead of
/// its stores, and an error after some of them leaves exactly those: the
/// answer states them in the order they ran. A result variable the script
/// wrote itself is written again by the command, last. The answer rests on
/// the route, the release it was proven under, and an integer code.
#[test]
fn catch_carries_the_writes_its_script_made() {
    let text = ExactValueOrUnavailable::exact_text;
    let boom = CompletionOutcome::Error {
        written: 0,
        message: text("boom"),
        error_code: text("NONE"),
    };
    let answer = caught(
        &["script", "m"],
        &[],
        Some("tcl8.6"),
        script_ending(&[("a", "1"), ("b", "2")], boom, text("")),
    );
    assert_eq!(
        caught_summary(&answer),
        (
            1,
            vec!["a".to_owned(), "b".to_owned()],
            vec!["write 1 boom".to_owned()]
        )
    );
    let EvalAnswer::Evaluated(outcome) = &answer else {
        panic!("{answer:?}");
    };
    assert_eq!(outcome.types.result, Some(tcl_registry::TclType::Int));
    let route = outcome.evidence.route.expect("the route");
    assert_eq!(
        route.route,
        EvalRoute::Direct {
            id: NativeEvalId::CatchProtected
        }
    );
    assert_eq!(
        outcome.evidence.release,
        Some(tcl_dialect::TclVersion::V8_6)
    );

    let rewritten = caught(
        &["script", "m"],
        &[],
        Some("tcl8.6"),
        script_ending(&[("m", "5")], CompletionOutcome::Normal, text("7")),
    );
    assert_eq!(
        caught_summary(&rewritten),
        (0, vec!["m".to_owned()], vec!["write 1 7".to_owned()])
    );
}

/// A result that is not exact is still stored: the variable is certainly
/// bound with a value the analysis cannot spell, which is what the options
/// dictionary always is. An error whose message the route does not prove
/// stores an unavailable string, whatever the code.
#[test]
fn catch_stores_an_unproven_result_as_an_unavailable_value() {
    let answer = caught(
        &["script", "m"],
        &[],
        Some("tcl8.6"),
        script_ending(
            &[],
            CompletionOutcome::error_unproven(0),
            ExactValueOrUnavailable::unproven_string(),
        ),
    );
    assert_eq!(
        caught_summary(&answer),
        (
            1,
            Vec::new(),
            vec!["unavailable 1 Some(String) -".to_owned()]
        )
    );
    let normal = caught(
        &["script", "m"],
        &[],
        Some("tcl8.6"),
        script_ending(
            &[],
            CompletionOutcome::Normal,
            ExactValueOrUnavailable::unproven_string(),
        ),
    );
    assert_eq!(
        caught_summary(&normal),
        (
            0,
            Vec::new(),
            vec!["unavailable 1 Some(String) -".to_owned()]
        )
    );
}

/// What `catch` will not state of its script and its words: the script's own
/// decline or pending answer, a script word that is not text, a word count
/// `wrong # args` answers, and an options word before 8.5 (declined under 8.4,
/// and on a profile naming no release, which cannot say).
#[test]
fn catch_declines_what_its_script_and_words_do_not_prove() {
    let text = ExactValueOrUnavailable::exact_text;
    let ends = || script_ending(&[], CompletionOutcome::Normal, text("1"));
    let evaluate = |inputs: &TestInputs<'_>| {
        let reg = CommandRegistry::build_default();
        let resolved = resolve_semantics(reg.get("catch").expect("catch"), None, None);
        resolved
            .semantics()
            .expect("declared")
            .evaluate(inputs, &mut Budget::evaluation())
    };
    let tcl86 = Some("tcl8.6");

    assert_eq!(
        caught(
            &["s", "m"],
            &[],
            tcl86,
            Box::new(|_, _| EvalAnswer::Pending)
        ),
        EvalAnswer::Pending
    );
    assert_eq!(
        caught(
            &["s", "m"],
            &[],
            tcl86,
            Box::new(|_, _| EvalAnswer::Declined(DeclineReason::NotExact))
        ),
        EvalAnswer::Declined(DeclineReason::NotExact)
    );
    let mut no_text = catch_inputs(&["s", "m"], &[], tcl86, ends());
    no_text.bodies.clear();
    assert_eq!(
        evaluate(&no_text),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
    for words in [&[][..], &["s", "a", "b", "c"][..]] {
        assert_eq!(
            caught(words, &[], tcl86, ends()),
            EvalAnswer::Declined(DeclineReason::Unsupported),
            "{words:?}"
        );
    }

    assert_eq!(
        caught(&["s", "m", "o"], &[], Some("tcl8.4"), ends()),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
    assert!(matches!(
        caught(&["s", "m", "o"], &[], None, ends()),
        EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(_))
    ));
    assert!(matches!(
        caught(&["s", "m"], &[], Some("tcl8.4"), ends()),
        EvalAnswer::Evaluated(_)
    ));
}

/// A variable the store may fail on: an element, which may fail on its array,
/// a place that may be an array or one the analysis knows nothing of. A fact
/// not yet reached is pending, and a scalar or an absent place is written.
#[test]
fn catch_declines_a_variable_its_store_may_fail_on() {
    let text = ExactValueOrUnavailable::exact_text;
    let ends = || script_ending(&[], CompletionOutcome::Normal, text("1"));
    let tcl86 = Some("tcl8.6");
    let fact = |existence| FactView::Domain(DomainFact::Existence(existence));
    let scalar = Existence::Bound(BindingKind::Scalar);
    for ok in [Existence::Unbound, scalar] {
        assert!(matches!(
            caught(&["s", "m"], &[("m", fact(ok))], tcl86, ends()),
            EvalAnswer::Evaluated(_)
        ));
    }
    for unsafe_kind in [
        Existence::Bound(BindingKind::Array),
        Existence::Bound(BindingKind::Either),
        Existence::MayBound,
    ] {
        assert_eq!(
            caught(&["s", "m"], &[("m", fact(unsafe_kind))], tcl86, ends()),
            EvalAnswer::Declined(DeclineReason::NotExact),
            "{unsafe_kind:?}"
        );
    }
    assert_eq!(
        caught(
            &["s", "m"],
            &[("m", fact(Existence::Pending))],
            tcl86,
            ends()
        ),
        EvalAnswer::Pending
    );
    assert_eq!(
        caught(&["s", "a(1)"], &[], tcl86, ends()),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
}

/// The script wrote the variable `catch` stores into: a scalar write leaves a
/// scalar, whatever the place held before, and anything else leaves it
/// unknown.
#[test]
fn catch_declines_a_variable_its_script_left_unknown() {
    let tcl86 = Some("tcl8.6");
    let fact = |existence| FactView::Domain(DomainFact::Existence(existence));
    let wrote = |store: fn(TargetId) -> StoreOutcome| -> NestedService<'static> {
        Box::new(move |_, state| {
            state
                .writes
                .push((PlaceRef::scalar("m"), store(TargetId(OperandId(0)))));
            EvalAnswer::Evaluated(Box::new(InvocationOutcome {
                completion: CompletionOutcome::Normal,
                result: ExactValueOrUnavailable::exact_text("1"),
                nested_writes: Vec::new(),
                ordered_stores: Vec::new(),
                types: TypeFacts::default(),
                evidence: DependencyEvidence::default(),
            }))
        })
    };
    let array = &[("m", fact(Existence::Bound(BindingKind::Array)))];
    assert!(matches!(
        caught(
            &["s", "m"],
            array,
            tcl86,
            wrote(|target| StoreOutcome::Write {
                target,
                value: ExactValue::text("x"),
            })
        ),
        EvalAnswer::Evaluated(_)
    ));
    for unknown in [
        (|target| StoreOutcome::Unbind { target }) as fn(TargetId) -> StoreOutcome,
        |target| StoreOutcome::MayWrite {
            target,
            facts: FactBounds {
                existence: Existence::MayBound,
                intrep: None,
                shape: None,
                segments: None,
                taint: None,
            },
        },
        |target| StoreOutcome::WriteUnavailable {
            target,
            facts: FactBounds {
                existence: Existence::Bound(BindingKind::Scalar),
                intrep: None,
                shape: None,
                segments: None,
                taint: None,
            },
        },
    ] {
        assert_eq!(
            caught(&["s", "m"], &[], tcl86, wrote(unknown)),
            EvalAnswer::Declined(DeclineReason::StatefulNested)
        );
    }
}

/// `catch` is a body run in the frame it is written in whose completion is
/// absorbed whole: the plan names the script, its result and options
/// variables, and declines the counts `wrong # args` answers.
#[test]
fn catch_structure_is_a_protected_body() {
    use tcl_registry::value_transfer::{BodyPlan, CompletionProtocol, Reconcile};
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("catch").expect("catch"), None, None);
    let semantics = resolved.semantics().expect("declared");
    let plan = |words: &[&'static str]| {
        let operands = words
            .iter()
            .enumerate()
            .map(|(at, text)| {
                literal(
                    text,
                    Some(if at == 0 {
                        ArgRole::Body
                    } else {
                        ArgRole::VarWrite
                    }),
                )
            })
            .collect();
        semantics.structure(&TestInputs::new("catch", operands))
    };
    let body = |result_var, options_var| PlanAnswer::Body {
        binders: Vec::new(),
        body: BodyPlan {
            body: OperandId(0),
            frame: tcl_registry::FrameLevel::Relative(0),
        },
        reconcile: Reconcile::None,
        completion: CompletionProtocol::CatchAll {
            result_var,
            options_var,
        },
    };
    let target = |at| Some(TargetId(OperandId(at)));
    assert_eq!(plan(&["s"]), body(None, None));
    assert_eq!(plan(&["s", "m"]), body(target(1), None));
    assert_eq!(plan(&["s", "m", "o"]), body(target(1), target(2)));
    assert_eq!(plan(&[]), PlanAnswer::Declined(DeclineReason::Unsupported));
    assert_eq!(
        plan(&["s", "a", "b", "c"]),
        PlanAnswer::Declined(DeclineReason::Unsupported)
    );
}
