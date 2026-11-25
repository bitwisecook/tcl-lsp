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

//! Thread package execution coordinates and shared-state contracts.
//!
//! Authored from Thread 3.0.4's threadCmd.c, threadSvCmd.c and threadPoolCmd.c.
//! The catalogue is available to spec consumers; selecting and installing a
//! package profile remains independent of these command declarations.

use crate::evaluation_context::{
    EvaluationContext, EvaluationContextSpec, EvaluationFrame, RoleEvaluationContext,
};
use crate::hover::{HoverSnippet, OptionSpec, OptionValue, ScriptTiming};
use crate::world_effect::{
    EffectAccessMode, StaticEffectAccess, StaticEffectFootprint, StaticInterpreterScope,
    StaticNamespaceScope, StaticSubjectScope, WorldEffectDescriptor, WorldStateDomain,
};
use crate::{
    ArgRole, Arity, BodyInterpreter, BodyKind, CommandSpec, InvocationArguments, RuntimeBacking,
};
use tcl_dialect::model::SpecSurface;

const SURFACE: &[SpecSurface] = &[SpecSurface::package("Thread")];
const NEW_THREAD_CONTEXT: EvaluationContext = EvaluationContext {
    interpreter: BodyInterpreter::Unknown,
    frame: EvaluationFrame::Global,
    timing: ScriptTiming::Deferred,
};
const SHARED_READ: &[StaticEffectAccess] = &[StaticEffectAccess::new(
    WorldStateDomain::ThreadSharedStore,
    EffectAccessMode::Read,
    StaticInterpreterScope::Any,
    StaticNamespaceScope::Any,
    StaticSubjectScope::Wildcard,
)];
const SHARED_UPDATE: &[StaticEffectAccess] = &[StaticEffectAccess::new(
    WorldStateDomain::ThreadSharedStore,
    EffectAccessMode::ReadWrite,
    StaticInterpreterScope::Any,
    StaticNamespaceScope::Any,
    StaticSubjectScope::Wildcard,
)];

#[derive(Clone, Copy)]
struct SendLayout {
    thread: usize,
    body: usize,
    result: Option<usize>,
    asynchronous: bool,
    callback: bool,
}

fn send_layout(arguments: InvocationArguments<'_>) -> Option<SendLayout> {
    let count = arguments.exact_argv_len()?;
    if !(2..=5).contains(&count) {
        return None;
    }
    let mut thread = 0;
    let mut asynchronous = false;
    let mut callback = false;
    while thread < count {
        match arguments.literal_at(thread) {
            Some("-async") => asynchronous = true,
            Some("-head") => {}
            Some("-command") => {
                asynchronous = true;
                callback = true;
            }
            Some(_) => break,
            None => return None,
        }
        thread += 1;
    }
    if count.checked_sub(thread)? < 2 {
        return None;
    }
    Some(SendLayout {
        thread,
        body: thread + 1,
        result: (count > thread + 2).then_some(thread + 2),
        asynchronous,
        callback,
    })
}

fn send_roles(
    arguments: InvocationArguments<'_>,
    _: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    let layout = send_layout(arguments)?;
    let mut roles = vec![(u8::try_from(layout.body).ok()?, ArgRole::Body)];
    if let Some(result) = layout.result {
        roles.push((
            u8::try_from(result).ok()?,
            if layout.callback {
                ArgRole::CommandPrefix
            } else {
                ArgRole::VarWrite
            },
        ));
    }
    Some(roles)
}

fn send_context(arguments: InvocationArguments<'_>, index: usize) -> Option<EvaluationContext> {
    let layout = send_layout(arguments)?;
    if index == layout.body {
        return Some(EvaluationContext {
            interpreter: BodyInterpreter::ThreadArgument(u8::try_from(layout.thread).ok()?),
            frame: EvaluationFrame::Global,
            timing: if layout.asynchronous {
                ScriptTiming::Deferred
            } else {
                ScriptTiming::SameInvocation
            },
        });
    }
    (layout.result == Some(index)).then_some(EvaluationContext {
        frame: if layout.asynchronous {
            EvaluationFrame::Global
        } else {
            EvaluationFrame::Invoking
        },
        timing: if layout.asynchronous {
            ScriptTiming::Deferred
        } else {
            ScriptTiming::SameInvocation
        },
        ..EvaluationContext::INVOKING
    })
}

