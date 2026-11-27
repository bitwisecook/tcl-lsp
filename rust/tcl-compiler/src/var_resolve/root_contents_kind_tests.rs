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

fn context(profile: &str) -> (ResolveContext, &'static CommandRegistry) {
    let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
    let mut context = ResolveContext::for_function("::p");
    context.invocation_dialect = Some(tcl_native_model::InvocationDialect::of_profile(
        registry.profile().unwrap(),
    ));
    (context, registry)
}

#[test]
fn root_kind_admission_preserves_native_tcl_and_jim_difference() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let (mut state, registry) = context(profile);
        state.define_literal("a(k)", "OLD", registry);
        let root = resolve_literal_place("a", &state, false, registry);
        assert_eq!(
            state.root_contents_kind(&root),
            Some(RootContentsKind::Array)
        );
        assert_eq!(state.store_would_error(&root), profile != "jim");
    }
}

#[test]
fn conflicting_paths_and_opaque_effects_withdraw_root_kind() {
    let (mut array, registry) = context("tcl8.6");
    array.define_literal("a(k)", "OLD", registry);
    let mut scalar = array.clone();
    crate::variable_bindings::destroy_literal_binding(&mut scalar, "a", 10, registry);
    scalar.define_literal("a", "NEW", registry);
    array.join(&scalar);
    let root = resolve_literal_place("a", &array, false, registry);
    assert_eq!(array.root_contents_kind(&root), None);
    scalar.record_contents_write(&crate::place::unknown_top(), 20, true);
    let root = resolve_literal_place("a", &scalar, false, registry);
    assert_eq!(scalar.root_contents_kind(&root), None);
}

#[test]
fn retired_array_kind_cannot_follow_a_new_same_name_scalar() {
    let (mut state, registry) = context("tcl9.0");
    state.define_literal("a(k)", "OLD", registry);
    let old_root = resolve_literal_place("a", &state, false, registry);
    crate::variable_bindings::destroy_literal_binding(&mut state, "a", 10, registry);
    state.define_literal("a", "NEW", registry);
    assert_eq!(state.root_contents_kind(&old_root), None);
    let new_element = resolve_literal_place("a(k)", &state, false, registry);
    assert!(state.store_would_error(&new_element));
}
