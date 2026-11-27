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

//! `oo::define` and `oo::objdefine`: the definition script and the definition-body commands.

use super::declared_variables::declare_variable;
use super::introspection::object_key;
use super::{
    DefTarget, Method, MethodParameters, display_oo_bytes, err_code, native_context,
    native_method_cache, native_method_key_for_target, object_message, original_name_operands,
    resolve_class_value,
};
use crate::command::parse_params_value;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::rc::Rc;
use tcl_runtime_api::{Code, Completion, OoId};

/// `private <def-command> …` — for `property`, the accessors are already
/// private; for the other definition commands, force unexported visibility.
pub(super) fn cmd_private(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if active_target(vm).is_none() {
        return def_body_cmd(vm, "private", args);
    }
    let Some((verb, rest)) = args.split_first() else {
        return ok(Value::int(i64::from(vm.oo.private_depth > 0)));
    };
    // TIP 500 block form: a lone argument is a definition *script*, not a
    // command name, so `private {variable X}` declares X privately. Raise the
    // private depth for its evaluation; `variable` and `method` inside consult
    // it. The depth is restored on every path, including an error unwinding
    // out of the script.
    vm.oo.private_depth += 1;
    let result = if rest.is_empty() {
        match vm.eval_original_script_value(
            verb,
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            None,
        ) {
            Ok(c) => c,
            Err(e) => crate::command::completion_from_tcl_error(vm, e),
        }
    } else {
        native_context::magic_definition_invoke(vm, verb, rest)
    };
    vm.oo.private_depth -= 1;
    result
}

// oo::define / oo::objdefine and the definition-body commands.

pub(super) fn cmd_oo_define(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    run_define(vm, args, true)
}

pub(super) fn cmd_oo_objdefine(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    run_define(vm, args, false)
}

/// `oo::define target …` / `oo::objdefine target …` — both the script form
/// (`target {body}`) and the single-command form (`target sub args…`).
pub(super) fn run_define(vm: &mut Vm, args: &[Value], is_class: bool) -> Completion<Value> {
    original_name_operands::run(vm, None, |vm, original_names| {
        let verb = if is_class {
            "oo::define"
        } else {
            "oo::objdefine"
        };
        if args.len() < 2 {
            return err(format!(
                "wrong # args: should be \"{verb} target ?arg ...?\""
            ));
        }
        let target = match object_key(vm, &args[0]) {
            Ok(target) => target,
            Err(completion) => return completion,
        };
        if is_class && !vm.oo.classes.contains_key(&target) {
            return err(object_message(
                vm,
                target,
                b"",
                b" does not refer to a class",
            ));
        }
        if let Err(error) = original_names.select_target(vm, is_class, target) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let mut invocation = vec![
            vm.invoked_name_value()
                .unwrap_or_else(|| Value::string(verb)),
        ];
        invocation.extend_from_slice(args);
        let usage = (args.len() > 2).then(|| invocation[..3].to_vec());
        let mut definition =
            match native_context::enter_definition(vm, is_class, target, invocation) {
                Ok(definition) => definition,
                Err(completion) => return completion,
            };
        if let Err(error) = definition.replace_usage(vm, usage) {
            let completion = crate::command::completion_from_cmd_error(vm, error);
            return definition.finish(vm, completion);
        }
        let result = original_names.in_current_frame(vm, |vm, original_names| {
            if args.len() == 2 {
                // Script form: evaluate the body with the def target active. An error
                // unwinding out of it gains the `(in definition script for …)` frame.
                let source = match original_names.read(vm, &args[1]) {
                    Ok(bytes) => tcl_lexer::SourceImage::native(bytes.as_ref()),
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                };
                let reached = vm.eval_source_image_at_internal(&source, args[1].source_location());
                definition_script_completion(vm, &definition, reached)
            } else {
                // Single-command form: dispatch `sub args…` as one definition directive
                // (no definition-script frame — the directive is the command itself).
                native_context::magic_definition_invoke(vm, &args[1], &args[2..])
            }
        });
        definition.finish(vm, result)
    })
}

