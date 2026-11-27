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

//! The commands of a running method activation: `my`, `self`, `next` and `nextto`.

use super::introspection::render_call_chain;
use super::method_invocation::{oo_invoke_value, original_call_words, run_step};
use super::method_resolution::unknown_method;
use super::{
    OoFrame, StepRole, display, display_oo_bytes, native_context, native_method_key,
    resolve_class_value,
};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_native_model::native_tcloo_method_definition::{
    NativeTclooMethodReportProtocol, NativeTclooMethodReportRole,
};
use tcl_runtime_api::Completion;

// self / my / next / nextto

pub(super) fn cmd_my(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let scope = match crate::interp::native_operation_currency::NativeOperationScope::enter(vm) {
        Ok(scope) => scope,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let reached = cmd_my_entered(vm, args);
    match scope.check(vm) {
        Ok(()) => reached,
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

fn cmd_my_entered(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(fr) = native_context::current_method(vm) else {
        return err("invalid command name \"my\"");
    };
    let obj = fr.object;
    let invoked = fr.invoked.clone();
    let Some((method, rest)) = args.split_first() else {
        return crate::command::native_wrong_args(vm, "my methodName ?arg ...?");
    };
    let key = match native_method_key(vm, method) {
        Ok(key) => key,
        Err(completion) => return completion,
    };
    oo_invoke_value(vm, obj, &key, method, rest, false, &invoked)
}

/// The object a method frame reports as `self` — its command name (C's
/// `TclOOObjectName`).
pub(super) fn frame_object_name(vm: &Vm, fr: &OoFrame) -> Value {
    native_context::object_name(vm, fr.object)
}

/// The running method's object as `self` / `self object` reports it — `None`
/// outside a method. The `tclooSelf` opcode's core.
pub(crate) fn current_object_name(vm: &Vm) -> Option<Value> {
    native_context::current_method(vm).map(|frame| frame_object_name(vm, frame))
}

/// Whether a method invocation is running — the context `self`/`next`/`nextto`
/// (and their opcodes) need.
pub(crate) fn in_method(vm: &Vm) -> bool {
    native_context::current_method(vm).is_some()
}

pub(super) fn cmd_self(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let context = match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
        Ok(context) => context,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let reached = cmd_self_entered(vm, args, &context);
    match context.finish(vm) {
        Ok(()) => reached,
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

fn self_outside_method(vm: &mut Vm) -> Completion<Value> {
    let Some(original) = vm.invoked_name_value() else {
        return vm.refuse_host_command("original self invocation head is unavailable".into());
    };
    let Some(protocol) = vm.actual_native_string_protocol() else {
        return vm
            .refuse_host_command("native self invocation string getter is unavailable".into());
    };
    let head = match original.native_string_bytes(protocol) {
        Ok(head) => head,
        Err(error) => {
            return crate::command::completion_from_cmd_error(
                vm,
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
            );
        }
    };
    native_context::helper_context_error(vm, &head)
}

fn self_call_chain(vm: &Vm, fr: &OoFrame) -> Completion<Value> {
    let unknown = fr.chain.iter().any(|s| s.role == StepRole::Unknown);
    ok(Value::list(vec![
        Value::list(render_call_chain(vm, &fr.chain, &fr.method, unknown)),
        Value::int(i64::try_from(fr.index).unwrap_or(0)),
    ]))
}

/// `self target` / `self filter`: the call a running filter wraps, or the
/// filter itself; an error outside a filter.
fn self_filter_context(vm: &Vm, fr: &OoFrame, sub: &str) -> Completion<Value> {
    let step = &fr.chain[fr.index];
    let Some(filter) = step.filter() else {
        return err("not inside a filtering context");
    };
    if sub == "filter" {
        let kind = if step.is_object { "object" } else { "class" };
        return ok(Value::list(vec![
            Value::from_native_string_bytes(display_oo_bytes(vm, step.provider)),
            Value::string(kind),
            Value::from_native_string_bytes(vm.oo.method_name_bytes(filter).to_vec()),
        ]));
    }
    let declarer = fr
        .chain
        .iter()
        .find(|s| s.filter().is_none())
        .map_or_else(Vec::new, |s| display_oo_bytes(vm, s.provider));
    ok(Value::list(vec![
        Value::from_native_string_bytes(declarer),
        Value::from_native_string_bytes(vm.oo.method_name_bytes(&fr.method).to_vec()),
    ]))
}

fn cmd_self_entered(
    vm: &mut Vm,
    args: &[Value],
    context: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Completion<Value> {
    let Some(original_frame) = native_context::OriginalMethodFrame::capture(vm) else {
        return self_outside_method(vm);
    };
    let fr = native_context::current_method(vm).expect("captured original method frame");
    let self_name = frame_object_name(vm, fr);
    let obj = fr.object;
    if args.len() > 1 {
        return crate::command::native_wrong_args(vm, "self subcommand");
    }
    let selected = args.first().map(|original| {
        vm.native_static_option_index_checked(
            original,
            tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS,
            false,
            "subcommand",
        )
    });
    if let Err(error) = original_frame.check(vm, context) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let sub = match selected {
        Some(Ok(index)) => Some(tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS[index]),
        Some(Err(error)) => return crate::command::completion_from_cmd_error(vm, error),
        None => None,
    };
    let fr = native_context::current_method(vm).expect("checked original method frame");
    match sub {
        None | Some("object") => ok(self_name),
        Some("namespace") => {
            let ns = vm.oo.objects.get(&obj).map_or_default(|o| o.ns.clone());
            ok(Value::string(display(&ns)))
        }
        Some("class") => {
            // The class that declares the running method (its step provider).
            let step = &fr.chain[fr.index];
            if step.is_object {
                return err("method not defined by a class");
            }
            ok(Value::from_native_string_bytes(display_oo_bytes(
                vm,
                step.provider,
            )))
        }
        Some("method") => {
            let role = fr.chain.report_role();
            if NativeTclooMethodReportProtocol::select(vm.actual_native_invocation_dialect())
                .is_none()
            {
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native method report protocol",
                    )
                    .into(),
                );
            }
            if role == NativeTclooMethodReportRole::Named {
                let step = &fr.chain[fr.index];
                ok(Value::from_native_string_bytes(
                    vm.oo
                        .method_name_bytes(step.filter().unwrap_or(&fr.method))
                        .to_vec(),
                ))
            } else if let Some(original) = vm
                .oo
                .method_reports
                .as_ref()
                .and_then(|reports| reports.report(role))
            {
                ok(original)
            } else {
                crate::command::completion_from_cmd_error(
                    vm,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original Foundation method report header",
                    )
                    .into(),
                )
            }
        }
        Some("call") => self_call_chain(vm, fr),
        Some(sub @ ("target" | "filter")) => self_filter_context(vm, fr, sub),
        Some(other) => err(format!("unsupported self subcommand \"{other}\"")),
    }
}

/// `next ?arg…?` — run the next implementation on the method chain. Also the
/// `tclooNext` opcode's core (which checks the method context itself, so it can
/// report C's `INST_TCLOO_NEXT` message instead of the command's
/// `invalid command name`).
pub(crate) fn cmd_next(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(head) = vm.invoked_name_value() else {
        return vm.refuse_host_command("original next invocation head is unavailable".into());
    };
    cmd_next_original(vm, &head, args)
}

pub(crate) fn cmd_next_original(vm: &mut Vm, head: &Value, args: &[Value]) -> Completion<Value> {
    let Some(fr) = native_context::current_method(vm) else {
        return native_context::helper_context_error(vm, b"next");
    };
    let obj = fr.object;
    let chain = fr.chain.clone();
    let index = fr.index;
    let method = fr.method.clone();
    let external = fr.external;
    let invoked = fr.invoked.clone();
    let usage = fr.usage_prefix.clone();
    if index + 1 >= chain.len() {
        // `unknown`'s chain ends in `oo::object`'s built-in, which reports the
        // miss for the name it was handed.
        if method == "unknown" && chain.report_role() == NativeTclooMethodReportRole::Named {
            return match args.first() {
                Some(missed) => match native_method_key(vm, missed) {
                    Ok(missed) => unknown_method(vm, obj, &missed, external),
                    Err(completion) => completion,
                },
                None => crate::command::native_wrong_args(vm, "next method ?arg ...?"),
            };
        }
        // Past the end there is no further implementation of this kind.
        let kind = if chain.report_role() == NativeTclooMethodReportRole::Constructor {
            "constructor"
        } else if chain.report_role() == NativeTclooMethodReportRole::Destructor {
            "destructor"
        } else {
            "method"
        };
        return err(format!("no next {kind} implementation"));
    }
    let call_identity = original_call_words(std::iter::once(head).chain(args));
    run_step(
        vm,
        obj,
        &chain,
        index + 1,
        method,
        args,
        external,
        invoked,
        usage,
        call_identity,
    )
}

/// `nextto class ?arg…?` — resume the method chain at `class`. Also the
/// `tclooNextClass` opcode's core (see [`cmd_next`]).
pub(crate) fn cmd_nextto(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(head) = vm.invoked_name_value() else {
        return vm.refuse_host_command("original nextto invocation head is unavailable".into());
    };
    cmd_nextto_original(vm, &head, args)
}

pub(crate) fn cmd_nextto_original(vm: &mut Vm, head: &Value, args: &[Value]) -> Completion<Value> {
    let Some(fr) = native_context::current_method(vm) else {
        return native_context::helper_context_error(vm, b"nextto");
    };
    let Some((cls, rest)) = args.split_first() else {
        return crate::command::native_wrong_args(vm, "nextto class ?arg...?");
    };
    let obj = fr.object;
    let chain = fr.chain.clone();
    let index = fr.index;
    let method = fr.method.clone();
    let external = fr.external;
    let invoked = fr.invoked.clone();
    let usage = fr.usage_prefix.clone();
    let target = match resolve_class_value(vm, cls) {
        Ok(target) => target,
        Err(completion) => return completion,
    };
    // Find the target class ahead of the current step.
    if let Some(pos) = chain
        .iter()
        .enumerate()
        .position(|(i, s)| i > index && !s.is_object && Some(s.provider) == target)
    {
        let call_identity = original_call_words(std::iter::once(head).chain(args));
        return run_step(
            vm,
            obj,
            &chain,
            pos,
            method,
            rest,
            external,
            invoked,
            usage,
            call_identity,
        );
    }
    err(format!(
        "method has no non-filter implementation by \"{}\"",
        cls.to_str()
    ))
}
