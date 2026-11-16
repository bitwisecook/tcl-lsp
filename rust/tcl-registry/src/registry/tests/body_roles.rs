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

//! Body arguments: body kinds, whether a body runs now or is stored, lambda positions and plain bodies.

use super::*;

/// `dict with` / `dict update` arg 0 (the dict variable) plays
/// both `VarRead` and `VarWrite` roles. This is emitted via
/// duplicate `(idx, role)` rows in the resolver (the multi-role
/// observable behaviour is what consumers query).
/// `DEFERS_BODY` marks the commands that **store** a body rather than
/// run it, and it is the fail-safe direction that matters: a consumer
/// asking "can an unreadable body word cost me the proof that this call
/// completes?" must get `yes` for everything that has not declared
/// otherwise.
///
/// `BodyKind` cannot answer this — `proc` and `uplevel` are both
/// `Structural`, because that descriptor answers *which frame*, not
/// *when* — and neither can the absence of `control_arm_semantics`, which
/// `uplevel` / `oo::define` share with `proc`. `apply`'s lambda position
/// is a typed frame boundary; it still declares no `DEFERS_BODY`,
/// which is what this test checks.
#[test]
fn defers_body_marks_only_stored_bodies() {
    use crate::body_kind::BodyKind;
    let mut reg = CommandRegistry::build_default();
    assert!(
        reg.get("proc")
            .unwrap()
            .traits
            .contains(Traits::DEFERS_BODY),
        "`proc` stores its body for a later call"
    );
    // Every command that runs its script as part of the same call — all
    // of which lack control-arm semantics exactly as `proc` does.
    for name in ["uplevel", "apply", "oo::define", "oo::objdefine", "eval"] {
        let spec = reg.get(name).unwrap_or_else(|| panic!("{name} is known"));
        assert!(
            !spec.traits.contains(Traits::DEFERS_BODY),
            "{name} executes its script argument at this call"
        );
    }
    // The trap the flag exists to close: same `BodyKind`, opposite timing.
    assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
    assert_eq!(reg.get("uplevel").unwrap().body_kind, BodyKind::Structural);
    // An iRules handler registration stores its body the same way.
    reg.load_irules();
    assert!(
        reg.get("when")
            .unwrap()
            .traits
            .contains(Traits::DEFERS_BODY)
    );
    // The flag is opt-in, so the silent default is the abstaining one.
    assert!(
        !CommandSpec::DEFAULT.traits.contains(Traits::DEFERS_BODY),
        "an undeclared command must never read as dormant"
    );
    assert!(
        !SubCommand::DEFAULT.traits.contains(Traits::DEFERS_BODY),
        "an undeclared subcommand must never read as dormant"
    );
}

/// `apply`'s lambda position is a typed **frame boundary**: the script
/// runs as part of this call, in a fresh procedure frame.
///
/// The declaration is what lets a consumer walking a statement stream
/// treat an unreadable `apply $lambda` as the missing answer it is — the
/// role alone never reached the question, so the walk claimed past a
/// lambda that can raise.
///
/// Gated on the role, not on the argument number: only the
/// [`ArgRole::LambdaLiteral`] word is the script. `apply`'s trailing
/// `arg1 arg2 …` are ordinary values bound to the lambda's parameters.
#[test]
fn apply_types_its_lambda_position_as_a_frame_boundary() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.control_arm_semantics("apply", &["{{} {puts hi}}"], 0),
        Some(ControlArmSemantics::FrameBoundary),
        "`apply`'s lambda runs now, in a fresh frame"
    );
    for index in 1..4 {
        assert_eq!(
            reg.control_arm_semantics("apply", &["{{a b} {puts hi}}", "1", "2"], index),
            None,
            "argument {index} is a value bound to a parameter, not a script"
        );
    }
    // The role is what the declaration keys on, so it survives a rename of
    // the position and answers for any future command sharing the shape.
    assert_eq!(
        reg.arg_indices_for_role("apply", &["{{} {puts hi}}"], ArgRole::LambdaLiteral),
        vec![0]
    );
}

