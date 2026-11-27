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

//! Retained namespace contexts and a retired name world.

use tcl_core_types::ROOT_NS;
use tcl_runtime_api::Namespaces;

#[test]
fn namespace_address_keeps_retained_context_and_live_root_distinct() {
    use tcl_cmd_core::namespace::NamespaceDeleteBackend;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
        let old = vm.activate_namespace_operand(b"::N").unwrap();
        let old_child = vm.activate_namespace_operand(b"::N::child").unwrap();
        vm.push_ns_eval_token_frame(old, Vec::new());
        vm.delete_selected_namespace(old).unwrap();
        let replacement = vm.activate_namespace_operand(b"::N").unwrap();
        let replacement_child = vm.activate_namespace_operand(b"::N::child").unwrap();
        assert_ne!(old, replacement);
        assert_ne!(old_child, replacement_child);
        assert_eq!(vm.current_ns_id(), old);
        assert_eq!(vm.find_namespace_bytes_checked(old, b"").unwrap(), None);
        assert_eq!(Namespaces::find_namespace_bytes(&vm, old, b""), None);
        assert_eq!(
            vm.find_namespace_bytes_checked(old, b"::").unwrap(),
            Some(ROOT_NS)
        );
        assert_eq!(
            vm.find_namespace_bytes_checked(old, b"child").unwrap(),
            Some(old_child)
        );
        assert_eq!(
            vm.find_namespace_bytes_checked(old, b"::N::child").unwrap(),
            Some(replacement_child)
        );
        assert!(vm.refused_completion().is_none());
    }
}

#[test]
fn namespace_address_retired_owner_is_unavailable_not_missing() {
    let vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    vm.name_world.borrow_mut().retire();
    assert!(vm.find_namespace_bytes_checked(ROOT_NS, b"").is_err());
}
