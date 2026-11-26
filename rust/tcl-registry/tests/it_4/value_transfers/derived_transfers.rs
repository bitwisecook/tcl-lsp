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

//! What a declaration derives: expression assembly, abstention at each scope, bind and unbind transfers and scope-alias plans.

use super::*;

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
            quoted: false,
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
            quoted: true,
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
    assert_eq!(EXPR.variable_reads(&quoted).len(), 0);

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
    // `dict for` carries the header shape and declares no iteration: its
    // declaration is the callback's `none`, which plans nothing.
    let dict = reg.get("dict").expect("dict");
    let sub = dict.subcommand("for").expect("dict for");
    assert!(sub.loop_list_header);
    let resolved = resolve_semantics(dict, Some(sub), None);
    assert_eq!(
        resolved.route(),
        Some(EvalRoute::None {
            reason: NoRouteReason::Callback
        })
    );
    assert_eq!(
        resolved.semantics().map(CommandSemantics::identity),
        Some("no-route:callback")
    );
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

/// A consumer with no SSA reads no existence: literal-word inputs
/// answer every existence read — a variable, a prior store, an operand —
/// `Unavailable` at the structure tier, which is neither bound nor unbound,
/// and a variable's exact value `NotExact`.
#[test]
fn literal_inputs_answer_existence_unavailable() {
    use tcl_registry::value_transfer::{AnalysisTier, LiteralInputs};
    let inputs = LiteralInputs::new("set", None, &["x", "1"], None);
    let unavailable = FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure));
    assert_eq!(inputs.variable("x", FactDomain::Existence), unavailable);
    assert_eq!(
        inputs.prior_store(&PlaceRef::scalar("x"), FactDomain::Existence),
        unavailable
    );
    assert_eq!(
        inputs.operand(OperandId(0), FactDomain::Existence),
        unavailable
    );
    assert_eq!(
        inputs.variable("x", FactDomain::ExactValue),
        FactView::Top(DeclineReason::NotExact)
    );
}

/// `const` binds only an absent place: over an unbound place it
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

/// `array unset`: without a pattern it unbinds an array and keeps
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
/// The binders that link a local to a cell another frame holds state that
/// plan and no value: `global`, `variable`, `my variable` and `sharedvar`
/// each name the locals they link — the operands the resolver gives the
/// `VarWrite` role, so `variable`'s values are not among them — and where
/// the cells live, and none has a route.
#[test]
fn the_binders_state_their_scope_alias_plan() {
    use tcl_registry::value_transfer::scope_alias::{GLOBAL, MY_VARIABLE, SHAREDVAR, VARIABLE};
    use tcl_registry::value_transfer::{AliasFrame, ScopeAliasPlan};
    for (semantics, frame) in [
        (&GLOBAL, AliasFrame::Global),
        (&VARIABLE, AliasFrame::Namespace),
        (&MY_VARIABLE, AliasFrame::Object),
        (&SHAREDVAR, AliasFrame::Connection),
    ] {
        let inputs = TestInputs::new(
            "variable",
            vec![
                literal("a", Some(ArgRole::VarWrite)),
                literal("1", None),
                literal("b", Some(ArgRole::VarWrite)),
            ],
        );
        assert_eq!(
            semantics.structure(&inputs),
            PlanAnswer::ScopeAlias(ScopeAliasPlan {
                locals: vec![OperandId(0), OperandId(2)],
                frame,
            }),
            "{}",
            semantics.identity
        );
        assert_eq!(
            semantics.route(),
            EvalRoute::None {
                reason: NoRouteReason::Declared
            }
        );
        assert_eq!(
            semantics.evaluate(&inputs, &mut Budget::evaluation()),
            EvalAnswer::Declined(DeclineReason::NoRoute(NoRouteReason::Declared))
        );
    }
}