/// Every shipped command carrying [`ArgRole::LambdaLiteral`] types that
/// position, so the analyser's "read the body only where the registry says
/// what running it means" gate is never the thing deciding a shipped
/// command's fate.
///
/// Pinned rather than assumed: `apply` is the only such
/// command today, so dropping that gate would change nothing here — but a
/// pack, or a future built-in, can carry the role without typing it, and
/// then the gate is what keeps an untyped script that runs *now* from
/// being walked into and claimed. This test says out loud that the shipped
/// set does not exercise it.
#[test]
fn every_shipped_lambda_position_is_typed() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
    let mut seen = 0usize;
    for name in names {
        for argc in 1..=4 {
            let args = vec!["{{} {puts hi}}"; argc];
            for index in reg.arg_indices_for_role(&name, &args, ArgRole::LambdaLiteral) {
                seen += 1;
                assert!(
                    reg.control_arm_semantics(&name, &args, index).is_some(),
                    "{name}/{argc} carries a lambda at {index} without typing what running \
                         it means"
                );
            }
        }
    }
    assert!(seen > 0, "the sweep must not be vacuous");
}

/// Every command that carries an untyped [`ArgRole::Body`] position and
/// runs it *now*, pinned by name.
///
/// The analyser reads the absence of `DEFERS_BODY` as "this body runs as
/// part of this invocation" and lets such a body stop
/// its fall-through walk. That makes a *missing* `DEFERS_BODY` a silent
/// wrong answer rather than a missing one, so the inventory is pinned:
/// adding a Body-role command fails this test until its author puts it on
/// one of the two lists below, which is the moment to ask the question.
///
/// Store-only forms do not appear here — they carry `DEFERS_BODY` and the
/// sweep skips them. Commands the registry *types* do not appear either:
/// a typed arm is answered by [`ControlArmSemantics`], not by this trait.
///
/// Every entry was measured against tclsh 8.6.16 and 9.0.4, which agreed
/// byte-for-byte, with the shape `proc p {} { FORM {error stop}; set
/// ::reached 1 }`: `::reached` unset means the body ran here.
const BODY_RUNS_NOW: &[&str] = &[
    // Core Tcl: script evaluators and definition scripts.
    "::tcl::dict::for",
    "::tcl::dict::map",
    "::tcl::dict::update",
    "::tcl::dict::with",
    "eval",
    "oo::define",
    "oo::objdefine",
    "time",
    "timerate",
    "uplevel",
    // Runs its body now, but *absorbs* its completion — the harness
    // catches a failing test body and reports it. That is a completion
    // boundary, which is a typed answer this trait cannot give, so it
    // sits here and the walk over-abstains on it. Tracked separately.
    "tcltest::test",
    // tcllib: iterators and definition scripts. `clay::define` measured
    // with tcllib 2.0 clay on tclsh 8.6.18 and 9.0.4.
    "clay::define",
    "control::do",
    "fileutil::foreachLine",
    "lfilter",
    "snit::compile",
    "snit::type",
    "snit::widget",
    "snit::widgetadaptor",
    "stooop::class",
    "uri::register",
    // itcl: `class` runs its definition script; `body` and `configbody`
    // install a body against an already-declared member and look like
    // store-only forms, but no itcl is installable in this environment,
    // so they are recorded as unmeasured rather than marked on a guess.
    "itcl::body",
    "itcl::class",
    "itcl::configbody",
    // iRules: these run the script now, in a different flow context.
    "clientside",
    "peer",
    "serverside",
];

