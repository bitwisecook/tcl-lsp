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

//! Which commands carry a trait: control flow, splice safety, purity, frame sensitivity, teardown, new-command definitions and the script-concat family.

use super::*;

// Control-flow and needs-start-cmd traits

/// registry-metadata: the `CONTROL_FLOW` trait is our classification. (`if`,
/// `for`, `while`, `foreach` and `set`, `puts` are all real tclsh commands —
/// verified in `registry.rs` — but "is control flow" is registry data.)
#[test]
fn control_flow_trait_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    let cf = reg.commands_with_trait(Traits::CONTROL_FLOW);
    for c in ["if", "for", "while", "foreach"] {
        assert!(cf.contains(&c), "{c} must be control-flow");
    }
    assert!(
        !reg.get("set")
            .unwrap()
            .traits
            .contains(Traits::CONTROL_FLOW)
    );
    assert!(
        !reg.get("puts")
            .unwrap()
            .traits
            .contains(Traits::CONTROL_FLOW)
    );
}

/// registry-metadata: `NEEDS_START_CMD` is our classification.
#[test]
fn needs_start_cmd_trait_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in ["expr", "break", "continue"] {
        let spec = reg.get(c).unwrap_or_else(|| panic!("{c} registered"));
        assert!(spec.traits.contains(Traits::NEEDS_START_CMD), "{c}");
    }
    assert!(
        !reg.get("set")
            .unwrap()
            .traits
            .contains(Traits::NEEDS_START_CMD)
    );
    assert!(
        !reg.get("puts")
            .unwrap()
            .traits
            .contains(Traits::NEEDS_START_CMD)
    );
}

/// The splice-safe set (`CommandRegistry::is_splice_safe`) must cover exactly
/// the inliner's former `SPLICE_SAFE_COMMANDS` list — frame-independent
/// value/IO builtins — and exclude the frame-observing / variable-writing /
/// control-transferring commands the old list deliberately left out.
///
/// registry-metadata: splice-safety is our classification (a composition of
/// the `FRAMELESS_RUNTIME` / frame-sensitivity / variable-name traits).
#[test]
fn splice_safe_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in [
        "list", "lindex", "lrange", "linsert", "llength", "lsort", "lsearch", "lreverse",
        "lreplace", "lrepeat", "concat", "split", "join", "string", "expr", "puts",
    ] {
        assert!(reg.is_splice_safe(c), "{c} must be splice-safe");
    }
    // The old list's documented exclusions: frame-affecting control flow,
    // frame-observing scope commands, and variable writers.
    for c in [
        "return",
        "break",
        "continue",
        "uplevel",
        "upvar",
        "global",
        "variable",
        "set",
        "append",
        "lassign",
        "namespace",
        "info",
        "unset",
    ] {
        assert!(!reg.is_splice_safe(c), "{c} must NOT be splice-safe");
    }
}

/// The frame-sensitive set (`CommandRegistry::is_frame_sensitive`) must cover
/// every member of the inline-proc code action's former `UNSAFE` list — the
/// commands whose meaning changes when a proc body is textually inlined into
/// a caller — and exclude plain value/IO commands.
///
/// registry-metadata: frame-sensitivity is our classification (the
/// `TERMINATES_BLOCK` | `TRANSFERS_CONTROL` | `CREATES_SCOPE_ALIAS` |
/// `CREATES_BARRIER` trait union).
/// Every `::tcl::mathop` operator command is `PURE`.
///
/// The namespace spec carried `PURE` but the commands generated under it did
/// not, so `classify_side_effects` treated `[+ $a $b]` as impure and the GVN
/// pass would neither hoist nor CSE it. Found by scanning all specs for trait
/// pairs that never co-occur: `OPERATOR_COMMAND` (87 specs) and `PURE` (379)
/// had no overlap at all, which for arithmetic is not a plausible fact.
///
/// Purity here is a statement about the command, not a policy: an operator
/// reads its operands and returns a result, touching no variable, channel, or
/// interpreter state. A bad operand raises an error, which the registry
/// models as control flow rather than a side effect.
#[test]
fn operator_commands_are_pure() {
    let reg = CommandRegistry::build_default();
    let ops = [
        "!", "!=", "%", "&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=", ">>", "^",
        "eq", "ge", "gt", "in", "le", "lt", "ne", "ni", "|", "~",
    ];
    for op in ops {
        let spec = reg
            .get(op)
            .unwrap_or_else(|| panic!("{op} is a registered operator"));
        assert!(
            spec.traits.contains(Traits::OPERATOR_COMMAND),
            "{op} is an operator command"
        );
        assert!(
            spec.traits.contains(Traits::PURE),
            "{op} computes a value with no side effect and must be PURE, or \
             the optimiser cannot hoist or CSE it"
        );
    }
    // The qualified spellings carry the traits too — the optimiser sees
    // whichever spelling the source used.
    for name in ["::tcl::mathop::+", "tcl::mathop::eq"] {
        let spec = reg
            .get(name)
            .unwrap_or_else(|| panic!("{name} is registered"));
        assert!(spec.traits.contains(Traits::PURE), "{name} must be PURE");
    }
}

