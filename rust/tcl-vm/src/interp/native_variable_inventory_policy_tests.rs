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
fn variable_inventory_uses_the_actual_selected_name_purpose() {
    // naming.invocation.original-variable-inventory-policy
    // docs/design/analysis/name-resolution-proofs/invocation-original-variable-inventory-policy.md
    // Software admission and result control, not an original-provider receipt.
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::interpreter(
            tcl_registry::model::ingress::resolve_environment(environment).unit_profile(),
        );
        let purpose = vm.name_policy_protocol().unwrap();
        assert_eq!(
            Namespaces::variable_lookup_policy(&vm),
            Some(purpose.variable_lookup_policy())
        );
        vm.set_var("r442", Value::string("RETAINED")).unwrap();
        let pattern = Value::string("r442");
        for result in [
            tcl_cmd_core::info::vars(&mut vm, Some(&pattern)).unwrap(),
            tcl_cmd_core::info::globals(&mut vm, Some(&pattern)).unwrap(),
        ] {
            let names = tcl_syntax::value::ValueOps::list_elements(&mut vm, &result).unwrap();
            assert_eq!(names.len(), 1, "{environment}");
            assert_eq!(
                tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &names[0])
                    .unwrap()
                    .as_ref(),
                b"r442",
                "{environment}"
            );
        }
    }
}

#[test]
fn unsupported_inventory_policy_refuses_before_a_pattern_getter() {
    // naming.invocation.original-variable-inventory-policy
    // docs/design/analysis/name-resolution-proofs/invocation-original-variable-inventory-policy.md
    // Standalone ingress supplies no independently retained actual native core.
    let mut vm = Vm::new();
    let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )));
    vm.set_dialect_profile(unknown);
    assert!(vm.name_policy_protocol().is_none());
    assert_eq!(Namespaces::variable_lookup_policy(&vm), None);
    let pattern = Value::int(17);
    assert!(pattern.resident_string_bytes().is_none());
    for error in [
        tcl_cmd_core::info::vars(&mut vm, Some(&pattern)).unwrap_err(),
        tcl_cmd_core::info::globals(&mut vm, Some(&pattern)).unwrap_err(),
    ] {
        assert_eq!(
            error.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "variable inventory lookup policy"
                )
            )
        );
        assert!(pattern.resident_string_bytes().is_none());
    }
}

#[test]
fn genuine_native_inventory_policy_survives_unknown_source_profile() {
    // naming.invocation.original-variable-inventory-policy
    // docs/design/analysis/name-resolution-proofs/invocation-original-variable-inventory-policy.md
    // Source compatibility cannot revoke the independently retained native issuer.
    let mut vm = crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    );
    let original = vm.name_policy_protocol().unwrap();
    vm.set_var("r444", Value::string("RETAINED")).unwrap();
    let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )));
    vm.set_dialect_profile(unknown);
    assert_eq!(vm.name_policy_protocol(), Some(original));
    assert_eq!(
        Namespaces::variable_lookup_policy(&vm),
        Some(original.variable_lookup_policy())
    );
    let pattern = Value::string("r444");
    for result in [
        tcl_cmd_core::info::vars(&mut vm, Some(&pattern)).unwrap(),
        tcl_cmd_core::info::globals(&mut vm, Some(&pattern)).unwrap(),
    ] {
        let names = tcl_syntax::value::ValueOps::list_elements(&mut vm, &result).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(
            tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &names[0])
                .unwrap()
                .as_ref(),
            b"r444"
        );
    }
}
