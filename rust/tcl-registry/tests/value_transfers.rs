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

/// A cell update of a place the inputs prove unbound runs over no prior
/// value only where every release the target names creates the cell
/// (`docs/design/compiler/value-transfers.md` § *Existence*, the release
/// rule): `incr fresh` is 1 and `incr fresh 2` is 2 from 8.5 and raises
/// `can't read "fresh": no such variable` under 8.4, so it declines under
/// 8.4 and under the `tcl` profile that spans both; `append` and `lappend`
/// create the cell in every release (tclsh 8.4 to 9.1).
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
    for dialect in ["tcl8.4", "tcl"] {
        assert_eq!(
            run("incr", &[], dialect),
            Err(DeclineReason::UnboundPlace),
            "{dialect}"
        );
    }
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

/// The may-write declarations (VT5.14, and the slice 5 review's S1): each
/// answers a may-bind of its `VarWrite` operand on the normal path, as the
/// kind the command binds — an array for `file stat` and `file lstat`, a
/// scalar for `file tempfile`'s name variable, `gets`, `chan gets` and
/// `tk_optionMenu`, either for the four `trace` forms — except `vwait`,
/// whose wait an unset ends too, so its transfer stays generic.
#[test]
fn each_may_write_declaration_answers_a_may_bind_of_its_target() {
    use tcl_registry::value_transfer::{BindingKind, LiteralInputs};
    type Declaration = (
        &'static str,
        Option<&'static str>,
        &'static [&'static str],
        Option<BindingKind>,
    );
    let declarations: [Declaration; 11] = [
        ("file", Some("stat"), &["f", "st"], Some(BindingKind::Array)),
        (
            "file",
            Some("lstat"),
            &["f", "st"],
            Some(BindingKind::Array),
        ),
        (
            "file",
            Some("tempfile"),
            &["path"],
            Some(BindingKind::Scalar),
        ),
        ("gets", None, &["chan", "line"], Some(BindingKind::Scalar)),
        (
            "chan",
            Some("gets"),
            &["chan", "line"],
            Some(BindingKind::Scalar),
        ),
        ("vwait", None, &["done"], None),
        (
            "tk_optionMenu",
            None,
            &[".m", "choice", "a", "b"],
            Some(BindingKind::Scalar),
        ),
        (
            "trace",
            Some("add"),
            &["variable", "v", "write", "cb"],
            Some(BindingKind::Either),
        ),
        (
            "trace",
            Some("remove"),
            &["variable", "v", "write", "cb"],
            Some(BindingKind::Either),
        ),
        (
            "trace",
            Some("variable"),
            &["v", "w", "cb"],
            Some(BindingKind::Either),
        ),
        (
            "trace",
            Some("vdelete"),
            &["v", "w", "cb"],
            Some(BindingKind::Either),
        ),
    ];
    let reg = full_registry();
    for (command, sub, args, kind) in declarations {
        let spec = reg.get(command).expect(command);
        let resolved = match sub {
            Some(name) => resolve_semantics(spec, Some(spec.subcommand(name).expect(name)), None),
            None => resolve_semantics(spec, None, None),
        };
        let semantics = resolved.semantics().expect("a declared semantics");
        assert_eq!(semantics.identity(), "may_write", "{command} {sub:?}");
        let words: Vec<&str> = sub.into_iter().chain(args.iter().copied()).collect();
        let targets = reg.arg_indices_for_role(command, &words, ArgRole::VarWrite);
        assert_eq!(targets.len(), 1, "{command} {sub:?}: one variable operand");
        let target = TargetId(OperandId(targets[0]));
        let inputs =
            LiteralInputs::new(command, sub, args, None).with_role(target.0, ArgRole::VarWrite);
        let answer = semantics.transfer(FactDomain::Existence, &inputs, &mut Budget::unbounded());
        match (kind, answer) {
            (Some(kind), TransferAnswer::Existence(transfer)) => {
                assert_eq!(transfer.paths.len(), 1, "{command} {sub:?}");
                assert_eq!(
                    transfer.paths[0].outcomes,
                    vec![(target, ExistenceOutcome::MayBind(kind))],
                    "{command} {sub:?}"
                );
            }
            (None, TransferAnswer::Generic) => {}
            (kind, answer) => panic!("{command} {sub:?}: {kind:?} answered {answer:?}"),
        }
        assert_eq!(
            semantics.transfer(FactDomain::Type, &inputs, &mut Budget::unbounded()),
            TransferAnswer::Generic,
            "{command} {sub:?}"
        );
    }
}

