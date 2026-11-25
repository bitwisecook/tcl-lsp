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

const SURFACE: &[SpecSurface] = SpecSurface::ALL_TCL;
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
    create_body_roles(arguments)
}

fn create_body_roles(arguments: InvocationArguments<'_>) -> Option<Vec<(u8, ArgRole)>> {
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

/// The script a thread or pool worker runs later, in its own interpreter.
fn deferred_scripts(roles: Option<Vec<(u8, ArgRole)>>) -> Vec<(u8, ScriptTiming)> {
    roles
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, role)| *role == ArgRole::Body)
        .map(|(index, _)| (index, ScriptTiming::Deferred))
        .collect()
}

fn create_timing(arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    deferred_scripts(create_body_roles(arguments))
}

fn pool_post_timing(arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    deferred_scripts(pool_post_body_roles(arguments))
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
        semantics: crate::value_transfer::SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::THREAD_RESULT,
        ),
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
        script_timing_resolver: Some(create_timing),
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
        semantics: if output_variable {
            crate::value_transfer::SemanticsDeclaration::Declared(
                &crate::value_transfer::builtins::THREAD_RESULT,
            )
        } else {
            crate::value_transfer::SemanticsDeclaration::Inherited
        },
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
        hover: Some(HoverSnippet::brief(
            "Create a pool of worker threads.",
            &["tpool::create ?options?"],
            "Thread 3.0 tpool(n)",
        )),
        ..CommandSpec::DEFAULT
    }
}

fn pool_post_roles(
    arguments: InvocationArguments<'_>,
    _: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    pool_post_body_roles(arguments)
}

fn pool_post_body_roles(arguments: InvocationArguments<'_>) -> Option<Vec<(u8, ArgRole)>> {
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
        script_timing_resolver: Some(pool_post_timing),
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
            "Post a job script to a thread pool.",
            &["tpool::post ?-detached? ?-nowait? tpool script"],
            "Thread 3.0 tpool(n)",
        )),
        ..CommandSpec::DEFAULT
    }
}

/// A Thread command known by name, arity and manual-page summary only; its
/// effects stay the conservative default.
struct PlainCommand {
    name: &'static str,
    arity: Arity,
    synopsis: &'static [&'static str],
    detail: &'static str,
}

fn plain_spec(command: &PlainCommand) -> CommandSpec {
    CommandSpec {
        name: command.name,
        surface: Some(SURFACE),
        required_package: Some("Thread"),
        runtime_backing: RuntimeBacking::None,
        arity: command.arity,
        hover: Some(HoverSnippet::brief(
            command.detail,
            command.synopsis,
            "Thread 3.0 thread(n), tsv(n), tpool(n)",
        )),
        ..CommandSpec::DEFAULT
    }
}

