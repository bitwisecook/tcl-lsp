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
use crate::analyser::Analyser;
use crate::analyser::types::MemberSide;

#[test]
fn native_class_object_body_retains_its_actual_source_table() {
    let source = "oo::class create C {self method {<constructor>} {} {next}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
    let cursor = u32::try_from(source.find("next").unwrap()).unwrap();
    let (_, member, body) = original_source_member_at(&analysis, &image, config, cursor).unwrap();
    let SourceMemberKind::NativeMethod { input, side } = member else {
        panic!("genuine Native name input")
    };
    assert_eq!(input.bytes(), b"<constructor>");
    assert_eq!(side, MemberSide::ClassObject);
    assert_eq!(&source[body.as_range()], "{next}");
}
