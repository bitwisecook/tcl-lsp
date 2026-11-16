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

//! Commands that run inside a method: `self`, `my`, `myclass`, `next`, `nextto` and `classvariable`, and the per-object namespace bindings.

use super::introspection::{not_object, render_call_chain};
use super::{
    err, native_context, wrong_args, CallStep, DefTarget, MethodInvocation,
    OriginalObjectPublication, OriginalOoStageCheck,
};
use crate::interp::{Code, Command, Interp};
use crate::obj;
use crate::obj::TclObj;
use tcl_core_types::OoId;
use tcl_registry::native_tcloo_method_definition::{
    NativeTclooMethodReportProtocol, NativeTclooMethodReportRole,
};

// method context: self / my / next

pub(super) fn self_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        let ctx = interp
            .oo
            .borrow()
            .call_stack
            .iter()
            .rev()
            .find(|frame| frame.activation == Some(interp.frames.borrow().current_activation()))
            .map(|frame| {
                let step = frame.chain.get(frame.index);
                (
                    frame.object,
                    step.map(|s| s.provider),
                    step.map(|s| s.method.clone()).unwrap_or_default(),
                    frame.chain.clone(),
                    frame.index,
                )
            });
        let Some((object, class, method, chain, index)) = ctx else {
            // Define-context `self`: inside an `oo::define`/`oo::class create` body,
            // `self ?subcmd …?` applies objdefine-style directives to the object
            // being defined (the classes-as-objects model). `self` alone returns it.
            let def_target = interp.active_def_target();
            if let Some(target) = def_target {
                // C has two `self` commands: `oo::objdefine`'s takes no arguments
                // (`TclOODefineObjSelfObjCmd`), while `oo::define`'s applies
                // class-side directives (`TclOODefineSelfObjCmd`).
                let (tfqn, is_class) = match target {
                    DefTarget::Class(c) => (c, true),
                    DefTarget::Object(o) => (o, false),
                };
                if !is_class {
                    if argv.len() != 1 {
                        return wrong_args(interp, b"self");
                    }
                    interp.set_result(interp.oo_original_name(tfqn));
                    return Code::Ok;
                }
                if argv.len() == 1 {
                    if tcl_registry::native_tcloo_bootstrap::class_definition_self_returns_name(
                        interp.native_invocation_dialect(),
                    ) == Some(true)
                    {
                        interp.set_result(interp.oo_original_name(tfqn));
                        return Code::Ok;
                    }
                    return wrong_args(interp, b"self arg ?arg ...?");
                }
                let creation_id = interp.oo.borrow().objects.get(&tfqn).map(|o| o.creation_id);
                if creation_id.is_none() {
                    return err(
                        interp,
                        b"this command cannot be called when the object has been deleted",
                    );
                }
                let caller = match interp.oo_enter_definition(&DefTarget::Object(tfqn), argv) {
                    Ok(caller) => caller,
                    Err(code) => return code,
                };
                let is_body = argv.len() == 2;
                let code = if is_body {
                    interp.eval_str(&oo_original_bytes!(interp, originals, 1))
                } else {
                    interp.dispatch(&argv[1..])
                };
                interp.oo_leave_definition(caller);
                // The `self { script }` body is a class-side definition: on error it
                // adds its own `(in definition script for class object "X")` frame.
                if is_body && code == Code::Error {
                    interp.add_def_script_frame(
                        b"class object",
                        &interp.oo_name(tfqn),
                        creation_id,
                    );
                }
                return code;
            }
            return err(interp, b"self may only be called from inside a method");
        };
        if argv.len() > 2 {
            return interp.wrong_args_for_prefix(argv, 1, b"subcommand");
        }
        let subcommand = match argv.get(1) {
            Some(&original) => match interp.native_static_string_option_index(
                original,
                tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS,
                false,
                "subcommand",
            ) {
                Ok(index) => {
                    Some(tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS[index].as_bytes())
                }
                Err(error) => return interp.report_cmd_error(error),
            },
            None => None,
        };
        match subcommand {
            None | Some(b"object") => interp.set_result(interp.oo_original_name(object)),
            // `self class` is the *declaring class* of the running method; a method
            // defined directly on the object (objdefine / class-side `self method`)
            // has no declaring class (C: `declaringClassPtr == NULL`).
            Some(b"class") if chain.get(index).is_none_or(|step| step.is_object) => {
                return err(interp, b"method not defined by a class");
            }
            Some(b"class") => interp.set_result(obj::new_string_bytes(
                &interp.oo_name(class.expect("declaring class checked above")),
            )),
            Some(b"method") => {
                // TclOOSelfObjCmd reports the entered current Method name, not
                // the selector spelling retained when its chain was assembled.
                let role = chain.report_role();
                let selected =
                    NativeTclooMethodReportProtocol::select(interp.native_invocation_dialect());
                let Some(_) = selected else {
                    return interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native method report protocol",
                        )
                        .into(),
                    );
                };
                let original = if role == NativeTclooMethodReportRole::Named {
                    chain
                        .get(index)
                        .and_then(|step| step.method_owner.as_ref())
                        .and_then(|record| record.original_name())
                } else {
                    let original = interp
                        .oo
                        .borrow()
                        .method_reports
                        .as_ref()
                        .and_then(|reports| reports.report(role));
                    let Some(original) = original else {
                        return interp.report_cmd_error(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original Foundation method report header",
                            )
                            .into(),
                        );
                    };
                    Some(original)
                };
                if let Some(original) = original {
                    oo_checked!(interp, obj::check_native_liveness(original));
                    interp.set_result(original);
                } else {
                    // Synthetic named intrinsics have no declaration name;
                    // this path cannot reconstruct a special-role header.
                    interp.set_result(obj::new_string_bytes(&method));
                }
            }
            Some(b"namespace") => {
                let ns = interp.oo.borrow().objects.get(&object).map(|o| o.var_ns);
                let name = ns
                    .map(|n| interp.namespaces().qualified_name(n))
                    .unwrap_or_default();
                interp.set_result(obj::new_string_bytes(&name));
            }
            // `self target` — the actual (filtered) method as a `{class method}`
            // pair: the provider/method of the first non-filter step.
            Some(b"target") => {
                let pair = interp
                    .oo
                    .borrow()
                    .call_stack
                    .iter()
                    .rev()
                    .find(|frame| {
                        frame.activation == Some(interp.frames.borrow().current_activation())
                    })
                    .and_then(|f| {
                        f.chain
                            .iter()
                            .find(|s| !s.is_filter)
                            .map(|s| (s.provider, s.method.clone()))
                    });
                let pair = pair.or_else(|| {
                    chain
                        .terminal_report()
                        .map(|step| (step.provider, step.method.clone()))
                });
                match pair {
                    Some((p, m)) => {
                        let objs = [
                            obj::new_string_bytes(&interp.oo_name(p)),
                            obj::new_string_bytes(&m),
                        ];
                        interp.set_result(interp.new_list_object(&objs));
                    }
                    None => interp.set_result_bytes(b""),
                }
            }
            // `self call` — `{chain index}`: the full call chain (each step a
            // `{callType method declarer methodType}` element) and the current index.
            Some(b"call") => {
                let inner = oo_checked!(interp, render_call_chain(interp, &chain));
                let outer = [
                    inner.as_ptr(),
                    obj::new_string_bytes(index.to_string().as_bytes()),
                ];
                interp.set_result(interp.new_list_object(&outer));
                drop(inner);
                oo_original_check!(interp, originals);
            }
            Some(other) => {
                let mut m = b"unsupported self subcommand \"".to_vec();
                m.extend_from_slice(other);
                m.push(b'"');
                return err(interp, &m);
            }
        }
        Code::Ok
    })
}

