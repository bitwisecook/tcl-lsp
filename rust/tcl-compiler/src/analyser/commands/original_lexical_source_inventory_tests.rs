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

use crate::analyser::Analyser;

#[test]
fn original_lexical_inventory_retains_brackets_without_parent_execution() {
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    for source in [
        "when HTTP_REQUEST {ILX::call [ILX::init p e] m}",
        "unknown [ILX::init p e]",
        "unknown \"prefix[ILX::init p e]suffix\"",
        "unknown $array([ILX::init p e])",
        "unknown [unknown [ILX::init p e]]",
    ] {
        let analysis = Analyser::new().analyse(source, "f5-irules");
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let offset = u32::try_from(source.find("ILX::init").unwrap()).unwrap();
        let occurrence = analysis
            .original_vendor_source_names()
            .find(|occurrence| {
                occurrence.site().offset == offset
                    && occurrence.original_words().first()
                        == Some(occurrence.name_input().original_word())
            })
            .unwrap_or_else(|| panic!("missing genuine lexical constructor: {source}"));
        assert_eq!(occurrence.original_words().len(), 3, "{source}");
        let last = occurrence.original_words().last().unwrap();
        assert_eq!(
            source.get(offset as usize..last.span().end() as usize),
            Some("ILX::init p e")
        );
        assert!(
            occurrence
                .original_words()
                .iter()
                .all(|word| word.image() == &image && word.config() == config)
        );
        assert_eq!(
            occurrence
                .name_input()
                .literal_units(tcl_syntax::naming::VendorSourceNamePurpose::CommandHead),
            Some(&b"ILX::init"[..])
        );
        assert!(
            analysis
                .original_vendor_source_name_in_source(
                    &image,
                    config,
                    occurrence.name_input().span()
                )
                .is_some()
        );
        assert!(
            analysis
                .original_vendor_source_name_in_source(
                    &tcl_lexer::SourceImage::document(&format!("{source}\n# changed")),
                    config,
                    occurrence.name_input().span()
                )
                .is_none()
        );
        assert!(
            analysis.original_completed_command_world().is_none(),
            "lexical children cannot supply a completed world: {source}"
        );
        assert_eq!(
            analysis.original_procedure_declarations().count(),
            0,
            "hosted syntax cannot publish native procedures"
        );
    }
}

#[test]
fn original_lexical_inventory_keeps_inert_braces_and_escapes_opaque() {
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    for source in [
        "unknown {[ILX::init p e]}",
        r"unknown \[ILX::init p e\]",
        "unknown [set text {[ILX::init p e]}]",
    ] {
        let analysis = Analyser::new().analyse(source, "f5-irules");
        let offset = u32::try_from(source.find("ILX::init").unwrap()).unwrap();
        assert!(
            !analysis
                .original_vendor_source_names()
                .any(|occurrence| occurrence.site().offset == offset),
            "inert source cannot acquire a command vector: {source}"
        );
        assert!(analysis.original_completed_command_world().is_none());
    }
}
