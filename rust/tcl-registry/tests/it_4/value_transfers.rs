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
    AnalysisContext, AnalysisInputs, Axis, BodyRegion, Budget, CommandSemantics, CompletionOutcome,
    ConstOps, ConstValue, DeclarationScope, DeclineReason, DerivedSemantics, EvalAnswer, EvalRoute,
    EvaluationState, EvaluatorOwner, ExactValue, ExactValueOrUnavailable, FactDomain, FactView,
    InvocationLayout, LiftedAnswer, NativeEvalId, Needs, NoRouteReason, NumericValue, OperandId,
    OperandView, ParameterDefault, PlaceRef, PlanAnswer, ResolvedInvocationView, ResolvedSemantics,
    SemanticsDeclaration, SemanticsOrigin, StoreOutcome, TargetId, TransferAnswer, ValueIdentity,
    WordPart, WordStructure, evaluate_lifted, resolve_semantics,
};
use tcl_registry::value_transfer::{
    BindingIdentity, BindingKind, DependencyEvidence, DomainFact, Existence, ExistenceOutcome,
    FactBounds, InvocationOutcome, IterableKind, ListGroup, TypeFacts,
};
use tcl_registry::value_transfer::{
    CompletionSupport, ContextDependency, DeclaredInput, EvaluatorCapability, Exactness, HostKind,
    ImplementationBudget, ImplementationIdentity,
};
use tcl_registry::{ArgRole, CommandRegistry, InvocationWordKind, Traits};
use tcl_syntax::value::ValueOps as _;

mod captures_and_destructuring;
mod catch;
mod cell_updates;
mod completions;
mod derived_transfers;
mod evaluation_contract;
mod iteration_plans;
mod route_identity;
mod selection_plans;
mod template_plans;
mod try_handlers;
mod value_routes;

use cell_updates::evaluate_increment;

/// The nested service a test supplies: what the analysis answers for a
/// script run under the state it is given.
type NestedService<'a> = Box<dyn Fn(&str, &mut EvaluationState) -> EvalAnswer + 'a>;

/// A test view: literal operands with the given roles, and a table of
/// facts the driver would have proven.
struct TestInputs<'a> {
    view: ResolvedInvocationView<'a>,
    operands: BTreeMap<usize, FactView>,
    places: BTreeMap<usize, Result<PlaceRef, DeclineReason>>,
    prior: BTreeMap<String, FactView>,
    structures: BTreeMap<usize, WordStructure>,
    bodies: BTreeMap<usize, BodyRegion>,
    nested: Option<NestedService<'a>>,
    defaults: BTreeMap<(String, String), ParameterDefault>,
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
                argument_offset: 0,
                arity: None,
            },
            operands: BTreeMap::new(),
            places: BTreeMap::new(),
            prior: BTreeMap::new(),
            structures: BTreeMap::new(),
            bodies: BTreeMap::new(),
            nested: None,
            defaults: BTreeMap::new(),
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

    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
        self.structures
            .get(&id.0)
            .cloned()
            .ok_or(DeclineReason::Unsupported)
    }

    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason> {
        self.bodies
            .get(&id.0)
            .cloned()
            .ok_or(DeclineReason::Unsupported)
    }

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        self.nested.as_ref().map_or(
            EvalAnswer::Declined(DeclineReason::Unsupported),
            |service| service(script, state),
        )
    }

    fn math_function(&self, _name: &str) -> Result<BindingIdentity, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn parameter_default(&self, procedure: &str, parameter: &str) -> ParameterDefault {
        self.defaults
            .get(&(procedure.to_owned(), parameter.to_owned()))
            .cloned()
            .unwrap_or(ParameterDefault::Unknown)
    }

    fn context(&self) -> &AnalysisContext {
        &self.context
    }
}

/// One keyed update, `dict <sub> d <words…>`, with `d` holding `prior`
/// under `dialect`: the new dictionary, which is both the result and the
/// one store, or the decline.
fn keyed_update(
    sub: &'static str,
    prior: FactView,
    words: &[&'static str],
    dialect: Option<&str>,
) -> Result<String, DeclineReason> {
    let reg = CommandRegistry::build_default();
    let spec = reg.get("dict").expect("dict");
    let resolved = resolve_semantics(spec, Some(spec.subcommand(sub).expect(sub)), None);
    let semantics = resolved.semantics().expect("a keyed update");
    let mut operands = vec![literal(sub, None), literal("d", Some(ArgRole::VarWrite))];
    operands.extend(words.iter().map(|word| literal(word, None)));
    let mut inputs = TestInputs::new("dict", operands);
    inputs.prior.insert("d".to_owned(), prior);
    inputs.context = AnalysisContext::detached(dialect.and_then(tcl_dialect::DialectProfile::find));
    let outcome = evaluated(semantics.evaluate(&inputs, &mut Budget::evaluation()))?;
    if let Some(label) = error_label(&outcome) {
        return Ok(label);
    }
    let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
        panic!("unavailable");
    };
    assert_eq!(
        outcome.ordered_stores,
        [StoreOutcome::Write {
            target: TargetId(OperandId(1)),
            value: result.clone(),
        }],
        "{sub} {words:?}: the new dictionary is the one store"
    );
    Ok(String::from_utf8(result.bytes.clone()).expect("text"))
}

fn held(text: &str) -> FactView {
    FactView::Exact(ExactValue::from_literal(text), None)
}