/// The commands the object system binds in every object's namespace, each with
/// the binding it takes: `my`, and `myclass` (TIP 478), a per-object command
/// dispatching on the object's class, for invoking class-side (`self method`)
/// methods. The backing report names them from here.
const OBJECT_NAMESPACE_COMMANDS: [(&str, ObjectBinding); 2] =
    [("my", Command::OoMy), ("myclass", Command::OoMyClass)];

/// The binding a command the object system installs in each object's namespace
/// takes, from the object's identity.
type ObjectBinding = fn(OoId) -> Command;

/// The names of the commands the object system binds in every object's
/// namespace.
pub(crate) fn object_namespace_command_names() -> impl Iterator<Item = &'static str> {
    OBJECT_NAMESPACE_COMMANDS.iter().map(|(name, _)| *name)
}

impl Interp {
    /// Register private dispatchers in the actual instance namespace.
    pub(crate) fn oo_register_my(&mut self, object: OoId) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let original = OriginalObjectPublication::capture(self, object, scope.currency())?;
        for (name, binding) in OBJECT_NAMESPACE_COMMANDS {
            original.check(self, scope.currency())?;
            self.bind_command_replacement(original.namespace, name.as_bytes(), binding(object));
            original.check(self, scope.currency())?;
        }
        Ok(())
    }
}

