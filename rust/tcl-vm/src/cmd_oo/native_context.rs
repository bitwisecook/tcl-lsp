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

//! Actual TclOO variable-frame and object-name ownership.

use super::*;

pub(super) fn install_object_helpers(vm: &mut Vm, namespace: &str) -> NsId {
    let token = vm.definition_namespace_token(namespace);
    let helpers = vm.definition_namespace_token("oo::Helpers");
    let name = vm.namespace_object_bytes(token);
    vm.push_ns_token(name, token);
    vm.ns_path_set(vec![helpers]);
    vm.pop_ns();
    token
}

pub(super) fn outer_namespace(vm: &Vm) -> NsId {
    let mut activation = vm.native_oo_variable_activation();
    let mut namespace = vm.current_ns_id();
    while let Some((_, _, caller, caller_activation)) = vm
        .oo
        .def_stack
        .iter()
        .rev()
        .find(|(_, active, _, _)| *active == activation)
    {
        namespace = *caller;
        activation = *caller_activation;
    }
    namespace
}

pub(super) fn definition_index(vm: &Vm) -> Option<usize> {
    let activation = vm.native_oo_variable_activation();
    vm.oo
        .def_stack
        .iter()
        .rposition(|(_, active, _, _)| *active == activation)
}

pub(super) fn current_method(vm: &Vm) -> Option<&OoFrame> {
    let activation = vm.native_oo_variable_activation();
    vm.oo
        .call_stack
        .iter()
        .rev()
        .find(|frame| frame.activation == Some(activation))
}

pub(super) fn object_name(vm: &Vm, object: OoId) -> Value {
    let owner = vm
        .oo
        .objects
        .get(&object)
        .map(|owner| &owner.cached_name)
        .or_else(|| {
            vm.oo
                .call_stack
                .iter()
                .rev()
                .find(|frame| frame.object == object)
                .map(|frame| &frame.name_owner)
        })
        .expect("object-name getter retains the actual OO owner");
    let mut cached = owner.borrow_mut();
    cached
        .get_or_insert_with(|| {
            if vm.oo.objects.contains_key(&object) {
                let bytes = display_oo_bytes(vm, object);
                let dialect = vm.actual_native_invocation_dialect();
                if let Some(protocol) = dialect
                    .native_string_protocol()
                    .filter(|protocol| protocol.tcl_version().is_some())
                {
                    Value::from_native_string_cache(
                        tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
                            protocol,
                            num_chars: None,
                            unicode: None,
                        },
                        dialect,
                        Some((
                            bytes.into(),
                            tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                        )),
                    )
                    .expect("actual C Tcl_GetCommandFullName append has valid String backing")
                } else {
                    Value::from_native_string_bytes(bytes)
                }
            } else {
                Value::empty()
            }
        })
        .clone()
}

pub(super) fn enter_definition(
    vm: &mut Vm,
    class: bool,
    argv: Vec<Value>,
) -> Result<(), Completion<Value>> {
    let name = if class {
        "::oo::define"
    } else {
        "::oo::objdefine"
    };
    let namespace = match tcl_runtime_api::Namespaces::find_namespace_bytes_checked(
        vm,
        vm.current_ns_id(),
        name.as_bytes(),
    ) {
        Ok(namespace) => namespace,
        Err(error) => return Err(vm.refuse_host_command(error.to_string())),
    };
    let Some(namespace) = namespace else {
        return Err(err("cannot process definitions; support namespace deleted"));
    };
    vm.push_ns_eval_token_frame(namespace, argv);
    Ok(())
}

pub(super) fn leave_definition(vm: &mut Vm) {
    vm.pop_call_frame();
    vm.pop_ns();
}

pub(super) fn unknown_definition(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if active_target(vm).is_none() {
        return err_code(
            "this command may only be called from within the context of an ::oo::define or ::oo::objdefine command",
            &["TCL", "OO", "MONKEY_BUSINESS"],
        );
    }
    let Some((requested, operands)) = args.split_first() else {
        return err("bad call of unknown handler");
    };
    let bytes = match vm.native_name_operand_bytes(requested) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let head = match definition_head(vm, bytes.as_ref()) {
        Some(head) => head,
        None => {
            let mut message = b"invalid command name \"".to_vec();
            message.extend_from_slice(bytes.as_ref());
            message.push(b'"');
            return err(message);
        }
    };
    vm.invoke_command_value_at(
        vm.current_ns_id(),
        &head,
        operands,
        &[],
        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
    )
}

