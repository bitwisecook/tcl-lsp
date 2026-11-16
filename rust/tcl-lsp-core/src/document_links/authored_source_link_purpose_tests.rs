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
fn authored_jim_path_advice_keeps_scope_without_becoming_public_native_value() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let source = "namespace eval N {set dir /FIRST; source $dir/a.tcl}; namespace eval N {source $dir/b.tcl}; source $dir/c.tcl";
    let profile = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let analysis = super::tests::path_analysis(source, profile, config);
    assert!(!analysis.allows_lexical_declaration_advice());
    let context = LinkContext {
        workspace_root: Some("/workspace"),
        ..LinkContext::default()
    };
    let links = lexical_document_links_in_context(source, &analysis, &context);
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///FIRST/a.tcl");
    assert_eq!(
        document_links_from_analysis(source, &analysis, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
    let mut foreign = analysis.clone();
    let mut changed_config = config;
    changed_config.leading_bom = match config.leading_bom {
        tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
    };
    assert_ne!(changed_config, config);
    foreign.body_lexer_config = Some(changed_config);
    assert_eq!(
        lexical_document_links_in_context(source, &foreign, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
    assert_eq!(
        lexical_document_links_in_context(&format!("# changed\n{source}"), &analysis, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
}
