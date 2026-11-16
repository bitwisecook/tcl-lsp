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

#[test]
fn native_method_input_reaches_target_without_unicode_redecoding() {
    let source = "oo::class create Base {self method {m😀} {} {return ok}}; oo::class create C {superclass Base; self method {m😀} {} {next}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
    let cursor = u32::try_from(source.find("next").unwrap()).unwrap();
    let (class, member, _) = original_source_member_at(&analysis, &image, config, cursor).unwrap();
    assert!(matches!(member, SourceMemberKind::NativeMethod { .. }));
    let hierarchy = analysis.class_hierarchy();
    let provider = hierarchy
        .member_next_provider(&class, &member, &class, None, source)
        .unwrap();
    let target = class_member_def(&analysis.all_classes[provider], &member).unwrap();
    assert_eq!(
        target.name_span,
        analysis.all_classes["::Base"].class_methods["m😀"].name_span
    );
}
