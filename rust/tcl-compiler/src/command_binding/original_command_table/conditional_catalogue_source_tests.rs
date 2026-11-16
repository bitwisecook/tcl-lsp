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

#[test]
fn original_conditional_source_catalogue_requires_current_custom_cell_barriers() {
    // naming.compiler.conditional-registry-source-metadata
    // docs/design/analysis/name-resolution-proofs/conditional-registry-source-metadata.md
    for (dialect, source, target) in [
        (
            "f5-irules",
            "proc call {args} {}\nwhen HTTP_REQUEST {call Lib::one}",
            "call Lib",
        ),
        (
            "tcl8.6",
            "namespace eval ::report {}\nproc ::report::defstyle {args} {}\n::report::defstyle st {} {top}",
            "::report::defstyle st",
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let mut actual = None;
        let start = u32::try_from(source.find(target).unwrap()).unwrap();
        // The actual whole source word-vector owner chooses a nested call
        // before metadata; no outer script's role can fill an inner miss.
        if analysis.has_original_vendor_source_names() {
            actual =
                crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                    source, &analysis, start,
                )
                .map(|(metadata, _)| metadata.shape().command().to_owned());
        } else if let Some(metadata) = analysis.original_conditional_registry_metadata_in_source(
            &tcl_lexer::SourceImage::document(source),
            config,
            start,
        ) {
            actual = Some(metadata.command().to_owned());
        }
        assert!(
            actual.is_none(),
            "{dialect}: custom source cell admitted {actual:?}"
        );
    }
}