/// `const` (VT8.8) binds only an absent place: over an unbound place it
/// writes the value and returns the empty string; over any other place it
/// declines, since an existing variable raises and an existing constant
/// keeps its value (tclsh 9.0 and 9.1: `const c 5; const c 7; set c` is 5,
/// `set x 1; const x 2` raises `can't make constant "x": variable already
/// exists`). Its existence transfer binds a scalar on the normal path.
#[test]
fn const_binds_only_an_absent_place() {
    use tcl_registry::value_transfer::{BindingKind, DomainFact, Existence};
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("const").expect("const"), None, None);
    let semantics = resolved.semantics().expect("a declared route");
    assert_eq!(semantics.identity(), "const-write");
    let with_prior = |fact: Existence| {
        let mut inputs = TestInputs::new(
            "const",
            vec![
                literal("c", Some(ArgRole::VarWrite)),
                literal("5", Some(ArgRole::Value)),
            ],
        );
        inputs.prior.insert(
            "c".to_owned(),
            FactView::Domain(DomainFact::Existence(fact)),
        );
        inputs
    };
    let absent = with_prior(Existence::Unbound);
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&absent, &mut Budget::unbounded())
    else {
        panic!("an absent place is written");
    };
    assert_eq!(
        outcome.result,
        ExactValueOrUnavailable::Exact(ExactValue::from_literal(""))
    );
    assert_eq!(
        outcome.ordered_stores,
        vec![StoreOutcome::Write {
            target: TargetId(OperandId(0)),
            value: ExactValue::from_literal("5"),
        }]
    );
    for fact in [
        Existence::Bound(BindingKind::Scalar),
        Existence::MayBound,
        Existence::Bound(BindingKind::Either),
    ] {
        assert_eq!(
            semantics.evaluate(&with_prior(fact), &mut Budget::unbounded()),
            EvalAnswer::Declined(DeclineReason::Unsupported),
            "{fact:?}"
        );
    }
    match semantics.transfer(FactDomain::Existence, &absent, &mut Budget::unbounded()) {
        TransferAnswer::Existence(transfer) => assert_eq!(
            transfer.paths[0].outcomes,
            vec![(
                TargetId(OperandId(0)),
                ExistenceOutcome::Bind(BindingKind::Scalar)
            )]
        ),
        other => panic!("{other:?}"),
    }
}

