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
use tcl_compiler::analyser::Analyser;

#[test]
fn document_export_adopter_keeps_conditional_clear_and_missing_input_unknown() {
    // One document's straight-line sequence orders its own removals, so
    // an unconditional top-level `-clear` written after the pattern
    // revokes it at the document end (tclsh 8.6/9.0: `namespace export`
    // answers empty after `namespace export p; namespace export -clear`).
    let source = "proc p {} {}; namespace export p; namespace export -clear\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let declaration = analysis
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name();
    let ctx = CallResolution::document_only();
    assert_eq!(
        exported_original_slot_at_import(
            &analysis,
            ctx,
            declaration.slot(),
            declaration.policy(),
            u32::try_from(source.len()).unwrap()
        ),
        ExportVerdict::NotExported
    );
    // A clear in a conditional branch is not straight-line sequence: it
    // stays withheld and the positive export evidence remains.
    let conditional = "proc p {} {}; namespace export p; if {$x} {namespace export -clear}\n";
    let branch = Analyser::new().analyse(conditional, "tcl8.6");
    assert_eq!(branch.original_namespace_exports().count(), 2);
    let declaration = branch
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name();
    assert_eq!(
        exported_original_slot_at_import(
            &branch,
            ctx,
            declaration.slot(),
            declaration.policy(),
            u32::try_from(conditional.len()).unwrap()
        ),
        ExportVerdict::Exported
    );
    let order = in_document_source_order(&analysis, ctx);
    let point = in_document_point(&analysis, ctx, 0);
    assert_eq!(order.has_run(point, point), None);
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let order = in_document_source_order(&missing, ctx);
    assert_eq!(order.has_run(point, point), None);
    assert!(order.navigation_precedence(point, point).is_none());
}
