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

#[test]
fn definition_workers_retain_the_selected_registration_namespace_and_token() {
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let dialect = tcl_native_model::InvocationDialect::of_static_profile(vm.dialect_profile());
        let native = crate::fixture::root_compilation_entry(&mut vm);
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
                let resolved = vm
                    .try_eval_source(&format!("namespace which -command {identity}"))
                    .unwrap();
                assert_eq!(
                    resolved.result.to_str().trim_start_matches("::"),
                    identity.trim_start_matches("::"),
                    "{engine}"
                );
                assert!(
                    native.commands.iter().any(|binding| matches!(
                        &binding.implementation,
                        tcl_runtime_api::native_compilation::NativeCommandImplementation::Registry {
                            identity: installed,
                            compiler_hook: false,
                        } if installed == identity
                    )),
                    "{engine}: {identity}"
                );
            }
        }
        assert!(
            vm.try_eval_source("info commands ::private")
                .unwrap()
                .result
                .to_str()
                .is_empty()
        );
    }
}