/// `array unset` (VT8.8): without a pattern it unbinds an array and keeps
/// a scalar or an absent name, which it leaves alone without raising
/// (tclsh 8.4 to 9.1: `set s 1; array unset s` leaves `s`); a place that
/// may be either keeps the generic widening. With a pattern the array
/// stays. `array default` (from 9.0) may bind its name as an array.
#[test]
fn array_unset_unbinds_only_an_array() {
    use tcl_registry::value_transfer::{BindingKind, DomainFact, Existence};
    let reg = CommandRegistry::build_default();
    let array = reg.get("array").expect("array");
    let unset = resolve_semantics(array, Some(array.subcommand("unset").expect("unset")), None);
    let unset = unset.semantics().expect("a declared semantics");
    let run = |words: Vec<OperandView<'static>>, fact: Existence| {
        let mut inputs = TestInputs::new("array", words);
        inputs.prior.insert(
            "a".to_owned(),
            FactView::Domain(DomainFact::Existence(fact)),
        );
        unset.transfer(FactDomain::Existence, &inputs, &mut Budget::unbounded())
    };
    let outcome = |answer: TransferAnswer| match answer {
        TransferAnswer::Existence(transfer) => Some(transfer.paths[0].outcomes.clone()),
        TransferAnswer::Generic => None,
        other => panic!("{other:?}"),
    };
    let whole = || {
        vec![
            literal("unset", None),
            literal("a", Some(ArgRole::VarWrite)),
        ]
    };
    let target = TargetId(OperandId(1));
    for (fact, want) in [
        (
            Existence::Bound(BindingKind::Array),
            Some(ExistenceOutcome::Unbind),
        ),
        (
            Existence::Bound(BindingKind::Scalar),
            Some(ExistenceOutcome::Preserve),
        ),
        (Existence::Unbound, Some(ExistenceOutcome::Preserve)),
        (Existence::Bound(BindingKind::Either), None),
    ] {
        assert_eq!(
            outcome(run(whole(), fact)),
            want.map(|want| vec![(target, want)]),
            "{fact:?}"
        );
    }
    let patterned = vec![
        literal("unset", None),
        literal("a", Some(ArgRole::VarWrite)),
        literal("k*", None),
    ];
    assert_eq!(
        outcome(run(patterned, Existence::Bound(BindingKind::Array))),
        Some(vec![(target, ExistenceOutcome::Preserve)])
    );
    let default = resolve_semantics(
        array,
        Some(array.subcommand("default").expect("default")),
        None,
    );
    let default = default.semantics().expect("a declared semantics");
    let inputs = TestInputs::new(
        "array",
        vec![
            literal("default", None),
            literal("set", None),
            literal("a", Some(ArgRole::VarWrite)),
            literal("7", None),
        ],
    );
    match default.transfer(FactDomain::Existence, &inputs, &mut Budget::unbounded()) {
        TransferAnswer::Existence(transfer) => assert_eq!(
            transfer.paths[0].outcomes,
            vec![(
                TargetId(OperandId(2)),
                ExistenceOutcome::MayBind(BindingKind::Array)
            )]
        ),
        other => panic!("{other:?}"),
    }
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
        // A source layout without its body is the command's error.
        let source = TestInputs::new(name, vec![literal("x", None), literal("a b c", None)]);
        assert!(matches!(
            semantics.structure(&source),
            PlanAnswer::Declined(DeclineReason::WrongRepresentation)
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
        ("::tcl::dict::update", "none:unauthored", "-"),
        ("::tcl::dict::with", "none:unauthored", "-"),
        ("append", "direct:cell-append", "registry"),
        ("append_to_collection", "none:declared", "-"),
        ("array default", "none:declared", "-"),
        ("array set", "direct:array-set", "registry"),
        ("array unset", "none:unauthored", "-"),
        ("binary format", "direct:binary-format", "registry"),
        ("binary scan", "direct:binary-scan", "registry"),
        ("chan gets", "none:declared", "-"),
        ("const", "direct:const-write", "registry"),
        ("dict append", "direct:dict-append", "registry"),
        ("dict incr", "direct:dict-incr", "registry"),
        ("dict lappend", "direct:dict-lappend", "registry"),
        ("dict set", "direct:dict-set", "registry"),
        ("dict unset", "direct:dict-unset", "registry"),
        ("dict update", "none:unauthored", "-"),
        ("dict with", "none:unauthored", "-"),
        ("expr", "expression:tcl.expr", "-"),
        ("file lstat", "none:platform", "-"),
        ("file stat", "none:platform", "-"),
        ("file tempfile", "none:platform", "-"),
        ("foreach", "none:unauthored", "-"),
        ("foreachLine", "none:unauthored", "-"),
        ("foreach_in_collection", "none:declared", "-"),
        ("format", "direct:format-template", "registry"),
        ("gets", "none:declared", "-"),
        ("incr", "direct:cell-increment", "registry"),
        ("lappend", "direct:cell-list-append", "registry"),
        ("lassign", "direct:list-assign", "registry"),
        ("list", "direct:list-of-args", "registry"),
        ("llength", "direct:list-length", "registry"),
        ("lmap", "none:unauthored", "-"),
        ("regexp", "direct:regexp-match", "registry"),
        ("regsub", "direct:regsub-substitute", "registry"),
        ("remove_from_collection", "none:declared", "-"),
        ("scan", "direct:scan-format", "registry"),
        ("set", "direct:cell-write", "registry"),
        ("string length", "direct:string-length", "registry"),
        ("string range", "direct:string-range", "registry"),
        ("subst", "none:unauthored", "-"),
        ("tk_optionMenu", "none:declared", "-"),
        ("trace add", "none:callback", "-"),
        ("trace remove", "none:callback", "-"),
        ("trace variable", "none:callback", "-"),
        ("trace vdelete", "none:callback", "-"),
        ("unset", "none:unauthored", "-"),
        ("vwait", "none:declared", "-"),
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
            NativeEvalId::ConstWrite => "direct:const-write",
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
            NativeEvalId::ScanFormat => "direct:scan-format",
            NativeEvalId::BinaryScan => "direct:binary-scan",
            NativeEvalId::ListAssign => "direct:list-assign",
            NativeEvalId::ArraySet => "direct:array-set",
            NativeEvalId::BinaryFormat => "direct:binary-format",
        },
        EvalRoute::Expression { .. } => "expression:tcl.expr",
        EvalRoute::Implementation(_) => "implementation",
        EvalRoute::None { reason } => match reason {
            NoRouteReason::Unauthored => "none:unauthored",
            NoRouteReason::Declared => "none:declared",
            NoRouteReason::FormUnsupported => "none:form-unsupported",
            NoRouteReason::Callback => "none:callback",
            NoRouteReason::Platform => "none:platform",
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

/// A writing route's answer for `command words…` under `dialect`'s profile,
/// the words marked `true` given the `VarWrite` role, rendered for
/// comparison: the result, then each store in order as `write N value`,
/// `element N key value` or `preserve N` — or the decline.
fn destructured(
    command: &str,
    sub: Option<&str>,
    words: &[(&str, bool)],
    dialect: Option<&str>,
) -> Result<(String, Vec<String>), DeclineReason> {
    use tcl_registry::value_transfer::validate_outcome;
    let reg = CommandRegistry::build_default();
    let spec = reg.get(command).expect(command);
    let semantics = match sub {
        Some(name) => resolve_semantics(spec, Some(spec.subcommand(name).expect(name)), None),
        None => resolve_semantics(spec, None, None),
    };
    let semantics = semantics.semantics().expect("a destructuring route");
    let operands = words
        .iter()
        .map(|&(text, target)| literal(text, target.then_some(ArgRole::VarWrite)))
        .collect();
    let mut inputs = TestInputs::new(command, operands);
    inputs.context = AnalysisContext::detached(
        dialect.map(|name| tcl_dialect::DialectProfile::find(name).expect(name)),
    );
    match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) => {
            assert_eq!(
                validate_outcome(
                    &semantics.structure(&inputs),
                    &semantics.store_targets(&inputs),
                    &outcome
                ),
                Ok(()),
                "{command} {words:?}"
            );
            let text = |value: &ExactValue| String::from_utf8(value.bytes.clone()).expect("text");
            let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
                panic!("no exact result: {outcome:?}");
            };
            let stores = outcome
                .ordered_stores
                .iter()
                .map(|store| match store {
                    StoreOutcome::Write { target, value } => {
                        format!("write {} {}", (target.0).0, text(value))
                    }
                    StoreOutcome::WriteElement { target, key, value } => {
                        format!("element {} {key} {}", (target.0).0, text(value))
                    }
                    StoreOutcome::Preserve { target } => format!("preserve {}", (target.0).0),
                    other => panic!("unexpected store {other:?}"),
                })
                .collect();
            Ok((text(result), stores))
        }
        EvalAnswer::Declined(reason) => Err(reason),
        EvalAnswer::Pending => panic!("pending over literal words"),
    }
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

