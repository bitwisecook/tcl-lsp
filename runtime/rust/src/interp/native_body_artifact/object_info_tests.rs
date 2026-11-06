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

//! Actual nested ensemble artifacts and original object-info instructions.

use super::*;

#[test]
fn nested_info_artifact_retains_original_maps_and_late_worker_binding() {
    fn replacement(interp: &mut Interp, _arguments: &[*mut TclObj]) -> Code {
        interp.set_result_bytes(b"REPLACED");
        Code::Ok
    }
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = super::tests::interpreter(engine);
        assert_eq!(
            interp.eval_str(b"oo::class create C; proc p {} {info class superclasses C}"),
            Code::Ok,
            "{engine}"
        );
        let declaration = interp.proc_def(b"p").unwrap();
        assert_eq!(interp.eval_str(b"p"), Code::Ok, "{engine}");
        assert_eq!(interp.result_bytes(), b"::oo::object", "{engine}");
        let original = cache(declaration.body.as_ptr()).expect("original nested ensemble artifact");
        assert!(original.scripts.values().any(|script| {
            script
                .commands
                .iter()
                .any(|command| matches!(command.operation, Operation::NamedInvocation(_)))
        }));
        interp.register_builtin(b"::oo::InfoClass::superclasses", replacement);
        assert_eq!(interp.eval_str(b"p"), Code::Ok, "{engine}");
        assert_eq!(interp.result_bytes(), b"REPLACED", "{engine}");
        assert_eq!(
            interp.eval_str(
                b"namespace ensemble configure ::oo::InfoClass -map {superclasses ::list}; p"
            ),
            Code::Ok,
            "{engine}"
        );
        assert_eq!(interp.result_bytes(), b"C", "{engine}");
    }
}

#[test]
fn compiled_object_info_preserves_original_lookup_and_execution_constants() {
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = super::tests::interpreter(engine);
        assert_eq!(interp.eval_str(b"oo::class create C; C create obj; namespace eval N {namespace export obj}; rename obj N::obj; namespace import N::obj; proc object_class {o} {info object class $o}; proc object_ns {o} {info object namespace $o}; proc object_test {o} {info object isa object $o}"), Code::Ok, "{engine}");
        assert_eq!(interp.eval_str(b"object_class obj"), Code::Ok, "{engine}");
        assert_eq!(interp.result_bytes(), b"::C", "{engine}");
        let class_result = interp.result.get();
        assert!(matches!(
            obj::native_object_snapshot(class_result).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        assert_eq!(
            interp.eval_str(b"object_class N::obj"),
            Code::Ok,
            "{engine}"
        );
        assert_eq!(
            interp.result.get(),
            class_result,
            "same actual cached class name"
        );
        assert_eq!(interp.eval_str(b"object_ns obj"), Code::Ok, "{engine}");
        assert!(interp.result_bytes().starts_with(b"::oo::Obj"), "{engine}");
        assert_eq!(interp.eval_str(b"object_test obj"), Code::Ok, "{engine}");
        let truth = interp.result.get();
        // Native instruction-owners probes observe ExecEnv and result roles.
        // SAFETY: the interpreter retains this exact live result header.
        assert_eq!(unsafe { (*truth).ref_count }, 2, "{engine} Boolean owners");
        assert_eq!(interp.result_bytes(), b"1", "{engine}");
        assert_eq!(interp.eval_str(b"object_test N::obj"), Code::Ok, "{engine}");
        assert_eq!(
            interp.result.get(),
            truth,
            "same interpreter execution constant"
        );
        assert_eq!(interp.eval_str(b"object_test absent"), Code::Ok, "{engine}");
        assert_eq!(interp.result_bytes(), b"0", "{engine}");
        assert_eq!(interp.error_code(), b"TCL LOOKUP OBJECT absent", "{engine}");
        for name in [b"object_class".as_slice(), b"object_ns", b"object_test"] {
            let declaration = interp.proc_def(name).unwrap();
            let artifact =
                cache(declaration.body.as_ptr()).expect("actual original ObjectInfo artifact");
            assert!(artifact.scripts.values().any(|script| script.commands.iter().any(|command| matches!(&command.operation, Operation::TclOoHelper(tcl_registry::native_tcloo_compilation::NativeTclOoInstruction::ObjectInfo { .. }, _)))));
        }
    }
}