fn definition_head(vm: &Vm, requested: &[u8]) -> Option<Value> {
    if requested.is_empty() || requested.windows(2).any(|pair| pair == b"::") {
        return None;
    }
    let names = vm.native_oo_definition_command_names();
    let exact = names.iter().find(|name| name.as_bytes() == requested);
    let selected = if let Some(exact) = exact {
        exact
    } else {
        let mut candidates = names
            .iter()
            .filter(|name| name.as_bytes().starts_with(requested));
        match (candidates.next(), candidates.next()) {
            (Some(name), None) => name,
            _ => return None,
        }
    };
    let mut full = b"::".to_vec();
    full.extend_from_slice(vm.namespace_object_bytes(vm.current_ns_id()).as_bytes());
    if !full.ends_with(b"::") {
        full.extend_from_slice(b"::");
    }
    full.extend_from_slice(selected.as_bytes());
    Some(Value::from_native_string_bytes(full))
}

pub(super) fn magic_definition_invoke(
    vm: &mut Vm,
    requested: &Value,
    operands: &[Value],
) -> Completion<Value> {
    let bytes = match vm.native_name_operand_bytes(requested) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let head = definition_head(vm, bytes.as_ref()).unwrap_or_else(|| requested.clone());
    vm.invoke_command_value_at(
        vm.current_ns_id(),
        &head,
        operands,
        &[],
        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
    )
}

pub(super) fn register_definition_workers(vm: &mut Vm) {
    use tcl_registry::ObjectDispatchLayer;
    let dialect = vm.actual_native_invocation_dialect();
    let workers: &[(&str, crate::command::BuiltinFn, crate::command::BuiltinFn)] = &[
        (
            "method",
            |v, a| definition_worker(v, a, true, "method"),
            |v, a| definition_worker(v, a, false, "method"),
        ),
        (
            "deletemethod",
            |v, a| definition_worker(v, a, true, "deletemethod"),
            |v, a| definition_worker(v, a, false, "deletemethod"),
        ),
        (
            "renamemethod",
            |v, a| definition_worker(v, a, true, "renamemethod"),
            |v, a| definition_worker(v, a, false, "renamemethod"),
        ),
        (
            "constructor",
            |v, a| definition_worker(v, a, true, "constructor"),
            |v, a| definition_worker(v, a, false, "constructor"),
        ),
        (
            "destructor",
            |v, a| definition_worker(v, a, true, "destructor"),
            |v, a| definition_worker(v, a, false, "destructor"),
        ),
        (
            "superclass",
            |v, a| definition_worker(v, a, true, "superclass"),
            |v, a| definition_worker(v, a, false, "superclass"),
        ),
        (
            "export",
            |v, a| definition_worker(v, a, true, "export"),
            |v, a| definition_worker(v, a, false, "export"),
        ),
        (
            "unexport",
            |v, a| definition_worker(v, a, true, "unexport"),
            |v, a| definition_worker(v, a, false, "unexport"),
        ),
        (
            "mixin",
            |v, a| definition_worker(v, a, true, "mixin"),
            |v, a| definition_worker(v, a, false, "mixin"),
        ),
        (
            "forward",
            |v, a| definition_worker(v, a, true, "forward"),
            |v, a| definition_worker(v, a, false, "forward"),
        ),
        (
            "variable",
            |v, a| definition_worker(v, a, true, "variable"),
            |v, a| definition_worker(v, a, false, "variable"),
        ),
        (
            "self",
            |v, a| definition_self_at(v, a, true),
            |v, a| definition_self_at(v, a, false),
        ),
    ];
    for layer in [ObjectDispatchLayer::Class, ObjectDispatchLayer::Object] {
        for &(member, class_handler, object_handler) in workers {
            if let Some(identity) =
                tcl_registry::native_tcloo_registration::definition_identity(member, layer, dialect)
            {
                vm.register_stock_builtin(
                    identity,
                    if layer == ObjectDispatchLayer::Class {
                        class_handler
                    } else {
                        object_handler
                    },
                );
            }
        }
    }
    if tcl_registry::native_tcloo_registration::compilation("oo::UnknownDefinition", dialect)
        .is_some()
    {
        vm.register_stock_builtin("oo::UnknownDefinition", unknown_definition);
        vm.install_native_oo_unknown(Value::string("::oo::UnknownDefinition"));
    }
}

