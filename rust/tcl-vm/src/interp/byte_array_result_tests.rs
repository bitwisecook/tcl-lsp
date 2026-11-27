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
use tcl_native_model::native_string_materialization::LogicalStringProvider;

#[test]
fn binary_producer_origin_survives_shared_materialisation_and_keeps_purposes_separate() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_native_engine_profile(host));
    let payload: Rc<[u8]> = Rc::from(b"\xff\0A".as_slice());
    let authored = vm
        .byte_array_result(
            Rc::clone(&payload),
            Some(LogicalStringProvider::Tcl84CoreSimulation),
        )
        .unwrap();
    let actual = vm.byte_array_result(Rc::clone(&payload), None).unwrap();
    let origin = authored.byte_array_origin().unwrap();
    assert_eq!(
        origin.logical_provider(),
        Some(LogicalStringProvider::Tcl84CoreSimulation)
    );
    assert_eq!(
        origin.protocol(),
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4)
    );
    assert_eq!(actual.byte_array_origin().unwrap().logical_provider(), None);
    assert_eq!(
        actual.byte_array_origin().unwrap().protocol(),
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0)
    );
    assert!(authored.is_pure_byte_array());
    let alias = authored.clone();
    assert_eq!(
        alias
            .native_string_bytes(origin.protocol())
            .unwrap()
            .as_ref(),
        b"\xc3\xbf\xc0\x80A"
    );
    assert_eq!(authored.byte_array_representation().unwrap(), payload);
    assert_eq!(alias.byte_array_origin(), Some(origin));
    assert_eq!(authored.byte_array_origin(), Some(origin));
    let imported = Value::byte_array(Rc::clone(&payload));
    assert_eq!(imported.byte_array_origin(), None);
    imported.native_string_bytes(origin.protocol()).unwrap();
    assert_eq!(imported.byte_array_origin(), None);
    assert_eq!(imported.byte_array_representation().unwrap(), payload);
    vm.active_native_profile = Some(host);
    assert!(
        vm.byte_array_result(
            Rc::clone(&payload),
            Some(LogicalStringProvider::Tcl84CoreSimulation)
        )
        .is_err()
    );
    assert!(vm.byte_array_result(payload, None).is_ok());
    vm.active_native_profile = None;
}

#[test]
fn unsupported_declared_binary_purpose_refuses_without_native_fallback() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = Vm::new();
    vm.set_runtime_version(tcl_dialect::TclVersion::V8_6);
    let payload: Rc<[u8]> = Rc::from(b"\xff\0A".as_slice());
    assert!(vm.byte_array_result(Rc::clone(&payload), None).is_ok());
    assert_eq!(
        vm.byte_array_result(payload, Some(LogicalStringProvider::Tcl84CoreSimulation))
            .unwrap_err()
            .native_access_refusal(),
        Some(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "byte-array result producer"
            )
        )
    );
}
