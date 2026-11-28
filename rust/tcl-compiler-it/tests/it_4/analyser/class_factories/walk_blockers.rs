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

//! Bodies the load-time walk cannot read or cannot walk past, and its depth limits.

use super::*;

#[test]
pub(super) fn a_declaration_body_the_walk_cannot_read_is_not_a_blocker() {
    crate::install_surfaces();
    // TP — `proc NAME ARGS [string map … {…}]` *stores* its body;
    // nothing in it runs at this call, so an unreadable body word says
    // nothing about whether the declaration falls through. `proc` says so
    // itself, through `Traits::DEFERS_BODY` — see
    // `an_immediately_executed_body_the_walk_cannot_read_still_abstains`
    // for why that has to be a declared fact rather than an inference.
    // This is tcllib clay's `proc ${NSPACE}::define {oclass args}
    // [string map [list %NSPACE% $NSPACE] {…}]`.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            // tclsh 8.4 to 9.1 refuse `proc` into a namespace that does not
            // exist (`can't create procedure "::T::D::define": unknown
            // namespace`), so the namespace is created first.
            "    namespace eval $NSPACE {}\n",
            "    proc ${NSPACE}::define {oclass args} [string map [list %NSPACE% $NSPACE] {\n",
            "        set class $oclass\n",
            "    }]\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::D::class"),
        "an unreadable proc body must not abstain the walk; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
    assert!(
        result.class_factories().contains_key("::T::D::class"),
        "the resolved metaclass must still publish a factory"
    );
}

#[test]
fn a_running_body_the_walk_cannot_read_still_abstains() {
    crate::install_surfaces();
    // FP guard: `namespace eval $ns $script` runs its body at this very
    // call and the registry types that arm (`FrameBoundary`), so a body
    // word the walk cannot read there is a fact it genuinely needed.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    namespace eval ::T [string map [list a b] {\n",
            "        set class 1\n",
            "    }]\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "an unreadable body that runs now must still abstain"
    );
}

