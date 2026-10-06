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

//! Actual `TclOO` private info ensemble registrations and original operands.

use crate::interp::Vm;
use crate::value::Value;
use tcl_registry::commands::tcl::{InfoOoEnsembleKind, info_oo_subcommands};
use tcl_runtime_api::Completion;

pub(super) fn register(vm: &mut Vm) {
    let dialect = vm.actual_native_invocation_dialect();
    let Some(version) = dialect.tcl_version else {
        return;
    };
    for (member, identity, kind, workers) in [
        (
            "class",
            "oo::InfoClass",
            InfoOoEnsembleKind::Class,
            CLASS_WORKERS,
        ),
        (
            "object",
            "oo::InfoObject",
            InfoOoEnsembleKind::Object,
            OBJECT_WORKERS,
        ),
    ] {
        let Some(namespace) = dialect.info_oo_ensemble_namespace(member) else {
            continue;
        };
        if vm.command_kind(namespace).is_some()
            && vm.stock_native_identity(namespace).as_deref() != Some(identity)
        {
            continue;
        }
        let admitted = info_oo_subcommands(kind, version);
        vm.register_stock_namespace_ensemble(identity, namespace, workers, admitted.names());
    }
}

macro_rules! workers {
    ($table:ident, $handler:path, $($function:ident => $member:literal),+ $(,)?) => {
        const $table: &[(&str, crate::command::BuiltinFn)] = &[
            $(($member, $function)),+
        ];
        $(fn $function(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            let mut invocation = Vec::with_capacity(args.len() + 1);
            invocation.push(Value::string($member));
            invocation.extend_from_slice(args);
            $handler(vm, &invocation)
        })+
    };
}

workers! {
    CLASS_WORKERS, crate::cmd_oo::info_class,
    class_call => "call",
    class_constructor => "constructor",
    class_definition => "definition",
    class_definitionnamespace => "definitionnamespace",
    class_destructor => "destructor",
    class_filters => "filters",
    class_forward => "forward",
    class_instances => "instances",
    class_methods => "methods",
    class_methodtype => "methodtype",
    class_mixins => "mixins",
    class_properties => "properties",
    class_subclasses => "subclasses",
    class_superclasses => "superclasses",
    class_variables => "variables",
}

workers! {
    OBJECT_WORKERS, crate::cmd_oo::info_object,
    object_call => "call",
    object_class => "class",
    object_creationid => "creationid",
    object_definition => "definition",
    object_filters => "filters",
    object_forward => "forward",
    object_isa => "isa",
    object_methods => "methods",
    object_methodtype => "methodtype",
    object_mixins => "mixins",
    object_namespace => "namespace",
    object_properties => "properties",
    object_variables => "variables",
    object_vars => "vars",
}
#[cfg(test)]
mod tests {
    use super::*;
    use tcl_runtime_api::Code;

