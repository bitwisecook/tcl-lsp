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
    WordPart, WordStructure, evaluate_lifted, resolve_semantics,
};
use tcl_registry::value_transfer::{BindingIdentity, ExistenceOutcome, IterableKind};
use tcl_registry::value_transfer::{
    CompletionSupport, ContextDependency, DeclaredInput, EvaluatorCapability, Exactness, HostKind,
    ImplementationBudget, ImplementationIdentity,
};
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
    structures: BTreeMap<usize, WordStructure>,
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
            },
            operands: BTreeMap::new(),
            places: BTreeMap::new(),
            prior: BTreeMap::new(),
            structures: BTreeMap::new(),
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

/// `expr`'s argument words assemble as the command specifies: one braced
/// word is the expression text, whose `$name` reads the engine performs
/// itself; any other word reaches the engine already substituted; several
/// words join with one space. `set a {1 + 1}; expr "$a * 2"` is 3 and
/// `expr 1 + 2` is 3 under tclsh 8.4 to 9.1. A word that is not yet a
/// value is pending, one that never is declines with its reason, no word
/// at all is the program's `wrong # args`, and a BPF-Tcl expression is
/// never assembled for the Tcl engine (`-7 / 2` is `-3` there, `-4` in
/// Tcl).
#[test]
fn expression_assembly_follows_the_word_kinds() {
    use tcl_registry::value_transfer::builtins::{BPF_EXPR, EXPR, ExpressionSource};
    let dynamic = |text| OperandView {
        text,
        kind: InvocationWordKind::Dynamic,
        role: None,
    };
    let span = tcl_lexer::Span::new;

    let mut braced = TestInputs::new("expr", vec![literal("$a * 2", None)]);
    braced.structures.insert(
        0,
        WordStructure {
            braced: true,
            parts: vec![WordPart::Literal {
                span: span(1, 7),
                text: "$a * 2".to_owned(),
            }],
        },
    );
    assert_eq!(
        EXPR.assemble(&braced),
        Ok(ExpressionSource::Braced {
            text: "$a * 2".to_owned(),
            base: 1,
        })
    );
    assert_eq!(EXPR.variable_reads(&braced), ["a"]);

    let mut quoted = TestInputs::new("expr", vec![dynamic("${a} * 2")]);
    quoted.structures.insert(
        0,
        WordStructure {
            braced: false,
            parts: vec![
                WordPart::VariableRead {
                    span: span(0, 4),
                    name: "a".to_owned(),
                    element: None,
                },
                WordPart::Literal {
                    span: span(4, 8),
                    text: " * 2".to_owned(),
                },
            ],
        },
    );
    quoted.operands.insert(0, held("1 + 1 * 2"));
    let substituted = EXPR.assemble(&quoted).expect("the substituted text");
    assert_eq!(substituted.text(), Ok("1 + 1 * 2"));
    assert!(matches!(substituted, ExpressionSource::Substituted(_)));
    // The substituted text is what the engine reads: its reads are its own.
    assert!(EXPR.variable_reads(&quoted).is_empty());

    let bare = TestInputs::new("expr", vec![literal("7", None)]);
    assert_eq!(
        EXPR.assemble(&bare)
            .map(|source| source.text().map(str::to_owned)),
        Ok(Ok("7".to_owned()))
    );

    let words = TestInputs::new(
        "expr",
        vec![literal("1", None), literal("+", None), literal("{2}", None)],
    );
    assert_eq!(
        EXPR.assemble(&words)
            .map(|source| source.text().map(str::to_owned)),
        Ok(Ok("1 + {2}".to_owned()))
    );

    let mut pending = TestInputs::new("expr", vec![dynamic("$a")]);
    pending.operands.insert(0, FactView::Pending);
    assert_eq!(EXPR.assemble(&pending), Err(EvalAnswer::Pending));
    let mut unknown = TestInputs::new("expr", vec![literal("1", None), dynamic("$a")]);
    unknown
        .operands
        .insert(1, FactView::Top(DeclineReason::NotExact));
    assert_eq!(
        EXPR.assemble(&unknown),
        Err(EvalAnswer::Declined(DeclineReason::NotExact))
    );

    let none = TestInputs::new("expr", Vec::new());
    assert_eq!(
        EXPR.assemble(&none),
        Err(EvalAnswer::Declined(DeclineReason::Unsupported))
    );
    assert_eq!(
        BPF_EXPR.assemble(&TestInputs::new("expr", vec![literal("-7 / 2", None)])),
        Err(EvalAnswer::Declined(DeclineReason::Unsupported))
    );
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
    assert!(CELL_WRITE.incoming_targets(&write).is_empty());

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
    assert!(outcome.ordered_stores.is_empty());
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
        set(held("a 1 b"), &["c", "3"]),
        Err(DeclineReason::WrongRepresentation)
    );
    assert_eq!(
        set(held("a b"), &["a", "c", "d"]),
        Err(DeclineReason::WrongRepresentation),
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
        incr(held("k abc"), &["k"], None),
        Err(DeclineReason::WrongRepresentation)
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
        lappend(held("k \\{"), &["k", "v"]),
        Err(DeclineReason::WrongRepresentation)
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

/// `list`, `llength` and `string length` run the shared cores over
/// `ConstOps` on registry-owned routes (tclsh 8.4 to 9.1 give `a {b c} {}`
/// and 2; `llength "a {b"` raises; `string length héllo` read from a UTF-8
/// file is 6 up to 8.6 and 5 from 9.0, so a non-ASCII subject declines
/// where the target does not decode source as UTF-8).
#[test]
fn list_and_length_routes_run_the_shared_cores() {
    use tcl_registry::value_transfer::builtins::{LIST_LENGTH, LIST_OF_ARGS, STRING_LENGTH};
    let run = |semantics: &dyn CommandSemantics,
               command: &'static str,
               words: &[&'static str],
               dialect: Option<&str>| {
        let mut inputs = TestInputs::new(
            command,
            words.iter().map(|word| literal(word, None)).collect(),
        );
        inputs.context =
            AnalysisContext::detached(dialect.and_then(tcl_dialect::DialectProfile::find));
        evaluated(semantics.evaluate(&inputs, &mut Budget::evaluation())).map(|outcome| {
            assert!(
                outcome.ordered_stores.is_empty(),
                "{command} writes nothing"
            );
            match outcome.result {
                ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).unwrap(),
                ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
            }
        })
    };
    assert_eq!(
        run(&LIST_OF_ARGS, "list", &["a", "b c", ""], None).as_deref(),
        Ok("a {b c} {}")
    );
    // A first element starting with `#` is brace-quoted from 8.5 and bare
    // in 8.4 (tclsh 8.4 prints `# a` for `puts [list # a]`, 8.5 to 9.1
    // print `{#} a`), so a profile naming no release cannot render it; a
    // `#` anywhere else is data in every release. `f5-irules` renders as
    // its 8.4 base does (ruling 8).
    for (dialect, want) in [
        (Some("tcl8.4"), Ok("# a")),
        (Some("tcl8.5"), Ok("{#} a")),
        (Some("tcl9.1"), Ok("{#} a")),
        (
            None,
            Err(DeclineReason::ReleaseAmbiguous(Axis::ListRendering)),
        ),
        (Some("f5-irules"), Ok("# a")),
    ] {
        assert_eq!(
            run(&LIST_OF_ARGS, "list", &["#", "a"], dialect),
            want.map(str::to_owned),
            "{dialect:?}"
        );
    }
    assert_eq!(
        run(&LIST_OF_ARGS, "list", &["a", "#b"], None).as_deref(),
        Ok("a #b")
    );
    assert_eq!(
        run(&LIST_LENGTH, "llength", &["a {b c}"], None).as_deref(),
        Ok("2")
    );
    assert_eq!(
        run(&LIST_LENGTH, "llength", &["a {b"], None),
        Err(DeclineReason::WrongRepresentation)
    );
    assert_eq!(
        run(
            &STRING_LENGTH,
            "string",
            &["length", "héllo"],
            Some("tcl9.0")
        )
        .as_deref(),
        Ok("5")
    );
    for dialect in [Some("tcl8.6"), None] {
        assert_eq!(
            run(&STRING_LENGTH, "string", &["length", "héllo"], dialect),
            Err(DeclineReason::ReleaseAmbiguous(Axis::SourceEncoding)),
            "{dialect:?}"
        );
    }
    assert_eq!(
        run(&STRING_LENGTH, "string", &["length", " a "], None).as_deref(),
        Ok("3")
    );
    for id in [
        NativeEvalId::ListOfArgs,
        NativeEvalId::ListLength,
        NativeEvalId::StringLength,
        NativeEvalId::FormatTemplate,
    ] {
        assert_eq!(id.owner(), EvaluatorOwner::Registry, "{id:?}");
    }
}

/// `format` over literal words under `dialect`'s release, through the
/// registry-owned route.
fn format_under(dialect: Option<&str>, words: &[&str]) -> Result<String, DeclineReason> {
    use tcl_registry::value_transfer::builtins::FORMAT_TEMPLATE;
    let profile = dialect.map(|name| tcl_dialect::DialectProfile::find(name).expect(name));
    let operands = words.iter().map(|word| literal(word, None)).collect();
    let mut inputs = TestInputs::new("format", operands);
    inputs.context = AnalysisContext::detached(profile);
    let outcome = evaluated(FORMAT_TEMPLATE.evaluate(&inputs, &mut Budget::evaluation()))?;
    match outcome.result {
        ExactValueOrUnavailable::Exact(value) => Ok(String::from_utf8(value.bytes).expect("text")),
        ExactValueOrUnavailable::Unavailable(_) => panic!("unavailable"),
    }
}

/// `format` runs the shared format core over `ConstOps` on its
/// registry-owned route. Oracle, tclsh 8.4 to 9.1: `format %5.2f 3.14159`
/// is ` 3.14`, `format %x 255` is `ff`, `format %s-%d a 5` is `a-5`,
/// `format %c 65` is `A`, `format %5s hi` is `   hi`, and `format %d abc`
/// raises — in every release, so under a profile that names none too.
#[test]
fn format_runs_the_shared_core() {
    for dialect in [
        Some("tcl8.4"),
        Some("tcl8.5"),
        Some("tcl8.6"),
        Some("tcl9.0"),
        Some("tcl9.1"),
        Some("f5-irules"),
        None,
    ] {
        for (words, want) in [
            (&["%5.2f", "3.14159"][..], " 3.14"),
            (&["%x", "255"][..], "ff"),
            (&["%s-%d", "a", "5"][..], "a-5"),
            (&["%c", "65"][..], "A"),
            (&["%5s", "hi"][..], "   hi"),
        ] {
            assert_eq!(
                format_under(dialect, words),
                Ok(want.to_owned()),
                "{dialect:?} {words:?}"
            );
        }
        assert_eq!(
            format_under(dialect, &["%d", "abc"]),
            Err(DeclineReason::WrongRepresentation),
            "{dialect:?}"
        );
    }
}

/// `format` answers under the target release's grammar, and a profile that
/// names no release answers only where every release agrees. Oracle,
/// tclsh 8.4 to 9.1: `%b` raises before 8.6 and `%p` and `%llu` before
/// 9.0; `format %d 010` is 8 up to 8.6 and 10 from 9.0; an unmodified `%d`
/// of 2147483648 is itself up to 8.6 and wraps to -2147483648 from 9.0;
/// `%#o 8` is `010` against `0o10` and `%#d 5` is `5` against `0d5`; and
/// `%.0d 0` is empty under 8.4, which formats through C's `printf`, and `0`
/// from 8.5 (D57).
#[test]
fn format_answers_per_release() {
    let error = Err(DeclineReason::WrongRepresentation);
    for (words, eight_four, eight_five, eight_six, nine) in [
        (&["%b", "5"][..], error, error, Ok("101"), Ok("101")),
        (&["%p", "255"][..], error, error, error, Ok("0xff")),
        (&["%llu", "5"][..], error, error, error, Ok("5")),
        (&["%d", "010"][..], Ok("8"), Ok("8"), Ok("8"), Ok("10")),
        (
            &["%d", "2147483648"][..],
            Ok("2147483648"),
            Ok("2147483648"),
            Ok("2147483648"),
            Ok("-2147483648"),
        ),
        (
            &["%#o", "8"][..],
            Ok("010"),
            Ok("010"),
            Ok("010"),
            Ok("0o10"),
        ),
        (&["%#d", "5"][..], Ok("5"), Ok("5"), Ok("5"), Ok("0d5")),
        (&["%.0d", "0"][..], Ok(""), Ok("0"), Ok("0"), Ok("0")),
    ] {
        for (dialect, want) in [
            ("tcl8.4", eight_four),
            ("tcl8.5", eight_five),
            ("tcl8.6", eight_six),
            ("tcl9.0", nine),
            ("tcl9.1", nine),
        ] {
            assert_eq!(
                format_under(Some(dialect), words),
                want.map(str::to_owned),
                "{dialect} {words:?}"
            );
        }
        // `f5-irules` formats as its 8.4 base does (ruling 8): tclsh 8.4
        // raises `bad field specifier` for `%b`, `%p` and `%llu`, and prints
        // 8, 2147483648, `010` and 5 for the rest.
        assert_eq!(
            format_under(Some("f5-irules"), words),
            eight_four.map(str::to_owned),
            "f5-irules {words:?}"
        );
        let answer = format_under(None, words);
        assert!(
            matches!(
                answer,
                Err(DeclineReason::ReleaseAmbiguous(
                    Axis::FormatVerbs | Axis::NumeralGrammar
                ))
            ),
            "{words:?}: {answer:?}"
        );
    }
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
    // Any declared specialisation stands in for the subcommand's own.
    static DECLARED: &tcl_registry::value_transfer::builtins::ListLengthSemantics =
        &tcl_registry::value_transfer::builtins::LIST_LENGTH;
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
        semantics: SemanticsDeclaration::Declared(DECLARED),
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

/// Every route stamp `reg` carries — `(spelling, route, owner)` — over each
/// command's resolved declaration and each subcommand that declares its own.
fn route_stamps(reg: &CommandRegistry) -> BTreeSet<(String, &'static str, &'static str)> {
    let mut stamps: BTreeSet<(String, &'static str, &'static str)> = BTreeSet::new();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            if let Some(route) = resolve_semantics(spec, None, None).route() {
                stamps.insert((spec.name.to_owned(), route_label(route), route_owner(route)));
            }
            for sub in spec.subcommands {
                if let SemanticsDeclaration::Declared(semantics) = sub.semantics {
                    stamps.insert((
                        format!("{} {}", spec.name, sub.name),
                        route_label(semantics.route()),
                        route_owner(semantics.route()),
                    ));
                }
            }
        }
    }
    stamps
}

