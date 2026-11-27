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

//! Original child-alias public windows and independently authored issuer controls.

#[test]
fn child_alias_publication_refuses_missing_or_foreign_issuer_before_binding() {
    // Authored host-policy mutation only. Native298's public child sources
    // confer no authority for these absent/foreign engine or table states.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for foreign in [false, true] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::core(profile);
            let name = vm.create_child(Some("child".into()), false);
            // Jim names a child by its handle command; it has no C interp path.
            let child = if engine == "jim" {
                vm.child_id(&name).unwrap()
            } else {
                let path = crate::Value::new_native_string_bytes(name.as_bytes());
                vm.resolve_interp_path_original(&path).unwrap()
            };
            let before = vm.in_interp(child, |receiver| {
                let selected = if foreign {
                    if engine == "jim" { "tcl8.6" } else { "jim" }
                } else {
                    "irules"
                };
                receiver.actual_engine_profile = Some(
                    tcl_registry::model::ingress::resolve_environment(selected).unit_profile(),
                );
                receiver.active_native_profile = None;
                receiver.logical_providers.names = None;
                receiver.registered_command_entries()
            });
            let result = vm.interp_alias_at(
                child,
                &crate::Value::new_native_string_bytes(b"::r2286_refused".as_slice()),
                vm.cur,
                vec![
                    crate::Value::new_native_string_bytes(b"list".as_slice()),
                    crate::Value::new_native_string_bytes(b"VALUE".as_slice()),
                ],
            );
            assert_eq!(result.code, crate::Code::Error);
            assert!(vm.refused_completion().is_some());
            assert_eq!(
                vm.in_interp(child, |receiver| receiver.registered_command_entries()),
                before
            );
        }
    }
}