fn send_timing(arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    let Some(layout) = send_layout(arguments) else {
        return vec![];
    };
    let Some(body) = u8::try_from(layout.body).ok() else {
        return vec![];
    };
    let mut timings = vec![(
        body,
        if layout.asynchronous {
            ScriptTiming::Deferred
        } else {
            ScriptTiming::SameInvocation
        },
    )];
    if layout.callback
        && let Some(result) = layout.result.and_then(|index| u8::try_from(index).ok())
    {
        timings.push((result, ScriptTiming::Deferred));
    }
    timings
}

fn send_prefixes(
    arguments: crate::invocation_words::CommandPrefixArguments<'_>,
) -> Vec<(u8, crate::arg_role::AppendedArity)> {
    let Some(layout) = send_layout(arguments.words()) else {
        return vec![];
    };
    if !layout.callback {
        return vec![];
    }
    layout
        .result
        .and_then(|index| u8::try_from(index).ok())
        .map(|index| (index, crate::arg_role::AppendedArity::Exactly(1)))
        .into_iter()
        .collect()
}

fn create_roles(
    arguments: InvocationArguments<'_>,
    _: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    let count = arguments.exact_argv_len()?;
    let mut script = 0;
    while script < count {
        match arguments.literal_at(script) {
            Some("-joinable" | "-preserved") => script += 1,
            Some("--") => {
                script += 1;
                if count != script + 1 {
                    return None;
                }
                break;
            }
            Some(_) => break,
            None => return None,
        }
    }
    match count - script {
        0 => Some(vec![]),
        1 => Some(vec![(u8::try_from(script).ok()?, ArgRole::Body)]),
        _ => None,
    }
}

fn send_spec() -> CommandSpec {
    CommandSpec {
        name: "thread::send",
        surface: Some(SURFACE),
        required_package: Some("Thread"),
        runtime_backing: RuntimeBacking::None,
        arity: Arity::new(2, 5),
        arg_role_layout_resolver: Some(send_roles),
        command_prefix_resolver: Some(send_prefixes),
        script_timing_resolver: Some(send_timing),
        arg_role_resolver_roles: &[ArgRole::Body, ArgRole::VarWrite, ArgRole::CommandPrefix],
        body_interpreter: BodyInterpreter::Unknown,
        body_kind: BodyKind::Structural,
        evaluation_contexts: Some(EvaluationContextSpec {
            resolver: Some(send_context),
            resolver_roles: &[ArgRole::Body, ArgRole::VarWrite, ArgRole::CommandPrefix],
            ..EvaluationContextSpec::EMPTY
        }),
        hover: Some(HoverSnippet::brief(
            "Evaluate a script in another thread's main interpreter.",
            &["thread::send ?-async? ?-head? id script ?varname?"],
            "Thread 3.0.4 threadCmd.c",
        )),
        ..CommandSpec::DEFAULT
    }
}

fn create_spec() -> CommandSpec {
    CommandSpec {
        name: "thread::create",
        surface: Some(SURFACE),
        required_package: Some("Thread"),
        runtime_backing: RuntimeBacking::None,
        arity: Arity::at_least(0),
        arg_role_layout_resolver: Some(create_roles),
        arg_role_resolver_roles: &[ArgRole::Body],
        body_interpreter: BodyInterpreter::Unknown,
        body_kind: BodyKind::Structural,
        evaluation_contexts: Some(EvaluationContextSpec {
            roles: &[RoleEvaluationContext {
                roles: &[ArgRole::Body],
                context: NEW_THREAD_CONTEXT,
            }],
            ..EvaluationContextSpec::EMPTY
        }),
        hover: Some(HoverSnippet::brief(
            "Create a thread with an independent Tcl interpreter.",
            &["thread::create ?-joinable? ?-preserved? ?script?"],
            "Thread 3.0.4 threadCmd.c",
        )),
        ..CommandSpec::DEFAULT
    }
}