pub(crate) fn my_cmd(interp: &mut Interp, object: OoId, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 2 {
            return wrong_args(interp, b"my methodName ?arg ...?");
        }
        // The command token carries its owner through rename and same-name
        // recreation instead of rediscovering an object by display spelling.
        if !interp.oo.borrow().objects.contains_key(&object) {
            return err(interp, b"my may only be called from inside a method");
        }
        let method = oo_original_bytes!(interp, originals, 1);
        interp.oo_invoke_with_original(object, &method, &argv[2..], false, None, Some(argv))
    })
}

pub(crate) fn myclass_cmd(interp: &mut Interp, object: OoId, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 2 {
            return wrong_args(interp, b"myclass methodName ?arg ...?");
        }
        // The owner is stable; its class can morph, so read that relation live.
        let class = interp.oo.borrow().objects.get(&object).map(|ob| ob.class);
        let Some(class) = class else {
            return err(interp, b"myclass may only be called from inside a method");
        };
        let method = oo_original_bytes!(interp, originals, 1);
        interp.oo_invoke_with_original(class, &method, &argv[2..], false, None, Some(argv))
    })
}

pub(super) fn next_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let ctx = interp
        .oo
        .borrow()
        .call_stack
        .iter()
        .rev()
        .find(|frame| frame.activation == Some(interp.frames.borrow().current_activation()))
        .map(|frame| {
            (
                frame.object,
                frame.chain.clone(),
                frame.index,
                frame.target.clone(),
                frame.external,
            )
        });
    let Some((object, chain, index, target, external)) = ctx else {
        return err(interp, b"next may only be called from inside a method");
    };
    // The chain is pre-built (filters then the method steps), so `next` simply
    // advances to the following step. A `next` *from a filter step* clears
    // `filter_handling`, so the wrapped method re-enables filters for its own
    // `my` calls (C's FILTER_HANDLING; oo-12.7); save/restore around the call.
    let from_filter = chain.get(index).is_some_and(|s| s.is_filter);
    if index + 1 < chain.len() {
        let saved_fh = interp.oo.borrow().filter_handling;
        if from_filter {
            interp.oo.borrow_mut().filter_handling = false;
        }
        let code = interp.oo_run(
            object,
            chain,
            index + 1,
            &target,
            &argv[1..],
            MethodInvocation {
                external,
                level_words: None,
                original_argv: Some(argv.to_vec()),
            },
        );
        interp.oo.borrow_mut().filter_handling = saved_fh;
        code
    } else if chain.report_role() == NativeTclooMethodReportRole::Constructor {
        // Past the last constructor in the chain — a no-op (C's default).
        interp.set_result_bytes(b"");
        Code::Ok
    } else if let Some(code) =
        interp.oo_builtin_method(object, &target, &argv[1..], external, Some(argv))
    {
        // Past the last user method: the `oo::object` built-ins (`eval`/
        // `variable`/`varname`) are the terminal implementations of those names,
        // so a user override that calls `next` reaches them (oo-18.5).
        code
    } else if target == b"destroy" {
        // `destroy` likewise has a built-in terminus (a filter/override chain
        // ending in the real teardown).
        interp.oo_destroy(object)
    } else {
        err(interp, b"no next method implementation")
    }
}