/// The destructuring writers run the shared cores (VT5.5; the Storage row's
/// "partial `scan`; … repeated targets; array and base overlap"): a
/// converted field writes its variable and a field the input did not reach
/// preserves it (`scan {12 nope} {%d %d} a b` is 1, `a` 12, `b` as it was);
/// `lassign` writes in order, a repeated variable twice, and returns the
/// rest; `binary scan` writes each field it scanned; `array set` writes one
/// element per key. Every answer is tclsh's under 8.4 to 9.1 (8.5 on for
/// `lassign`, `destructuring_witnesses_match_every_release_on_path`), and a
/// form a release reads differently declines on its axis.
#[test]
fn destructuring_writers_run_the_shared_cores() {
    let (t, w) = (target, word);
    let tcl90 = Some("tcl9.0");

    assert_eq!(
        destructured(
            "scan",
            None,
            &[w("12 nope"), w("%d %d"), t("a"), t("b")],
            tcl90
        ),
        Ok(answered("1", &["write 2 12", "preserve 3"]))
    );
    assert_eq!(
        destructured("scan", None, &[w(""), w("%d %d"), t("a"), t("b")], tcl90),
        Ok(answered("-1", &["preserve 2", "preserve 3"])),
        "the input ended before any conversion"
    );
    assert_eq!(
        destructured("scan", None, &[w("12 34"), w("%d %d")], None),
        Ok(answered("12 34", &[])),
        "the inline form"
    );
    assert_eq!(
        destructured("scan", None, &[w("abc"), w("%d")], None),
        Ok(answered("{}", &[])),
        "a failed inline field is the empty string"
    );
    // Past the 32-bit range the releases disagree: `4294967296` is kept up
    // to 8.6 and clamps to `2147483647` from 9.0, so no release is taken.
    assert_eq!(
        destructured("scan", None, &[w("4294967296"), w("%d"), t("a")], tcl90),
        Err(DeclineReason::ReleaseAmbiguous(Axis::IntTower))
    );
    // `%b` arrives in 8.6.
    assert!(matches!(
        destructured("scan", None, &[w("101"), w("%b"), t("a")], Some("tcl8.5")),
        Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(_)))
    ));
    assert_eq!(
        destructured("scan", None, &[w("101"), w("%b"), t("a")], Some("tcl8.6")),
        Ok(answered("1", &["write 2 5"]))
    );

    assert_eq!(
        destructured(
            "lassign",
            None,
            &[w("first second extra"), t("a"), t("a")],
            tcl90
        ),
        Ok(answered("extra", &["write 1 first", "write 2 second"]))
    );
    assert_eq!(
        destructured("lassign", None, &[w("a b"), t("x"), t("y"), t("z")], tcl90),
        Ok(answered("", &["write 1 a", "write 2 b", "write 3 "])),
        "past the end the variable is the empty string"
    );
    assert!(matches!(
        destructured("lassign", None, &[w("a b"), t("x")], Some("tcl8.4")),
        Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(_)))
    ));

    scan_declines_what_the_matcher_reads_apart();
    the_byte_and_array_writers_run_the_shared_cores();
}

/// [`destructuring_writers_run_the_shared_cores`]'s `scan` declines where
/// the shared matcher and the releases part: `%u` (the matcher reads it
/// signed, where `scan -1 %u` is `18446744073709551615` on every release),
/// an infinity spelling (`-Inf` from 8.5, no conversion in the matcher) and
/// a negative zero (`scan -0 %f` is `0.0` from 8.5).
fn scan_declines_what_the_matcher_reads_apart() {
    let (t, w) = (target, word);
    let tcl90 = Some("tcl9.0");
    for (subject, format) in [("-1", "%u"), ("-inf", "%f"), ("-0", "%f")] {
        assert_eq!(
            destructured("scan", None, &[w(subject), w(format), t("v")], tcl90),
            Err(DeclineReason::Unsupported),
            "scan {subject} {format}"
        );
    }
}

/// [`destructuring_writers_run_the_shared_cores`]'s `binary scan` and
/// `array set` half.
fn the_byte_and_array_writers_run_the_shared_cores() {
    let (t, w) = (target, word);
    let tcl90 = Some("tcl9.0");
    assert_eq!(
        destructured(
            "binary",
            Some("scan"),
            &[w("scan"), w("\u{1}\u{2}"), w("cc"), t("a"), t("b")],
            tcl90
        ),
        Ok(answered("2", &["write 3 1", "write 4 2"]))
    );
    assert_eq!(
        destructured(
            "binary",
            Some("scan"),
            &[w("scan"), w("\u{1}"), w("cc"), t("a"), t("b")],
            tcl90
        ),
        Ok(answered("1", &["write 3 1", "preserve 4"])),
        "the data ran out before the second field"
    );
    assert_eq!(
        destructured(
            "binary",
            Some("scan"),
            &[w("scan"), w("\u{1}\u{2}"), w("cc"), t("a")],
            tcl90
        ),
        Err(DeclineReason::WrongRepresentation),
        "a field without a variable raises once the scan reaches it with data left"
    );

    assert_eq!(
        destructured(
            "array",
            Some("set"),
            &[w("set"), t("arr"), w("k1 v1 k2 v2 k1 v3")],
            None
        ),
        Ok(answered("", &["element 1 k1 v3", "element 1 k2 v2"]))
    );
    assert_eq!(
        destructured(
            "array",
            Some("set"),
            &[w("set"), t("arr"), w("k1 v1 k2")],
            None
        ),
        Err(DeclineReason::WrongRepresentation),
        "an odd-length list raises"
    );
}

