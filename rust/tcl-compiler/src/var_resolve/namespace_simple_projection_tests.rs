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
fn namespace_simple_key_projection_keeps_resolved_bytes_separate_from_display_and_frames() {
    // naming.analysis.resolved-static-variable-patterns
    // docs/design/analysis/name-resolution-proofs/resolved-static-variable-patterns.md
    // Actual authored resolver places only; no Native namespace/getter or TMM grant.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let context = ResolveContext::for_namespace("::static");
    let mut selected = resolve_literal_place("débug", &context, false, registry);
    assert_eq!(namespace_variable_simple_name(&selected), Some("débug"));
    assert_eq!(root_namespace_variable_simple_name(&selected), None);
    selected.name = "misleading::debug".into();
    selected.ns = "::".into();
    assert_eq!(namespace_variable_simple_name(&selected), Some("débug"));
    assert_eq!(root_namespace_variable_simple_name(&selected), None);
    let root = resolve_literal_place(
        "debug",
        &ResolveContext::for_namespace("::"),
        false,
        registry,
    );
    assert_eq!(namespace_variable_simple_name(&root), Some("debug"));
    assert_eq!(root_namespace_variable_simple_name(&root), Some("debug"));
    let local = resolve_literal_place(
        "debug",
        &ResolveContext::for_function("::p"),
        false,
        registry,
    );
    assert_eq!(namespace_variable_simple_name(&local), None);
    assert_eq!(root_namespace_variable_simple_name(&local), None);
}
