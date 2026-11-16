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
use crate::namespace_import::NamespaceExportOracle;
use tcl_compiler::analyser::Analyser;

#[test]
fn original_export_snapshot_joins_opaque_slots_and_rejects_ui_and_policy_substitutes() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; namespace export p\uD800; namespace export -clear p\uD801";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let declarations = analysis
        .original_procedure_declarations()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    analysis.namespace_exports.clear();
    analysis.all_procs.clear();
    let index = WorkspaceIndex::from_documents([("file:///exports.tcl", &analysis)]);
    let snapshot = index.export_snapshot();
    let ask = |at, name: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
        snapshot.exported_at_original(
            name.slot(),
            name.policy(),
            RunPoint {
                uri: "file:///exports.tcl",
                at,
                enclosing_body: None,
            },
        )
    };
    let first = declarations
        .iter()
        .find(|record| record.name().slot().simple.as_bytes() == b"p\xed\xa0\x80")
        .unwrap()
        .name();
    let second = declarations
        .iter()
        .find(|record| record.name().slot().simple.as_bytes() == b"p\xed\xa0\x81")
        .unwrap()
        .name();
    let at = u32::try_from(source.len()).unwrap();
    assert_eq!(ask(at, first), ExportVerdict::NotExported);
    assert_eq!(ask(at, second), ExportVerdict::Exported);
    assert_eq!(
        snapshot.exported_at(
            "::",
            "p\u{fffd}",
            RunPoint {
                uri: "file:///exports.tcl",
                at,
                enclosing_body: None
            }
        ),
        ExportVerdict::Unknown
    );
    let foreign =
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_0);
    assert_eq!(
        snapshot.exported_at_original(
            second.slot(),
            foreign,
            RunPoint {
                uri: "file:///exports.tcl",
                at,
                enclosing_body: None
            }
        ),
        ExportVerdict::Unknown
    );
}

#[test]
fn original_import_index_retains_opaque_patterns_without_reporting_rows() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = r"namespace import -force ::n\uD800::p\uD801";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let pattern = analysis
        .original_namespace_patterns()
        .next()
        .expect("genuine original import")
        .clone();
    assert!(pattern.forced());
    analysis.namespace_imports.clear();
    let index = WorkspaceIndex::from_documents([("file:///import.tcl", &analysis)]);
    let rows = index.original_namespace_patterns().collect::<Vec<_>>();
    assert_eq!(rows, vec![("file:///import.tcl", &pattern)]);
    assert_eq!(pattern.parts().tail.as_bytes(), b"p\xed\xa0\x81");
}

#[test]
fn original_export_unknown_operand_blocks_source_absence_and_match_advice() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = "proc p {} {}; namespace export p; namespace export $unavailable";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let name = analysis
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name();
    assert_ne!(analysis.original_namespace_export_unknowns(), []);
    let index = WorkspaceIndex::from_documents([("file:///unknown.tcl", &analysis)]);
    assert_eq!(
        index.export_snapshot().exported_at_original(
            name.slot(),
            name.policy(),
            RunPoint {
                uri: "file:///unknown.tcl",
                at: u32::try_from(source.len()).unwrap(),
                enclosing_body: None,
            }
        ),
        ExportVerdict::Unknown
    );
}
