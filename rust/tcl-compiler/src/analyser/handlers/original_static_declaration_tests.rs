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
use tcl_lexer::SourceImage;

#[test]
fn original_nested_procedure_metadata_retains_full_source_words_and_namespace_geometry() {
    let source = r"namespace eval N {proc p\uD800 {} {return A}; proc p\uD801 {} {return B}}";
    let mut result = Analyser::new().analyse(source, "tcl8.6");
    let declarations = result.original_procedure_declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    assert_ne!(declarations[0].name().slot(), declarations[1].name().slot());
    for (declaration, written, bytes) in [
        (declarations[0], r"p\uD800", b"p\xed\xa0\x80".as_slice()),
        (declarations[1], r"p\uD801", b"p\xed\xa0\x81".as_slice()),
    ] {
        assert_eq!(declaration.name_input().bytes(), bytes);
        assert_eq!(
            declaration.name_input().source_image(),
            &SourceImage::document(source)
        );
        assert_eq!(
            declaration.name_input().span().start(),
            u32::try_from(source.find(written).unwrap()).unwrap()
        );
        assert_eq!(declaration.name().reported_full_name(), None);
    }
    let body = u32::try_from(source.find("proc p").unwrap()).unwrap();
    let namespace = result.original_namespace_scope_at(body).unwrap().clone();
    let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        declarations[0].name_input().clone(),
    );
    result.all_procs.clear();
    result.global_scope.children[0].procs.clear();
    assert_eq!(
        result
            .procedures_for_original_name(&input, &namespace)
            .len(),
        1
    );
}

#[test]
fn original_static_declaration_advice_does_not_fabricate_a_computed_word() {
    let source = r"proc $unknown {} {return A}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert!(result.original_procedure_declarations().next().is_none());
}
