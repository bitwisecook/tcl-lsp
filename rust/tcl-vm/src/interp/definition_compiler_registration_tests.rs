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
use tcl_runtime_api::native_compilation::{
    NativeCommandImplementation, NativeCompilerHookPresence,
};

type Registrations = Rc<RefCell<Vec<(Option<String>, NativeCompilerHookPresence)>>>;

struct Probe(Registrations);

impl crate::command::NativeCommand for Probe {
    fn invoke(&self, vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        let selected = vm
            .lookup_command_bytes_checked(vm.current_ns_id(), b"method")
            .unwrap();
        let (identity, hook) = match selected {
            Some((key, command)) => {
                let implementation = vm.native_compilation_builtin(&key);
                let hook = vm.native_hook_for_implementation(&implementation, &command);
                let identity = match implementation {
                    NativeCommandImplementation::Registry { identity, .. } => Some(identity),
                    _ => None,
                };
                (identity, hook)
            }
            None => (None, NativeCompilerHookPresence::Unknown),
        };
        self.0.borrow_mut().push((identity, hook));
        super::ok(Value::empty())
    }
}

#[test]
fn definition_compiler_registration_uses_the_actual_definition_frame() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let records = Rc::new(RefCell::new(Vec::new()));
        vm.register_command(
            "registration_probe",
            Command::Native(Rc::new(Probe(Rc::clone(&records)))),
        );
        // The embedder command can mutate the interpreter. Each probe is
        // the final command at an independently captured compile boundary.
        for source in [
            "registration_probe",
            "oo::class create C {method m {} {return OK}; registration_probe}",
            "oo::objdefine C {registration_probe}",
            "registration_probe",
        ] {
            let result = vm.eval_source(source).unwrap();
            assert_eq!(result.code, Code::Ok);
        }
        assert_eq!(
            records.borrow().as_slice(),
            &[
                (None, NativeCompilerHookPresence::Unknown),
                (
                    Some("oo::define::method".to_owned()),
                    NativeCompilerHookPresence::Absent
                ),
                (
                    Some("oo::objdefine::method".to_owned()),
                    NativeCompilerHookPresence::Absent
                ),
                (None, NativeCompilerHookPresence::Unknown),
            ]
        );
        let result = vm.eval_source("rename ::oo::define::method ::oo::define::held_method; proc ::oo::define::method args {return REPLACED}; oo::define C {registration_probe}").unwrap();
        assert_eq!(result.code, Code::Ok);
        assert_eq!(records.borrow().last().unwrap().0, None);
    }
}
