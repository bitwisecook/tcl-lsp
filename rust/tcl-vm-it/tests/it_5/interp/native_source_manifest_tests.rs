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

//! Framework source on an iRules vendor VM runs on the host core and leaves the vendor dialect selected.

use crate::prelude::*;

fn host() -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
}

fn vendor_vm() -> Vm {
    let mut vm = crate::fixture::interpreter(host());
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_command_surface_profile(host()));
    vm
}

#[test]
fn genuine_framework_source_uses_core_manifest_and_restores_vendor_pin() {
    let mut vm = vendor_vm();
    let completion = vm
        .try_eval_native_host_source("set framework_value 42")
        .unwrap();
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.string_bytes().as_ref(), b"42");
    assert_eq!(
        vm.dialect_profile().cache_key(),
        tcl_dialect::DialectProfile::irules().cache_key()
    );
}