/// The route stamps of every loadable dialect and the shipped packs.
fn pinned_route_stamps() -> BTreeSet<(String, &'static str, &'static str)> {
    [
        ("::tcl::dict::append", "direct:dict-append", "registry"),
        ("::tcl::dict::incr", "direct:dict-incr", "registry"),
        ("::tcl::dict::lappend", "direct:dict-lappend", "registry"),
        ("::tcl::dict::set", "direct:dict-set", "registry"),
        ("::tcl::dict::unset", "direct:dict-unset", "registry"),
        ("append", "direct:cell-append", "registry"),
        ("append_to_collection", "none:declared", "-"),
        ("dict append", "direct:dict-append", "registry"),
        ("dict incr", "direct:dict-incr", "registry"),
        ("dict lappend", "direct:dict-lappend", "registry"),
        ("dict set", "direct:dict-set", "registry"),
        ("dict unset", "direct:dict-unset", "registry"),
        ("expr", "expression:tcl.expr", "-"),
        ("foreach", "none:unauthored", "-"),
        ("foreach_in_collection", "none:declared", "-"),
        ("format", "direct:format-template", "registry"),
        ("incr", "direct:cell-increment", "registry"),
        ("lappend", "direct:cell-list-append", "registry"),
        ("list", "direct:list-of-args", "registry"),
        ("llength", "direct:list-length", "registry"),
        ("lmap", "none:unauthored", "-"),
        ("regexp", "direct:regexp-match", "registry"),
        ("regsub", "direct:regsub-substitute", "registry"),
        ("remove_from_collection", "none:declared", "-"),
        ("set", "direct:cell-write", "registry"),
        ("string length", "direct:string-length", "registry"),
        ("string range", "direct:string-range", "registry"),
        ("unset", "none:unauthored", "-"),
    ]
    .into_iter()
    .map(|(name, route, owner)| (name.to_owned(), route, owner))
    .collect()
}

