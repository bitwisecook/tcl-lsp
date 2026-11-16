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

#[test]
fn outer_paramlist_cannot_borrow_a_nested_static_header() {
    let source = "proc outer $parameters {proc nested {} {}; puts literal}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
    let commands = walk.segment_document(source);
    assert!(logical_source_variable_spellings(source, &analysis, &commands[0], &walk).is_none());
    let source = "proc outer {value} {puts literal}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
    let commands = walk.segment_document(source);
    let names = logical_source_variable_spellings(source, &analysis, &commands[0], &walk).unwrap();
    assert!(names.contains("value"));
}

#[test]
fn member_formals_keep_the_selected_original_paramlist_word() {
    let source = "oo::class create C {method m {value} {puts literal}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let image = tcl_lexer::SourceImage::document(source);
    let member = analysis
        .logical_source_member_at(
            &image,
            analysis.body_lexer_config.unwrap(),
            u32::try_from(source.find("m {value}").unwrap()).unwrap(),
        )
        .unwrap();
    let parameter = member.original_parameter_word().unwrap();
    assert_eq!(parameter.bytes(), b"{value}");
    assert_eq!(member.original_source_formals().unwrap()[0].name, "value");
}
