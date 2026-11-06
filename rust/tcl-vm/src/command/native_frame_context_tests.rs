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

//! Native frame-object conversions use the actual original Jim context.

use super::*;

#[test]
fn generated_jim_level_objects_use_actual_frame_context() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::interpreter(profile);
    let namespace = vm.current_ns_id();
    let explicit = Value::string("0");
    assert_eq!(
        runtime_explicit_frame_selection(&mut vm, &explicit).unwrap(),
        0
    );
    assert_eq!(explicit.integer_representation(), Some(0));
    let leading = Value::string("0");
    assert_eq!(
        runtime_frame_selection(
            &mut vm,
            tcl_registry::FrameEffectSpec::UPLEVEL,
            &[leading.clone(), Value::string("ignored")],
        )
        .unwrap(),
        (1, 0)
    );
    assert_eq!(leading.integer_representation(), Some(0));
    assert_eq!(vm.current_level(), 0);
    assert_eq!(vm.current_ns_id(), namespace);
}

#[test]
fn jim_level_object_cannot_borrow_a_foreign_interpreter_context() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let owner = crate::native_fixture::interpreter(profile);
    let mut other = crate::native_fixture::interpreter(profile);
    let context = crate::interp::InterpState::native_jim_object_context(&owner).unwrap();
    let original = Value::string("0");
    original.bind_native_jim_context(&context).unwrap();
    let namespace = other.current_ns_id();
    assert!(runtime_explicit_frame_selection(&mut other, &original).is_err());
    assert!(other.refused_completion().is_some());
    assert!(Rc::ptr_eq(
        &original.native_jim_context().unwrap(),
        &context
    ));
    assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"0");
    assert_eq!(other.current_level(), 0);
    assert_eq!(other.current_ns_id(), namespace);
}