fn absent() -> FactView {
    FactView::Domain(tcl_registry::value_transfer::DomainFact::Existence(
        tcl_registry::value_transfer::Existence::Unbound,
    ))
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

/// How an outcome that ended in the command's error reads in a comparison:
/// `error after N`, N the stores that ran before it.
fn error_label(outcome: &tcl_registry::value_transfer::InvocationOutcome) -> Option<String> {
    match outcome.completion {
        CompletionOutcome::Error { written, .. } => Some(format!("error after {written}")),
        CompletionOutcome::Normal | CompletionOutcome::Code { .. } => None,
    }
}

/// What an answer that evaluated to the command's error proves: the stores
/// that ran, and the message and `-errorcode` where they are exact.
fn raised(answer: &EvalAnswer) -> Option<(usize, Option<String>, Option<String>)> {
    let EvalAnswer::Evaluated(outcome) = answer else {
        return None;
    };
    let CompletionOutcome::Error {
        written,
        message,
        error_code,
    } = &outcome.completion
    else {
        return None;
    };
    let text = |field: &ExactValueOrUnavailable| match field {
        ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes.clone()).ok(),
        ExactValueOrUnavailable::Unavailable(_) => None,
    };
    Some((*written, text(message), text(error_code)))
}

/// The targets an evaluated answer publishes a type for, in order.
fn typed_targets(answer: &EvalAnswer) -> Vec<usize> {
    let EvalAnswer::Evaluated(outcome) = answer else {
        panic!("not evaluated: {answer:?}");
    };
    outcome
        .types
        .per_target
        .iter()
        .map(|(target, _)| (target.0).0)
        .collect()
}

/// A word the destructuring tests pass: its text, and whether the resolver
/// gives it the `VarWrite` role.
const fn target(text: &str) -> (&str, bool) {
    (text, true)
}

/// A word the resolver gives no `VarWrite` role.
const fn word(text: &str) -> (&str, bool) {
    (text, false)
}

/// [`destructured`]'s rendering of an answer: the result and each store.
fn answered(result: &str, stores: &[&str]) -> (String, Vec<String>) {
    (
        result.to_owned(),
        stores.iter().map(ToString::to_string).collect(),
    )
}

/// A short offset as a span coordinate.
fn small(offset: usize) -> u32 {
    u32::try_from(offset).expect("a small offset")
}

/// A literal switch word, its own spelling.
fn switch(text: &'static str) -> (&'static str, Option<FactView>) {
    (text, None)
}

/// The `$name` read at word offset `start`.
fn read(start: usize, name: &str) -> tcl_registry::value_transfer::VariableRead {
    tcl_registry::value_transfer::VariableRead {
        span: tcl_lexer::Span::new(small(start), small(start + 1 + name.len())),
        name: name.to_owned(),
        element: None,
    }
}

/// The existence fact of a place the rung proves bound as `kind`.
fn bound_as(kind: tcl_registry::value_transfer::BindingKind) -> FactView {
    FactView::Domain(tcl_registry::value_transfer::DomainFact::Existence(
        tcl_registry::value_transfer::Existence::Bound(kind),
    ))
}

/// The completion a writing route proves for `command words…` under
/// `dialect`, with the places named in `facts` holding the existence fact
/// given: the stores that ran, the message and the `-errorcode` where they
/// are exact, and each planned store as `write N`, `preserve N` or
/// `unbind N`.
fn completed(
    command: &str,
    sub: Option<&str>,
    words: &[(&str, bool)],
    facts: &[(&str, FactView)],
    dialect: Option<&str>,
) -> EvalAnswer {
    let reg = CommandRegistry::build_default();
    let spec = reg.get(command).expect(command);
    let semantics = match sub {
        Some(name) => resolve_semantics(spec, Some(spec.subcommand(name).expect(name)), None),
        None => resolve_semantics(spec, None, None),
    };
    let semantics = semantics.semantics().expect("a registry-owned route");
    let operands = words
        .iter()
        .map(|&(text, target)| literal(text, target.then_some(ArgRole::VarWrite)))
        .collect();
    let mut inputs = TestInputs::new(command, operands);
    for (name, fact) in facts {
        inputs.prior.insert((*name).to_owned(), fact.clone());
    }
    inputs.context = AnalysisContext::detached(
        dialect.map(|name| tcl_dialect::DialectProfile::find(name).expect(name)),
    );
    semantics.evaluate(&inputs, &mut Budget::evaluation())
}

/// Each store an answer plans, in order.
fn planned(answer: &EvalAnswer) -> Vec<String> {
    let EvalAnswer::Evaluated(outcome) = answer else {
        panic!("not evaluated: {answer:?}");
    };
    outcome
        .ordered_stores
        .iter()
        .map(|store| match store {
            StoreOutcome::Write { target, .. } => format!("write {}", (target.0).0),
            StoreOutcome::WriteElement { target, key, .. } => {
                format!("element {} {key}", (target.0).0)
            }
            StoreOutcome::Preserve { target } => format!("preserve {}", (target.0).0),
            StoreOutcome::Unbind { target } => format!("unbind {}", (target.0).0),
            StoreOutcome::MayWrite { target, .. } => format!("may-write {}", (target.0).0),
            StoreOutcome::WriteUnavailable { target, .. } => {
                format!("write-unavailable {}", (target.0).0)
            }
        })
        .collect()
}
