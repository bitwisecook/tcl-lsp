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

//! Running a script body in a namespace frame: `namespace eval` and `namespace inscope`.

use crate::command::settle_control_options;
use crate::interp::Vm;
use crate::value::Value;
use tcl_native_model::native_eval_object::{EvalObjectPurpose, NativeNamespaceFramePurpose};
use tcl_runtime_api::completion_options::ControlOptionPolicy;
use tcl_runtime_api::{Code, Completion};

/// Run `body` as a script in namespace `target`, absorbing a top-level
/// `return` at the boundary (a namespace body completes like a proc body).
///
/// The body runs in its own call frame (like a proc) so `info level` counts it
/// and `uplevel`/`upvar` from a proc called within reach it (and its namespace
/// variables). `call_argv` is the invoking command (e.g. `namespace eval ::ns
/// {…}`) for `info level N`.
fn eval_in_ns(
    vm: &mut Vm,
    original: &Value,
    body: NamespaceBody<'_>,
    frame_purpose: NativeNamespaceFramePurpose,
    create: bool,
) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        // Native eval first resolves the original namespace object. Creation on a
        // miss uses its written spelling, without installing a cache for that miss.
        let token = match vm.namespace_object_lookup(original) {
            Ok(Some(token)) => token,
            Ok(None) => {
                let written = match original_names.read(vm, original) {
                    Ok(bytes) => bytes,
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                };
                if create {
                    match vm.activate_namespace_operand(&written) {
                        Ok(token) => token,
                        Err(completion) => return completion,
                    }
                } else {
                    return vm.namespace_lookup_error_at(
                        &written,
                        tcl_syntax::naming::NativeNamespaceLookupOperation::Inscope,
                    );
                }
            }
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let selected_current = vm.native_namespace_token_has_owner(token);
        if let Err(error) = original_names.require(
            vm,
            selected_current,
            "selected original namespace evaluation holder",
        ) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        // Native handlers fetch their frame argv after namespace lookup, whose
        // getters can independently withdraw an active ensemble root rewrite.
        let call_argv = match namespace_frame_arguments(vm, frame_purpose, &original_names) {
            Ok(arguments) => arguments,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        vm.push_ns_eval_token_frame(token, call_argv);
        vm.enter_ns_script();
        let result = eval_namespace_body(vm, body);
        // The namespace the script ran in names its error frame (C's
        // `NsEval_Callback`): read while it is still current.
        let failed = match &result {
            Ok(completion) => completion.code == Code::Error,
            Err(_) => true,
        };
        let frame_namespace = if failed {
            tcl_cmd_core::namespace::current_original(vm)
                .ok()
                .map(|fqn| fqn.string_bytes().to_vec())
        } else {
            None
        };
        vm.leave_ns_script();
        vm.pop_ns();
        vm.pop_call_frame();
        if let Err(error) = original_names.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let completion = match result {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(vm, error),
        };
        if completion.code == Code::Error
            && let Some(fqn) = frame_namespace
        {
            let command = match frame_purpose {
                NativeNamespaceFramePurpose::Eval => "eval",
                NativeNamespaceFramePurpose::Inscope => "inscope",
            };
            let text =
                tcl_cmd_core::namespace::script_frame_text(command, &fqn, vm.runtime_version());
            let line = vm.error_line();
            vm.append_frame_text(&format!("{} line {line}", String::from_utf8_lossy(&text)));
        }
        settle_control_options(vm, completion, ControlOptionPolicy::FRESH_FORWARDED)
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

/// Original body operands retain their own source and concatenation protocols.
/// Assembly occurs in the already selected namespace frame, after argv is set.
#[derive(Clone, Copy)]
enum NamespaceBody<'a> {
    Eval(&'a [Value]),
    Inscope {
        script: &'a Value,
        extra: &'a [Value],
        jim: bool,
    },
}

fn eval_namespace_body(
    vm: &mut Vm,
    body: NamespaceBody<'_>,
) -> Result<Completion<Value>, crate::error::TclError> {
    let concatenated;
    let (body, purpose, location) = match body {
        NamespaceBody::Eval([original]) => (
            original,
            EvalObjectPurpose::NamespaceBody,
            original.source_location(),
        ),
        NamespaceBody::Eval(parts) => {
            concatenated = tcl_cmd_core::list::concat_selected(vm, parts).map_err(|error| {
                crate::error::TclError::from_completion(crate::command::completion_from_cmd_error(
                    vm, error,
                ))
            })?;
            (&concatenated, EvalObjectPurpose::NamespaceConcat, None)
        }
        NamespaceBody::Inscope { script, extra, jim } => {
            let location = extra.is_empty().then(|| script.source_location()).flatten();
            if extra.is_empty() && !jim {
                (script, EvalObjectPurpose::NamespaceBody, location)
            } else {
                concatenated = inscope_script(vm, script, extra)
                    .map_err(crate::error::TclError::from_completion)?;
                (&concatenated, EvalObjectPurpose::NamespaceConcat, location)
            }
        }
    };
    vm.eval_original_script_value(body, purpose, location)
}

/// Carry the genuine invocation values into the selected namespace frame.
/// A worker selector manufactured only for local dispatch is never an argv word.
pub(super) fn namespace_frame_arguments(
    vm: &mut Vm,
    purpose: NativeNamespaceFramePurpose,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Result<Vec<Value>, tcl_cmd_core::CmdError> {
    use tcl_native_model::native_eval_object::NativeNamespaceFrameArguments;
    use tcl_syntax::value::ValueError;

    original_names.check(vm)?;
    let recipe = vm
        .eval_object_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original namespace frame argument issuer",
        ))?
        .namespace_frame_arguments(purpose);
    if recipe == NativeNamespaceFrameArguments::Empty {
        return Ok(Vec::new());
    }
    let entered = vm
        .native_invocation
        .arguments
        .as_ref()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original namespace frame invocation vector",
        ))?
        .lifetime_view();
    let words: Vec<_> = entered
        .elements()?
        .iter()
        .map(Value::native_lifetime_lease)
        .collect();
    let words = if recipe == NativeNamespaceFrameArguments::EnsembleRoot {
        let rewrites: Vec<_> = vm
            .native_invocation
            .usage_rewrites
            .iter()
            .map(|rewrite| tcl_cmd_core::ensemble::ArgumentUsageRewrite {
                original_prefix: rewrite
                    .original_prefix
                    .iter()
                    .map(Value::native_lifetime_lease)
                    .collect(),
                removed_words: rewrite.removed_words,
            })
            .collect();
        tcl_cmd_core::ensemble::rewrite_argument_usage(&words, &rewrites)
    } else {
        words
    };
    for word in &words {
        original_names.current_for_value(vm, word.value())?;
    }
    entered.elements()?;
    original_names.check(vm)?;
    Ok(words
        .into_iter()
        .map(crate::value::NativeObjectLifetimeLease::into_value)
        .collect())
}

