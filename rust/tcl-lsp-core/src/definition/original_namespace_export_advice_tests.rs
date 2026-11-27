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

use super::{CallResolution, ImportQuery, exported_original_slot_at_import, live_import_at};
use crate::namespace_import::ExportVerdict;
use tcl_compiler::analyser::Analyser;

#[test]
fn original_document_export_query_uses_retained_slot_after_reporting_rows_are_removed() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; namespace export p\uD800; namespace export -clear p\uD801";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let names = analysis
        .original_procedure_declarations()
        .map(|record| record.name().clone())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 2);
    analysis.namespace_exports.clear();
    analysis.all_procs.clear();
    let at = u32::try_from(source.len()).unwrap();
    let ask = |bytes: &[u8]| {
        let name = names
            .iter()
            .find(|name| name.slot().simple.as_bytes() == bytes)
            .unwrap();
        exported_original_slot_at_import(
            &analysis,
            CallResolution::document_only(),
            name.slot(),
            name.policy(),
            at,
        )
    };
    assert_eq!(ask(b"p\xed\xa0\x80"), ExportVerdict::NotExported);
    assert_eq!(ask(b"p\xed\xa0\x81"), ExportVerdict::Exported);
}

#[test]
fn original_document_export_observability_keeps_opaque_namespace_and_policy() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = r"namespace eval n\uD800 {proc p {} {}; namespace export other}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let name = analysis
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name();
    let scope = tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
        name.slot().namespace.clone(),
    );
    assert!(scope.display().is_none());
    assert!(super::namespace_exports_observable(
        &analysis,
        &scope,
        name.policy()
    ));
    assert_eq!(
        super::export_verdict_for_slot(
            &analysis,
            CallResolution::document_only(),
            name.slot(),
            name.policy(),
            u32::try_from(source.len()).unwrap()
        ),
        ExportVerdict::NotExported
    );
    let foreign = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
        .authored_name_policy()
        .unwrap();
    assert!(!super::namespace_exports_observable(
        &analysis, &scope, foreign
    ));
}

#[test]
fn original_import_query_keeps_intermediate_source_geometry_without_a_local_declaration() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = "namespace eval C {proc p {} {}; namespace export p}\nnamespace eval B {namespace import ::C::*; namespace export p}\nnamespace eval A {namespace import ::B::*}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        analysis
            .original_procedure_declarations()
            .all(|record| record.name().slot().namespace.as_segments().len() == 1)
    );
    assert_eq!(
        live_import_at(
            &analysis,
            CallResolution::document_only(),
            "::A",
            "p",
            u32::MAX,
            ImportQuery::Resolve
        ),
        Some("::B".to_owned())
    );
}