/// A definition worker's static synopsis follows its actual original directive
/// words. Script form has no outer directive prefix; nested directives restore
/// the previous rewrite after leaving their authentic definition activation.
pub(super) fn definition_wrong_args(vm: &mut Vm, usage: &str) -> Completion<Value> {
    let Some(words) = vm.oo.definition_usage.words().cloned() else {
        return crate::command::native_wrong_args(vm, usage);
    };
    let mut header = match vm.native_argument_usage_header(&words) {
        Ok(header) => header,
        Err(error) => return error,
    };
    if let Some((_, tail)) = usage.split_once(' ') {
        header.push(b' ');
        header.extend_from_slice(tail.as_bytes());
    }
    crate::command::native_wrong_args_bytes(vm, &header)
}

/// Append the `(in definition script for {class,object} "::NAME" line N)` frame
/// to `errorInfo` as an error unwinds out of an `oo::define`/`oo::objdefine`
/// *script* body — the context frame C's `TclOODefineObjCmd` adds. `line` is the
/// body-relative source line of the failing directive (C's `iPtr->errorLine`).
pub(super) fn append_define_frame(vm: &mut Vm, is_class: bool, target: OoId, msg: &[u8]) {
    let kind = if is_class { "class" } else { "object" };
    let line = vm.error_line();
    let mut frame = format!("\n    (in definition script for {kind} ").into_bytes();
    frame.push(b'"');
    frame.extend_from_slice(&display_oo_bytes(vm, target));
    frame.extend_from_slice(format!("\" line {line})").as_bytes());
    vm.seed_error_info_frame(msg, &frame);
}