/// The rest of the Thread roster (`thread`, `tsv` and `tpool` manual pages).
const PLAIN_COMMANDS: &[PlainCommand] = &[
    PlainCommand {
        name: "thread::id",
        arity: Arity::exact(0),
        synopsis: &["thread::id"],
        detail: "Return the current thread's id.",
    },
    PlainCommand {
        name: "thread::names",
        arity: Arity::exact(0),
        synopsis: &["thread::names"],
        detail: "List the ids of every existing thread.",
    },
    PlainCommand {
        name: "thread::exists",
        arity: Arity::exact(1),
        synopsis: &["thread::exists id"],
        detail: "Whether a thread with this id exists.",
    },
    PlainCommand {
        name: "thread::wait",
        arity: Arity::exact(0),
        synopsis: &["thread::wait"],
        detail: "Enter the event loop until the thread is released.",
    },
    PlainCommand {
        name: "thread::release",
        arity: Arity::new(0, 2),
        synopsis: &["thread::release ?-wait? ?id?"],
        detail: "Decrement a thread's reference count.",
    },
    PlainCommand {
        name: "thread::preserve",
        arity: Arity::new(0, 1),
        synopsis: &["thread::preserve ?id?"],
        detail: "Increment a thread's reference count.",
    },
    PlainCommand {
        name: "thread::join",
        arity: Arity::exact(1),
        synopsis: &["thread::join id"],
        detail: "Wait for a joinable thread to exit.",
    },
    PlainCommand {
        name: "thread::transfer",
        arity: Arity::exact(2),
        synopsis: &["thread::transfer id channel"],
        detail: "Move a channel to another thread.",
    },
    PlainCommand {
        name: "thread::detach",
        arity: Arity::exact(1),
        synopsis: &["thread::detach channel"],
        detail: "Detach a channel from the current thread.",
    },
    PlainCommand {
        name: "thread::attach",
        arity: Arity::exact(1),
        synopsis: &["thread::attach channel"],
        detail: "Attach a previously detached channel.",
    },
    PlainCommand {
        name: "thread::broadcast",
        arity: Arity::exact(1),
        synopsis: &["thread::broadcast script"],
        detail: "Send a script to every thread asynchronously.",
    },
    PlainCommand {
        name: "thread::eval",
        arity: Arity::at_least(1),
        synopsis: &["thread::eval ?-lock mutex? arg ?arg ...?"],
        detail: "Evaluate a script under a mutex.",
    },
    PlainCommand {
        name: "thread::errorproc",
        arity: Arity::new(0, 1),
        synopsis: &["thread::errorproc ?procname?"],
        detail: "Set or query the asynchronous error handler.",
    },
    PlainCommand {
        name: "thread::cancel",
        arity: Arity::new(1, 3),
        synopsis: &["thread::cancel ?-unwind? id ?result?"],
        detail: "Cancel the script running in a thread.",
    },
    PlainCommand {
        name: "thread::unwind",
        arity: Arity::exact(0),
        synopsis: &["thread::unwind"],
        detail: "Exit the event loop entered by thread::wait.",
    },
    PlainCommand {
        name: "thread::exit",
        arity: Arity::new(0, 1),
        synopsis: &["thread::exit ?status?"],
        detail: "Terminate the current thread.",
    },
    PlainCommand {
        name: "thread::configure",
        arity: Arity::at_least(1),
        synopsis: &["thread::configure id ?option? ?value? ..."],
        detail: "Query or set a thread's options.",
    },
    PlainCommand {
        name: "thread::mutex",
        arity: Arity::at_least(1),
        synopsis: &["thread::mutex option ?arg ...?"],
        detail: "Create, lock, unlock or destroy a mutex.",
    },
    PlainCommand {
        name: "thread::rwmutex",
        arity: Arity::at_least(1),
        synopsis: &["thread::rwmutex option ?arg ...?"],
        detail: "Create and use a reader/writer mutex.",
    },
    PlainCommand {
        name: "thread::cond",
        arity: Arity::at_least(1),
        synopsis: &["thread::cond option ?arg ...?"],
        detail: "Create and use a condition variable.",
    },
    PlainCommand {
        name: "tsv::names",
        arity: Arity::new(0, 1),
        synopsis: &["tsv::names ?pattern?"],
        detail: "List the shared arrays.",
    },
    PlainCommand {
        name: "tsv::object",
        arity: Arity::new(2, 3),
        synopsis: &["tsv::object array element ?lazyproc?"],
        detail: "Create a command bound to a shared element.",
    },
    PlainCommand {
        name: "tsv::move",
        arity: Arity::exact(3),
        synopsis: &["tsv::move array element newelement"],
        detail: "Rename a shared element.",
    },
    PlainCommand {
        name: "tsv::pop",
        arity: Arity::new(2, 3),
        synopsis: &["tsv::pop array element ?varname?"],
        detail: "Return a shared element and unset it.",
    },
    PlainCommand {
        name: "tsv::array",
        arity: Arity::at_least(2),
        synopsis: &["tsv::array option array ?arg ...?"],
        detail: "Manage a shared array as a whole.",
    },
    PlainCommand {
        name: "tsv::handlers",
        arity: Arity::exact(0),
        synopsis: &["tsv::handlers"],
        detail: "List the persistent storage handlers.",
    },
    PlainCommand {
        name: "tsv::lock",
        arity: Arity::at_least(2),
        synopsis: &["tsv::lock array arg ?arg ...?"],
        detail: "Evaluate a script with a shared array locked.",
    },
    PlainCommand {
        name: "tsv::lindex",
        arity: Arity::at_least(2),
        synopsis: &["tsv::lindex array element ?index ...?"],
        detail: "Return an element of a shared list.",
    },
    PlainCommand {
        name: "tsv::linsert",
        arity: Arity::at_least(4),
        synopsis: &["tsv::linsert array element index value ?value ...?"],
        detail: "Insert into a shared list.",
    },
    PlainCommand {
        name: "tsv::llength",
        arity: Arity::exact(2),
        synopsis: &["tsv::llength array element"],
        detail: "Return the length of a shared list.",
    },
    PlainCommand {
        name: "tsv::lpop",
        arity: Arity::new(2, 3),
        synopsis: &["tsv::lpop array element ?index?"],
        detail: "Remove and return a shared list element.",
    },
    PlainCommand {
        name: "tsv::lpush",
        arity: Arity::new(3, 4),
        synopsis: &["tsv::lpush array element value ?index?"],
        detail: "Insert a value into a shared list.",
    },
    PlainCommand {
        name: "tsv::lrange",
        arity: Arity::exact(4),
        synopsis: &["tsv::lrange array element first last"],
        detail: "Return a range of a shared list.",
    },
    PlainCommand {
        name: "tsv::lreplace",
        arity: Arity::at_least(4),
        synopsis: &["tsv::lreplace array element first last ?value ...?"],
        detail: "Replace a range of a shared list.",
    },
    PlainCommand {
        name: "tsv::lsearch",
        arity: Arity::at_least(3),
        synopsis: &["tsv::lsearch array element ?mode? pattern"],
        detail: "Search a shared list.",
    },
    PlainCommand {
        name: "tsv::lset",
        arity: Arity::at_least(4),
        synopsis: &["tsv::lset array element index ?index ...? value"],
        detail: "Set an element of a shared list.",
    },
    PlainCommand {
        name: "tsv::keyldel",
        arity: Arity::exact(3),
        synopsis: &["tsv::keyldel array element key"],
        detail: "Delete a key from a shared keyed list.",
    },
    PlainCommand {
        name: "tsv::keylget",
        arity: Arity::new(2, 4),
        synopsis: &["tsv::keylget array element ?key? ?varname?"],
        detail: "Read a shared keyed list.",
    },
    PlainCommand {
        name: "tsv::keylkeys",
        arity: Arity::new(2, 3),
        synopsis: &["tsv::keylkeys array element ?key?"],
        detail: "List the keys of a shared keyed list.",
    },
    PlainCommand {
        name: "tsv::keylset",
        arity: Arity::at_least(4),
        synopsis: &["tsv::keylset array element key value ?key value ...?"],
        detail: "Set keys of a shared keyed list.",
    },
    PlainCommand {
        name: "tpool::names",
        arity: Arity::exact(0),
        synopsis: &["tpool::names"],
        detail: "List the existing thread pools.",
    },
    PlainCommand {
        name: "tpool::get",
        arity: Arity::exact(2),
        synopsis: &["tpool::get tpool job"],
        detail: "Return a completed job's result.",
    },
    PlainCommand {
        name: "tpool::wait",
        arity: Arity::new(2, 4),
        synopsis: &["tpool::wait tpool joblist ?varname?"],
        detail: "Wait for jobs to complete.",
    },
    PlainCommand {
        name: "tpool::cancel",
        arity: Arity::new(2, 3),
        synopsis: &["tpool::cancel tpool joblist ?varname?"],
        detail: "Cancel pending jobs.",
    },
    PlainCommand {
        name: "tpool::suspend",
        arity: Arity::exact(1),
        synopsis: &["tpool::suspend tpool"],
        detail: "Suspend job processing.",
    },
    PlainCommand {
        name: "tpool::resume",
        arity: Arity::exact(1),
        synopsis: &["tpool::resume tpool"],
        detail: "Resume job processing.",
    },
    PlainCommand {
        name: "tpool::preserve",
        arity: Arity::exact(1),
        synopsis: &["tpool::preserve tpool"],
        detail: "Increment a pool's reference count.",
    },
    PlainCommand {
        name: "tpool::release",
        arity: Arity::exact(1),
        synopsis: &["tpool::release tpool"],
        detail: "Decrement a pool's reference count.",
    },
];

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
    .into_iter()
    .chain(PLAIN_COMMANDS.iter().map(plain_spec))
    .collect()
}

/// Whether `name` (rooted or not) is a command of the Thread package pack.
#[must_use]
pub fn is_thread_command(name: &str) -> bool {
    let name = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
    thread_command_specs().iter().any(|spec| spec.name == name)
}

#[cfg(test)]
mod tests;
