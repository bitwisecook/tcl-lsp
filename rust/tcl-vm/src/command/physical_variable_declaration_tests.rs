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

use crate::Value;
use tcl_runtime_api::Code;
use tcl_syntax::value::ValueOps;

#[test]
fn namespace_variable_declaration_uses_the_supplied_host_with_separate_source_policy() {
    let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
    let source = tcl_dialect::DialectProfile::irules();
    let mut vm = crate::native_fixture::core(host);
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    let policy = vm.native_compiler_policy();
    assert!(vm.native_invocation_dialect().execution_point().is_none());
    assert_eq!(
        vm.try_invoke_command("variable", &[]).unwrap().code,
        Code::Ok
    );
    let args = [
        Value::new_native_string_bytes(b"host_variable".as_slice()),
        Value::new_native_string_bytes(b"VALUE".as_slice()),
    ];
    assert_eq!(
        vm.try_invoke_command("variable", &args).unwrap().code,
        Code::Ok
    );
    let value = vm
        .get_var("host_variable")
        .expect("actual declared namespace cell");
    assert_eq!(vm.native_string_bytes(&value).unwrap().as_ref(), b"VALUE");
    assert!(vm.execution_refusal.is_none());
    assert_eq!(vm.source_profile().cache_key(), source.cache_key());
    assert_eq!(
        vm.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
    assert_eq!(vm.native_compiler_policy(), policy);
}

#[test]
fn namespace_variable_declaration_keeps_release_arity_closed() {
    let source = tcl_dialect::DialectProfile::irules();
    let host = tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let mut original = crate::native_fixture::core(host);
    original.set_dialect_profile(source);
    assert_eq!(super::cmd_variable(&mut original, &[]).code, Code::Error);
    assert!(original.execution_refusal.is_none());
    assert_eq!(
        original.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
}
