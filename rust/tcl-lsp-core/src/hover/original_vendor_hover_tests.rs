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
fn original_vendor_header_hover_keeps_source_only_opaque_cards_after_ui_clear() {
    // Implementation contract: naming.vendor.original-source-declaration-consumers
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
    let source = "proc {p\\uD800} {argument} {return $argument}\n";
    let mut analysis = Analyser::new().analyse(source, "f5-iapps");
    assert_eq!(analysis.original_vendor_procedure_declarations().count(), 1);
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    let card = hover(source, 0, 8, &analysis, None).unwrap();
    assert!(card.value.contains("p\\uD800"));
    assert!(card.value.contains("source declaration"));
    assert!(card.value.contains("materialisation is unavailable"));
    assert!(hover("# displaced\n", 0, 8, &analysis, None).is_none());
}
