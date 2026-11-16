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
fn original_class_methods_complete_distinct_native_names_with_ui_tables_cleared() {
    let source = "oo::class create C\\uD800 {self method m\\uD800 {} {}; self method m\\uD801 {} {}}\nC\\uD800 ";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    analysis.all_classes.clear();
    analysis.superseded_classes.clear();
    let record = analysis.original_class_declarations().next().unwrap();
    let items = original_class_method_items(&analysis, record, "");
    assert_eq!(items.len(), 2);
    assert_ne!(items[0].insert_text, items[1].insert_text);
    let cursor = u32::try_from(source.len()).unwrap();
    let std::ops::ControlFlow::Break(Some(selected)) =
        crate::original_oo::class_completion_context(&analysis, source, cursor)
    else {
        panic!("authentic escaped class command head");
    };
    assert_eq!(selected.declaration_site(), record.declaration_site());
}

#[test]
fn original_method_completion_replaces_the_complete_grouped_selector() {
    let source = "oo::class create C {self method m\\uD800 {} {}}\nC {m\\uD8}";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let record = analysis.original_class_declarations().next().unwrap();
    let wanted = record
        .metadata()
        .original_members
        .methods(tcl_compiler::analyser::types::MemberSide::ClassObject)
        .unwrap()
        .remove(0);
    let cursor = u32::try_from(source.len() - 1).unwrap();
    let (span, replacement) = crate::original_oo::method_selector_replacement(
        &analysis,
        source,
        cursor,
        wanted.original_name_input().bytes(),
        wanted.original_name_input().policy(),
    )
    .unwrap();
    assert_eq!(source.get(span.as_range()), Some(r"{m\uD8}"));
    let rewritten = format!(
        "{}{}{}",
        &source[..span.start() as usize],
        replacement,
        &source[span.end() as usize..]
    );
    let config = analysis.body_lexer_config.unwrap();
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(&rewritten),
        tcl_lexer::Span::new(0, u32::try_from(rewritten.len()).unwrap()),
        config,
    )
    .unwrap();
    let word = &plan.commands.last().unwrap().words[1];
    let key =
        tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            wanted.original_name_input().policy(),
        )
        .unwrap();
    assert_eq!(key.bytes(), wanted.original_name_input().bytes());
}

#[test]
fn original_class_method_advice_does_not_inherit_own_self_methods() {
    let source = "oo::class create Base {self method own {} {}}\noo::class create Child {superclass Base; self method child {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let child = analysis
        .original_class_declarations()
        .find(|record| record.name_input().bytes() == b"Child")
        .unwrap();
    let items = original_class_method_items(&analysis, child, "");
    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["child"]
    );
}
