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
fn qualifiers_selects_the_actual_shared_empty_without_changing_root_or_tail_identity() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::core(profile);
    let context = vm.native_jim_object_context().unwrap();
    let empty = context.empty_object().clone();
    for bytes in [b"absent".as_slice(), b":", b""] {
        let original = Value::from_native_string_bytes(bytes);
        let completion = ns_text_op(&mut vm, std::slice::from_ref(&original), false);
        assert_eq!(completion.code, Code::Ok);
        assert!(completion.result.is_same_object(&empty));
        assert!(!completion.result.is_same_object(&original));
    }
    let rooted = Value::from_native_string_bytes(b"::".as_slice());
    let completion = ns_text_op(&mut vm, &[rooted], false);
    assert_eq!(completion.code, Code::Ok);
    assert!(!completion.result.is_same_object(&empty));
    let tail = Value::from_native_string_bytes(b"absent".as_slice());
    let completion = ns_text_op(&mut vm, std::slice::from_ref(&tail), true);
    assert_eq!(completion.code, Code::Ok);
    assert!(completion.result.is_same_object(&tail));
    assert!(vm.refused_completion().is_none());
}
