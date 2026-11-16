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

use super::tests::path_analysis;
use super::*;
use tcl_compiler::analyser::Analyser;

#[test]
fn original_hosted_links_keep_source_schema_and_name_purposes_independent() {
    // Implementation contract: naming.vendor.original-source-navigation
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-navigation.md
    let context = LinkContext {
        workspace_root: Some("/work"),
        ..LinkContext::default()
    };
    for dialect in ["f5-iapps", "f5-tmsh"] {
        let source = "source {a $b [c].tcl}\npackage require -exact {pkg $[x]} 1.0";
        let mut analysis = Analyser::new().analyse(source, dialect);
        analysis.source_targets.clear();
        analysis.package_requires.clear();
        let links = document_links_from_analysis(source, &analysis, &context);
        assert_eq!(links.len(), 2, "{dialect}: {links:?}");
        assert!(
            links
                .iter()
                .any(|link| link.target == "file:///work/a%20$b%20%5Bc%5D.tcl")
        );
        assert!(
            links
                .iter()
                .any(|link| link.tooltip.as_deref()
                    == Some("Package requirement candidate: pkg $[x]"))
        );
        assert_eq!(
            document_links_from_analysis(&source.replace("a $b", "z $b"), &analysis, &context),
            [] as [crate::document_links::DocumentLink; 0]
        );
        for source in [
            "source {f\\uD800.tcl}",
            "source $path",
            "source -encoding utf-8 a.tcl",
            "proc source args {}; source a.tcl",
            "rename source {}; source a.tcl",
            "proc package args {}; package require P",
        ] {
            let analysis = Analyser::new().analyse(source, dialect);
            assert!(
                document_links_from_analysis(source, &analysis, &context).is_empty(),
                "{dialect}: {source}"
            );
        }
    }
    let source = "source a.tcl\npackage require P";
    let analysis = Analyser::new().analyse(source, "f5-irules");
    assert_eq!(
        document_links_from_analysis(source, &analysis, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
}
#[test]
fn original_source_path_links_use_selected_operations_and_authentic_bound_values() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let context = LinkContext {
        workspace_root: Some("/workspace"),
        ..LinkContext::default()
    };
    let source =
        "interp alias {} build {} file join {/ROOT with space}; source [build {literal$é.tcl}]";
    let analysis = path_analysis(source, profile, config);
    let links = lexical_document_links_in_context(source, &analysis, &context);
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(
        links[0].target,
        file_uri_for_path("/ROOT with space/literal$é.tcl")
    );
    for source in [
        "rename file moved; interp alias {} file {} list; source [file join /ROOT leaf]",
        "proc file {args} {}; source [file join /ROOT leaf]",
    ] {
        let analysis = path_analysis(source, profile, config);
        assert_eq!(
            lexical_document_links_in_context(source, &analysis, &context),
            [] as [crate::document_links::DocumentLink; 0]
        );
    }
}