/// What a consumer with no compilation unit reads of a binder: a
/// scope alias's frame, from its plan, for the invocation it holds — none for
/// `upvar`, whose frame its level word selects — and an alias-pair call's
/// level and pairs, the level word present by argument-count parity.
#[test]
fn a_binder_names_its_frame_and_its_level() {
    use tcl_registry::frame_effect::FrameLevel;
    use tcl_registry::value_transfer::AliasFrame;
    let registry = tcl_registry::default_registry();
    for (name, args, frame) in [
        ("global", &["a", "b"][..], Some(AliasFrame::Global)),
        ("::global", &["a"][..], Some(AliasFrame::Global)),
        ("variable", &["a", "1"][..], Some(AliasFrame::Namespace)),
        ("my", &["variable", "a"][..], Some(AliasFrame::Object)),
        ("upvar", &["1", "a", "b"][..], None),
        ("set", &["a", "1"][..], None),
    ] {
        assert_eq!(
            registry.alias_frame(name, args, None),
            frame,
            "{name} {args:?}"
        );
    }
    let upvar = registry
        .frame_effect("upvar")
        .expect("upvar's frame effect");
    for (args, level, pairs) in [
        (
            &["$lvl", "a", "b"][..],
            FrameLevel::Dynamic,
            &["a", "b"][..],
        ),
        (&["1", "b"][..], FrameLevel::DEFAULT, &["1", "b"][..]),
        (
            &["#0", "a", "b"][..],
            FrameLevel::Absolute(0),
            &["a", "b"][..],
        ),
        (
            &["0", "a", "b", "c", "d"][..],
            FrameLevel::Relative(0),
            &["a", "b", "c", "d"][..],
        ),
    ] {
        assert_eq!(upvar.resolve_in(args, registry), (level, pairs), "{args:?}");
    }
}

/// `info default procname arg varname` writes the variable on every normal
/// completion — the parameter's default and the result 1, or the empty
/// string and the result 0 for a parameter with none — as tclsh 8.4 to 9.1
/// do; where the analysis proves neither the procedure nor the parameter,
/// the variable is still written, with a value the source does not give,
/// and the result is an unknown boolean.
#[test]
fn info_default_writes_the_default_the_analysis_proves() {
    use tcl_registry::value_transfer::scope_alias::INFO_DEFAULT;
    let inputs = |defaults: &[((&str, &str), ParameterDefault)]| {
        let mut inputs = TestInputs::new(
            "info",
            vec![
                literal("default", None),
                literal("f", None),
                literal("b", None),
                literal("v", Some(ArgRole::VarWrite)),
            ],
        );
        inputs.view.argument_offset = 1;
        inputs.defaults = defaults
            .iter()
            .map(|((procedure, parameter), default)| {
                (
                    ((*procedure).to_owned(), (*parameter).to_owned()),
                    default.clone(),
                )
            })
            .collect();
        inputs
    };
    let target = TargetId(OperandId(3));
    let run = |defaults: &[((&str, &str), ParameterDefault)]| {
        evaluated(INFO_DEFAULT.evaluate(&inputs(defaults), &mut Budget::evaluation()))
            .expect("info default evaluates")
    };
    let five = ExactValue::from_literal("5");
    let outcome = run(&[(("f", "b"), ParameterDefault::Value(five.clone()))]);
    assert_eq!(
        outcome.result,
        ExactValueOrUnavailable::Exact(ExactValue::from_literal("1"))
    );
    assert_eq!(
        outcome.ordered_stores,
        [StoreOutcome::Write {
            target,
            value: five
        }]
    );
    let outcome = run(&[(("f", "b"), ParameterDefault::None)]);
    assert_eq!(
        outcome.result,
        ExactValueOrUnavailable::Exact(ExactValue::from_literal("0"))
    );
    assert_eq!(
        outcome.ordered_stores,
        [StoreOutcome::Write {
            target,
            value: ExactValue::from_literal("")
        }]
    );
    let outcome = run(&[]);
    assert!(matches!(
        outcome.result,
        ExactValueOrUnavailable::Unavailable(_)
    ));
    assert!(matches!(
        outcome.ordered_stores.as_slice(),
        [StoreOutcome::WriteUnavailable { target: written, .. }] if *written == target
    ));
    assert_eq!(
        INFO_DEFAULT.route(),
        EvalRoute::Direct {
            id: NativeEvalId::ParameterDefault
        }
    );
}