#[test]
fn frame_sensitive_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in [
        "return", "break", "continue", "tailcall", "yield", "uplevel", "upvar", "global",
        "variable",
    ] {
        assert!(reg.is_frame_sensitive(c), "{c} must be frame-sensitive");
        assert!(
            reg.frame_sensitive_commands().contains(&c),
            "{c} in the scan set"
        );
    }
    for c in ["puts", "set", "expr", "string", "list"] {
        assert!(
            !reg.is_frame_sensitive(c),
            "{c} must NOT be frame-sensitive"
        );
    }
}

/// Coroutine-family membership is registry data consumed by restricted
/// backends; spelling and release handling must therefore resolve here rather
/// than in each backend.
#[test]
fn coroutine_primitive_membership_is_versioned_and_qualified() {
    let (tcl86, tcl86_set) = reg_and_set("tcl8.6");
    for name in ["coroutine", "::coroutine", "yield", "::yield", "yieldto"] {
        assert!(
            tcl86
                .invocation_traits(name, &[], tcl86_set)
                .contains(Traits::COROUTINE_PRIMITIVE),
            "{name} must resolve through the Tcl 8.6 coroutine family"
        );
    }
    for name in ["coroinject", "coroprobe"] {
        assert!(
            !tcl86
                .invocation_traits(name, &[], tcl86_set)
                .contains(Traits::COROUTINE_PRIMITIVE),
            "{name} is not available before Tcl 9.0"
        );
    }

    let (tcl90, tcl90_set) = reg_and_set("tcl9.0");
    for name in ["coroinject", "::coroinject", "coroprobe", "::coroprobe"] {
        assert!(
            tcl90
                .invocation_traits(name, &[], tcl90_set)
                .contains(Traits::COROUTINE_PRIMITIVE),
            "{name} must resolve through the Tcl 9.0 coroutine family"
        );
    }

    let (tcl85, tcl85_set) = reg_and_set("tcl8.5");
    assert!(
        !tcl85
            .invocation_traits("coroutine", &[], tcl85_set)
            .contains(Traits::COROUTINE_PRIMITIVE)
    );
}

/// The fire-and-forget teardown set (`FIRE_AND_FORGET_TEARDOWN`) must cover
/// the W302 suppression idiom's documented members, at both the command and
/// the subcommand level, and stay off destructive-but-not-teardown
/// subcommands (`file rename` / `file mkdir` fail for real reasons).
///
/// registry-metadata: the fire-and-forget classification is our metadata.
#[test]
fn fire_and_forget_teardown_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in ["close", "unset", "rename"] {
        let spec = reg.get(c).unwrap_or_else(|| panic!("{c} registered"));
        assert!(
            spec.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN),
            "{c}"
        );
    }
    for (c, sub) in [
        ("after", "cancel"),
        ("chan", "close"),
        ("array", "unset"),
        ("dict", "unset"),
        ("interp", "delete"),
        ("file", "delete"),
        ("namespace", "delete"),
        ("namespace", "forget"),
    ] {
        let sc = reg
            .get(c)
            .and_then(|s| s.subcommand(sub))
            .unwrap_or_else(|| panic!("{c} {sub} registered"));
        assert!(
            sc.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN),
            "{c} {sub}"
        );
    }
    for (c, sub) in [
        ("file", "rename"),
        ("file", "mkdir"),
        ("chan", "configure"),
        ("namespace", "eval"),
    ] {
        let sc = reg.get(c).and_then(|s| s.subcommand(sub));
        assert!(
            sc.is_none_or(|s| !s.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN)),
            "{c} {sub} must NOT be fire-and-forget"
        );
    }
}

// defines_command_at — spec-declared "argument names a new command" facts.

/// `coroutine NAME cmd ?arg …?` creates the command NAME
/// (`TclNRCoroutineObjCmd`, `tclBasic.c`) — the spec carries the name index
/// so the analyser's W123 suppression stays registry-driven.
///
/// tclsh (8.6 & 9.0): `coroutine nextNum apply {{} {yield}}; info commands
/// nextNum` lists it.
#[test]
fn coroutine_defines_command_at_its_name_argument() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_surface("coroutine", ds)
        .expect("coroutine registered in tcl8.6");
    assert_eq!(spec.defines_command_at, Some(0), "coroutine names arg 0");
}

