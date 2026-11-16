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

//! `try`: handler chains, the handler protocol, the variables a handler binds and what the plan cannot place.

use super::*;

/// The handler chain of a `try` from `(selection, selector, falls_through)`
/// rows, the selectors decoded under `dialect`'s numerals.
fn handler_chain(
    handlers: &[(&str, &'static str, bool)],
    dialect: &str,
) -> tcl_registry::value_transfer::completion::HandlerChain {
    use tcl_registry::value_transfer::HandlerMatch;
    use tcl_registry::value_transfer::completion::{HandlerChain, HandlerLink};
    HandlerChain::new(
        handlers
            .iter()
            .map(|&(matches, selector, falls_through)| HandlerLink {
                matches: match matches {
                    "on" => HandlerMatch::CompletionCode,
                    "trap" => HandlerMatch::ErrorCodePrefix,
                    other => panic!("{other}"),
                },
                selector,
                falls_through,
            }),
        tcl_syntax::number::Numbers::of_dialect_name(Some(dialect)),
    )
}

/// A handler's code is the selector's under the release's numerals, and
/// `TCL_ERROR` for a `trap` whatever its pattern. Tclsh 8.6 runs `on 010`
/// for `return -code 8 -level 0 x` and 9.0 and 9.1 for code 10; `on 0` is
/// `ok`, `on 1` `error`, and `on 0x10` and `on 0o10` are 16 and 8 in every
/// release that has `try`.
#[test]
fn a_handler_chain_names_the_code_each_handler_selects() {
    use tcl_registry::completion::CompletionCode as Code;
    let on = |selector: &'static str, dialect| handler_chain(&[("on", selector, false)], dialect);
    for (selector, up_to_86, from_90) in [
        ("ok", Some(Code::Ok), Some(Code::Ok)),
        ("error", Some(Code::Error), Some(Code::Error)),
        ("return", Some(Code::Return), Some(Code::Return)),
        ("break", Some(Code::Break), Some(Code::Break)),
        ("continue", Some(Code::Continue), Some(Code::Continue)),
        ("0", Some(Code::Ok), Some(Code::Ok)),
        ("1", Some(Code::Error), Some(Code::Error)),
        ("5", Some(Code::from_int(5)), Some(Code::from_int(5))),
        ("0x10", Some(Code::from_int(16)), Some(Code::from_int(16))),
        ("0o10", Some(Code::from_int(8)), Some(Code::from_int(8))),
        ("010", Some(Code::from_int(8)), Some(Code::from_int(10))),
        ("nonsense", None, None),
    ] {
        assert_eq!(
            on(selector, "tcl8.6").code(0),
            up_to_86,
            "on {selector}, 8.6"
        );
        assert_eq!(
            on(selector, "tcl9.0").code(0),
            from_90,
            "on {selector}, 9.0"
        );
    }
    // A target that may be either reads only what every release agrees on.
    let unknown = tcl_registry::value_transfer::completion::HandlerChain::selected(
        tcl_registry::value_transfer::HandlerMatch::CompletionCode,
        "010",
        tcl_syntax::number::Numbers::Unknown,
    );
    assert_eq!(unknown, None);
    for selector in ["X Y", "", "$dynamic"] {
        assert_eq!(
            handler_chain(&[("trap", selector, false)], "tcl8.6").code(0),
            Some(Code::Error),
            "trap {selector}"
        );
    }
}

/// The first handler that matches runs, a `-` handler selects its own code and
/// runs the next handler's script, and a handler that cannot be reached is
/// the one an unconditional handler before its group pre-empts. Each program
/// is tclsh 8.6, 9.0 and 9.1's, the script that ran given beside it.
#[test]
fn a_handler_chain_runs_the_first_match_as_tclsh_does() {
    use tcl_registry::completion::CompletionCode as Code;
    let (on_error, on_ok) = (("on", "error", false), ("on", "ok", false));
    let dash = |selector| ("on", selector, true);

    // try {error boom} on error {} {A} on error {} {B}                 ;# A
    let chain = handler_chain(&[on_error, on_error], "tcl8.6");
    assert_eq!(chain.first_taking(Code::Error), Some(0));
    assert!(!chain.preempted(0) && chain.preempted(1));
    assert_eq!(chain.live_group(0), [0]);
    assert_eq!(chain.live_group(1).len(), 0);

    // try {error boom} on error {} {A} on 1 {} {B}                      ;# A
    let chain = handler_chain(&[on_error, ("on", "1", false)], "tcl8.6");
    assert!(chain.preempted(1), "two spellings of one code");

    // try {error boom} on error {} - on ok {} {X}                       ;# X
    let chain = handler_chain(&[dash("error"), on_ok], "tcl8.6");
    assert_eq!(chain.first_taking(Code::Error), Some(0));
    assert_eq!((chain.owner(0), chain.owner(1)), (1, 1));
    assert_eq!(chain.live_group(0).len(), 0);
    assert_eq!(chain.live_group(1), [0, 1]);

    // try {error boom} on error {} - on error {} {B} on ok {} {C}       ;# B
    let chain = handler_chain(&[dash("error"), on_error, on_ok], "tcl8.6");
    assert_eq!(chain.first_taking(Code::Error), Some(0));
    assert!(!chain.preempted(0) && !chain.preempted(1), "one group");
    assert_eq!(chain.live_group(1), [0, 1]);
    assert_eq!(chain.live_group(2), [2]);

    // try {error boom} on error {} {A} on error {} - on ok {} {C}       ;# A
    let chain = handler_chain(&[on_error, dash("error"), on_ok], "tcl8.6");
    assert!(chain.preempted(1) && !chain.preempted(2));
    assert_eq!(chain.live_group(2), [2], "the pre-empted member leaves");

    // try {error boom} on ok {} - on error {} {E}                       ;# E
    let chain = handler_chain(&[dash("ok"), on_error], "tcl8.6");
    assert_eq!(chain.first_taking(Code::Error), Some(1));
    assert_eq!(chain.first_taking(Code::Ok), Some(0));
    assert_eq!(chain.owner(chain.first_taking(Code::Ok).unwrap()), 1);

    // try {error boom} trap {X} {} {T} on error {} {E}                  ;# E
    // (T when the error's -errorcode is `X Y`: a trap pre-empts nothing)
    let chain = handler_chain(&[("trap", "X", false), on_error], "tcl8.6");
    assert!(!chain.preempted(1));
    assert_eq!(chain.first_taking(Code::Error), None, "a trap may miss");

    // try {error boom} on error {} {E} trap {} {} {T}                   ;# E
    let chain = handler_chain(&[on_error, ("trap", "", false)], "tcl8.6");
    assert!(chain.preempted(1));
    assert_eq!(chain.live_group(1).len(), 0);

    // The loop jumps a handler takes: a trap selects errors only, and a
    // selector the registry cannot read might select anything.
    //   foreach i 1 {try {break} trap {X} {} {T} on break {} {B}; …}    ;# B
    let chain = handler_chain(&[("trap", "X", false), ("on", "break", false)], "tcl8.6");
    assert_eq!(chain.first_taking(Code::Break), Some(1));
    assert!(chain.misses(0, Code::Break) && !chain.misses(1, Code::Break));
    assert!(chain.takes(1, Code::Break) && !chain.takes(0, Code::Error));
    //   foreach i 1 {try {break} on break {} - on error {} {E}; …}      ;# E
    let chain = handler_chain(&[dash("break"), on_error], "tcl8.6");
    assert_eq!(chain.first_taking(Code::Break), Some(0));
    assert_eq!(chain.owner(0), 1);
    let chain = handler_chain(
        &[("on", "$dynamic", false), ("on", "break", false)],
        "tcl8.6",
    );
    assert_eq!(chain.first_taking(Code::Break), None);
    assert!(!chain.misses(0, Code::Break));
    assert_eq!(chain.first_taking(Code::Continue), None);
    assert_eq!(
        handler_chain(&[("on", "break", false)], "tcl8.6").first_taking(Code::Continue),
        None
    );

    // A last handler that is `-` is the command's error; it owns itself.
    let chain = handler_chain(&[dash("error")], "tcl8.6");
    assert_eq!(chain.owner(0), 0);
    assert_eq!(chain.len(), 1);
    assert!(!chain.is_empty());
}

/// The plan of `try` with `words` as literal operands.
fn try_plan(words: &[&'static str]) -> PlanAnswer {
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("try").expect("try"), None, None);
    let semantics = resolved.semantics().expect("declared");
    semantics.structure(&TestInputs::new(
        "try",
        words.iter().map(|text| literal(text, None)).collect(),
    ))
}

/// `try` declares the body, its handlers and `finally`: how each handler's
/// pattern selects it, where the pattern stands, the names its variable list
/// binds (the first two) and its script — none for a `-` handler.
#[test]
fn try_declares_its_handler_protocol() {
    use tcl_registry::value_transfer::{
        Binder, BinderName, BodyPlan, CompletionProtocol, HandlerMatch, HandlerPlan, Reconcile,
    };
    let names = |names: &[&str]| -> Vec<Binder> {
        names
            .iter()
            .map(|name| Binder {
                name: BinderName::Declared((*name).to_owned()),
                kind: BindingKind::Scalar,
            })
            .collect()
    };
    let body = |handlers, finally| PlanAnswer::Body {
        binders: Vec::new(),
        body: BodyPlan {
            body: OperandId(0),
            frame: tcl_registry::FrameLevel::Relative(0),
        },
        reconcile: Reconcile::None,
        completion: CompletionProtocol::Handlers { handlers, finally },
    };
    let at = |index| OperandId(index);

    assert_eq!(
        try_plan(&[
            "b", "on", "error", "m o", "h", "trap", "X Y", "", "t", "finally", "f"
        ]),
        body(
            vec![
                HandlerPlan {
                    matches: HandlerMatch::CompletionCode,
                    pattern: at(2),
                    binders: names(&["m", "o"]),
                    body: Some(at(4)),
                },
                HandlerPlan {
                    matches: HandlerMatch::ErrorCodePrefix,
                    pattern: at(6),
                    binders: Vec::new(),
                    body: Some(at(8)),
                },
            ],
            Some(at(10))
        )
    );
    // A `-` handler has no script of its own.
    assert_eq!(
        try_plan(&["b", "on", "error", "", "-", "on", "ok", "r", "h"]),
        body(
            vec![
                HandlerPlan {
                    matches: HandlerMatch::CompletionCode,
                    pattern: at(2),
                    binders: Vec::new(),
                    body: None,
                },
                HandlerPlan {
                    matches: HandlerMatch::CompletionCode,
                    pattern: at(6),
                    binders: names(&["r"]),
                    body: Some(at(8)),
                },
            ],
            None
        )
    );
    // No handler, and `finally` alone (tclsh 8.6 to 9.1: both are valid).
    assert_eq!(try_plan(&["b"]), body(Vec::new(), None));
    assert_eq!(
        try_plan(&["b", "finally", "f"]),
        body(Vec::new(), Some(at(2)))
    );
    // The operands count from the form's own arguments: a leading word that
    // selects the form stands the same plan one operand on.
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("try").expect("try"), None, None);
    let mut shifted = TestInputs::new(
        "try",
        ["x", "b", "on", "error", "m", "h", "finally", "f"]
            .iter()
            .map(|text| literal(text, None))
            .collect(),
    );
    shifted.view.argument_offset = 1;
    assert_eq!(
        resolved.semantics().expect("declared").structure(&shifted),
        PlanAnswer::Body {
            binders: Vec::new(),
            body: BodyPlan {
                body: at(1),
                frame: tcl_registry::FrameLevel::Relative(0),
            },
            reconcile: Reconcile::None,
            completion: CompletionProtocol::Handlers {
                handlers: vec![HandlerPlan {
                    matches: HandlerMatch::CompletionCode,
                    pattern: at(3),
                    binders: names(&["m"]),
                    body: Some(at(5)),
                }],
                finally: Some(at(7)),
            },
        }
    );
}

/// A handler's variable list binds its first two names, the result variable
/// then the options variable: tclsh 8.6 to 9.1 give `on error {a b c} {set a}`
/// the message `boom` and ignore the third, `on error {{} o}` binds the options
/// alone and `on error {m {}}` the result alone, so an empty name keeps its
/// place and binds nothing.
#[test]
fn try_binds_the_first_two_names_of_a_variable_list() {
    use tcl_registry::value_transfer::{Binder, BinderName, CompletionProtocol};
    let names = |names: &[&str]| -> Vec<Binder> {
        names
            .iter()
            .map(|name| Binder {
                name: BinderName::Declared((*name).to_owned()),
                kind: BindingKind::Scalar,
            })
            .collect()
    };
    for (list, bound) in [
        ("m o", names(&["m", "o"])),
        ("a b c", names(&["a", "b"])),
        ("{} o", names(&["", "o"])),
        ("m {}", names(&["m", ""])),
        ("m", names(&["m"])),
        ("", Vec::new()),
    ] {
        let words = ["b", "on", "error", list, "h"];
        let PlanAnswer::Body {
            completion: CompletionProtocol::Handlers { handlers, .. },
            ..
        } = try_plan(&words)
        else {
            panic!("{list}");
        };
        assert_eq!(handlers[0].binders, bound, "{list}");
    }
}

/// What the plan cannot place declines, and each is a call `try` itself
/// rejects or decides by value (tclsh 8.6 to 9.1: `wrong # args`,
/// `bad handler type "foo"`, `finally clause must be last`, `last non-finally
/// clause must not have a body of "-"`, `unmatched open brace in list`).
#[test]
fn try_declines_what_its_plan_cannot_place() {
    let reg = CommandRegistry::build_default();
    let resolved = resolve_semantics(reg.get("try").expect("try"), None, None);
    let semantics = resolved.semantics().expect("declared");
    let declined = |reason| PlanAnswer::Declined(reason);
    let wrong = declined(DeclineReason::WrongRepresentation);
    assert_eq!(try_plan(&[]), wrong);
    assert_eq!(try_plan(&["b", "foo", "bar"]), wrong);
    assert_eq!(
        try_plan(&["b", "finally", "f", "on", "error", "", "h"]),
        wrong
    );
    assert_eq!(try_plan(&["b", "finally", "f", "finally", "f"]), wrong);
    assert_eq!(try_plan(&["b", "on", "error", "", "-"]), wrong);
    assert_eq!(try_plan(&["b", "on", "error", "{", "h"]), wrong);
    assert_eq!(try_plan(&["b", "on", "error"]), wrong);

    let computed = |text| OperandView {
        text,
        kind: InvocationWordKind::Dynamic,
        role: None,
    };
    let plan = |words: Vec<OperandView<'static>>, facts: &[(usize, FactView)]| {
        let mut inputs = TestInputs::new("try", words);
        for (at, fact) in facts {
            inputs.operands.insert(*at, fact.clone());
        }
        semantics.structure(&inputs)
    };
    // A keyword or a `-` that is computed is decided by its value, and so is a
    // handler's script, which may be `-`; the variable list that is computed
    // names no binders. The protected body and `finally` are never `-`, so a
    // computed one is a place the plan names and does not read.
    let literals = |words: &[&'static str]| -> Vec<OperandView<'static>> {
        words.iter().map(|text| literal(text, None)).collect()
    };
    let with = |words: &[&'static str], at: usize, text| {
        let mut operands = literals(words);
        operands[at] = computed(text);
        operands
    };
    assert_eq!(
        plan(with(&["b", "on", "error"], 1, "$k"), &[]),
        declined(DeclineReason::NotExact)
    );
    assert_eq!(
        plan(
            with(&["b", "on", "error", "", "-", "on", "ok", "", "h"], 4, "$d"),
            &[]
        ),
        declined(DeclineReason::NotExact)
    );
    assert_eq!(
        plan(with(&["b", "on", "error", "x", "h"], 4, "$script"), &[]),
        declined(DeclineReason::NotExact)
    );
    for (fact, reason) in [
        (
            FactView::Top(DeclineReason::DynamicName),
            DeclineReason::DynamicName,
        ),
        (FactView::Pending, DeclineReason::NotExact),
    ] {
        assert_eq!(
            plan(
                with(&["b", "on", "error", "x", "h"], 3, "$vars"),
                &[(3, fact)]
            ),
            declined(reason)
        );
    }
    assert!(matches!(
        plan(
            with(&["b", "on", "error", "m", "h", "finally", "f"], 0, "$body"),
            &[]
        ),
        PlanAnswer::Body { .. }
    ));
    assert!(matches!(
        plan(
            with(&["b", "on", "error", "m", "h", "finally", "f"], 6, "$f"),
            &[]
        ),
        PlanAnswer::Body { .. }
    ));
    // A word that expands leaves the clauses unknown.
    let expanded = OperandView {
        text: "$handlers",
        kind: InvocationWordKind::Expanded,
        role: None,
    };
    assert_eq!(
        plan(vec![literal("b", None), expanded], &[]),
        declined(DeclineReason::Unsupported)
    );
}