    #[test]
    fn specialized_object_info_keeps_original_cache_and_result_owners() {
        use tcl_dialect::TclVersion;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            let mut vm = crate::native_fixture::core(profile);
            let created = vm.invoke_command(
                "::oo::object",
                &[Value::string("create"), Value::string("O")],
            );
            assert_eq!(created.code, Code::Ok, "{version:?}");
            let original = Value::new_native_string_bytes(b"O".as_slice());
            let object = crate::cmd_oo::object_key(&mut vm, &original).unwrap();
            assert!(original.native_command_name_cache().is_some());
            original.invalidate_native_string_for_test();
            assert_eq!(
                crate::cmd_oo::object_key(&mut vm, &original).unwrap(),
                object
            );
            assert!(
                original.resident_string_bytes().is_none(),
                "{version:?} cache hit must precede getter"
            );
            let class = crate::cmd_oo::object_class_name(&vm, object);
            let repeated = crate::cmd_oo::object_class_name(&vm, object);
            assert!(class.is_same_object(&repeated));
            assert_eq!(class.native_object_type_name(), "string");
            assert_eq!(
                class.resident_string_bytes().unwrap().as_ref(),
                b"::oo::object"
            );
            let namespace = crate::cmd_oo::object_namespace_name(&mut vm, object).unwrap();
            assert_eq!(
                namespace.native_namespace_name_cache().is_some(),
                version >= TclVersion::V9_0
            );
            assert!(namespace.resident_string_bytes().is_some());
            let first = vm.native_oo_predicate_result(true).unwrap();
            let second = vm.native_oo_predicate_result(true).unwrap();
            assert!(first.is_same_object(&second));
            assert_eq!(first.native_object_type_name(), "int");
            assert!(first.resident_string_bytes().is_none());
            let absent = Value::new_native_string_bytes(b"absent".as_slice());
            assert!(!crate::cmd_oo::is_object(&mut vm, &absent).unwrap());
            let code =
                vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged);
            assert_eq!(code.native_object_type_name(), "list");
            assert_eq!(code.string_bytes().as_ref(), b"TCL LOOKUP OBJECT absent");
            let error = crate::cmd_oo::object_key(&mut vm, &absent).unwrap_err();
            assert_eq!(error.code, Code::Error);
            assert_eq!(error.result.native_object_type_name(), "string");
            let creation = crate::cmd_oo::object_creation_id(&mut vm, object).unwrap();
            assert_eq!(creation.native_object_type_name(), "int");
            assert!(creation.resident_string_bytes().is_none());
            let rename =
                vm.invoke_command("rename", &[Value::string("O"), Value::string("renamed")]);
            assert_eq!(rename.code, Code::Ok, "{version:?}");
            assert!(
                crate::cmd_oo::object_key(&mut vm, &original).is_err(),
                "{version:?} stale stringless name must refuse"
            );
            let moved = Value::new_native_string_bytes(b"renamed".as_slice());
            assert_eq!(crate::cmd_oo::object_key(&mut vm, &moved).unwrap(), object);
            let after = crate::cmd_oo::object_namespace_name(&mut vm, object).unwrap();
            assert_eq!(
                after.resident_string_bytes(),
                namespace.resident_string_bytes()
            );
        }
    }

    #[test]
    fn specialized_object_info_executes_original_operand_before_native_getter() {
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::native_fixture::core(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let result = vm.eval_source("::oo::object create O; set count 0; proc original {} {global count; incr count; return O}; proc c {} {info object class [original]}; proc n {} {info object namespace [original]}; proc i {} {info object isa object [original]}; list [c] [string match ::oo::* [n]] [i] $count").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{engine}: {}",
                result.result.to_str()
            );
            assert_eq!(
                result.result.string_bytes().as_ref(),
                b"::oo::object 1 1 3",
                "{engine}"
            );
            if engine == "tcl9.1" {
                let epoch = vm
                    .eval_source("proc d {} {info object creationid [original]}; d")
                    .unwrap();
                assert_eq!(epoch.code, Code::Ok);
                assert_eq!(epoch.result.native_object_type_name(), "int");
            }
        }
    }

    #[test]
    fn original_tcloo_info_bootstrap_preserves_private_ensemble_targets() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::core(profile);
            let available = vm
                .actual_native_invocation_dialect()
                .info_oo_ensemble_namespace("class")
                .is_some();
            for name in ["::oo::InfoClass", "::oo::InfoObject"] {
                assert_eq!(
                    vm.command_kind(name),
                    available.then_some("ensemble"),
                    "{engine} {name}"
                );
            }
            assert!(vm.command_kind("::tcl::info::class").is_none(), "{engine}");
            assert!(vm.command_kind("::tcl::info::object").is_none(), "{engine}");
            if !available {
                continue;
            }
            let original = Value::string("oo::class");
            let result = vm.invoke_command(
                "::oo::InfoClass::superclasses",
                std::slice::from_ref(&original),
            );
            assert_eq!(result.code, Code::Ok, "{engine}");
            assert_eq!(
                result.result.string_bytes().as_ref(),
                b"::oo::object",
                "{engine}"
            );
            let public = vm.invoke_command(
                "info",
                &[
                    Value::string("class"),
                    Value::string("superclasses"),
                    original,
                ],
            );
            assert_eq!(public.code, Code::Ok, "{engine}");
            assert_eq!(
                public.result.string_bytes().as_ref(),
                b"::oo::object",
                "{engine}"
            );
        }
    }
}