fn shared_spec(
    name: &'static str,
    arity: Arity,
    reads_only: bool,
    output_variable: bool,
) -> CommandSpec {
    CommandSpec {
        name,
        surface: Some(SURFACE),
        required_package: Some("Thread"),
        runtime_backing: RuntimeBacking::None,
        arity,
        arg_roles: if output_variable {
            &[(0, ArgRole::Name), (2, ArgRole::VarWrite)]
        } else {
            &[(0, ArgRole::Name)]
        },
        evaluation_contexts: Some(EvaluationContextSpec {
            arguments: &[(
                0,
                EvaluationContext {
                    interpreter: BodyInterpreter::ProcessShared,
                    frame: EvaluationFrame::Unknown,
                    timing: ScriptTiming::ReferenceOnly,
                },
            )],
            roles: &[RoleEvaluationContext {
                roles: &[ArgRole::VarWrite],
                context: EvaluationContext::REFERENCE,
            }],
            ..EvaluationContextSpec::EMPTY
        }),
        world_effects: crate::knowledge::Knowledge::Partial(WorldEffectDescriptor {
            static_footprint: StaticEffectFootprint {
                accesses: if reads_only {
                    SHARED_READ
                } else {
                    SHARED_UPDATE
                },
                ..StaticEffectFootprint::EMPTY
            },
            ..WorldEffectDescriptor::EMPTY
        }),
        hover: Some(HoverSnippet::brief(
            "Access the process-wide Thread shared-variable store.",
            &[],
            "Thread 3.0.4 threadSvCmd.c",
        )),
        ..CommandSpec::DEFAULT
    }
}

fn pool_create_spec() -> CommandSpec {
    CommandSpec {
        name: "tpool::create",
        surface: Some(SURFACE),
        required_package: Some("Thread"),
        runtime_backing: RuntimeBacking::None,
        arity: Arity::at_least(0),
        options: const {
            &[
                OptionSpec {
                    name: "-initcmd",
                    value: OptionValue::deferred_script(),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-exitcmd",
                    value: OptionValue::deferred_script(),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-minworkers",
                    value: OptionValue::value("number"),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-maxworkers",
                    value: OptionValue::value("number"),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-idletime",
                    value: OptionValue::value("seconds"),
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        body_interpreter: BodyInterpreter::Unknown,
        body_kind: BodyKind::Structural,
        evaluation_contexts: Some(EvaluationContextSpec {
            roles: &[RoleEvaluationContext {
                roles: &[ArgRole::Body],
                context: NEW_THREAD_CONTEXT,
            }],
            ..EvaluationContextSpec::EMPTY
        }),
        ..CommandSpec::DEFAULT
    }
}

fn pool_post_roles(
    arguments: InvocationArguments<'_>,
    _: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    let count = arguments.exact_argv_len()?;
    let mut pool = 0;
    while pool < count {
        match arguments.literal_at(pool) {
            Some("-detached" | "-nowait") => pool += 1,
            Some(word) if word.starts_with('-') => return None,
            Some(_) => break,
            None => return None,
        }
    }
    if count != pool + 2 {
        return None;
    }
    Some(vec![(u8::try_from(pool + 1).ok()?, ArgRole::Body)])
}

fn pool_post_spec() -> CommandSpec {
    CommandSpec {
        name: "tpool::post",
        surface: Some(SURFACE),
        required_package: Some("Thread"),
        runtime_backing: RuntimeBacking::None,
        arity: Arity::new(2, 4),
        arg_role_layout_resolver: Some(pool_post_roles),
        arg_role_resolver_roles: &[ArgRole::Body],
        body_interpreter: BodyInterpreter::Unknown,
        body_kind: BodyKind::Structural,
        evaluation_contexts: Some(EvaluationContextSpec {
            roles: &[RoleEvaluationContext {
                roles: &[ArgRole::Body],
                context: NEW_THREAD_CONTEXT,
            }],
            ..EvaluationContextSpec::EMPTY
        }),
        ..CommandSpec::DEFAULT
    }
}

/// Source-authored Thread contracts; package/profile admission is a separate operation.
#[must_use]
pub fn thread_command_specs() -> Vec<CommandSpec> {
    vec![
        send_spec(),
        create_spec(),
        pool_create_spec(),
        pool_post_spec(),
        shared_spec("tsv::set", Arity::new(2, 3), false, false),
        shared_spec("tsv::get", Arity::new(2, 3), true, true),
        shared_spec("tsv::exists", Arity::exact(2), true, false),
        shared_spec("tsv::unset", Arity::new(1, 2), false, false),
        shared_spec("tsv::incr", Arity::new(2, 3), false, false),
        shared_spec("tsv::append", Arity::at_least(3), false, false),
        shared_spec("tsv::lappend", Arity::at_least(3), false, false),
    ]
}

#[cfg(test)]
mod tests;
