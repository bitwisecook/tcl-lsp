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

//! Behaviour tests moved from `tcl-vm`'s `value_ops::raw_value_tests`.

use crate::prelude::*;

#[test]
fn reached_host_refusal_cannot_initialise_or_write_an_increment_receiver() {
    fn refuse(vm: &mut Vm, _: &[Value]) -> tcl_runtime_api::Completion<Value> {
        vm.refuse_host_command("test native-access refusal".to_owned())
    }
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = crate::fixture::interpreter(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        vm.register("refuse", refuse);
        vm.set_var("x", Value::int(10)).unwrap();
        crate::fixture::eval(&mut vm, "trace add variable x read refuse");
        assert!(matches!(
            vm.try_eval_source("incr x"),
            Err(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_))
        ));
        assert_eq!(vm.get_var("x").unwrap().string_bytes().as_ref(), b"10");
    }
}