/// Settle a definition script's outcome, seeding the definition-script frame
/// on an error through the checked definition exit.
pub(super) fn definition_script_completion(
    vm: &mut Vm,
    definition: &native_context::OriginalDefinitionExit,
    reached: Result<Completion<Value>, crate::error::TclError>,
) -> Completion<Value> {
    match reached {
        Ok(c) if c.code == Code::Return => ok(c.result),
        Ok(c) if c.code == Code::Error => {
            if let Err(error) = definition.append_error_frame(vm, &c.result) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            c
        }
        Ok(c) => c,
        Err(e) => {
            if let Some(completion) = e.guest_completion()
                && completion.code == Code::Error
                && let Err(error) = definition.append_error_frame(vm, &completion.result)
            {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            crate::command::completion_from_tcl_error(vm, e)
        }
    }
}

/// The active definition target at the exact selected variable activation.
pub(super) fn active_target(vm: &Vm) -> Option<(bool, OoId)> {
    let (dt, _, _, _) = &vm.oo.def_stack[native_context::definition_index(vm)?];
    Some(match dt {
        DefTarget::Class(c) => (true, *c),
        DefTarget::Object(o) => (false, *o),
    })
}

/// A definition-body command (`method`, `superclass`, …). Resolves the active
/// target and applies the directive.
pub(super) fn def_body_cmd(vm: &mut Vm, verb: &str, args: &[Value]) -> Completion<Value> {
    let Some((is_class, target)) = active_target(vm) else {
        return err_code(
            "this command may only be called from within the context of an ::oo::define or ::oo::objdefine command",
            &["TCL", "OO", "MONKEY_BUSINESS"],
        );
    };
    if !vm.oo.objects.contains_key(&target) {
        return err_code(
            "this command cannot be called when the object has been deleted",
            &["TCL", "OO", "MONKEY_BUSINESS"],
        );
    }
    match verb {
        "method" => def_method(vm, is_class, target, args),
        "deletemethod" => native_method_cache::delete_method(vm, is_class, target, args),
        "renamemethod" => native_method_cache::rename_method(vm, is_class, target, args),
        "constructor" => def_constructor(vm, is_class, target, args),
        "destructor" => def_destructor(vm, is_class, target, args),
        "superclass" => def_superclass(vm, is_class, target, args),
        "export" => def_export(vm, is_class, target, args, true),
        "unexport" => def_export(vm, is_class, target, args, false),
        "mixin" => def_mixin(vm, is_class, target, args),
        "filter" => def_filter(vm, is_class, target, args),
        "forward" => def_forward(vm, is_class, target, args),
        "variable" => declare_variable(vm, args, vm.oo.private_depth > 0),
        _ => err(format!("unknown definition command \"{verb}\"")),
    }
}

/// Retain a method/constructor declaration with the native definition owner.
pub(super) fn build_method(
    vm: &mut Vm,
    params_spec: &Value,
    body: &Value,
    exported: bool,
) -> Result<Method, Completion<Value>> {
    let body_src = vm
        .capture_procedure_declaration_body(params_spec, body)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let (params, has_args) = parse_params_value(vm, params_spec, b"")?;
    Ok(Method {
        parameters: MethodParameters {
            values: params,
            has_args,
        },
        compiled_body: None,
        body_src,
        forward: None,
        property: None,
        intrinsic: None,
        visibility_only: false,
        exported,
        private: false,
    })
}

pub(super) fn def_method(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    original_name_operands::run(vm, Some((is_class, target)), |vm, original_names| {
        use tcl_registry::native_tcloo_method_definition::{
            NativeTclooMethodDefinitionProtocol, NativeTclooMethodVisibility,
        };
        let Some(protocol) =
            NativeTclooMethodDefinitionProtocol::select(vm.actual_native_invocation_dialect())
        else {
            return vm
                .refuse_host_command("native method declaration policy is unavailable".into());
        };
        let Some(layout) = protocol.layout(args.len()) else {
            return definition_wrong_args(vm, protocol.usage());
        };
        let explicit = if let Some(index) = layout.option {
            let selected = match vm.native_static_option_index_checked(
                &args[index],
                protocol.export_modes().unwrap(),
                false,
                "export flag",
            ) {
                Ok(selected) => selected,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            protocol.option_visibility(selected)
        } else {
            None
        };
        let name =
            match native_method_key_for_target(vm, &args[layout.name], Some((is_class, target))) {
                Ok(name) => name,
                Err(completion) => return completion,
            };
        let visibility = explicit.unwrap_or_else(|| {
            protocol.default_visibility(vm.oo.method_name_bytes(&name), vm.oo.private_depth > 0)
        });
        let mut method = match build_method(
            vm,
            &args[layout.parameters],
            &args[layout.body],
            visibility == NativeTclooMethodVisibility::Public,
        ) {
            Ok(m) => m,
            Err(e) => return e,
        };
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        method.private = visibility == NativeTclooMethodVisibility::Private;
        // A fresh definition carries its own visibility, superseding an earlier
        // `export`/`unexport` of the name here (C replaces the method record).
        let retired = if is_class {
            vm.oo.classes.get_mut(&target).and_then(|entry| {
                entry.exported.remove(&name);
                entry.unexported.remove(&name);
                entry.methods.insert_for_retirement(name, method)
            })
        } else {
            vm.oo.objects.get_mut(&target).and_then(|entry| {
                entry.exported.remove(&name);
                entry.unexported.remove(&name);
                entry.methods.insert_for_retirement(name, method)
            })
        };
        drop(retired);
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Err(error) = vm.native_property_method_created(target, is_class) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}

pub(super) fn def_constructor(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    original_name_operands::run(vm, Some((is_class, target)), |vm, original_names| {
        if !is_class {
            return err("constructors are only for classes");
        }
        let [params, body] = args else {
            return definition_wrong_args(vm, "constructor arguments body");
        };
        // An empty counted body installs *no* constructor — C TclOO's
        // optimisation, so `C new <extra args>` is then silently accepted.
        let body_bytes = match original_names.read(vm, body) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        if body_bytes.is_empty() {
            let retired = vm
                .oo
                .classes
                .get_mut(&target)
                .and_then(|entry| entry.constructor.take());
            let changed = retired.is_some();
            drop(retired);
            if let Err(error) = original_names.check_target(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            if changed && let Err(error) = vm.native_property_structure_changed(target, true) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            return ok(Value::empty());
        }
        let method = match build_method(vm, params, body, true) {
            Ok(m) => m,
            Err(e) => return e,
        };
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let retired = vm
            .oo
            .classes
            .get_mut(&target)
            .and_then(|entry| entry.constructor.install_for_retirement(method));
        drop(retired);
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Err(error) = vm.native_property_method_created(target, true) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Err(error) = vm.native_property_structure_changed(target, true) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}

pub(super) fn def_destructor(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    original_name_operands::run(vm, Some((is_class, target)), |vm, original_names| {
        if !is_class {
            return err("destructors are only for classes");
        }
        let [body] = args else {
            return definition_wrong_args(vm, "destructor body");
        };
        let body_bytes = match original_names.read(vm, body) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        if body_bytes.is_empty() {
            let retired = vm
                .oo
                .classes
                .get_mut(&target)
                .and_then(|entry| entry.destructor.take());
            let changed = retired.is_some();
            drop(retired);
            if let Err(error) = original_names.check_target(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            if changed && let Err(error) = vm.native_property_structure_changed(target, true) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            return ok(Value::empty());
        }
        let method = match build_method(vm, &Value::empty(), body, true) {
            Ok(m) => m,
            Err(e) => return e,
        };
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let retired = vm
            .oo
            .classes
            .get_mut(&target)
            .and_then(|entry| entry.destructor.install_for_retirement(method));
        drop(retired);
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Err(error) = vm.native_property_method_created(target, true) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Err(error) = vm.native_property_structure_changed(target, true) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}

fn def_superclass(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    original_name_operands::run(vm, Some((is_class, target)), |vm, original_names| {
        if !is_class {
            return err("superclass is only for classes");
        }
        let mut selected = Vec::new();
        let mut facets = Vec::new();
        for argument in args {
            let Some(selected_class) = (match resolve_class_value(vm, argument) {
                Ok(class) => class,
                Err(completion) => return completion,
            }) else {
                return err("only a class can be a superclass");
            };
            if let Err(error) = original_names.check_target(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            let facet = vm
                .oo
                .classes
                .get(&selected_class)
                .map(|entry| Rc::downgrade(&entry.birth));
            if let Err(error) = original_names.require_current(
                vm,
                facet.is_some(),
                "original OO relation class facet",
            ) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            facets.push((selected_class, facet.unwrap()));
            if selected_class == target {
                return err("attempt to form circular dependency graph");
            }
            selected.push(selected_class);
        }
        let current = facets.iter().all(|(id, original)| {
            original.upgrade().is_some_and(|birth| {
                vm.oo
                    .classes
                    .get(id)
                    .is_some_and(|entry| Rc::ptr_eq(&birth, &entry.birth))
            })
        });
        if let Err(error) =
            original_names.require_current(vm, current, "original OO relation class facet")
        {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Some(entry) = vm.oo.classes.get_mut(&target) {
            entry.supers = selected;
        }
        if let Err(error) = vm.native_property_structure_changed(target, is_class) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}

fn def_export(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    args: &[Value],
    export: bool,
) -> Completion<Value> {
    original_name_operands::run(vm, Some((is_class, target)), |vm, original_names| {
        let mut changed = false;
        if !is_class && !args.is_empty() {
            vm.oo.native_methods.instance_table_created(target);
        }
        for a in args {
            let name = match native_method_key_for_target(vm, a, Some((is_class, target))) {
                Ok(name) => name,
                Err(completion) => return completion,
            };
            if let Err(error) = original_names.check_target(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            if is_class {
                if let Some(c) = vm.oo.classes.get_mut(&target) {
                    if let Some(method) = c.methods.owner(&name) {
                        changed |= method.borrow().private;
                        method.borrow_mut().private = false;
                    }
                    if export {
                        changed |= c.unexported.remove(&name);
                        changed |= c.exported.insert(name);
                    } else {
                        changed |= c.exported.remove(&name);
                        changed |= c.unexported.insert(name);
                    }
                }
            } else if let Some(o) = vm.oo.objects.get_mut(&target) {
                o.methods.mark_allocated();
                if let Some(method) = o.methods.owner(&name) {
                    changed |= method.borrow().private;
                    method.borrow_mut().private = false;
                }
                if export {
                    changed |= o.unexported.remove(&name);
                    changed |= o.exported.insert(name);
                } else {
                    changed |= o.exported.remove(&name);
                    changed |= o.unexported.insert(name);
                }
            }
        }
        if changed && let Err(error) = vm.native_property_structure_changed(target, is_class) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}

fn def_mixin(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    original_name_operands::run(vm, Some((is_class, target)), |vm, original_names| {
        let mut selected = Vec::new();
        let mut facets = Vec::new();
        for argument in args {
            let Some(selected_class) = (match resolve_class_value(vm, argument) {
                Ok(class) => class,
                Err(completion) => return completion,
            }) else {
                return err("may only mix in classes");
            };
            if let Err(error) = original_names.check_target(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            let facet = vm
                .oo
                .classes
                .get(&selected_class)
                .map(|entry| Rc::downgrade(&entry.birth));
            if let Err(error) = original_names.require_current(
                vm,
                facet.is_some(),
                "original OO relation class facet",
            ) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            facets.push((selected_class, facet.unwrap()));
            selected.push(selected_class);
        }
        let current = facets.iter().all(|(id, original)| {
            original.upgrade().is_some_and(|birth| {
                vm.oo
                    .classes
                    .get(id)
                    .is_some_and(|entry| Rc::ptr_eq(&birth, &entry.birth))
            })
        });
        if let Err(error) =
            original_names.require_current(vm, current, "original OO relation class facet")
        {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if is_class {
            if let Some(entry) = vm.oo.classes.get_mut(&target) {
                entry.mixins = selected;
            }
        } else {
            if let Some(entry) = vm.oo.objects.get_mut(&target) {
                entry.mixins = selected;
            }
            vm.oo.recompute_native_method_class_cache(target);
        }
        if let Err(error) = vm.native_property_structure_changed(target, is_class) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}

/// The `filter` slot: methods wrapping every call. Bare names append (C's
/// default); `-set`, `-append`, `-remove` and `-clear` select the operation.
/// Duplicates are stored as given; dispatch collapses them.
fn def_filter(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    let (op, names) = match args.split_first() {
        Some((first, rest)) if first.to_str().starts_with('-') => {
            (first.to_str().to_string(), rest)
        }
        _ => ("-append".to_owned(), args),
    };
    let mut selected = Vec::with_capacity(names.len());
    for name in names {
        let key = match native_method_key_for_target(vm, name, Some((is_class, target))) {
            Ok(key) => key,
            Err(completion) => return completion,
        };
        selected.push(tcl_core_types::NameBytes::from(
            vm.oo.method_name_bytes(&key),
        ));
    }
    let list = if is_class {
        vm.oo.classes.get_mut(&target).map(|c| &mut c.filters)
    } else {
        vm.oo.objects.get_mut(&target).map(|o| &mut o.filters)
    };
    let Some(list) = list else {
        return ok(Value::empty());
    };
    match op.as_str() {
        "-set" => *list = selected,
        "-append" => list.extend(selected),
        "-remove" => list.retain(|n| !selected.contains(n)),
        "-clear" => list.clear(),
        other => return err(format!("unsupported slot operation \"{other}\"")),
    }
    ok(Value::empty())
}

fn def_forward(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    original_name_operands::run(vm, Some((is_class, target)), |vm, original_names| {
        use tcl_registry::native_tcloo_method_definition::{
            NativeTclooMethodDefinitionProtocol, NativeTclooMethodVisibility,
        };
        let Some((name, prefix)) = args.split_first().filter(|(_, prefix)| !prefix.is_empty())
        else {
            return definition_wrong_args(vm, "forward name cmdName ?arg ...?");
        };
        let name = match native_method_key_for_target(vm, name, Some((is_class, target))) {
            Ok(name) => name,
            Err(completion) => return completion,
        };
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let Some(protocol) =
            NativeTclooMethodDefinitionProtocol::select(vm.actual_native_invocation_dialect())
        else {
            return vm
                .refuse_host_command("native forward declaration policy is unavailable".into());
        };
        let visibility =
            protocol.default_visibility(vm.oo.method_name_bytes(&name), vm.oo.private_depth > 0);
        let method = Method {
            parameters: MethodParameters {
                values: Vec::new(),
                has_args: true,
            },
            compiled_body: None,
            body_src: Value::empty(),
            forward: Some(prefix.to_vec()),
            property: None,
            intrinsic: None,
            visibility_only: false,
            exported: visibility == NativeTclooMethodVisibility::Public,
            private: visibility == NativeTclooMethodVisibility::Private,
        };
        let retired = if is_class {
            vm.oo
                .classes
                .get_mut(&target)
                .and_then(|entry| entry.methods.insert_for_retirement(name, method))
        } else {
            vm.oo
                .objects
                .get_mut(&target)
                .and_then(|entry| entry.methods.insert_for_retirement(name, method))
        };
        drop(retired);
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Err(error) = vm.native_property_method_created(target, is_class) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}
