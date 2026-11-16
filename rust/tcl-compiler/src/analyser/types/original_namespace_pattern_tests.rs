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
fn original_namespace_patterns_retain_opaque_source_and_tail_without_ui_rows() {
    let source = r"namespace import ::n\uD800::*; namespace forget ::n\uD801::p\uD802";
    let mut result = Analyser::new().analyse(source, "tcl8.6");
    let patterns = result
        .original_namespace_patterns()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(patterns.len(), 2);
    assert_eq!(patterns[0].parts().tail.as_bytes(), b"*");
    assert_eq!(patterns[1].parts().tail.as_bytes(), b"p\xed\xa0\x82");
    assert_ne!(patterns[0].parts().source, patterns[1].parts().source);
    assert_eq!(
        patterns[0].purpose(),
        tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
    );
    assert_eq!(
        patterns[1].purpose(),
        tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern
    );
    assert_eq!(
        patterns[0]
            .name_input()
            .original_word_key()
            .unwrap()
            .source_image(),
        &tcl_lexer::SourceImage::document(source)
    );
    result.namespace_imports.clear();
    result.namespace_forgets.clear();
    assert_eq!(
        result
            .original_namespace_patterns()
            .cloned()
            .collect::<Vec<_>>(),
        patterns
    );
}
