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
use crate::source_graph::SourceNavigationOrderPurpose;
use tcl_compiler::analyser::Analyser;

#[test]
fn workspace_order_keeps_original_export_evidence_without_conditional_clear_completion() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    // Within one document the source sequence is known, so this
    // same-document unconditional `-clear` follows the pattern and revokes
    // it. The cross-document shape keeps the conditional answer: see
    // `workspace_order_keeps_cross_document_clear_conditional`.
    let source = "proc p {} {}; namespace export p; namespace export -clear\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let declaration = analysis
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name()
        .clone();
    let exports = analysis
        .original_namespace_exports()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(exports.len(), 2);
    analysis.namespace_exports.clear();
    analysis.command_invocations.clear();
    analysis.dialect = "display-only-label".into();
    let uri = "file:///original-order.tcl";
    let index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    let order = index.run_order();
    let first = RunPoint {
        uri,
        at: exports[0].span().start(),
        enclosing_body: None,
    };
    let clear = RunPoint {
        uri,
        at: exports[1].span().start(),
        enclosing_body: None,
    };
    let relation = order.navigation_precedence(first, clear).unwrap();
    assert_eq!(
        relation.purpose(),
        SourceNavigationOrderPurpose::DocumentSequence
    );
    assert_eq!(relation.modeled_source_exclusion(), Some(true));
    assert_eq!(order.has_run(first, clear), None);
    let query = RunPoint {
        uri,
        at: u32::try_from(source.len()).unwrap(),
        enclosing_body: None,
    };
    assert_eq!(
        index.export_snapshot().exported_at_original(
            declaration.slot(),
            declaration.policy(),
            query
        ),
        ExportVerdict::NotExported
    );
}

#[test]
fn workspace_order_keeps_cross_document_clear_conditional() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    // Which of two documents runs first is not a static fact, so a
    // `-clear` in another document is never ordered after the pattern and
    // cannot revoke the original export evidence.
    let library = "proc p {} {}; namespace export p\n";
    let clearing = "namespace export -clear\n";
    let mut lib = Analyser::new().analyse(library, "tcl8.6");
    let mut clr = Analyser::new().analyse(clearing, "tcl8.6");
    let declaration = lib
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name()
        .clone();
    let pattern = lib.original_namespace_exports().next().unwrap().span();
    let tombstone = clr.original_namespace_exports().next().unwrap().span();
    for analysis in [&mut lib, &mut clr] {
        analysis.namespace_exports.clear();
        analysis.command_invocations.clear();
    }
    let (lib_uri, clr_uri) = ("file:///original-lib.tcl", "file:///original-clear.tcl");
    let index = WorkspaceIndex::from_documents([(lib_uri, &lib), (clr_uri, &clr)]);
    let order = index.run_order();
    let first = RunPoint {
        uri: lib_uri,
        at: pattern.start(),
        enclosing_body: None,
    };
    let clear = RunPoint {
        uri: clr_uri,
        at: tombstone.start(),
        enclosing_body: None,
    };
    for (a, b) in [(first, clear), (clear, first)] {
        assert!(order.navigation_precedence(a, b).is_none_or(|relation| {
            relation.purpose() == SourceNavigationOrderPurpose::Conditional
                && relation.modeled_source_exclusion().is_none()
        }));
    }
    let query = RunPoint {
        uri: lib_uri,
        at: u32::try_from(library.len()).unwrap(),
        enclosing_body: None,
    };
    assert_eq!(
        index.export_snapshot().exported_at_original(
            declaration.slot(),
            declaration.policy(),
            query
        ),
        ExportVerdict::Exported
    );
}

#[test]
fn indexed_order_context_is_replaced_and_missing_full_input_remains_unknown() {
    let source = "set original 1\nset second 2\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let uri = "file:///order-currency.tcl";
    let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    let first = RunPoint {
        uri,
        at: 0,
        enclosing_body: None,
    };
    let second = RunPoint {
        uri,
        at: u32::try_from(source.find("set second").unwrap()).unwrap(),
        enclosing_body: None,
    };
    assert!(
        index
            .run_order()
            .navigation_precedence(first, second)
            .is_some()
    );
    let mut stale = analysis.clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    index.add_document(uri, &stale);
    assert!(
        index
            .run_order()
            .navigation_precedence(first, second)
            .is_none()
    );
    index.remove_document(uri);
    assert!(
        index
            .run_order()
            .navigation_precedence(first, second)
            .is_none()
    );
    index.add_document(uri, &analysis);
    assert!(
        index
            .run_order()
            .navigation_precedence(first, second)
            .is_some()
    );
    let other = Analyser::new().analyse("set different 3\n", "tcl8.6");
    index.add_document(uri, &other);
    // Accumulation is intentional for source-site views; a foreign source
    // issuer cannot replace the original context of existing rows.
    assert!(
        index
            .run_order()
            .navigation_precedence(first, second)
            .is_none()
    );
}