/// The pinned-set gate: the specs carrying each route, over every loadable
/// dialect and the shipped packs. A route cannot appear, vanish, or move
/// without this list changing beside it.
#[test]
fn route_stamps_match_the_pinned_set() {
    let actual = route_stamps(&full_registry());
    let expected = pinned_route_stamps();
    let missing: Vec<_> = expected.difference(&actual).collect();
    let extra: Vec<_> = actual.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "route stamps drifted from the pinned set\nmissing: {missing:?}\nextra: {extra:?}"
    );
}

/// Slice 4's exit — "shipped builtins stay on the direct route"
/// (`docs/design/compiler/value-transfers-migration.md`): a workspace pack
/// declaring evaluators of its own moves no shipped route. Installing the
/// value-transfer lane's executable example (VT4.13) over every loadable
/// dialect and the shipped packs adds exactly its three spellings, each on
/// the implementation route; every shipped stamp is still the pinned set's,
/// and every direct route is still the registry's own.
#[test]
fn shipped_builtins_stay_on_the_direct_route() {
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/tenant.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
        },
        include_str!("../../tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec").to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    let mut reg = full_registry();
    for pack in &packs.packs {
        for command in &pack.commands {
            reg.insert(command.spec.clone());
        }
    }
    let actual = route_stamps(&reg);
    let pinned = pinned_route_stamps();
    let moved: Vec<_> = pinned.difference(&actual).collect();
    assert!(moved.is_empty(), "a shipped route moved: {moved:?}");
    let added: Vec<(&str, &str, &str)> = actual
        .difference(&pinned)
        .map(|(name, route, owner)| (name.as_str(), *route, *owner))
        .collect();
    assert_eq!(
        added,
        [
            ("tenant label", "implementation", "-"),
            ("tenant::label", "implementation", "-"),
            ("tenant::tag", "implementation", "-"),
        ]
    );
    assert!(
        actual
            .iter()
            .filter(|(_, route, _)| route.starts_with("direct:"))
            .all(|(_, _, owner)| *owner == "registry"),
        "{actual:#?}"
    );
}

