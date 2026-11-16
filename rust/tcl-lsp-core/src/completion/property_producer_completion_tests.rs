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

use super::original_method_candidate_items;
use tcl_compiler::analyser::types::MemberSide;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

#[test]
fn foreign_property_ledger_terminates_method_candidates() {
    let source = "oo::configurable create C {property size; method own {} {}}";
    let current = Analyser::new().analyse(source, "tcl9.0");
    let class = current.original_class_declarations().next().unwrap();
    assert_ne!(
        original_method_candidate_items(&current, class, "", MemberSide::Instance),
        [] as [crate::completion::CompletionItem; 0]
    );
    let input = current.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let distinct =
        std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(context.commands())));
    let other_input = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        distinct,
        input.lexer_config(),
    );
    let other = Analyser::new()
        .with_resolved_input(other_input)
        .analyse(source, "tcl9.0");
    let mut mixed = current.clone();
    mixed.original_class_metadata = other.original_class_metadata;
    let class = mixed.original_class_declarations().next().unwrap();
    assert_eq!(
        original_method_candidate_items(&mixed, class, "", MemberSide::Instance),
        [] as [crate::completion::CompletionItem; 0]
    );
}