/// The gate for [`BODY_RUNS_NOW`]: no command may carry an untyped
/// `ArgRole::Body` position without an explicit stance on whether that
/// body runs now or is stored.
#[test]
fn every_untyped_body_role_declares_whether_its_body_runs_now() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
    let mut unclassified: Vec<String> = Vec::new();
    let mut seen_runs_now = 0usize;
    for name in &names {
        // Role resolution is argv-shaped, so probe the argument counts a
        // grammar could take, and the leading words that select the
        // script-bearing subforms of the ensembles in this sweep. Both
        // questions are asked per shape, exactly as the analyser asks
        // them: `DEFERS_BODY` can live on a subcommand rather than the
        // command (`package ifneeded`), so a spec-level test would miss it.
        let mut untyped_body = false;
        for argc in 1..=6usize {
            for lead in ["x", "idle", "ifneeded", "0"] {
                let mut args = vec!["x"; argc];
                args[0] = lead;
                // One name can carry more than one spec, resolved by
                // dialect — `after` has a Tcl spec and an iRules one, and
                // a sweep that looked at only one would let the other lose
                // its trait unnoticed. Iterating the specs (rather than a
                // list of points) also keeps every probe at a point the
                // command actually resolves at: an unresolvable call
                // answers with an empty trait set, which is indistinguish-
                // able from a spec whose only trait was `DEFERS_BODY`
                // until it was dropped — precisely the regression this
                // gate exists to catch.
                for spec in reg.specs(name) {
                    // Probe at a point the spec actually resolves under:
                    // its own first row, or any Tcl release when it names
                    // no provider.
                    let point = spec.surface.and_then(|rows| rows.first()).map_or(
                        SurfaceQuery::any_release(Family::Tcl),
                        |row| match row.provider {
                            SpecProvider::Core(family) => SurfaceQuery::any_release(family),
                            SpecProvider::Package(_) => SurfaceQuery::any_release(Family::Tcl),
                        },
                    );
                    let traits = reg.invocation_traits(name, &args, Some(point));
                    if spec.traits.contains(Traits::DEFERS_BODY)
                            || traits.contains(Traits::DEFERS_BODY)
                            // A `CONTROL_FLOW` command is the registry's own
                            // business: the analyser refuses to walk one whose
                            // arm it cannot type rather than reading the body,
                            // so a malformed probe shape of `if` / `try` is
                            // not a missing stance.
                            || traits.contains(Traits::CONTROL_FLOW)
                    {
                        continue;
                    }
                    let Some(invocation) = reg.resolve_invocation(name, &args, Some(point)) else {
                        continue;
                    };
                    for index in invocation
                        .argument_roles()
                        .0
                        .into_iter()
                        .filter(|(_, role)| *role == ArgRole::Body)
                        .map(|(position, _)| {
                            invocation.semantics.argument_offset + usize::from(position)
                        })
                    {
                        if reg.control_arm_semantics(name, &args, index).is_none()
                            && !reg
                                .get_for_surface(name, Some(point))
                                .is_some_and(|selected| {
                                    has_authored_body_timing(selected, &invocation, &args, index)
                                })
                        {
                            untyped_body = true;
                        }
                    }
                }
            }
        }
        if !untyped_body {
            continue;
        }
        if BODY_RUNS_NOW.contains(&name.as_str()) {
            seen_runs_now += 1;
        } else {
            unclassified.push(name.clone());
        }
    }
    assert!(
        seen_runs_now > 0,
        "the sweep must not be vacuous — no runs-now command was reached"
    );
    assert!(
        unclassified.is_empty(),
        "these commands carry an untyped ArgRole::Body and declare no \
             stance: supply exact argument-sensitive script timing, store the body (give the spec \
             Traits::DEFERS_BODY, with an oracle row) or they run it now (add \
             them to BODY_RUNS_NOW, with an oracle row). Leaving them out \
             makes the analyser's fall-through walk treat their body as one \
             that can stop control — see issue #1672. Unclassified: {unclassified:?}"
    );
}

fn has_authored_body_timing(
    spec: &CommandSpec,
    invocation: &ResolvedInvocation<'_, '_>,
    arguments: &[&str],
    index: usize,
) -> bool {
    let (resolver, offset) = invocation
        .subcommand
        .resolved()
        .and_then(|selected| spec.subcommand(selected.canonical_name))
        .map_or((spec.script_timing_resolver, 0), |sub| {
            (sub.script_timing_resolver, 1)
        });
    resolver.is_some_and(|resolve| {
        resolve(crate::InvocationArguments::literals(
            arguments.get(offset..).unwrap_or(&[]),
        ))
        .iter()
        .any(|(position, _)| offset + usize::from(*position) == index)
    })
}

/// `DEFERS_BODY` and typed control-arm semantics are answers to different
/// questions, and today no command gives both: everything the registry
/// types (`if`, `switch`, `namespace eval`, `catch`, `try`, the loops,
/// `apply`'s lambda)
/// runs the arm it types.
///
/// That is *why* a consumer cannot currently observe which of the two
/// checks fires first — dropping either leaves every shipped command
/// classified the same. It is a property of today's data, not of the
/// question, so it is pinned rather than assumed: a future command that
/// stored a body the registry also typed as an arm would make the order
/// load-bearing, and this is the assertion that says so out loud before
/// the analyser silently starts trusting a dormant body's arm.
#[test]
fn no_command_both_defers_a_body_and_types_it_as_a_control_arm() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let deferring: Vec<String> = reg
        .command_names()
        .filter(|name| {
            reg.get(name)
                .is_some_and(|spec| spec.traits.contains(Traits::DEFERS_BODY))
        })
        .map(str::to_owned)
        .collect();
    assert!(!deferring.is_empty(), "the sweep must not be vacuous");
    // Positive control: the probe below has to be able to *see* a typed
    // arm, or the sweep proves nothing. Role resolution is argv-shaped, so
    // an empty argument list makes every command answer `None` — including
    // `if`. Probe each deferring command across the argument counts its
    // grammar could take instead.
    assert_eq!(
        reg.control_arm_semantics("if", &["{$c}", "{body}"], 1),
        Some(ControlArmSemantics::Selected),
        "the probe must detect a typed arm, else the sweep is inert"
    );
    for name in deferring {
        for argc in 1..=6 {
            let args = vec!["x"; argc];
            for index in 0..argc {
                assert_eq!(
                    reg.control_arm_semantics(&name, &args, index),
                    None,
                    "{name}/{argc} declares DEFERS_BODY but types argument {index} as a \
                         control arm; `unreadable_body_is_material`'s check order now decides \
                         its behaviour"
                );
            }
        }
    }
}