/// `nextto class ?arg ...?` (C's `TclOONextToObjCmd`): like `next`, but resumes
/// the call chain at the non-filter step declared by `class`, which must lie
/// *ahead* of the current step (no jumping backwards).
pub(super) fn nextto_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        let dialect = interp.native_invocation_dialect();
        if dialect.native_tcloo_method_cache_protocol().is_none() {
            return nextto_cmd_with_original(interp, argv, originals, None);
        }
        let Some(version) = dialect.tcl_version else {
            return interp.report_cmd_error(
                originals.retain_refusal(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "selected original C NextTo version",
                    )
                    .into(),
                ),
            );
        };
        // Ordinary C9.1 reads its head before context selection. C8.6/C9.0
        // select first and read the head only for an absent-context diagnostic.
        let head = if version == tcl_dialect::TclVersion::V9_1 {
            Some(oo_original_bytes!(interp, originals, 0))
        } else {
            None
        };
        let selected = match native_context::OriginalOoNextContext::capture(interp) {
            Ok(Some(selected)) => selected,
            Ok(None) => {
                let head = match head {
                    Some(head) => head,
                    None => oo_original_bytes!(interp, originals, 0),
                };
                return interp.native_oo_context_required(tcl_core_types::c_string_extent(&head));
            }
            Err(first) => return interp.report_cmd_error(first),
        };
        let current = |actual: &Interp| {
            originals.check_for(actual)?;
            selected.check(actual)
        };
        nextto_cmd_with_original(interp, argv, originals, Some(&current))
    })
}

pub(super) fn nextto_cmd_using(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    extra: Option<OriginalOoStageCheck<'_>>,
) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        nextto_cmd_with_original(interp, argv, originals, extra)
    })
}

fn nextto_cmd_with_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    originals: &crate::interp::native_arguments::OriginalArguments,
    extra: Option<OriginalOoStageCheck<'_>>,
) -> Code {
    if let Some(check) = extra {
        if let Err(first) = check(interp) {
            return interp.report_cmd_error(first);
        }
    }
    let ctx = extra
        .is_none()
        .then(|| {
            interp
                .oo
                .borrow()
                .call_stack
                .iter()
                .rev()
                .find(|frame| frame.activation == Some(interp.frames.borrow().current_activation()))
                .map(|frame| {
                    (
                        frame.object,
                        frame.chain.clone(),
                        frame.index,
                        frame.target.clone(),
                        frame.external,
                    )
                })
        })
        .flatten();
    let selected_present = ctx.is_some()
        || (extra.is_some()
            && interp.oo.borrow().call_stack.iter().any(|frame| {
                frame.activation == Some(interp.frames.borrow().current_activation())
            }));
    if !selected_present {
        return err(interp, b"nextto may only be called from inside a method");
    };
    if argv.len() < 2 {
        return wrong_args(interp, b"nextto class ?arg...?");
    }
    // The class argument resolves like a command: current namespace then the
    // global fallback (C's Tcl_GetObjectFromObj), not relative to the object's
    // instance namespace that the method runs in.
    let original = match originals.original(1) {
        Ok(original) => original,
        Err(error) => return interp.report_cmd_error(error),
    };
    let reached = interp.oo_resolve_original_object_using(original, extra);
    if let Err(first) = originals
        .check()
        .and_then(|()| extra.map_or(Ok(()), |check| check(interp)))
    {
        return interp.report_cmd_error(first);
    }
    let class = match reached {
        Ok(class) => class,
        Err(error) => return interp.report_cmd_error(error),
    };
    if !interp.oo.borrow().objects.contains_key(&class) {
        let name = match native_context::next_original_bytes_using(interp, originals, 1, extra) {
            Ok(bytes) => bytes,
            Err(first) => return interp.report_cmd_error(first),
        };
        return not_object(interp, &name);
    }
    if !interp.oo.borrow().classes.contains_key(&class) {
        let raw = match native_context::next_original_bytes_using(interp, originals, 1, extra) {
            Ok(bytes) => bytes,
            Err(first) => return interp.report_cmd_error(first),
        };
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&raw);
        m.extend_from_slice(b"\" is not a class");
        return err(interp, &m);
    }
    // Re-read the selected context's mutable payload at the actual post-
    // lookup stage. The caller checks the original weak frame/chain first.
    let ctx = ctx.or_else(|| {
        interp
            .oo
            .borrow()
            .call_stack
            .iter()
            .rev()
            .find(|frame| frame.activation == Some(interp.frames.borrow().current_activation()))
            .map(|frame| {
                (
                    frame.object,
                    frame.chain.clone(),
                    frame.index,
                    frame.target.clone(),
                    frame.external,
                )
            })
    });
    let Some((object, chain, index, target, external)) = ctx else {
        return interp.report_cmd_error(
            originals.retain_refusal(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original selected NextTo context",
                )
                .into(),
            ),
        );
    };
    // The actual producer flag distinguishes filters from target steps.
    let is_target = |s: &CallStep| !s.is_filter;
    // Search forward (past the current step) for the class's implementation.
    for i in index + 1..chain.len() {
        if chain[i].provider == class && is_target(&chain[i]) {
            return interp.oo_run(
                object,
                chain,
                i,
                &target,
                &argv[2..],
                MethodInvocation {
                    external,
                    level_words: None,
                    original_argv: Some(argv.to_vec()),
                },
            );
        }
    }
    // Not reachable ahead: distinguish "behind us" from "not in the chain".
    let method_type: &[u8] = if chain.report_role() == NativeTclooMethodReportRole::Constructor {
        b"constructor"
    } else if chain.report_role() == NativeTclooMethodReportRole::Destructor {
        b"destructor"
    } else {
        b"method"
    };
    let reachable_behind = (0..=index).any(|i| chain[i].provider == class && is_target(&chain[i]));
    let raw = match native_context::next_original_bytes_using(interp, originals, 1, extra) {
        Ok(bytes) => bytes,
        Err(first) => return interp.report_cmd_error(first),
    };
    let mut m = method_type.to_vec();
    if reachable_behind {
        m.extend_from_slice(b" implementation by \"");
        m.extend_from_slice(&raw);
        m.extend_from_slice(b"\" not reachable from here");
    } else {
        m.extend_from_slice(b" has no non-filter implementation by \"");
        m.extend_from_slice(&raw);
        m.push(b'"');
    }
    err(interp, &m)
}

