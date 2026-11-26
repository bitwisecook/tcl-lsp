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

mod commands {
    mod command_tokens {
        mod semantic_identity_tests {
            use tcl_runtime_api::{Namespaces, ROOT_NS, semantic_identity::CommandIdentityTokens};

            #[test]
            fn command_refs_survive_rename_and_split_on_recreation() {
                let mut vm = tcl_vm::test_support::native_vm();
                vm.set_compiler(Box::new(
                    tcl_codegen::compile_service::BytecodeCompileService::default(),
                ));
                assert!(
                    vm.try_eval_source("proc p {} {return p}")
                        .unwrap()
                        .code
                        .is_ok()
                );
                let initial_handle = vm.find_command(ROOT_NS, "p").unwrap();
                let initial = vm.command_ref(initial_handle).into_proven().unwrap();
                assert!(vm.try_eval_source("rename p q").unwrap().code.is_ok());
                let moved_handle = vm.find_command(ROOT_NS, "q").unwrap();
                assert_eq!(initial, vm.command_ref(moved_handle).into_proven().unwrap());
                assert!(
                    vm.try_eval_source("proc p {} {return newp}")
                        .unwrap()
                        .code
                        .is_ok()
                );
                let recreated = vm.find_command(ROOT_NS, "p").unwrap();
                assert_ne!(initial, vm.command_ref(recreated).into_proven().unwrap());
                assert!(
                    vm.try_eval_source("rename q {}; proc q {} {return fresh}")
                        .unwrap()
                        .code
                        .is_ok()
                );
                let fresh = vm.find_command(ROOT_NS, "q").unwrap();
                assert_ne!(initial, vm.command_ref(fresh).into_proven().unwrap());
                assert_eq!(
                    initial,
                    vm.command_ref(initial_handle).into_proven().unwrap()
                );
                let mut counts = tcl_core_types::answer::ConformanceCounts::default();
                counts.observe(
                    &vm.command_ref(moved_handle)
                        .map(|reference| reference == initial),
                    Some(&true),
                );
                counts.observe(
                    &vm.command_ref(recreated)
                        .map(|reference| reference != initial),
                    Some(&true),
                );
                counts.observe(
                    &vm.command_ref(fresh).map(|reference| reference != initial),
                    Some(&true),
                );
                println!("vm: {counts}");
            }
        }
    }
}