fn definition_worker(vm: &mut Vm, args: &[Value], class: bool, member: &str) -> Completion<Value> {
    let Some((_, object)) = active_target(vm) else {
        return def_body_cmd(vm, member, args);
    };
    if class && !vm.oo.classes.contains_key(&object) {
        return err_code("attempt to misuse API", &["TCL", "OO", "MONKEY_BUSINESS"]);
    }
    let index = definition_index(vm).expect("selected definition activation");
    let original = vm.oo.def_stack[index].0;
    vm.oo.def_stack[index].0 = if class {
        DefTarget::Class(object)
    } else {
        DefTarget::Object(object)
    };
    let result = def_body_cmd(vm, member, args);
    vm.oo.def_stack[index].0 = original;
    result
}

fn definition_self_at(vm: &mut Vm, args: &[Value], class: bool) -> Completion<Value> {
    let Some((_, object)) = active_target(vm) else {
        return definition_self(vm, args);
    };
    if class && !vm.oo.classes.contains_key(&object) {
        return err_code("attempt to misuse API", &["TCL", "OO", "MONKEY_BUSINESS"]);
    }
    let index = definition_index(vm).expect("selected definition activation");
    let original = vm.oo.def_stack[index].0;
    vm.oo.def_stack[index].0 = if class {
        DefTarget::Class(object)
    } else {
        DefTarget::Object(object)
    };
    let result = definition_self(vm, args);
    vm.oo.def_stack[index].0 = original;
    result
}

