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

//! Origin lookups over a retired name world and root-relative names.

#[test]
fn origin_retired_world_refuses_instead_of_reporting_a_missing_command() {
    let profile = crate::environment::profile_for_dialect("jim");
    let mut vm = crate::native_fixture::core(profile);
    let original = super::Value::string("puts");
    vm.name_world.borrow_mut().retire();
    assert!(vm.native_namespace_command_name(&original, true).is_err());
}

#[test]
fn origin_canonical_root_relative_retains_the_actual_original_header() {
    use tcl_registry::native_jim_lookup::NativeJimNamespaceOriginBackend;
    let profile = crate::environment::profile_for_dialect("jim");
    let mut vm = crate::native_fixture::core(profile);
    let original = super::Value::from_native_string_bytes(b"puts".as_slice());
    let entered =
        super::native_original_lookup::OriginalLookup::enter(&mut vm, super::ROOT_NS, &original)
            .unwrap();
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_jim_lookup_protocol()
        .unwrap();
    let mut backend = super::native_command_names::JimNamespaceOriginBackend {
        vm: &mut vm,
        entered: &entered,
        protocol,
    };
    let canonical = backend.canonical(&original).unwrap();
    assert!(canonical.is_same_object(&original));
    let absolute = super::Value::from_native_string_bytes(b"::::puts".as_slice());
    let canonical = backend.canonical(&absolute).unwrap();
    assert!(!canonical.is_same_object(&absolute));
    assert_eq!(canonical.resident_string_bytes().unwrap().as_ref(), b"puts");
}
