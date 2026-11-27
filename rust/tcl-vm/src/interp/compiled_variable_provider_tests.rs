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
use tcl_native_model::native_compiled_variables::LogicalCompiledVariableProvider;
use tcl_syntax::naming::{NativeCompiledVariableAuthority, NativeCompiledVariableRecipe};

#[test]
fn logical_local_compiler_is_explicit_retained_and_independent_of_host() {
    let mut vm = Vm::new();
    let f5 = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let provider = LogicalCompiledVariableProvider::Tcl84CoreSimulation;
    vm.set_dialect_profile(f5);
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.compiled_variable_protocol().is_none());
    let prior = vm.native_compiler_policy();
    assert!(vm.set_logical_compiled_variable_provider(provider));
    assert_ne!(vm.native_compiler_policy(), prior);
    let policy = vm.compiled_variable_protocol().unwrap();
    assert_eq!(
        policy.authority(),
        NativeCompiledVariableAuthority::AuthoredSimulation
    );
    assert_eq!(
        policy.recipe(),
        NativeCompiledVariableRecipe::C(tcl_dialect::TclVersion::V8_4)
    );
    let entry = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
    assert_eq!(entry.compiled_variable_protocol, Some(policy));
    assert_eq!(
        entry.execution_point,
        Some(tcl_dialect::model::DialectPoint::for_tcl_version(
            tcl_dialect::TclVersion::V9_0
        ))
    );
    let child = vm.fork_child_state();
    assert_eq!(child.logical_providers.compiled_variables, Some(provider));
    assert_eq!(child.compiled_variable_protocol(), Some(policy));
    vm.active_native_profile = Some(host);
    let actual = vm.compiled_variable_protocol().unwrap();
    assert_eq!(actual.authority(), NativeCompiledVariableAuthority::Native);
    assert_eq!(
        actual.recipe(),
        NativeCompiledVariableRecipe::C(tcl_dialect::TclVersion::V9_0)
    );
    vm.active_native_profile = None;
    assert_eq!(vm.compiled_variable_protocol(), Some(policy));
}
