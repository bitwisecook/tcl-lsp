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
fn logical_alias_reads_keep_source_coverage_and_variable_edit_obligations() {
    let source = "proc p {} {global shared; puts $shared}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    let offset = u32::try_from(source.rfind("$shared").unwrap() + 1).unwrap();
    let read = analysis.original_variable_symbol_at_offset(offset).unwrap();
    assert_eq!(read.original_local_alias().unwrap().1.as_bytes(), b"shared");
    assert!(read.has_source_applicability_obligation());
    assert!(analysis.original_variable_alias_source_is_complete(&image, config));
    assert!(!analysis.original_variable_rename_is_complete(read.symbol()));
    assert!(
        analysis
            .original_variable_symbol_in_source(&image, config, offset)
            .is_some()
    );
    assert!(
        analysis
            .original_variable_symbol_in_source(
                &tcl_lexer::SourceImage::document(&format!("{source} ")),
                config,
                offset
            )
            .is_none()
    );
    let mut changed = config;
    changed.strict_quoting = !changed.strict_quoting;
    assert!(
        analysis
            .original_variable_symbol_in_source(&image, changed, offset)
            .is_none()
    );
}

#[test]
fn logical_alias_reads_preserve_competing_unknown_and_frame_barriers() {
    for source in [
        "proc p {} {global shared; namespace upvar ::other value shared; puts $shared}",
        "proc p {} {global shared; upvar 0 $unknown shared; puts $shared}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        assert!(
            !analysis.original_variable_alias_source_is_complete(&image, config),
            "{source}"
        );
        if let Some(read) = analysis.original_variable_symbol_at_offset(
            u32::try_from(source.rfind("$shared").unwrap() + 1).unwrap(),
        ) {
            assert!(
                !analysis.original_variable_rename_is_complete(read.symbol()),
                "{source}"
            );
        }
    }
    let source = "proc p {} {global shared; puts $shared}; proc q {} {puts $shared}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let offset = u32::try_from(source.rfind("$shared").unwrap() + 1).unwrap();
    let read = analysis.original_variable_symbol_at_offset(offset).unwrap();
    assert!(read.original_local_alias().is_none());
    assert!(!read.symbol().is_namespace());
    assert!(!analysis.original_variable_rename_is_complete(read.symbol()));
}
