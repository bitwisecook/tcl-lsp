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
use tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole as Role;

#[test]
fn externally_retained_original_survives_without_retired_interpreter_authority() {
    let dialect = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    );
    let context = NativeJimObjectContext::new(dialect).unwrap();
    let original = context.empty_object().clone();
    assert!(Rc::ptr_eq(
        &original.native_jim_context().unwrap(),
        &context
    ));
    context.release_object(Role::Empty);
    context.retire();
    assert!(original.native_jim_context().is_err());
    assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"");
}
