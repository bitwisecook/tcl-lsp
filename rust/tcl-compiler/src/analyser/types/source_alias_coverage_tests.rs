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
fn selected_alias_source_coverage_requires_current_input_and_all_actual_receipts() {
    let source = "proc p {} {global shared; puts $shared}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    assert_ne!(
        analysis.original_variable_alias_obligations,
        [] as [(u32, usize); 0]
    );
    assert_eq!(analysis.original_variable_alias_receipts.len(), 1);
    assert!(analysis.original_variable_alias_source_is_complete(&image, config));
    let mut missing_receipts = analysis.clone();
    missing_receipts.original_variable_alias_receipts.clear();
    assert!(!missing_receipts.original_variable_alias_source_is_complete(&image, config));
    let mut missing_input = analysis.clone();
    missing_input.resolved_input = None;
    assert!(!missing_input.original_variable_alias_source_is_complete(&image, config));
    assert!(!analysis.original_variable_alias_source_is_complete(
        &tcl_lexer::SourceImage::document(&format!("{source} ")),
        config
    ));
    let mut changed_config = config;
    changed_config.leading_bom = match config.leading_bom {
        tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
        tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
    };
    assert!(!analysis.original_variable_alias_source_is_complete(&image, changed_config));
    let dynamic_source = "set $unknown 1";
    let dynamic = crate::analyser::Analyser::new().analyse(dynamic_source, "tcl");
    assert!(!dynamic.original_variable_alias_source_is_complete(
        &tcl_lexer::SourceImage::document(dynamic_source),
        dynamic.body_lexer_config.unwrap()
    ));
}