fn definition_self(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((class, target)) = active_target(vm) else {
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
    if !class {
        return if args.is_empty() {
            ok(object_name(vm, target))
        } else {
            crate::command::native_wrong_args(vm, "self")
        };
    }
    if args.is_empty() {
        return crate::command::native_wrong_args(vm, "self arg ?arg ...?");
    }
    let mut incoming = vec![object_name(vm, target)];
    incoming.extend_from_slice(args);
    run_define(vm, &incoming, false)
}

pub(crate) fn helper_context_error(vm: &mut Vm, head: &[u8]) -> Completion<Value> {
    let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
    else {
        return vm.refuse_host_command("native method context error protocol unavailable".into());
    };
    let mut message = head.to_vec();
    message.extend_from_slice(b" may only be called from inside a method");
    let code = Value::native_list_constructor(
        ["TCL", "OO", "CONTEXT_REQUIRED"]
            .into_iter()
            .map(Value::string)
            .collect(),
        protocol,
    );
    let options = Value::native_list_constructor(vec![Value::string("-errorcode"), code], protocol);
    Completion::new_error_metadata(
        Code::Error,
        Value::from_native_string_bytes(message),
        options,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm(version: tcl_dialect::TclVersion) -> Vm {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm
    }

    #[test]
    fn helper_cpp_executes_only_the_actual_method_frame_and_original_words() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result=vm.eval_source(r"oo::class create B {method m args {list BASE {*}$args}; method by args {list BASE {*}$args}; method static args {list STATIC {*}$args}}
oo::class create C {
 superclass B
 method m args {next {*}$args}
 method by args {nextto B {*}$args}
 method static {} {nextto {*}{B ONE}}
 method object {} {self o}
 method space {} {self n}
 method nested {} {ordinary}
}
proc ordinary {} {::oo::Helpers::self}
C create obj
list [obj m A B] [obj by C D] [obj static] [obj object] [string match ::oo::Obj* [obj space]] [catch {obj nested}] [llength [info commands ::self]]
").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str().as_ref()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "{BASE A B} {BASE C D} {STATIC ONE} ::obj 1 1 0",
                "{version:?}"
            );
            let nested = vm.eval_source("obj nested").unwrap();
            assert_eq!(nested.code, Code::Error);
            assert_eq!(
                nested.options.as_list().unwrap()[1].to_str().as_ref(),
                "TCL OO CONTEXT_REQUIRED"
            );
            let evaluated = vm
                .eval_source(
                    "oo::define C method evalContext {} {my eval {self object}}; obj evalContext",
                )
                .unwrap();
            assert_eq!(evaluated.code, Code::Ok);
            assert_eq!(evaluated.result.to_str().as_ref(), "::obj");
        }
    }

    #[test]
    fn definition_namespace_and_variable_activation_are_original() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result = vm.eval_source("proc ::method args {error GLOBAL_WORKER}; namespace eval caller {proc build {} {set private CALLER; oo::class create ::C {set scope [namespace current]; set inherited [info exists private]; set local [uplevel 1 {set private}]; proc born {} {return HERE}; meth m {} {return OK}}}}; caller::build; C create obj; list [obj m] $::oo::define::scope $::oo::define::inherited $::oo::define::local [::oo::define::born]").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str().as_ref()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "OK ::oo::define 0 CALLER HERE"
            );
            let outside = vm.invoke_command(
                "::oo::define::method",
                &[Value::string("m"), Value::empty(), Value::empty()],
            );
            assert_eq!(outside.code, Code::Error);
            assert_eq!(
                outside.options.as_list().unwrap()[1].to_str().as_ref(),
                "TCL OO MONKEY_BUSINESS"
            );
            assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
        }
    }

    #[test]
    fn nested_definitions_retain_actual_caller_frames_and_qualified_workers() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result = vm.eval_source(r"namespace eval ::outer {
 oo::class create B {}
 oo::class create C {}
 oo::class create D {}
 proc build {} {
  oo::define C {oo::define ::outer::D {superclass B; uplevel 1 {::oo::define::method resumed {} {return OUTER}}}}
 }
 build
}
::outer::C create obj
set chained [list [info class superclasses ::outer::D] [obj resumed]]
oo::class create ::oo::define::B {}
proc ::oo::define::bridge {} {oo::define ::outer::D {superclass B}}
oo::define ::outer::C {bridge}
set ordinary [info class superclasses ::outer::D]
proc ::inspect args {error GLOBAL}
proc ::oo::define::inspect args {list [lindex [info level 0] 0] {*}$args}
set magic [list [oo::define ::outer::C insp FIRST SECOND] [oo::define ::outer::C inspect THIRD]]
set outside [catch {::oo::define::method outside {} {}} result options]
list $chained $ordinary $magic [namespace current] $outside [dict get $options -errorcode]").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str().as_ref()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "{::outer::B OUTER} ::oo::define::B {{::oo::define::inspect FIRST SECOND} {::oo::define::inspect THIRD}} :: 1 {TCL OO MONKEY_BUSINESS}"
            );
            assert!(vm.oo.def_stack.is_empty());
        }
    }

    #[test]
    fn cached_object_name_retains_the_original_header_until_rename() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            assert_eq!(vm.eval_source("oo::class create C {method object {} {self object}; method nested {} {p}; method shifted {} {uplevel #0 {::oo::Helpers::self}}}; proc p {} {::oo::Helpers::self}; C create obj").unwrap().code, Code::Ok);
            let first = vm.invoke_command("obj", &[Value::string("object")]);
            assert_eq!(first.code, Code::Ok);
            // The native observer retains Tcl_GetObjResult across later calls.
            // Completion itself supplies only a lifetime view of that role.
            let first = first.result.into_native_reference();
            first.as_list().unwrap();
            let second = vm.invoke_command("obj", &[Value::string("object")]);
            assert_eq!(second.code, Code::Ok);
            assert_eq!(
                first.native_object_identity(),
                second.result.native_object_identity()
            );
            assert_eq!(
                vm.invoke_command("obj", &[Value::string("nested")]).code,
                Code::Error
            );
            assert_eq!(
                vm.invoke_command("obj", &[Value::string("shifted")]).code,
                Code::Error
            );
            assert_eq!(
                vm.invoke_command("rename", &[Value::string("obj"), Value::string("renamed")])
                    .code,
                Code::Ok
            );
            let renamed = vm.invoke_command("renamed", &[Value::string("object")]);
            assert_eq!(renamed.code, Code::Ok);
            let renamed = renamed.result.into_native_reference();
            assert_ne!(
                first.native_object_identity(),
                renamed.native_object_identity()
            );
            let deleted = vm
                .eval_source(
                    "oo::define C method deleted {} {rename [self] {}; self}; renamed deleted",
                )
                .unwrap();
            assert_eq!(deleted.code, Code::Ok);
            assert_eq!(deleted.result.to_str().as_ref(), "");
            assert_eq!(first.to_str().as_ref(), "::obj");
            assert_eq!(renamed.to_str().as_ref(), "::renamed");
        }
    }

    #[test]
    fn uplevel_selection_restores_original_namespace_for_source_and_object_bodies() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result = vm.eval_source("namespace eval ::caller {proc parent {} {set private CALLER; set got [namespace eval ::child {set fromScript [uplevel 1 {set private}]; set fromList [uplevel 1 [list set private]]; list $fromScript $fromList [namespace current]}]; list $got [namespace current]}; parent}").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "{CALLER CALLER ::child} ::caller"
            );
            assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
        }
    }
}
