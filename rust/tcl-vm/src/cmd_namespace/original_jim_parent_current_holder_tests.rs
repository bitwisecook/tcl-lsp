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
fn parent_reaches_the_original_current_holder_only_for_its_no_argument_form() {
    for explicit in [false, true] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jimtcl"));
        let original = Value::int(42);
        let context =
            crate::interp::OriginalJimNamespaceContext::for_lambda(&mut vm, &original).unwrap();
        vm.push_call_frame(Some("parent_current_holder_control".into()), Vec::new());
        vm.install_jim_procedure_namespace_owner(None, original.clone());
        vm.push_procedure_namespace_context(crate::interp::ProcedureNamespace::OriginalJim(
            context.clone(),
        ));
        assert!(original.resident_string_bytes().is_none());
        let mut arguments = vec![Value::string("parent")];
        if explicit {
            arguments.push(Value::string("absent"));
        }
        let completion = cmd_namespace(&mut vm, &arguments);
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            b"::"
        );
        assert_eq!(original.resident_string_bytes().is_some(), !explicit);
        assert_eq!(
            vm.current_namespace_execution_context()
                .resolved_token()
                .is_some(),
            !explicit
        );
        if !explicit {
            assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"42");
        }
        assert!(vm.execution_refusal.is_none());
        vm.pop_ns();
        vm.pop_call_frame();
    }
}
