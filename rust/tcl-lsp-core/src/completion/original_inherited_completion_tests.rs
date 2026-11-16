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
use tcl_compiler::analyser::{Analyser, types::MemberSide};

fn selected_bytes(
    item: &CompletionItem,
    record: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
) -> Vec<u8> {
    let key = record.name_input();
    let image = tcl_lexer::SourceImage::document(&item.insert_text);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(item.insert_text.len()).unwrap()),
        key.lexer_config(),
    )
    .unwrap();
    let [command] = plan.commands.as_slice() else {
        panic!("one inserted command word");
    };
    let [word] = command.words.as_slice() else {
        panic!("one inserted selector");
    };
    tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
        word,
        key.word_value_rules(),
        key.policy(),
    )
    .unwrap()
    .bytes()
    .to_vec()
}

#[test]
fn original_inherited_method_candidates_use_byte_relations_after_reporting_clear() {
    let source = r"oo::class create Base\uD800 {method b\uD800 {} {}}; oo::class create Base\uD801 {method wrong {} {}}; oo::class create Mix {method mixed {} {}}; oo::class create Child {superclass Base\uD800; mixin Mix; method own {} {}}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_classes.clear();
    analysis.superseded_classes.clear();
    analysis.global_scope.classes.clear();
    let record = analysis
        .original_class_declarations()
        .find(|record| record.name_input().bytes() == b"Child")
        .unwrap();
    let items = original_method_candidate_items(&analysis, record, "", MemberSide::Instance);
    let mut values = items
        .iter()
        .map(|item| selected_bytes(item, record))
        .collect::<Vec<_>>();
    values.sort();
    assert_eq!(
        values,
        [
            b"b\xed\xa0\x80".to_vec(),
            b"mixed".to_vec(),
            b"own".to_vec()
        ]
    );
    assert!(
        items
            .iter()
            .all(|item| item.detail.as_deref() == Some("method — source declaration candidate"))
    );
}

#[test]
fn original_property_accessor_candidates_follow_typed_source_inheritance_only() {
    let source = "oo::configurable create Base {property x}; oo::class create Child {superclass Base; method own {} {}}";
    let mut analysis = Analyser::new().analyse(source, "tcl9.0");
    analysis.all_classes.clear();
    analysis.global_scope.classes.clear();
    let record = analysis
        .original_class_declarations()
        .find(|record| record.name_input().bytes() == b"Child")
        .unwrap();
    let items = original_method_candidate_items(&analysis, record, "", MemberSide::Instance);
    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["configure", "own"]
    );
    assert_eq!(
        items[0].detail.as_deref(),
        Some("method — property declaration candidate")
    );
    assert_eq!(
        original_method_candidate_items(&analysis, record, "", MemberSide::ClassObject),
        [] as [crate::completion::CompletionItem; 0]
    );
}