/// The loops' source layout answers an iteration plan (VT5.7): one binder
/// per name of the var-list word, padded past the list's end, over the one
/// list, the body in the caller's frame with `break` and `continue`
/// absorbed, and nothing bound on the zero-iteration path. Several var-list
/// and list pairs are several iterables, which one plan does not describe;
/// a var-list the analysis does not know names no binders; an empty one is
/// the command's error.
#[test]
fn the_source_layout_answers_an_iteration_plan() {
    use tcl_registry::FrameLevel;
    use tcl_registry::value_transfer::{
        Binder, BinderName, BindingKind, BodyPlan, CompletionProtocol, ExitRule, IterationPlan,
    };
    let reg = CommandRegistry::build_default();
    for name in ["foreach", "lmap"] {
        let resolved = resolve_semantics(reg.get(name).expect(name), None, None);
        let semantics = resolved.semantics().expect("declared");
        let words = |var_list| {
            TestInputs::new(
                name,
                vec![
                    literal(var_list, None),
                    literal("1 10 2 20", None),
                    literal("puts $a", Some(ArgRole::Body)),
                ],
            )
        };
        let PlanAnswer::Iterate(plan) = semantics.structure(&words("a b")) else {
            panic!("{name}: no iteration plan");
        };
        assert_eq!(
            plan,
            IterationPlan {
                binders: ["a", "b"]
                    .map(|binder| Binder {
                        name: BinderName::Declared(binder.to_owned()),
                        kind: BindingKind::Scalar,
                    })
                    .to_vec(),
                iterable: IterableKind::List(OperandId(1)),
                body: Some(BodyPlan {
                    body: OperandId(2),
                    frame: FrameLevel::Relative(0),
                }),
                exit: ExitRule::Exhaustion,
                zero_iterations_bind: false,
                completion: CompletionProtocol::Absorb(&[
                    tcl_registry::completion::CompletionCode::Break,
                    tcl_registry::completion::CompletionCode::Continue,
                ]),
            },
            "{name}"
        );
        assert_eq!(
            semantics.structure(&words("")),
            PlanAnswer::Declined(DeclineReason::WrongRepresentation),
            "{name}: an empty var-list"
        );
        let mut unknown = words("a b");
        unknown
            .operands
            .insert(0, FactView::Top(DeclineReason::NotExact));
        assert_eq!(
            semantics.structure(&unknown),
            PlanAnswer::Declined(DeclineReason::NotExact),
            "{name}: an unknown var-list"
        );
        let lockstep = TestInputs::new(
            name,
            vec![
                literal("a", None),
                literal("1 2", None),
                literal("b", None),
                literal("3 4", None),
                literal("puts $a$b", Some(ArgRole::Body)),
            ],
        );
        assert_eq!(
            semantics.structure(&lockstep),
            PlanAnswer::Declined(DeclineReason::Unsupported),
            "{name}: two lists in lockstep"
        );
    }
}