/// Every spec defaults to `BodyKind::Plain` unless it
/// opts into `Structural`.
#[test]
fn body_kind_default_plain() {
    use crate::body_kind::BodyKind;
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.get("set").unwrap().body_kind, BodyKind::Plain);
    assert_eq!(reg.get("if").unwrap().body_kind, BodyKind::Plain);
    assert_eq!(reg.get("while").unwrap().body_kind, BodyKind::Plain);
    assert_eq!(reg.get("foreach").unwrap().body_kind, BodyKind::Plain);
}

/// `proc` / `oo::class` / `oo::define` / `oo::objdefine`
/// stamp `Structural` so SSA skips their body args from the
/// enclosing block's data flow.
#[test]
fn body_kind_structural_marks() {
    use crate::body_kind::BodyKind;
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
    assert_eq!(
        reg.get("oo::class").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("oo::define").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("oo::objdefine").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("snit::method").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("snit::typemethod").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("uri::register").unwrap().body_kind,
        BodyKind::Structural
    );
}

/// iRules `when` event handler bodies are structural.
#[test]
fn body_kind_irules_when_structural() {
    use crate::body_kind::BodyKind;
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    assert_eq!(reg.get("when").unwrap().body_kind, BodyKind::Structural);
}

/// `plain_body_arg_indices` surfaces the same-frame control-flow /
/// `eval` bodies a nested dispatch (e.g. `TclOO` `my method`) still
/// executes inside — every `Plain`-kind `ArgRole::Body` argument.
#[test]
fn plain_body_arg_indices_covers_same_frame_bodies() {
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.plain_body_arg_indices("if", &["1", "{body}"]), vec![1]);
    assert_eq!(
        reg.plain_body_arg_indices("while", &["1", "{body}"]),
        vec![1]
    );
    assert_eq!(
        reg.plain_body_arg_indices("foreach", &["v", "$list", "{body}"]),
        vec![2]
    );
    assert_eq!(reg.plain_body_arg_indices("eval", &["{body}"]), vec![0]);
    assert_eq!(
        reg.plain_body_arg_indices("catch", &["{body}", "res"]),
        vec![0]
    );
}

/// `Structural` bodies (`proc`, `oo::class create`, `uplevel`,
/// `namespace eval`) never come back from `plain_body_arg_indices` —
/// they run in a definition / different-frame context, so a dispatch
/// found inside is not still the caller's scope.
#[test]
fn plain_body_arg_indices_excludes_structural_bodies() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.plain_body_arg_indices("proc", &["name", "args", "{body}"]),
        Vec::<usize>::new()
    );
    assert_eq!(
        reg.plain_body_arg_indices("uplevel", &["1", "{body}"]),
        Vec::<usize>::new()
    );
    assert_eq!(
        reg.plain_body_arg_indices("namespace", &["eval", "ns", "{body}"]),
        Vec::<usize>::new()
    );
}

/// An unknown command name (a user proc, a `TclOO` method body's own
/// `unknownProc arg` call) has no registry spec at all — returns empty
/// rather than panicking.
#[test]
fn plain_body_arg_indices_unknown_command_is_empty() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.plain_body_arg_indices("myUserProc", &["a", "b"]),
        Vec::<usize>::new()
    );
}

/// `body_arg_implicit_args` defaults to 0 and is set on
/// `fileutil::updateInPlace` (which appends file contents to
/// the body's first command at runtime).
#[test]
fn body_arg_implicit_args_defaults_zero_except_fileutil_updateinplace() {
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.get("set").unwrap().body_arg_implicit_args, 0);
    assert_eq!(reg.get("proc").unwrap().body_arg_implicit_args, 0);
    assert_eq!(
        reg.get("fileutil::updateInPlace")
            .unwrap()
            .body_arg_implicit_args,
        1,
    );
}