pub(super) fn ns_inscope(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, parts)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"",
        );
    };
    let Some((script, extra)) = parts.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"",
        );
    };
    let jim = vm
        .name_policy_protocol()
        .is_some_and(|policy| policy.recipe().is_jim084());
    let create = vm
        .name_policy_protocol()
        .is_some_and(|policy| policy.recipe().is_jim084());
    eval_in_ns(
        vm,
        ns,
        NamespaceBody::Inscope { script, extra, jim },
        NativeNamespaceFramePurpose::Inscope,
        create,
    )
}

/// The script `namespace inscope ns script ?arg ...?` evaluates:
/// `Tcl_ConcatObj(script, list(arg …))` — `NamespaceInscopeCmd`
/// (`generic/tclNamesp.c`) collects the trailing words into a **list object**
/// and concatenates that list's string representation onto `script`. So the
/// tail arrives as list *elements*, not as space-joined script text, and each
/// word reaches the invoked command as exactly one argument however much
/// whitespace or list punctuation it holds:
///
/// ```text
/// namespace inscope :: {puts} {a b}   → prints "a b"  (one argument)
/// namespace eval    :: {puts} {a b}   → error: can not find channel named "a"
/// ```
///
/// (`namespace eval`'s plain space-join is right for *its* concat semantics;
/// `inscope` is the one family member that list-quotes — the registry models
/// the split as `SCRIPT_APPENDS_LIST_ARGS` refining `SCRIPT_CONCATENATES_ARGS`.
/// Space-joining here too would turn `{x y}` into two arguments.)
///
/// Both halves reuse the canonical implementations rather than re-deriving
/// them: the list's string rep comes from `Value::list`'s
/// `tcl_syntax::list::join_list` quoting (`Tcl_ScanElement` /
/// `Tcl_ConvertElement`), and the concatenation itself is the shared
/// `tcl_cmd_core::list::concat_selected`, which owns the selected engine's
/// string or representation-sensitive concatenation protocol. C Tcl trims
/// each part and drops one that is empty after trimming, so an all-whitespace
/// `script` contributes no leading separator.
///
/// With no trailing words C takes the `objc == 3` arm and evaluates `script`
/// verbatim (no concat, hence no trim and no trailing space), which the early
/// return mirrors.
fn inscope_script(
    vm: &mut Vm,
    script: &Value,
    extra: &[Value],
) -> Result<Value, Completion<Value>> {
    let tail = Value::list(extra.to_vec());
    tcl_cmd_core::list::concat_selected(vm, &[script.clone(), tail])
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
}

pub(super) fn ns_eval(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, body_parts)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace eval name arg ?arg ...?\"",
        );
    };
    if body_parts.is_empty() {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace eval name arg ?arg ...?\"",
        );
    }
    eval_in_ns(
        vm,
        ns,
        NamespaceBody::Eval(body_parts),
        NativeNamespaceFramePurpose::Eval,
        true,
    )
}
