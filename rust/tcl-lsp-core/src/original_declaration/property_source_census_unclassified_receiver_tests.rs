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
fn unclassified_object_receivers_and_unknown_heads_preserve_partial_counts() {
    for suffix in [
        "; set obj [C new]; $obj size",
        "; mystery",
        "; set selector size; my $selector",
    ] {
        let source =
            format!("oo::class create C {{property size; method body {{}} {{my size}}}}{suffix}");
        let analysis = Analyser::new().analyse(&source, "tcl9.0");
        let class = analysis.original_class_declarations().next().unwrap();
        let property = class
            .metadata()
            .original_properties
            .declarations()
            .find(|property| property.original_name_input().bytes() == b"size")
            .unwrap();
        let identity =
            OriginalDeclarationIdentity::for_property("", &source, &analysis, class, property)
                .unwrap();
        let report = readonly_reference_report(&identity, &source, &analysis, true).unwrap();
        assert_eq!(
            report.coverage(),
            ReadonlyReferenceCoverage::Partial,
            "{source}"
        );
        assert!(
            !report.spans().is_empty(),
            "existing positive source selector remains"
        );
    }
}
