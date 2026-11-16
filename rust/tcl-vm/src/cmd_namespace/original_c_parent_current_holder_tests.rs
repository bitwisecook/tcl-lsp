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

#[test]
fn parent_without_operand_uses_the_entered_namespace_incarnation() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(release));
        let parent = vm.activate_namespace_operand(b"::outer").unwrap();
        let entered = vm.activate_namespace_operand(b"::outer::inner").unwrap();
        vm.push_ns_token("outer::inner", entered);
        let arguments = [Value::string("parent")];
        let result = cmd_namespace(&mut vm, &arguments);
        assert_eq!(result.code, Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"::outer");
        assert!(vm.native_namespace_token_has_owner(parent));

        assert!(vm.delete_namespace_operand(b"::outer::inner").unwrap());
        let replacement = vm.activate_namespace_operand(b"::outer::inner").unwrap();
        assert_ne!(entered, replacement);
        // Tcl_DeleteNamespace detaches an active root's physical parent;
        // the original frame retains that birth, independently of recreation.
        let result = cmd_namespace(&mut vm, &arguments);
        assert_eq!(result.code, Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"");
        assert_eq!(vm.current_ns_id(), entered);
        vm.pop_ns();
        assert!(!vm.native_namespace_token_has_owner(entered));

        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(release));
        let entered = vm.activate_namespace_operand(b"::outer::inner").unwrap();
        vm.push_ns_token("outer::inner", entered);
        let original =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm)
                .unwrap();
        assert!(vm.delete_namespace_operand(b"::outer::inner").unwrap());
        assert!(original.current_namespace_token(&mut vm).is_err());
        let first = vm.execution_refusal.clone().unwrap();
        let result = cmd_namespace(&mut vm, &arguments);
        assert_eq!(result.code, Code::Error);
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    }
}
