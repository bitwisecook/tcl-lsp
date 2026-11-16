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

#[cfg(not(windows))]
#[test]
fn selected_source_links_preserve_a_posix_backslash_as_filename_data() {
    let source = r"source {/work/a\b.tcl}";
    for dialect in ["tcl", "tcl8.6"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let links = document_links_from_analysis(source, &analysis, &LinkContext::default());
        let [link] = links.as_slice() else {
            panic!("one original path link: {dialect}: {links:?}")
        };
        assert_eq!(link.target, "file:///work/a%5Cb.tcl");
        assert_eq!(
            document_links_from_analysis(
                &source.replace("a\\b", "a/b"),
                &analysis,
                &LinkContext::default()
            ),
            [] as [crate::document_links::DocumentLink; 0]
        );
    }
}

#[test]
// Implementation contract: naming.core.original-document-link-selection
// docs/design/analysis/name-resolution-proofs/original-document-link-selection.md
fn original_document_links_use_actual_handlers_values_and_current_source() {
    let source = "source {a $b [c].tcl}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let ctx = LinkContext {
        workspace_root: Some("/project"),
        ..LinkContext::default()
    };
    let links = document_links_from_analysis(source, &analysis, &ctx);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file:///project/a%20$b%20%5Bc%5D.tcl");
    analysis.source_targets.clear();
    analysis.command_invocations.clear();
    analysis.dialect = "f5-irules".into();
    assert_eq!(document_links_from_analysis(source, &analysis, &ctx), links);
    assert_eq!(
        document_links_from_analysis(&format!("#{source}"), &analysis, &ctx),
        [] as [crate::document_links::DocumentLink; 0]
    );
    for source in [
        "proc source args {}; source {a.tcl}",
        "source [unknown]",
        "source {-encoding} utf-8 a.tcl extra",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            document_links_from_analysis(source, &analysis, &ctx).is_empty(),
            "{source}"
        );
    }
}

#[test]
// Implementation contract: naming.core.original-document-link-selection
// docs/design/analysis/name-resolution-proofs/original-document-link-selection.md
fn original_document_links_retain_package_inputs_and_versioned_source_grammar() {
    let ctx = LinkContext {
        workspace_root: Some("/project"),
        ..LinkContext::default()
    };
    for (dialect, source, expected) in [
        ("tcl8.4", "source -encoding utf-8 a.tcl", 0),
        ("tcl8.6", "source -encoding utf-8 a.tcl", 1),
        ("tcl9.0", "source -nopkg a.tcl", 1),
    ] {
        let analysis = Analyser::new().analyse(source, dialect);
        assert_eq!(
            document_links_from_analysis(source, &analysis, &ctx).len(),
            expected,
            "{dialect}"
        );
    }
    let source = "package require {p$[x]}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(analysis.package_requires.len(), 1);
    analysis.package_requires[0].name = "counterfactual".into();
    let links = document_links_from_analysis(source, &analysis, &ctx);
    assert_eq!(links.len(), 1);
    assert!(links[0].tooltip.as_ref().unwrap().contains("p$[x]"));
    assert_eq!(
        document_links_from_analysis("package require other", &analysis, &ctx),
        [] as [crate::document_links::DocumentLink; 0]
    );
}