/// Who implements a direct route, for the pinned set; `-` for any other
/// family.
fn route_owner(route: EvalRoute) -> &'static str {
    match route {
        EvalRoute::Direct { id } => match id.owner() {
            EvaluatorOwner::Registry => "registry",
            EvaluatorOwner::Transitional {
                retires_in_slice: 3,
            } => "transitional until slice 3",
            EvaluatorOwner::Transitional { .. } => "transitional",
        },
        EvalRoute::Expression { .. } | EvalRoute::Implementation(_) | EvalRoute::None { .. } => "-",
    }
}

fn route_label(route: EvalRoute) -> &'static str {
    match route {
        EvalRoute::Direct { id } => match id {
            NativeEvalId::CellIncrement => "direct:cell-increment",
            NativeEvalId::CellAppend => "direct:cell-append",
            NativeEvalId::CellListAppend => "direct:cell-list-append",
            NativeEvalId::CellWrite => "direct:cell-write",
            NativeEvalId::DictSet => "direct:dict-set",
            NativeEvalId::DictUnset => "direct:dict-unset",
            NativeEvalId::DictIncr => "direct:dict-incr",
            NativeEvalId::DictAppend => "direct:dict-append",
            NativeEvalId::DictListAppend => "direct:dict-lappend",
            NativeEvalId::StringRange => "direct:string-range",
            NativeEvalId::ListOfArgs => "direct:list-of-args",
            NativeEvalId::FormatTemplate => "direct:format-template",
            NativeEvalId::ListLength => "direct:list-length",
            NativeEvalId::StringLength => "direct:string-length",
            NativeEvalId::RegexpMatch => "direct:regexp-match",
            NativeEvalId::RegsubSubstitute => "direct:regsub-substitute",
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

/// The capability is part of the route (`value-evaluation.md` § *The
/// capability declaration*): two declared implementations that differ in any
/// one field — the pack, the id, the body's content hash, the target axes,
/// an input, a dependency, the budget — are two routes, so nothing keyed by
/// the route can serve one's answer for the other. The same declaration
/// twice is one route.
#[test]
fn the_capability_is_part_of_the_route_identity() {
    static INPUTS: [DeclaredInput; 1] = [DeclaredInput::Operand {
        index: 0,
        exactness: Exactness::Exact,
    }];
    static OTHER_INPUTS: [DeclaredInput; 1] = [DeclaredInput::IncomingTarget { index: 0 }];
    static DEPENDS: [ContextDependency; 2] = [
        ContextDependency::TclProfile,
        ContextDependency::ImplementationIdentity,
    ];
    static FEWER_DEPENDS: [ContextDependency; 1] = [ContextDependency::TclProfile];
    let base = EvaluatorCapability {
        identity: ImplementationIdentity {
            pack: "tenant",
            id: "tenant.label.v1",
            content_hash: 1,
        },
        host: HostKind::BoundedTcl,
        target: Needs::NONE,
        inputs: &INPUTS,
        depends: &DEPENDS,
        budget: ImplementationBudget {
            commands: Some(2000),
            wall_clock_ms: Some(20),
            value_bytes: Some(65536),
        },
        completion: CompletionSupport::NormalOnly,
    };
    let variants = [
        EvaluatorCapability {
            identity: ImplementationIdentity {
                pack: "other",
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            identity: ImplementationIdentity {
                id: "tenant.label.v2",
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            identity: ImplementationIdentity {
                content_hash: 2,
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            target: Needs::NUMERAL_GRAMMAR,
            ..base
        },
        EvaluatorCapability {
            inputs: &OTHER_INPUTS,
            ..base
        },
        EvaluatorCapability {
            depends: &FEWER_DEPENDS,
            ..base
        },
        EvaluatorCapability {
            budget: ImplementationBudget {
                commands: Some(1000),
                ..base.budget
            },
            ..base
        },
    ];
    let route = EvalRoute::Implementation(base);
    assert_eq!(route, EvalRoute::Implementation(base));
    assert_eq!(route.family(), "implementation");
    assert!(route.is_enabled());
    let mut routes = BTreeSet::new();
    routes.insert(format!("{route:?}"));
    let mut hashed = std::collections::HashSet::from([route]);
    for variant in variants {
        let other = EvalRoute::Implementation(variant);
        assert_ne!(other, route, "{variant:?}");
        assert!(hashed.insert(other), "{variant:?}");
        assert!(routes.insert(format!("{other:?}")), "{variant:?}");
    }
    assert_eq!(hashed.len(), variants.len() + 1);
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
}

/// The regexp owner's route for `command words…` over literal operands,
/// those marked `true` given the `VarWrite` role the resolver gives match
/// and result variables, with its store targets checked by the driver's
/// own validation before the answer is returned.
fn regex_route(command: &str, words: &[(&str, bool)], budget: &mut Budget) -> EvalAnswer {
    use tcl_registry::value_transfer::validate_outcome;
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get(command).expect(command), None, None);
    let semantics = semantics.semantics().expect("the regexp owner's route");
    let operands = words
        .iter()
        .map(|&(text, target)| literal(text, target.then_some(ArgRole::VarWrite)))
        .collect();
    let inputs = TestInputs::new(command, operands);
    let answer = semantics.evaluate(&inputs, budget);
    if let EvalAnswer::Evaluated(outcome) = &answer {
        assert_eq!(
            validate_outcome(
                &semantics.structure(&inputs),
                &semantics.store_targets(&inputs),
                outcome
            ),
            Ok(()),
            "{command} {words:?}"
        );
    }
    answer
}

/// The result text and each store as `(operand, Some(written text))` for a
/// write or `(operand, None)` for a preserve, of an evaluated answer.
fn regex_stores(answer: &EvalAnswer) -> (String, Vec<(usize, Option<String>)>) {
    let EvalAnswer::Evaluated(outcome) = answer else {
        panic!("not evaluated: {answer:?}");
    };
    let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
        panic!("no exact result: {outcome:?}");
    };
    let stores = outcome
        .ordered_stores
        .iter()
        .map(|store| match store {
            StoreOutcome::Write { target, value } => (
                (target.0).0,
                Some(String::from_utf8(value.bytes.clone()).expect("text")),
            ),
            StoreOutcome::Preserve { target } => ((target.0).0, None),
            other => panic!("unexpected store {other:?}"),
        })
        .collect();
    (
        String::from_utf8(result.bytes.clone()).expect("text"),
        stores,
    )
}

/// `regexp` writes or preserves its match variables (VT5.4; the Storage
/// row's "`regexp` no-match" and the Regexp row): a match writes one value
/// per match variable — an unmatched subgroup the empty string, or `-1 -1`
/// with `-indices` — and answers the count; a completed no-match preserves
/// every one and answers 0; `-inline` writes nothing and answers the list;
/// `-all` counts, the variables holding the last match; `-about` answers
/// the pattern's shape. Each answer is tclsh 8.4.20 to 9.1b0's
/// (`regexp_witnesses_match_every_release_on_path`).
#[test]
fn regexp_writes_or_preserves_its_match_variables() {
    use tcl_registry::TclType;
    let budget = || Budget::evaluation();
    let t = |text| (text, true);
    let w = |text| (text, false);

    // A completed no-match preserves every match variable.
    let answer = regex_route(
        "regexp",
        &[w("(x)(y)"), w("zz"), t("a"), t("b")],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        ("0".into(), vec![(2, None), (3, None)])
    );

    // A match writes each one; the unmatched subgroup writes the empty
    // string, or `-1 -1` with `-indices`, typed as the value it built.
    let answer = regex_route(
        "regexp",
        &[w("(a)(b)?"), w("ac"), t("m"), t("g1"), t("g2")],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        (
            "1".into(),
            vec![
                (2, Some("a".into())),
                (3, Some("a".into())),
                (4, Some(String::new()))
            ]
        )
    );
    let answer = regex_route(
        "regexp",
        &[
            w("-indices"),
            w("(a)(b)?"),
            w("ac"),
            t("m"),
            t("g1"),
            t("g2"),
        ],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        (
            "1".into(),
            vec![
                (3, Some("0 0".into())),
                (4, Some("0 0".into())),
                (5, Some("-1 -1".into()))
            ]
        )
    );
    let EvalAnswer::Evaluated(outcome) = &answer else {
        unreachable!()
    };
    assert_eq!(outcome.types.result, Some(TclType::Int));
    assert!(
        outcome
            .types
            .per_target
            .iter()
            .all(|(_, ty)| *ty == TclType::List),
        "{:?}",
        outcome.types
    );

    // `-inline` writes nothing and answers the list; `-all` counts, its
    // variables holding the last match; `-about` answers the pattern.
    for (words, want) in [
        (
            &[w("-inline"), w("-indices"), w("(a)(b)?"), w("ac")][..],
            "{0 0} {0 0} {-1 -1}",
        ),
        (&[w("-all"), w("a*"), w("xaax")][..], "3"),
        (&[w("-about"), w("(?:a)")][..], "0 REG_UNONPOSIX"),
        (&[w("-about"), w("a(b)c")][..], "1 {}"),
        (
            &[w("-start"), w("2"), w("-inline"), w("."), w("abcdef")][..],
            "c",
        ),
    ] {
        assert_eq!(
            regex_stores(&regex_route("regexp", words, &mut budget())),
            (want.into(), Vec::new()),
            "{words:?}"
        );
    }
    let answer = regex_route(
        "regexp",
        &[w("-all"), w("(a)"), w("banana"), t("m"), t("g")],
        &mut budget(),
    );
    assert_eq!(
        regex_stores(&answer),
        (
            "3".into(),
            vec![(3, Some("a".into())), (4, Some("a".into()))]
        )
    );
}

/// A `regexp` that established neither a match nor a no-match declines the
/// whole answer, never a no-match: a search cut short by the budget is
/// `Approximate`; a malformed pattern is the command's error, never a
/// value; a `-start` index the releases read differently is not evaluated;
/// and the variables the core writes must be the operands the resolver
/// named, or nothing is published.
#[test]
fn a_regexp_that_established_nothing_declines() {
    let budget = || Budget::evaluation();
    let t = |text| (text, true);
    let w = |text| (text, false);

    // A search cut short declines the whole answer: never a no-match.
    let mut starved = Budget::evaluation();
    starved.fuel = 5_000;
    let long = "a".repeat(300);
    assert_eq!(
        regex_route("regexp", &[w("^(a+)+b$"), w(&long), t("m")], &mut starved),
        EvalAnswer::Declined(DeclineReason::Approximate)
    );
    // The command's error is never a value.
    assert_eq!(
        regex_route("regexp", &[w("("), w("x")], &mut budget()),
        EvalAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    // A `-start` index the releases read differently (8 up to 8.6, 10
    // from 9.0) is not evaluated.
    assert_eq!(
        regex_route(
            "regexp",
            &[
                w("-start"),
                w("010"),
                w("-inline"),
                w("."),
                w("abcdefghijkl")
            ],
            &mut budget()
        ),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
    // The variables the core writes are the operands the resolver named,
    // or nothing is published: here `-nocase` is an option the roles took
    // for the pattern.
    assert_eq!(
        regex_route(
            "regexp",
            &[w("-nocase"), w("A"), t("a"), t("m")],
            &mut budget()
        ),
        EvalAnswer::Declined(DeclineReason::Unsupported)
    );
}

/// `regsub` answers the substituted text, or the count with the text
/// written to its variable — whether or not anything matched, as tclsh 8.4
/// to 9.1 do — and its callback form (`-command`, from 9.0) has no route.
#[test]
fn regsub_writes_its_variable_and_declines_its_callback() {
    let budget = || Budget::evaluation();
    let t = |text| (text, true);
    let w = |text| (text, false);

    // `regsub`: the text, or the count with the text written — whether or
    // not anything matched; the callback form has no route.
    assert_eq!(
        regex_stores(&regex_route(
            "regsub",
            &[w("-all"), w(""), w("abc"), w("-")],
            &mut budget()
        )),
        ("-a-b-c".into(), Vec::new())
    );
    assert_eq!(
        regex_stores(&regex_route(
            "regsub",
            &[w("-all"), w("a"), w("banana"), w("o"), t("v")],
            &mut budget()
        )),
        ("3".into(), vec![(4, Some("bonono".into()))])
    );
    assert_eq!(
        regex_stores(&regex_route(
            "regsub",
            &[w("z"), w("abc"), w("X"), t("v")],
            &mut budget()
        )),
        ("0".into(), vec![(3, Some("abc".into()))])
    );
    assert_eq!(
        regex_route(
            "regsub",
            &[w("-command"), w("a"), w("abc"), w("string toupper")],
            &mut budget()
        ),
        EvalAnswer::Declined(DeclineReason::NoRoute(NoRouteReason::Callback))
    );
}
