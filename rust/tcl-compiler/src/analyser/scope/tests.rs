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
use tcl_lexer::TokenType;

mod command_resolution;
mod constant_values;
mod logical_scope_advice;
mod namespace_variables;
mod original_variable_occurrences;
mod scope_diagnostics;
mod scope_tree;

fn span(start: u32, end: u32) -> Span {
    Span::new(start, end)
}

fn make_analyser_with_namespace(ns_name: &str) -> Analyser {
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, ns_name));
    a
}

fn var(name: &str, def_span: Span) -> VarDef {
    VarDef {
        name: name.to_string(),
        definition_span: def_span,
        references: Vec::new(),
        warn_if_unused: false,
        array_indices: std::collections::BTreeSet::new(),
        link_target: None,
        link_target_span: None,
    }
}
