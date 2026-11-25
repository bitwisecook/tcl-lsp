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

use super::*;
use crate::{CommandRegistry, InterpreterScope, InvocationWord, InvocationWords};

fn registry() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    for spec in thread_command_specs() {
        registry.insert(spec);
    }
    registry
}

#[test]
fn send_body_and_reply_keep_separate_execution_worlds_and_timing() {
    let registry = registry();
    for (arguments, asynchronous) in [
        (vec!["tid1", "body", "reply"], false),
        (vec!["-async", "tid1", "body", "reply"], true),
    ] {
        let invocation = registry
            .resolve_invocation("thread::send", &arguments, None)
            .unwrap();
        let contexts = invocation.argument_evaluation_contexts().unwrap();
        let body = contexts
            .iter()
            .find(|context| context.role == ArgRole::Body)
            .unwrap();
        let reply = contexts
            .iter()
            .find(|context| context.role == ArgRole::VarWrite)
            .unwrap();
        assert_eq!(body.interpreter, InterpreterScope::Any);
        assert_eq!(
            body.context.unwrap().interpreter,
            BodyInterpreter::ThreadArgument(u8::from(asynchronous))
        );
        assert_eq!(body.context.unwrap().frame, EvaluationFrame::Global);
        assert_eq!(reply.interpreter, InterpreterScope::Current);
        assert_eq!(
            reply.context.unwrap().frame,
            if asynchronous {
                EvaluationFrame::Global
            } else {
                EvaluationFrame::Invoking
            }
        );
        assert_eq!(
            reply.context.unwrap().timing,
            if asynchronous {
                ScriptTiming::Deferred
            } else {
                ScriptTiming::SameInvocation
            }
        );
    }
}

#[test]
fn computed_thread_selector_cannot_become_an_interpreter_path() {
    assert_eq!(
        BodyInterpreter::ThreadArgument(0).resolve(InvocationArguments::literals(&["tid1"])),
        InterpreterScope::Any
    );
    let registry = registry();
    let arguments = [InvocationWord::Dynamic, InvocationWord::Literal("body")];
    let invocation = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("thread::send"), &arguments),
            None,
        )
        .resolved()
        .unwrap();
    assert!(invocation.argument_evaluation_contexts().is_none());
}

#[test]
fn shared_store_is_not_an_interpreters_variable_cell_domain() {
    let specs = thread_command_specs();
    let get = specs.iter().find(|spec| spec.name == "tsv::get").unwrap();
    let effects = get.world_effects.stated().unwrap();
    let access = effects.static_footprint.accesses[0];
    assert_eq!(access.domain, WorldStateDomain::ThreadSharedStore);
    assert_eq!(access.interpreter, StaticInterpreterScope::Any);
    let contexts = get.evaluation_contexts.unwrap();
    let values = InvocationArguments::literals(&["shared", "key", "local"]);
    assert_eq!(
        contexts
            .context_at(values, 0, ArgRole::Name)
            .unwrap()
            .interpreter,
        BodyInterpreter::ProcessShared
    );
    assert_eq!(
        contexts
            .context_at(values, 2, ArgRole::VarWrite)
            .unwrap()
            .interpreter,
        BodyInterpreter::Current
    );
    assert!(get.arg_roles.contains(&(2, ArgRole::VarWrite)));
}

#[test]
fn pool_callbacks_and_jobs_are_foreign_without_guessing_a_worker() {
    let registry = registry();
    let invocation = registry
        .resolve_invocation(
            "tpool::create",
            &["-initcmd", "setup", "-exitcmd", "cleanup"],
            None,
        )
        .unwrap();
    for context in invocation.argument_evaluation_contexts().unwrap() {
        assert_eq!(context.interpreter, InterpreterScope::Any);
        assert_eq!(context.context.unwrap().frame, EvaluationFrame::Global);
    }
    let invocation = registry
        .resolve_invocation("tpool::post", &["-nowait", "pool1", "job"], None)
        .unwrap();
    let context = invocation
        .argument_evaluation_contexts()
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(context.argument, 2);
    assert_eq!(context.interpreter, InterpreterScope::Any);
}

#[test]
fn send_option_layout_matches_source_bound_and_ignored_trailing_words() {
    let extra = send_layout(InvocationArguments::literals(&[
        "tid1", "body", "reply", "ignored", "ignored",
    ]))
    .unwrap();
    assert_eq!(extra.body, 1);
    assert_eq!(extra.result, Some(2));
    let combined = send_layout(InvocationArguments::literals(&[
        "-async", "-head", "tid1", "body", "reply",
    ]))
    .unwrap();
    assert_eq!(combined.thread, 2);
    assert_eq!(combined.body, 3);
    assert!(combined.asynchronous);
    let callback = send_layout(InvocationArguments::literals(&[
        "-command", "-head", "tid1", "body", "prefix",
    ]))
    .unwrap();
    assert!(callback.callback);
    assert_eq!(callback.result, Some(4));
    assert!(
        send_layout(InvocationArguments::literals(&[
            "-async", "-head", "-command", "tid1", "body", "prefix"
        ]))
        .is_none()
    );
}
