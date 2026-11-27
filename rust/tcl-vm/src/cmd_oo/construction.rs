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

//! Creating objects and classes: the `create`/`new` factory, the constructor chain and `oo::class create`.

use super::definition::definition_script_completion;
use super::destruction::{OriginalConstructedRecord, finish_failed_constructor};
use super::method_invocation::{original_call_words, run_step};
use super::{
    Class, Object, Step, display_oo_bytes, native_context, native_method_cache,
    original_name_operands,
};
use crate::command::Command;
use crate::interp::{CommandSidecarKey, Vm, err, ok};
use crate::value::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use tcl_registry::native_tcloo_method_definition::NativeTclooMethodReportRole;
use tcl_runtime_api::{Code, Completion, OoId};

// Object / class creation.

/// The `create objectName ?arg…?` / `new ?arg…?` factory built-ins of a class.
/// `class_invoked` is the class as the caller named it (for the constructor's
/// `wrong # args` usage).
pub(super) fn factory(
    vm: &mut Vm,
    class_key: OoId,
    class_invoked: &Value,
    original_factory: &Value,
    anon: bool,
    args: &[Value],
) -> Completion<Value> {
    original_name_operands::run(vm, Some((true, class_key)), |vm, original_names| {
        // Instantiating a metaclass (a class whose MRO includes `::oo::class`)
        // builds a *class*, and its trailing arg is a definition script, not
        // constructor arguments.
        let is_meta = vm
            .oo
            .class_linear_of(class_key)
            .iter()
            .any(|s| Some(s.provider) == vm.oo.class_root);
        if is_meta {
            let (name, body) = if anon {
                (fresh_obj_name(vm).into_bytes(), args.first())
            } else {
                let Some((n, rest)) = args.split_first() else {
                    let mut usage = match vm
                        .native_argument_usage_header(std::slice::from_ref(class_invoked))
                    {
                        Ok(header) => header,
                        Err(completion) => return completion,
                    };
                    usage.extend_from_slice(b" create className ?definitionScript?");
                    return crate::command::native_wrong_args_bytes(vm, &usage);
                };
                let n = match original_names.read(vm, n) {
                    Ok(bytes) => bytes,
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                };
                if n.is_empty() {
                    return err("object name must not be empty");
                }
                (n.to_vec(), rest.first())
            };
            return make_class_bytes(vm, &name, body, class_key);
        }

        let (obj_key, obj_ns, ctor_args, ctor_usage, ctor_identity) = if anon {
            // An anonymous object's command *is* its instance namespace (`oo::ObjN`).
            let n = fresh_obj_name(vm);
            let identity =
                original_call_words([class_invoked, original_factory].into_iter().chain(args));
            (
                n.as_bytes().to_vec(),
                n,
                args,
                vec![class_invoked.clone(), original_factory.clone()],
                identity,
            )
        } else {
            let Some((name, rest)) = args.split_first() else {
                let mut usage =
                    match vm.native_argument_usage_header(std::slice::from_ref(class_invoked)) {
                        Ok(header) => header,
                        Err(completion) => return completion,
                    };
                usage.extend_from_slice(b" create objectName ?arg ...?");
                return crate::command::native_wrong_args_bytes(vm, &usage);
            };
            let name_bytes = match original_names.read(vm, name) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            if name_bytes.is_empty() {
                return err("object name must not be empty");
            }
            // A named object's instance namespace is still a fresh `oo::ObjN`,
            // decoupled from its command name (C's `oo::Obj` counter).
            let identity = original_call_words(
                [class_invoked, original_factory, name]
                    .into_iter()
                    .chain(rest),
            );
            (
                name_bytes.to_vec(),
                fresh_obj_name(vm),
                rest,
                vec![
                    class_invoked.clone(),
                    original_factory.clone(),
                    name.clone(),
                ],
                identity,
            )
        };
        oo_new(
            vm,
            class_key,
            &obj_key,
            &obj_ns,
            ctor_args,
            &ctor_usage,
            ctor_identity,
        )
    })
}

/// A fresh `oo::ObjN` name (canonical) for an anonymous object.
pub(super) fn fresh_obj_name(vm: &mut Vm) -> String {
    let n = vm.oo.counter;
    vm.oo.counter += 1;
    format!("oo::Obj{n}")
}