/// `classvariable name ...` — the method-context command that links each `name`
/// in the running method's frame to the like-named variable in the *declaring
/// class's* namespace, so every instance of that class shares it. Distinct from
/// the object `variable` method (`oo_builtin_method`), which targets the object's
/// own instance namespace: here the target is the class object's namespace, and
/// it is resolved per *declaring* class, so a method reached via `next` links to
/// its own class rather than the leaf object's.
pub(super) fn classvariable_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if let Some(selected) =
            tcl_registry::native_tcloo_method_definition::NativeTclooClassVariableProtocol::select(
                interp.native_invocation_dialect(),
            )
        {
            return classvariable_original(interp, argv, originals, selected);
        }
        // Method-context only. C exposes `classvariable` through `::oo::Helpers` on a
        // method's namespace path; the runtime models the helpers as global builtins
        // that consult the call stack (as `self`/`my`/`next` do), so outside a method
        // it reports the same "from inside a method" error those use.
        let step = interp
            .oo
            .borrow()
            .call_stack
            .last()
            .and_then(|f| f.chain.get(f.index).cloned());
        let Some(step) = step else {
            return err(
                interp,
                b"classvariable may only be called from inside a method",
            );
        };
        // The providing method must belong to a *class*: a per-object method (a plain
        // object's `objdefine` method, or a class-side `self method`) has no class
        // namespace to share into (C: "method not defined by a class").
        if step.is_object {
            return err(interp, b"method not defined by a class");
        }
        if argv.len() < 2 {
            return wrong_args(interp, b"classvariable name ...");
        }
        // The declaring class is itself an object (classes-as-objects); its instance
        // namespace holds the shared variables.
        let Some(class_ns) = interp
            .oo
            .borrow()
            .objects
            .get(&step.provider)
            .map(|o| o.var_ns)
        else {
            return err(interp, b"method not defined by a class");
        };
        for index in 1..argv.len() {
            let name = oo_original_bytes!(interp, originals, index);
            // C validates the local name as it would any `upvar` target, in this
            // order: an array-element look-alike first, then a namespace separator.
            if crate::frame::split_array_ref(&name).1.is_some() {
                let mut m = b"bad variable name \"".to_vec();
                m.extend_from_slice(&name);
                m.extend_from_slice(
                    b"\": can't create a scalar variable that looks like an array element",
                );
                return interp.set_error(&m);
            }
            if tcl_syntax::naming::is_qualified(&name) {
                let mut m = b"bad variable name \"".to_vec();
                m.extend_from_slice(&name);
                m.extend_from_slice(
                    b"\": can't create a local variable with a namespace separator in it",
                );
                return interp.set_error(&m);
            }
            let code = interp.make_variable(class_ns, &name);
            if code != Code::Ok {
                return code;
            }
        }
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

