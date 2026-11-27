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

thread_local! {
    static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
}

fn refuse_in_hidden_namespace(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    let completion = vm.refuse_host_command("first hidden namespace worker".to_owned());
    FIRST.with(|first| *first.borrow_mut() = vm.execution_refusal.clone());
    completion
}

#[test]
fn hidden_namespace_host_refusal_restores_the_original_frame_and_first_cause() {
    let profile = crate::environment::profile_for_dialect("tcl8.6");
    let mut vm = crate::native_fixture::core(profile);
    vm.register_command_in_slot(
        CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from(b"refuse"),
        },
        Command::Builtin(refuse_in_hidden_namespace),
    );
    vm.hide_command_bytes(b"refuse", b"held").unwrap();
    let caller = vm.activate_namespace_operand(b"Caller").unwrap();
    vm.push_ns_eval_token_frame(caller, Vec::new());
    let original = Value::new_native_string_bytes(b"held".as_slice());
    let receiver = vm.cur_interp();
    let completion = vm
        .invoke_hidden_value_by_id_in_namespace(receiver, &original, &[], Some(b"Chosen"))
        .unwrap();
    assert_eq!(completion.code, Code::Error);
    let first = FIRST.with(|first| first.borrow().clone()).unwrap();
    assert_eq!(vm.execution_refusal, Some(first));
    assert_eq!(vm.current_level(), 1);
    assert_eq!(vm.current_ns_id(), caller);
}
