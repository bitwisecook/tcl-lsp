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

//! Selected framework source keeps the genuine core's complete manifest context.

fn host() -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
}

#[test]
fn complete_installed_core_input_survives_vendor_repins() {
    let mut vm = crate::native_fixture::core(host());
    let mut context = vm.runtime_context().clone();
    context
        .packages
        .push(("host_contract".into(), "1.0".into()));
    vm.pin_context(&context).unwrap();
    let expected = vm.held_identity().clone();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    vm.active_native_profile = Some(host());
    assert_eq!(vm.selected_source_identity().unwrap(), &expected);
    assert_eq!(*vm.native_core_pin.as_ref().unwrap().context, context);
    let mut invalid = context.clone();
    invalid.build = tcl_dialect::model::BuildProfileId::Unknown;
    assert!(vm.pin_context(&invalid).is_err());
    assert_eq!(vm.selected_source_identity().unwrap(), &expected);
}