#[test]
fn an_immediately_executed_body_the_walk_cannot_read_still_abstains() {
    crate::install_surfaces();
    // FP guard, and the reason dormancy has to be *declared* rather than
    // inferred.
    //
    // None of these commands carries control-arm semantics, exactly like
    // `proc`. Unlike `proc`, every one of them **runs** its script as part
    // of this very call, so the script can raise, `return`, or otherwise
    // stop control ever reaching the creation below it. Reading "no typed
    // control semantics" as proof of dormancy would therefore let the walk
    // materialise a class created after a script that aborts.
    //
    // `BodyKind` cannot separate them either: `proc` and `uplevel` are
    // both `Structural`, because that descriptor answers *which frame*,
    // not *when*. Only `Traits::DEFERS_BODY` does, and it is unset here.
    for prefix in [
        "    uplevel 1 $script\n",
        "    uplevel 0 $script\n",
        "    uplevel #0 $script\n",
        "    oo::define ::T::Mother $script\n",
    ] {
        let src = COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            &format!(
                "{prefix}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"
            ),
        );
        let result = analysis(&src, "tcl9.0");
        assert!(
            !result.all_classes.contains_key("::T::D::class"),
            "an immediately-executed unreadable body must abstain: {prefix:?}; got {:?}",
            result.all_classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn an_aborting_uplevel_script_cannot_be_walked_past() {
    crate::install_surfaces();
    // The concrete harm the guard above prevents, in one vector: the
    // script `uplevel` runs really does end the procedure before the
    // creation, so a walk that claimed the class would be reporting a
    // class no interpreter ever makes.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    uplevel 1 $script\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        !result.class_factories().contains_key("::T::D::class"),
        "no factory may be published past an aborting script; got {:?}",
        result.class_factories().keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_readable_body_that_runs_now_but_is_untyped_still_abstains() {
    crate::install_surfaces();
    // TP — the readable twin of the guard above. These commands carry
    // no typed control-arm semantics, so the walk must not claim the
    // creation below merely on the strength of having no descriptor —
    // it must actually read the body.
    //
    // Every one of them runs its script as part of this call, and tclsh
    // 8.6.16 / 9.0.4 agree the statement after it is not reached — the
    // class does not exist afterwards — for both blocking completions the
    // registry can prove: a raise (`error stop`, `Terminates`) and a
    // propagating return (`return`, `ReturnsResult`). The `return` row is
    // not merely the conservative reading: both interpreters really do
    // leave the enclosing procedure, `oo::define`'s definition script
    // included.
    for prefix in [
        "    uplevel 1 {error stop}\n",
        "    eval {error stop}\n",
        "    oo::define ::T::Mother {error stop}\n",
        "    uplevel 1 {return}\n",
        "    eval {return}\n",
        "    oo::define ::T::Mother {return}\n",
    ] {
        let src = COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            &format!(
                "{prefix}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"
            ),
        );
        let result = analysis(&src, "tcl9.0");
        assert!(
            !result.all_classes.contains_key("::T::D::class"),
            "an aborting body that runs now must abstain: {prefix:?}; got {:?}",
            result.all_classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_readable_body_that_runs_now_and_completes_is_not_a_blocker() {
    crate::install_surfaces();
    // TP control: the rule must cost nothing when the body does not block.
    // tclsh 8.6.16 / 9.0.4 agree the class *is* created after every prefix
    // here.
    //
    // The third vector is the one that sets the burden of proof. A
    // definition script is written in a vocabulary this walk cannot type —
    // `method` is not a command, it is a word `oo::define` understands —
    // so requiring an untyped body to *prove* it falls through would
    // abstain here, and on `oo::class create C { superclass D }`, which is
    // the construct the computed-metaclass walk exists to read. The rule is
    // therefore the other way round: walk past unless the body proves it
    // blocks (see `untyped_body_provably_blocks`).
    for prefix in [
        "    eval {set zz 1}\n",
        "    uplevel 1 {set zz 1}\n",
        "    oo::define ::T::Mother {method extra {} {}}\n",
    ] {
        let src = COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            &format!(
                "{prefix}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"
            ),
        );
        let result = analysis(&src, "tcl9.0");
        assert!(
            result.all_classes.contains_key("::T::D::class"),
            "a body that completes must not abstain: {prefix:?}; got {:?}",
            result.all_classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_stored_body_that_cannot_fall_through_is_still_not_a_blocker() {
    crate::install_surfaces();
    // FP guard, and the reason the rule is "runs now" rather than
    // "readable": a `proc` body is *stored*. It never runs at this call,
    // so what it would do to control flow says nothing about the
    // declaration, which completes normally — tclsh 8.6.16 / 9.0.4 both
    // create the class after `proc helper {a} {return $a}`, whose body
    // manifestly does not fall through.
    //
    // `after` is the same fact with a different mechanism: the script is
    // scheduled, not run, so both oracles reach the creation even when the
    // scheduled script would raise.
    // The rest of the list enumerates and measures every store-only form
    // the registry ships: reading the *absence* of `DEFERS_BODY` as
    // "runs now" would make a missing trait a silent wrong answer. Each
    // row below is one
    // that gained the trait, and each was proved on tclsh 8.6.16 and
    // 9.0.4 with `proc p {} { FORM {error stop}; set ::reached 1 }` —
    // `::reached` is set on both for all of them.
    for prefix in [
        "    proc helper {a} {return $a}\n",
        "    after 0 {error stop}\n",
        "    after idle {error stop}\n",
        "    package ifneeded probe 1.0 {error stop}\n",
        "    fileevent $ch readable {error stop}\n",
        "    ::tcl::OptProc probe {} {error stop}\n",
        "    tcltest::loadScript {error stop}\n",
        "    lambda {} {error stop}\n",
        "    lambda@ :: {} {error stop}\n",
        "    defer::defer {error stop}\n",
        "    defer::with {} {error stop}\n",
        "    defer::autowith {error stop}\n",
        "    processman::onexit probe {error stop}\n",
        "    report::defstyle probe {} {error stop}\n",
        "    snit::macro probe {} {error stop}\n",
        "    snit::method ::T probe {} {error stop}\n",
        "    snit::typemethod ::T probe {} {error stop}\n",
    ] {
        let src = COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            &format!(
                "{prefix}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"
            ),
        );
        let result = analysis(&src, "tcl9.0");
        assert!(
            result.all_classes.contains_key("::T::D::class"),
            "a stored body must stay ignorable: {prefix:?}; got {:?}",
            result.all_classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_deferred_tk_option_body_is_not_a_load_time_blocker() {
    crate::install_surfaces();
    // A widget constructor stores -command for a later UI event.  The
    // callback's `error` therefore cannot stop the constructor invocation
    // or the computed metaclass creation that follows it.  This timing is
    // attached to the exact option value, not inferred from BodyKind or a
    // command-wide DEFERS_BODY bit.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    button .b -command {error stop}\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::D::class"),
        "a deferred Tk callback must not abort the load-time walk; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn an_apply_of_an_unreadable_lambda_still_abstains() {
    crate::install_surfaces();
    // FP guard — the same soundness class as the `uplevel` vector
    // above, reached through the *other* role. `apply`'s argument is
    // `ArgRole::LambdaLiteral`, not `Body`, so before this fix
    // `control_arms_for_segment` never saw it: the walk neither read the
    // lambda nor abstained on it, and claimed the creation below.
    //
    // A lambda word this walk cannot read can be anything, including a
    // body that raises — tclsh 8.6.16 and 9.0.4 agree that the statement
    // after `apply $lambda` is not reached when it does.
    // The last two vectors are readable as *text* but not as a script: a
    // body element that is not `{braced}` is list-decoded before `apply`
    // evaluates it, so its source slice is not the script that runs.
    //
    // The quoted one is the sharp case, and the reason this walk takes
    // only a braced body. Its slice is `puts a\;error stop` — an escaped
    // semicolon, so *as source* it is one `puts` command that falls
    // through. The script `apply` actually runs is the list-decoded
    // `puts a;error stop`: two commands, the second of which raises.
    // tclsh 8.6.16 / 9.0.4 agree (both print `a`, raise `stop`, and never
    // create the class), so a walk that read the slice would claim a
    // creation neither interpreter performs.
    for prefix in [
        "    apply $lambda\n",
        "    apply $lambda a b\n",
        "    apply [set lambda]\n",
        "    apply {{} error\\ stop}\n",
        "    apply {{} \"puts a\\;error stop\"}\n",
    ] {
        let src = COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            &format!(
                "{prefix}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"
            ),
        );
        let result = analysis(&src, "tcl9.0");
        assert!(
            !result.all_classes.contains_key("::T::D::class"),
            "an unreadable lambda must abstain: {prefix:?}; got {:?}",
            result.all_classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn an_aborting_literal_lambda_cannot_be_walked_past() {
    crate::install_surfaces();
    // The concrete harm, with the lambda in plain sight: both oracles
    // agree the creation never happens.
    //
    //   proc mk {name} { apply {{} {error stop}}; Mother create … }
    //
    // tclsh 8.6.16 / 9.0.4: `::T::D::class` does not exist afterwards.
    for prefix in [
        "    apply {{} {error stop}}\n",
        "    apply {{} {return -code error stop}}\n",
    ] {
        let src = COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            &format!(
                "{prefix}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"
            ),
        );
        let result = analysis(&src, "tcl9.0");
        assert!(
            !result.class_factories().contains_key("::T::D::class"),
            "no factory may be published past an aborting lambda: {prefix:?}; got {:?}",
            result.class_factories().keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_readable_lambda_that_falls_through_is_not_a_blocker() {
    crate::install_surfaces();
    // TP control for the two guards above: the fix must not cost the walk
    // every `apply`. This lambda's body is right there, it completes
    // normally, and tclsh 8.6.16 / 9.0.4 both reach the statement after
    // it — so the creation stays proved.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    apply {{} {set q 1}}\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::D::class"),
        "a readable falling-through lambda must not abstain the walk; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
    assert!(
        result.class_factories().contains_key("::T::D::class"),
        "the resolved metaclass must still publish a factory"
    );
}

#[test]
fn four_hundred_nested_foreach_bodies_do_not_abort_the_process() {
    crate::install_surfaces();
    // Run on this test's own default-sized thread — which is
    // the whole point: the walks this drives must contain themselves on
    // the 2 MiB every unremarkable caller gets, not only on the 64 MiB
    // the CLI and the LSP worker ask for. No parameterised creation
    // anywhere in the file, so none of the machinery above runs; this
    // is the plain braced-body descent, whose caps were hand-picked at
    // 256 and wanted about 4.6 MiB to reach it.
    //
    // Past the cap the analyser abstains with a notice rather than
    // truncating in silence, so the assertion is E207 and a returned
    // result — not a resolution of anything inside.
    let src: String = (0..400)
        .map(|i| format!("foreach v{i} {{a b}} {{\n"))
        .chain(std::iter::once("set inner 1\n".to_owned()))
        .chain((0..400).map(|_| "}\n".to_owned()))
        .collect();
    let codes: Vec<_> = analysis(&src, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|code| code == "E207"),
        "past the cap the walk must say it stopped; got {codes:?}"
    );
}

#[test]
fn deeply_nested_untyped_bodies_do_not_blow_the_static_walk_stack() {
    crate::install_surfaces();
    // The same native-stack safety net for the *untyped* body relaxation.
    // `untyped_body_provably_blocks` re-enters the statement walk,
    // which can re-enter it: `eval {eval {eval {…}}}` drives that recursion
    // by nesting alone, so it carries the same `MAX_UNKNOWN_BODY_DEPTH`
    // bound. Past the cap it abstains, the direction it takes for anything
    // it cannot read, so this asserts termination and a well-formed result
    // rather than a resolution.
    //
    // 40 is many times the cap (8) and well under the braced-body descent's
    // own limit, so a failure here is this walk's recursion, not that one's.
    let mut nest = "    set zz 1\n".to_owned();
    for _ in 0..40 {
        nest = format!("    eval {{\n{nest}    }}\n");
    }
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        &format!("{nest}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"),
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::Mother"),
        "the literally-written metaclass must survive the deep walk"
    );
}

#[test]
fn deeply_nested_collection_loops_do_not_blow_the_static_walk_stack() {
    crate::install_surfaces();
    // Native-stack safety net for the relaxation above: the fall-through
    // walk re-enters itself once per control body it descends into,
    // including loop bodies, so it carries the same
    // `MAX_UNKNOWN_BODY_DEPTH` bound every other recursive walk in the
    // analyser has. Past the cap it abstains — the
    // direction it already takes for anything it cannot read — so this
    // asserts termination and a well-formed result, not a resolution.
    //
    // 40 is many times this walk's cap (8) and comfortably below the
    // braced-body descent's own limit (`MAX_BODY_DEPTH`, derived from a
    // stack budget in `depth_guard`), so a failure
    // here is this walk's recursion and not the generic one's.
    let nest: String = (0..40)
        .map(|i| format!("    foreach v{i} {{a b}} {{\n"))
        .chain(std::iter::once("    set zz 1\n".to_owned()))
        .chain((0..40).map(|_| "    }\n".to_owned()))
        .collect();
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        &format!("{nest}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"),
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::Mother"),
        "the literally-written metaclass must survive the deep walk"
    );
}