/// The structural plan `dict SUB` (or its `::tcl::dict::` spelling when
/// `qualified`) answers over `words`, with `d` holding `prior` when it is
/// not empty.
fn dict_body_plan(
    reg: &CommandRegistry,
    sub: &str,
    qualified: bool,
    words: &[(&'static str, Option<ArgRole>)],
    prior: &str,
) -> PlanAnswer {
    let dict = reg.get("dict").expect("dict");
    let (semantics, command) = if qualified {
        let spec = reg
            .get(if sub == "with" {
                "::tcl::dict::with"
            } else {
                "::tcl::dict::update"
            })
            .expect("the qualified spelling");
        (resolve_semantics(spec, None, None), spec.name)
    } else {
        (
            resolve_semantics(dict, Some(dict.subcommand(sub).expect(sub)), None),
            "dict",
        )
    };
    let semantics = semantics.semantics().expect("a body plan");
    let mut operands: Vec<OperandView<'static>> = Vec::new();
    if !qualified {
        operands.push(literal(if sub == "with" { "with" } else { "update" }, None));
    }
    operands.extend(words.iter().map(|&(text, role)| literal(text, role)));
    let mut inputs = TestInputs::new(command, operands);
    if !qualified {
        inputs.view.argument_offset = 1;
    }
    if !prior.is_empty() {
        inputs.prior.insert(
            "d".to_owned(),
            FactView::Exact(ExactValue::from_literal(prior), None),
        );
    }
    semantics.structure(&inputs)
}

/// `dict with` and `dict update` are structural plans (VT5.7), under both
/// spellings: the binders are a projection on body entry — the proven keys
/// of the dictionary for `dict with` (`set d {a 1}; dict with d {incr a;
/// set result done}` binds `a`; tclsh 8.5 to 9.1 answer `done` and leave
/// `d` as `a 2`), a key path's nested dictionary's keys, and the declared
/// variables for `dict update` — the body runs in the caller's frame, the
/// bound keys are written back into the dictionary operand, and the body's
/// completion is the command's. A dictionary the analysis does not know
/// names no binders, so `dict with` declines; a path key it lacks, or a
/// value that is no dictionary, is the command's error.
#[test]
fn dict_with_binds_the_proven_keys() {
    use tcl_registry::FrameLevel;
    use tcl_registry::value_transfer::{
        Binder, BinderName, BindingKind, BodyPlan, CompletionProtocol, Reconcile,
    };
    let reg = CommandRegistry::build_default();
    let plan_of = |sub, qualified, words: &[(&'static str, Option<ArgRole>)], prior| {
        dict_body_plan(&reg, sub, qualified, words, prior)
    };
    let declared = |names: &[&str]| -> Vec<Binder> {
        names
            .iter()
            .map(|name| Binder {
                name: BinderName::Declared((*name).to_owned()),
                kind: BindingKind::Scalar,
            })
            .collect()
    };
    let body = |at: usize| BodyPlan {
        body: OperandId(at),
        frame: FrameLevel::Relative(0),
    };
    let var = ("d", Some(ArgRole::VarWrite));
    let script = ("incr a; set result done", Some(ArgRole::Body));
    for qualified in [false, true] {
        let offset = usize::from(!qualified);
        assert_eq!(
            plan_of("with", qualified, &[var, script], "a 1"),
            PlanAnswer::Body {
                binders: declared(&["a"]),
                body: body(offset + 1),
                reconcile: Reconcile::WriteBackKeys(OperandId(offset)),
                completion: CompletionProtocol::TclBody,
            },
            "qualified: {qualified}"
        );
        assert_eq!(
            plan_of(
                "with",
                qualified,
                &[var, ("x", None), script],
                "x {a 1 b 2 a 3} y 4"
            ),
            PlanAnswer::Body {
                binders: declared(&["a", "b"]),
                body: body(offset + 2),
                reconcile: Reconcile::WriteBackKeys(OperandId(offset)),
                completion: CompletionProtocol::TclBody,
            },
            "qualified: {qualified}: a key path"
        );
        assert_eq!(
            plan_of("with", qualified, &[var, script], ""),
            PlanAnswer::Declined(DeclineReason::NotExact),
            "qualified: {qualified}: an unknown dictionary"
        );
        assert_eq!(
            plan_of("with", qualified, &[var, ("z", None), script], "x 1"),
            PlanAnswer::Declined(DeclineReason::WrongRepresentation),
            "qualified: {qualified}: a path key the dictionary lacks"
        );
        assert_eq!(
            plan_of(
                "update",
                qualified,
                &[
                    var,
                    ("k", None),
                    ("v", None),
                    ("j", None),
                    ("w", None),
                    script
                ],
                ""
            ),
            PlanAnswer::Body {
                binders: [offset + 2, offset + 4]
                    .map(|at| Binder {
                        name: BinderName::Operand(OperandId(at)),
                        kind: BindingKind::Scalar,
                    })
                    .to_vec(),
                body: body(offset + 5),
                reconcile: Reconcile::WriteBackKeys(OperandId(offset)),
                completion: CompletionProtocol::TclBody,
            },
            "qualified: {qualified}: dict update"
        );
    }
}

/// `subst switches… {template}` under `dialect`'s profile through `subst`'s
/// declared template plan: the template braced, its content from 1, and
/// each switch overridden by the fact the driver would prove.
fn template_plan_of(
    dialect: &str,
    switches: &[(&'static str, Option<FactView>)],
    template: &'static str,
) -> PlanAnswer {
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    let semantics = semantics.semantics().expect("the template plan");
    let mut operands: Vec<OperandView<'static>> = switches
        .iter()
        .map(|&(text, _)| literal(text, None))
        .collect();
    operands.push(literal(template, None));
    let last = operands.len() - 1;
    let mut inputs = TestInputs::new("subst", operands);
    for (index, (_, fact)) in switches.iter().enumerate() {
        if let Some(fact) = fact {
            inputs.operands.insert(index, fact.clone());
        }
    }
    inputs.structures.insert(
        last,
        WordStructure {
            braced: true,
            parts: vec![WordPart::Literal {
                span: tcl_lexer::Span::new(1, 1 + small(template.len())),
                text: template.to_owned(),
            }],
        },
    );
    // `tcl` is the permissive sink, which names no release; an empty name
    // is no profile at all.
    let profile = tcl_dialect::DialectProfile::find(dialect)
        .or_else(|| (dialect == "tcl").then(tcl_dialect::DialectProfile::plain_tcl));
    inputs.context = AnalysisContext::detached(profile);
    semantics.structure(&inputs)
}

/// A short offset as a span coordinate.
fn small(offset: usize) -> u32 {
    u32::try_from(offset).expect("a small offset")
}

/// A literal switch word, its own spelling.
fn switch(text: &'static str) -> (&'static str, Option<FactView>) {
    (text, None)
}

/// The kinds `backslashes`, `commands`, `variables`.
fn kinds(
    backslashes: bool,
    commands: bool,
    variables: bool,
) -> tcl_registry::substitution::SubstitutionKinds {
    tcl_registry::substitution::SubstitutionKinds {
        backslashes,
        commands,
        variables,
    }
}

/// The `[script]` region at word offset `start`.
fn region(start: usize, script: &str) -> tcl_registry::value_transfer::ScriptRegion {
    tcl_registry::value_transfer::ScriptRegion {
        span: tcl_lexer::Span::new(small(start), small(start + script.len() + 2)),
        script: BodyRegion {
            script: script.to_owned(),
            base_offset: start + 1,
            frame: tcl_registry::FrameLevel::Relative(0),
        },
    }
}

/// The `$name` read at word offset `start`.
fn read(start: usize, name: &str) -> tcl_registry::value_transfer::VariableRead {
    tcl_registry::value_transfer::VariableRead {
        span: tcl_lexer::Span::new(small(start), small(start + 1 + name.len())),
        name: name.to_owned(),
        element: None,
    }
}

/// A braced template's plan, the template operand at `operand`.
fn braced_plan(
    operand: usize,
    kinds: tcl_registry::substitution::SubstitutionKinds,
    script_regions: Vec<tcl_registry::value_transfer::ScriptRegion>,
    reads: Vec<tcl_registry::value_transfer::VariableRead>,
    escapes: Vec<tcl_lexer::Span>,
) -> PlanAnswer {
    PlanAnswer::TemplateWord(tcl_registry::value_transfer::TemplateWordPlan {
        operand: OperandId(operand),
        kinds,
        braced: true,
        dynamic: false,
        script_regions,
        reads,
        escapes,
    })
}

/// The page's first five template programs under `dialect`, each the
/// plan and the plan the program must have: the kinds the switches run
/// and the regions, reads and escapes they leave.
fn switch_witnesses(dialect: &str) -> Vec<(PlanAnswer, PlanAnswer)> {
    let span = tcl_lexer::Span::new;
    vec![
        // `a$b5`: the bracket still runs.
        (
            template_plan_of(dialect, &[switch("-novariables")], "a$b[set b]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(4, "set b")],
                vec![],
                vec![],
            ),
        ),
        // `a5[set b]`.
        (
            template_plan_of(dialect, &[switch("-nocommands")], "a$b[set b]"),
            braced_plan(
                1,
                kinds(true, false, true),
                vec![],
                vec![read(2, "b")],
                vec![],
            ),
        ),
        // `x6`: `$b` inside the region substitutes, as the region's own.
        (
            template_plan_of(dialect, &[switch("-novariables")], "x[expr {$b+1}]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(2, "expr {$b+1}")],
                vec![],
                vec![],
            ),
        ),
        // `a1`.
        (
            template_plan_of(dialect, &[switch("-novariables")], "a[string length $b]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(2, "string length $b")],
                vec![],
                vec![],
            ),
        ),
        // `a$b[set b]A`: only the escape materialises.
        (
            template_plan_of(
                dialect,
                &[switch("-novariables"), switch("-nocommands")],
                "a$b[set b]\\x41",
            ),
            braced_plan(
                2,
                kinds(true, false, false),
                vec![],
                vec![],
                vec![span(11, 15)],
            ),
        ),
    ]
}

/// The page's next six template programs under `dialect`, as
/// [`switch_witnesses`]: escapes, a proven switch, the regions that run in
/// the caller's frame, and the two families together.
fn template_witnesses(dialect: &str) -> Vec<(PlanAnswer, PlanAnswer)> {
    let span = tcl_lexer::Span::new;
    let proven = |text: &str| Some(FactView::Exact(ExactValue::from_literal(text), None));
    vec![
        // `a\tb`, four characters.
        (
            template_plan_of(dialect, &[switch("-nobackslashes")], "a\\tb"),
            braced_plan(1, kinds(false, true, true), vec![], vec![], vec![]),
        ),
        // `a$b5`: the escape protects the `$`.
        (
            template_plan_of(dialect, &[], "a\\$b[set b]"),
            braced_plan(
                0,
                kinds(true, true, true),
                vec![region(5, "set b")],
                vec![],
                vec![span(2, 4)],
            ),
        ),
        // `set opt -novariables; subst $opt {hello $name}` is `hello
        // $name`: the proven switch reads as its spelling.
        (
            template_plan_of(dialect, &[("$opt", proven("-novariables"))], "hello $name"),
            braced_plan(1, kinds(true, true, false), vec![], vec![], vec![]),
        ),
        // `p` returns 2: the region runs in the caller's frame.
        (
            template_plan_of(dialect, &[], "[set c 2]"),
            braced_plan(
                0,
                kinds(true, true, true),
                vec![region(1, "set c 2")],
                vec![],
                vec![],
            ),
        ),
        // `q` returns `2 2`.
        (
            template_plan_of(dialect, &[switch("-novariables")], "[incr c]"),
            braced_plan(
                1,
                kinds(true, true, false),
                vec![region(1, "incr c")],
                vec![],
                vec![],
            ),
        ),
        // The two families together: an error on every release.
        (
            template_plan_of(
                dialect,
                &[switch("-nocommands"), switch("-variables")],
                "a$b",
            ),
            PlanAnswer::Declined(DeclineReason::WrongRepresentation),
        ),
    ]
}

/// `subst`'s template-word plan (VT5.8) answers the page's fourteen
/// programs (`docs/design/compiler/value-transfers.md` § *The template-word
/// plan*): the kinds its switches run, read over their proven values, and
/// the braced template's script regions, variable reads and escapes under
/// those kinds, each at its offset in the word (the content from 1). A
/// template the parser substitutes reaches the command computed. The two
/// families together are an error on every release.
#[test]
fn the_template_plan_answers_the_fourteen_witnesses() {
    let span = tcl_lexer::Span::new;
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let rows = switch_witnesses(dialect)
            .into_iter()
            .chain(template_witnesses(dialect));
        for (index, (got, want)) in rows.enumerate() {
            assert_eq!(got, want, "{dialect}: row {index}");
        }
    }
    // `set t {a$b}; subst -nocommands $t` is `a5`: a template the parser
    // substitutes reaches the command computed.
    let mut dynamic = TestInputs::new(
        "subst",
        vec![literal("-nocommands", None), literal("$t", None)],
    );
    dynamic.structures.insert(
        1,
        WordStructure {
            braced: false,
            parts: vec![WordPart::VariableRead {
                span: span(0, 2),
                name: "t".to_owned(),
                element: None,
            }],
        },
    );
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    assert_eq!(
        semantics
            .semantics()
            .expect("the template plan")
            .structure(&dynamic),
        PlanAnswer::TemplateWord(tcl_registry::value_transfer::TemplateWordPlan {
            operand: OperandId(1),
            kinds: kinds(true, false, true),
            braced: false,
            dynamic: true,
            script_regions: vec![],
            reads: vec![],
            escapes: vec![],
        })
    );
}

/// The 9.1 positive family (VT5.8): it answers under a 9.1 profile, is the
/// command's error below it (`bad switch "-variables"` on tclsh 8.4 and
/// 8.5, `bad option` on 8.6 and 9.0), and declines as release-ambiguous
/// under a profile that spans both, while a question with no profile reads
/// every switch; the two families together raise on every release, so
/// they raise under the spanning profile too.
#[test]
fn the_positive_switches_are_9_1s() {
    let span = tcl_lexer::Span::new;
    let spanning = PlanAnswer::Declined(DeclineReason::ReleaseAmbiguous(Axis::Availability(
        tcl_dialect::model::SpecSurface::TCL91[0],
    )));
    let positive = [
        (
            "-variables",
            "a$b[set b]",
            braced_plan(
                1,
                kinds(false, false, true),
                vec![],
                vec![read(2, "b")],
                vec![],
            ),
        ),
        (
            "-backslashes",
            "a$b[set b]\\x41",
            braced_plan(
                1,
                kinds(true, false, false),
                vec![],
                vec![],
                vec![span(11, 15)],
            ),
        ),
    ];
    for (word, template, answered) in positive {
        assert_eq!(
            template_plan_of("tcl9.1", &[switch(word)], template),
            answered
        );
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                template_plan_of(dialect, &[switch(word)], template),
                PlanAnswer::Declined(DeclineReason::WrongRepresentation),
                "{dialect} {word}"
            );
        }
        assert_eq!(template_plan_of("tcl", &[switch(word)], template), spanning);
        // With no profile at all the question is surface-blind, as a
        // profile-less registry's own: every switch is available.
        assert_eq!(
            template_plan_of("", &[switch(word)], template),
            template_plan_of("tcl9.1", &[switch(word)], template)
        );
    }
    assert_eq!(
        template_plan_of("tcl", &[switch("-nocommands"), switch("-variables")], "a$b"),
        PlanAnswer::Declined(DeclineReason::WrongRepresentation)
    );
}

/// A finite set of switch values joins per member (VT5.8), a raising member
/// contributing nothing; an unproven switch runs every kind; a call without
/// its template, or a template holding a construct `subst` rejects, is the
/// command's error; and an array index substitutes
/// whatever the kinds say — `subst -nocommands {$a([set b])}` runs `set b`
/// (tclsh 8.4 to 9.1 read `a(5)`).
#[test]
fn a_template_plan_joins_proven_switches_and_reads_indexes() {
    let set = |members: &[&str]| {
        Some(FactView::Finite(
            members
                .iter()
                .map(|member| ExactValue::from_literal(member))
                .collect(),
            None,
        ))
    };
    assert_eq!(
        template_plan_of("tcl9.1", &[("$s", set(&["-variables", "-commands"]))], "x"),
        braced_plan(1, kinds(false, true, true), vec![], vec![], vec![])
    );
    assert_eq!(
        template_plan_of("tcl8.6", &[("$s", set(&["-novariables", "-bogus"]))], "x"),
        braced_plan(1, kinds(true, true, false), vec![], vec![], vec![])
    );
    assert_eq!(
        template_plan_of(
            "tcl8.6",
            &[("$s", Some(FactView::Top(DeclineReason::NotExact)))],
            "a$b"
        ),
        braced_plan(
            1,
            kinds(true, true, true),
            vec![],
            vec![read(2, "b")],
            vec![]
        )
    );
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    assert_eq!(
        semantics
            .semantics()
            .expect("the template plan")
            .structure(&TestInputs::new("subst", vec![])),
        PlanAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    // A construct `subst` rejects is the command's error: `subst {a[set b}`
    // raises `missing close-bracket` on tclsh 8.4 to 9.1.
    assert_eq!(
        template_plan_of("tcl8.6", &[], "a[set b"),
        PlanAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    assert_eq!(
        template_plan_of("tcl8.6", &[switch("-nocommands")], "a[set b"),
        braced_plan(1, kinds(true, false, true), vec![], vec![], vec![])
    );
    let PlanAnswer::TemplateWord(indexed) =
        template_plan_of("tcl8.6", &[switch("-nocommands")], "$a([set b])")
    else {
        panic!("a template plan");
    };
    assert_eq!(
        indexed.reads,
        [tcl_registry::value_transfer::VariableRead {
            span: tcl_lexer::Span::new(1, 12),
            name: "a".to_owned(),
            element: Some("[set b]".to_owned()),
        }]
    );
    assert_eq!(
        indexed
            .script_regions
            .iter()
            .map(|region| region.script.script.as_str())
            .collect::<Vec<_>>(),
        ["set b"]
    );
}
