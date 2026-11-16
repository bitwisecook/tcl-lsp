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
fn readonly_native_diagnostic_body_requires_its_current_card_function_and_tree() {
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let unit = CompilationUnit::build_for_profile(
        "proc p {arg} {puts $missing}",
        registry,
        false,
        environment.unit_profile(),
    );
    let id = *unit
        .ir_module
        .original_native_declaration_body_units
        .keys()
        .next()
        .expect("genuine original Native declaration card");
    let function = unit.body_units.get(&id).unwrap();
    let procedure = unit
        .original_native_declaration_for_function(function, registry)
        .expect("readonly diagnostic body correspondence");
    assert_eq!(
        procedure.body_kind,
        crate::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(procedure.publication.is_none());
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    let foreign_function = (*function).clone();
    assert!(
        unit.original_native_declaration_for_function(&foreign_function, registry)
            .is_none()
    );
    let mut missing = unit.clone();
    missing
        .ir_module
        .original_native_declaration_body_units
        .clear();
    assert!(
        missing
            .original_native_declaration_for_function(
                missing.body_units.get(&id).unwrap(),
                registry
            )
            .is_none()
    );
    let mut changed = unit.clone();
    let body = changed.ir_module.body_units.get_mut(&id).unwrap();
    let span = body.body.statements[0].span();
    body.body.statements[0] = crate::ir::Statement::NativeCall {
        span,
        words: Vec::new(),
    };
    assert!(
        changed
            .original_native_declaration_for_function(
                changed.body_units.get(&id).unwrap(),
                registry
            )
            .is_none()
    );
    let foreign =
        crate::environment_ingress::resolve_environment("tcl9.1").default_context_registry();
    assert!(
        unit.original_native_declaration_for_function(function, foreign.commands())
            .is_none()
    );
}