/// `interp create ?-safe? ?--? ?name?` binds `name` as the child
/// interpreter's command — declared on the `create` subcommand, index
/// relative to the word after `create`.
///
/// tclsh (8.6 & 9.0): `interp create child; child eval {expr 1}` works.
#[test]
fn interp_create_defines_command_at_its_name_argument() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_surface("interp", ds)
        .expect("interp registered in tcl8.6");
    let create = spec.subcommand("create").expect("interp create modelled");
    assert_eq!(create.defines_command_at, Some(0), "create names sub-arg 0");
    // The other interp subcommands bind no command name.
    let eval = spec.subcommand("eval").expect("interp eval modelled");
    assert_eq!(eval.defines_command_at, None);
}

/// `BRANCH_SELECTED_BODY` is exactly `if` and `try` — the commands whose
/// bodies are chosen at run time without iterating, so nothing established
/// inside one dominates the code after it.
///
/// Deliberately narrower than `CONTROL_FLOW`, which also covers the loops.
/// A loop body is skippable *and* repeatable, a different question with a
/// different consumer (`control_flow_body_depth` versus
/// `conditional_depth`) — the two must not be merged.
///
/// registry-metadata: "which bodies are branch-selected" is our
/// classification. (`if`, `try`, `while`, `for`, `foreach`, and `switch` are
/// all real tclsh 8.6/9.0 commands.)
#[test]
fn branch_selected_body_trait_membership() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let mut carriers = reg.commands_with_trait(Traits::BRANCH_SELECTED_BODY);
    carriers.sort_unstable();
    assert_eq!(carriers, ["if", "try"]);
    for negative in ["while", "for", "foreach", "switch", "lmap", "catch"] {
        assert!(
            !reg.get_for_surface(negative, ds)
                .expect("registered")
                .traits
                .contains(Traits::BRANCH_SELECTED_BODY),
            "{negative} iterates or is otherwise not branch-selected"
        );
    }
    // …and every carrier is control flow, so the narrower trait is a strict
    // subset rather than an orthogonal axis.
    for carrier in carriers {
        assert!(
            reg.get_for_surface(carrier, ds)
                .expect("registered")
                .traits
                .contains(Traits::CONTROL_FLOW),
            "{carrier} must also be control flow"
        );
    }
}

/// The `Tcl_ConcatObj` eval family: every command whose trailing words after
/// the first `ArgRole::Body` argument are part of the script it evaluates.
///
/// tclsh8.6.14 / tclsh9.0.4: `eval set l2 hello` sets `l2` to `hello`;
/// `namespace eval ::n set l2 hello` sets `::n::l2`; `interp eval i {set y} 7`
/// sets `y` to `7` in the child.  `catch` and `apply` take a single bounded
/// script/lambda argument (`apply {{} {set q 1}} extra` errors "wrong # args"),
/// so both stay out.
#[test]
fn script_concat_family_membership() {
    let (reg, ds) = reg_and_set("tcl8.6");
    for name in ["eval", "uplevel"] {
        assert!(
            reg.get_for_surface(name, ds)
                .expect("registered")
                .traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "{name} concatenates its script words"
        );
    }
    let ns = reg.get_for_surface("namespace", ds).expect("namespace");
    for sub in ["eval", "inscope"] {
        assert!(
            ns.subcommand(sub)
                .expect("modelled")
                .traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "namespace {sub} concatenates its script words"
        );
    }
    assert!(
        reg.get_for_surface("interp", ds)
            .expect("interp")
            .subcommand("eval")
            .expect("interp eval modelled")
            .traits
            .contains(Traits::SCRIPT_CONCATENATES_ARGS)
    );
    for negative in ["catch", "apply", "subst", "if"] {
        assert!(
            !reg.get_for_surface(negative, ds)
                .expect("registered")
                .traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "{negative} takes a bounded script argument, not a concatenated tail"
        );
    }
}

/// `namespace inscope` is the one family member whose trailing words are
/// appended as **list elements** rather than space-joined, so it carries the
/// refinement trait and nothing else does.
///
/// tclsh8.6.14 / tclsh9.0.4: `namespace inscope :: {puts} {a b}` prints
/// `a b` (one argument), while `namespace eval :: {puts} {a b}` errors
/// "can not find channel named "a"".
#[test]
fn script_append_list_refinement_is_inscope_only() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let ns = reg.get_for_surface("namespace", ds).expect("namespace");
    let refined = reg.subcommands_with_trait("namespace", Traits::SCRIPT_APPENDS_LIST_ARGS);
    assert_eq!(refined, ["inscope"]);
    assert!(
        !ns.subcommand("eval")
            .expect("modelled")
            .traits
            .contains(Traits::SCRIPT_APPENDS_LIST_ARGS),
        "namespace eval space-joins its tail"
    );
    for name in ["eval", "uplevel"] {
        assert!(
            !reg.get_for_surface(name, ds)
                .expect("registered")
                .traits
                .contains(Traits::SCRIPT_APPENDS_LIST_ARGS)
        );
    }
}
