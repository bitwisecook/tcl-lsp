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
fn definition_workers_retain_the_selected_registration_namespace_and_token() {
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
        let dialect = vm.actual_native_invocation_dialect();
        let native = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
        for member in [
            "private",
            "classmethod",
            "definitionnamespace",
            "initialise",
            "initialize",
            "class",
        ] {
            for layer in [
                tcl_registry::ObjectDispatchLayer::Class,
                tcl_registry::ObjectDispatchLayer::Object,
            ] {
                let Some(identity) = tcl_registry::native_tcloo_registration::definition_identity(
                    member, layer, dialect,
                ) else {
                    continue;
                };
                let key = vm
                    .resolve_command_bytes_checked(ROOT_NS, identity.as_bytes(), true)
                    .unwrap()
                    .expect("actual qualified worker");
                let token = vm.visible_command_generation(&key).unwrap();
                let found = native
                    .commands
                    .iter()
                    .find(|binding| binding.token == token)
                    .unwrap();
                assert!(
                    matches!(&found.implementation, tcl_runtime_api::native_compilation::NativeCommandImplementation::Registry {identity: installed, compiler_hook:false} if installed == identity)
                );
            }
        }
        assert!(
            vm.resolve_command_bytes_checked(ROOT_NS, b"::private", true)
                .unwrap()
                .is_none()
        );
    }
}