/// Create object `obj_key` as an instance of `class_key`, run its constructor
/// chain, and register its command. On a constructor error the half-built
/// object is torn down and the error re-raised.
fn oo_new(
    vm: &mut Vm,
    class_key: OoId,
    obj_key: &[u8],
    obj_ns: &str,
    ctor_args: &[Value],
    ctor_usage: &[Value],
    ctor_identity: Vec<Value>,
) -> Completion<Value> {
    let slot = match vm.native_object_publication_slot(obj_key) {
        Ok(selected) => selected,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    match vm.command_at_exact_slot_checked(&slot, true) {
        Ok(Some(_)) => {
            let message = vm
                .name_policy_protocol()
                .and_then(|policy| policy.recipe().oo_object_collision_message(obj_key).ok());
            return match message {
                Some(message) => err(message),
                None => vm.refuse_host_command("TclOO object collision naming issuer".into()),
            };
        }
        Ok(None) => {}
        Err(error) => return vm.refuse_host_command(error.to_string()),
    }
    let object_display = tcl_core_types::NameBytes::from(vm.command_slot_display_bytes(&slot));
    let object_id = vm.oo.allocate(object_display);
    let ns = obj_ns.to_string();
    vm.declare_namespace(&tcl_syntax::naming::root_unrooted_key(&ns));
    let namespace_token = native_context::install_object_helpers(vm, &ns);
    vm.oo.objects.insert(
        object_id,
        Object {
            cached_name: Rc::new(RefCell::new(None)),
            class: class_key,
            ns: ns.clone(),
            namespace_token: Some(namespace_token),
            creation_id: object_id.0,
            methods: native_method_cache::MethodTable::default(),
            variables: Vec::new(),
            private_variables: Vec::new(),
            mixins: Vec::new(),
            filters: Vec::new(),
            exported: BTreeSet::new(),
            unexported: BTreeSet::new(),
            readable_properties: BTreeSet::new(),
            writable_properties: BTreeSet::new(),
            destroyed: false,
        },
    );
    // A class instantiated as an object is itself a class only when it is a
    // metaclass (its MRO includes `::oo::class`); v1 handles the direct
    // `oo::class create` case in `def_body`'s class creation, not here.
    let command_key = vm.register_command_in_slot(slot, Command::Object(object_id));
    vm.oo
        .command_keys
        .insert(object_id, CommandSidecarKey::visible(command_key));

    // Constructor chain: classes in the linearisation with a constructor,
    // most-derived first.
    let chain: Vec<Step> = vm
        .oo
        .class_linear_of(class_key)
        .into_iter()
        .filter(|s| {
            vm.oo
                .classes
                .get(&s.provider)
                .is_some_and(|c| c.constructor.is_some())
        })
        .collect();
    let chain: native_method_cache::ReachedMethodChain = native_method_cache::retain_owners(
        &vm.oo,
        chain,
        "",
        NativeTclooMethodReportRole::Constructor,
    )
    .into();
    let chain = chain.with_report_role(NativeTclooMethodReportRole::Constructor);
    let construction = crate::interp::oo_cleanup::OriginalOoCleanup::enter(vm);
    let original_record = OriginalConstructedRecord::capture(&vm.oo, object_id);
    if !chain.is_empty() {
        let res = run_step(
            vm,
            object_id,
            &chain,
            0,
            String::new(),
            ctor_args,
            false,
            Value::from_native_string_bytes(display_oo_bytes(vm, object_id)),
            ctor_usage.to_vec(),
            ctor_identity,
        );
        if let Err(error) = construction.check(vm) {
            crate::interp::oo_cleanup::retain_error(vm, &error);
        }
        drop(chain);
        if (!res.code.is_ok() && res.code != Code::Return) || construction.check(vm).is_err() {
            return finish_failed_constructor(vm, object_id, &original_record, &construction, res);
        }
    }
    let result = ok(native_context::object_name(vm, object_id));
    construction.finish_completion(vm, result)
}

/// Create a class named `class_key` (canonical), optionally running `body` as
/// its definition script. `metaclass` is the class being instantiated to build
/// it (`oo::class`, `oo::configurable`, or a user metaclass): it becomes the new
/// class's object-facet class, and — when it is `oo::configurable` or descends
/// from it — marks the new class `configurable` (TIP 558).
pub(super) fn make_class_bytes(
    vm: &mut Vm,
    class_key: &[u8],
    body: Option<&Value>,
    metaclass: OoId,
) -> Completion<Value> {
    let slot = match vm.native_object_publication_slot(class_key) {
        Ok(selected) => selected,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    match vm.command_at_exact_slot_checked(&slot, true) {
        Ok(Some(_)) => {
            let message = vm
                .name_policy_protocol()
                .and_then(|policy| policy.recipe().oo_object_collision_message(class_key).ok());
            return match message {
                Some(message) => err(message),
                None => vm.refuse_host_command("TclOO object collision naming issuer".into()),
            };
        }
        Ok(None) => {}
        Err(error) => return vm.refuse_host_command(error.to_string()),
    }
    // A class is configurable when instantiated *from* `oo::configurable` (or a
    // metaclass descending from it) — an identity test, distinct from the MRO
    // walk used for `configure` availability (which inherits down subclasses).
    let configurable = Some(metaclass) == vm.oo.configurable_root
        || vm
            .oo
            .class_linear_of(metaclass)
            .iter()
            .any(|s| Some(s.provider) == vm.oo.configurable_root);
    let class_display = tcl_core_types::NameBytes::from(vm.command_slot_display_bytes(&slot));
    let class_namespace = fresh_obj_name(vm);
    let class_id = vm.oo.allocate(class_display);
    let mixins = if configurable {
        vm.oo.configurable_support.into_iter().collect()
    } else {
        Vec::new()
    };
    let supers = vm.oo.object_root.into_iter().collect();
    vm.oo.classes.insert(
        class_id,
        Class {
            supers,
            mixins,
            configurable,
            ..Class::default()
        },
    );
    vm.oo.objects.insert(
        class_id,
        Object {
            cached_name: Rc::new(RefCell::new(None)),
            class: metaclass,
            ns: class_namespace.clone(),
            namespace_token: None,
            creation_id: class_id.0,
            methods: native_method_cache::MethodTable::default(),
            variables: Vec::new(),
            private_variables: Vec::new(),
            mixins: Vec::new(),
            filters: Vec::new(),
            exported: BTreeSet::new(),
            unexported: BTreeSet::new(),
            readable_properties: BTreeSet::new(),
            writable_properties: BTreeSet::new(),
            destroyed: false,
        },
    );
    vm.activate_namespace_written(&tcl_syntax::naming::root_unrooted_key(&class_namespace));
    let namespace_token = native_context::install_object_helpers(vm, &class_namespace);
    vm.oo.objects.get_mut(&class_id).unwrap().namespace_token = Some(namespace_token);
    let command_key = vm.register_command_in_slot(slot, Command::Object(class_id));
    vm.oo
        .command_keys
        .insert(class_id, CommandSidecarKey::visible(command_key));
    let construction = crate::interp::oo_cleanup::OriginalOoCleanup::enter(vm);
    let original_record = OriginalConstructedRecord::capture(&vm.oo, class_id);
    if let Some(body) = body {
        let res = evaluate_class_definition(vm, class_id, body);
        if (!res.code.is_ok() && res.code != Code::Return) || construction.check(vm).is_err() {
            return finish_failed_constructor(vm, class_id, &original_record, &construction, res);
        }
    }
    let result = ok(native_context::object_name(vm, class_id));
    construction.finish_completion(vm, result)
}

pub(super) fn evaluate_class_definition(
    vm: &mut Vm,
    class_id: OoId,
    body: &Value,
) -> Completion<Value> {
    original_name_operands::run(vm, Some((true, class_id)), |vm, original_names| {
        let argv = vec![
            vm.oo
                .define_name
                .as_ref()
                .expect("native OO Foundation define head")
                .clone(),
            native_context::object_name(vm, class_id),
            body.clone(),
        ];
        let definition = match native_context::enter_definition(vm, true, class_id, argv) {
            Ok(definition) => definition,
            Err(completion) => return completion,
        };
        let res = original_names.in_current_frame(vm, |vm, original_names| {
            let source = match original_names.read(vm, body) {
                Ok(bytes) => tcl_lexer::SourceImage::native(bytes.as_ref()),
                Err(error) => {
                    return crate::command::completion_from_cmd_error(vm, error);
                }
            };
            let reached = vm.eval_source_image_at_internal(&source, body.source_location());
            definition_script_completion(vm, &definition, reached)
        });
        definition.finish(vm, res)
    })
}
