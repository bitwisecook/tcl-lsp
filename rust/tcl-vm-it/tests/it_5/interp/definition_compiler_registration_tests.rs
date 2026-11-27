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

//! What `method` names inside `oo::define` and `oo::objdefine`, read from the definition frame by a host command.

use crate::prelude::*;
use std::cell::RefCell;

type Registrations = Rc<RefCell<Vec<(String, bool)>>>;

/// Records what `method` resolves to in the frame the probe runs in, and
/// whether that command is a procedure.
struct Probe(Registrations);

impl tcl_vm::NativeCommand for Probe {
    fn invoke(&self, vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        let which = crate::fixture::call(
            vm,
            "namespace",
            &[Value::string("which"), Value::string("method")],
        );
        let resolved = which.result.to_str().to_string();
        let procedure = !resolved.is_empty()
            && !crate::fixture::call(
                vm,
                "info",
                &[Value::string("procs"), Value::string(resolved.as_str())],
            )
            .result
            .to_str()
            .is_empty();
        self.0.borrow_mut().push((resolved, procedure));
        Completion::new(Code::Ok, Value::empty(), Value::empty())
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
        let mut vm = crate::fixture::interpreter(profile);
        let records = Rc::new(RefCell::new(Vec::new()));
        vm.register_native_command("registration_probe", Rc::new(Probe(Rc::clone(&records))));
        for source in [
            "registration_probe",
            "oo::class create C {method m {} {return OK}; registration_probe}",
            "oo::objdefine C {registration_probe}",
            "registration_probe",
        ] {
            assert_eq!(crate::fixture::eval(&mut vm, source).code, Code::Ok);
        }
        assert_eq!(
            records.borrow().as_slice(),
            &[
                (String::new(), false),
                ("::oo::define::method".to_owned(), false),
                ("::oo::objdefine::method".to_owned(), false),
                (String::new(), false),
            ]
        );
        let result = crate::fixture::eval(
            &mut vm,
            "rename ::oo::define::method ::oo::define::held_method; proc ::oo::define::method args {return REPLACED}; oo::define C {registration_probe}",
        );
        assert_eq!(result.code, Code::Ok);
        assert_eq!(
            records.borrow().last().unwrap(),
            &("::oo::define::method".to_owned(), true)
        );
    }
}