fn classvariable_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    originals: &crate::interp::native_arguments::OriginalArguments,
    selected: tcl_registry::native_tcloo_method_definition::NativeTclooClassVariableProtocol,
) -> Code {
    use tcl_registry::native_tcloo_method_definition::NativeTclooClassVariableHeadStage;

    if argv.len() < 2 {
        return wrong_args(interp, b"classvariable name ...");
    }
    let (activation, frame_owner) = {
        let frames = interp.frames.borrow();
        (
            frames.current_activation(),
            frames.current_activation_owner(),
        )
    };
    let frame_current = |interpreter: &Interp| -> Result<(), tcl_cmd_core::CmdError> {
        originals.check()?;
        let frames = interpreter.frames.borrow();
        if frame_owner.upgrade().is_some()
            && frames.current_activation() == activation
            && frame_owner.ptr_eq(&frames.current_activation_owner())
        {
            Ok(())
        } else {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original classvariable method frame",
            )
            .into())
        }
    };
    let head = if selected.head_stage() == NativeTclooClassVariableHeadStage::BeforeContext {
        let head = oo_original_bytes!(interp, originals, 0);
        oo_checked!(interp, frame_current(interp));
        Some(head)
    } else {
        None
    };
    let step = {
        let state = interp.oo.borrow();
        state
            .call_stack
            .iter()
            .rev()
            .find(|frame| frame.activation == Some(activation))
            .and_then(|frame| frame.chain.get(frame.index).cloned())
    };
    let Some(step) = step else {
        let head = head.unwrap_or_else(Vec::new);
        let head = if selected.head_stage() == NativeTclooClassVariableHeadStage::MissingContext {
            oo_original_bytes!(interp, originals, 0)
        } else {
            head
        };
        oo_checked!(interp, frame_current(interp));
        return interp.native_oo_context_required(tcl_core_types::c_string_extent(&head));
    };
    if step.is_object {
        return interp.error_with_code(
            b"method not defined by a class",
            b"TCL OO UNMATCHED_CONTEXT",
        );
    }
    let target = oo_checked!(
        interp,
        native_context::OriginalOoTarget::capture(interp, step.provider, true)
    );
    let namespace = {
        let state = interp.oo.borrow();
        state.objects[&step.provider].var_ns
    };
    let holder = match interp.native_namespace_token_at(namespace) {
        Some(holder) => holder,
        None => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original classvariable namespace",
                )
                .into(),
            );
        }
    };
    let check = |interpreter: &Interp| -> Result<(), tcl_cmd_core::CmdError> {
        frame_current(interpreter)?;
        target.check_instance_namespace(interpreter, namespace, &holder)
    };
    oo_checked!(interp, check(interp));
    for index in 1..argv.len() {
        let bytes = originals.bytes(index);
        oo_checked!(interp, check(interp));
        let bytes = oo_checked!(interp, bytes);
        if let Err(error) = selected.validate_name(&bytes) {
            return interp.error_with_code(&error.message(&bytes), error.error_code());
        }
    }
    for &original in &argv[1..] {
        oo_checked!(interp, check(interp));
        let target = interp.prepare_original_c_namespace_link_target(original, namespace);
        oo_checked!(interp, check(interp));
        let target = match target {
            Ok(target) => target,
            Err(code) => return code,
        };
        let code = interp.bind_selected_c_alias_local(original, target);
        oo_checked!(interp, check(interp));
        if code != Code::Ok {
            return code;
        }
    }
    Code::Ok
}
