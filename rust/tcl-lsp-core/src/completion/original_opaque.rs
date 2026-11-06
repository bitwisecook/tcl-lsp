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

//! Registry command authoring inside a genuine retained opaque-script operand.

use tcl_compiler::analyser::AnalysisResult;

use super::CompletionItem;

pub(super) fn items(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    offer_registry: bool,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> Option<Vec<CompletionItem>> {
    let position =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?;
    let authoring = crate::source_structure::SourceStructure::opaque_script_authoring_at(
        source,
        analysis,
        position.offset(),
    )?;
    // This route has only a command-authoring purpose. It cannot fall through
    // into variable, argument, Scoped, workspace or enclosing-frame advice.
    let Some(prefix) = authoring.command_prefix(source, analysis, position.offset()) else {
        return Some(Vec::new());
    };
    let Some(partial) = tcl_syntax::word_rules::original_word_cursor_literal_prefix(&prefix) else {
        return Some(Vec::new());
    };
    if !offer_registry {
        return Some(Vec::new());
    }
    let items = super::builtin_completions(
        current.registry(),
        current.profile(),
        &partial,
        &rustc_hash::FxHashMap::default(),
        false,
        analysis,
        crate::oo_dispatch::OoFrame {
            resolves: false,
            method: false,
        },
    );
    Some(super::original_word_completion_edits(
        source, line, &prefix, items,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn complete(marked: &str) -> Vec<CompletionItem> {
        let cursor = u32::try_from(marked.find("<|>").unwrap()).unwrap();
        let source = marked.replacen("<|>", "", 1);
        let analysis = Analyser::new().analyse(&source, "sslictcl");
        let position = tcl_lexer::LineIndex::new(&source).position_at_utf16(cursor, &source);
        super::super::completions(
            &source,
            position.line,
            position.character.get(),
            &analysis,
            analysis.resolved_registry(),
            None,
            analysis.resolved_profile().unwrap(),
        )
    }

    #[test]
    fn opaque_command_authoring_offers_registry_without_parent_or_binding_advice() {
        let source = "# 😀\nsslictcl 1\npolicy baseline {\n check bespoke {\n  predicate {\n   se<|>\n  }\n }\n}\n";
        let items = complete(source);
        let set = items
            .iter()
            .find(|item| item.label == "set")
            .expect("ordinary command authoring");
        let edit = set.text_edit.as_ref().unwrap();
        assert_eq!((edit.start_char, edit.end_char), (3, 5));
        assert_eq!(edit.new_text, "set");
        let all = complete(&source.replace("se<|>", "<|>"));
        for prohibited in ["severity", "sslictcl", "policy", "endpoint", "my", "self"] {
            assert!(
                !all.iter().any(|item| item.label == prohibited),
                "{prohibited}: {all:?}"
            );
        }
        for body in [
            "set x se<|>",
            "set known value\n puts $kn<|>",
            "set data {se<|>}",
            "# se<|>",
        ] {
            let marked = source.replace("se<|>", body);
            assert!(complete(&marked).is_empty(), "{body}");
        }
    }

    #[test]
    fn opaque_command_authoring_refuses_lookalike_data_through_public_completion() {
        for marked in [
            "set data {se<|>}",
            "policy p {check c {message {se<|>}}}",
            "policy p {check c {$head {se<|>}}}",
            "policy p {check c {{*}predicate {se<|>}}}",
            "policy p {check c {predicate {*}{se<|>}}}",
            "policy p {check c {predicate \"s\\x65<|>\"}}",
            "policy p {check c {predicate \"se<|>$suffix\"}}",
            "policy p {check c {predicate {se<|>} extra}}",
        ] {
            let items = complete(marked);
            assert!(
                !items.iter().any(|item| item.label == "set"),
                "opaque-looking data must not donate command authoring: {marked}: {items:?}"
            );
        }
    }

    #[test]
    fn opaque_command_authoring_replaces_the_whole_word_after_same_line_utf16() {
        let marked = "policy p {check c {predicate {set banner 😀; se<|>suffix}}}";
        let items = complete(marked);
        let item = items
            .iter()
            .find(|item| item.label == "set")
            .expect("actual opaque command authoring");
        let edit = item.text_edit.as_ref().unwrap();
        assert_eq!((edit.start_char, edit.end_char), (45, 53));
        let source = marked.replacen("<|>", "", 1);
        let mut after = source.encode_utf16().collect::<Vec<_>>();
        after.splice(
            usize::try_from(edit.start_char).unwrap()..usize::try_from(edit.end_char).unwrap(),
            edit.new_text.encode_utf16(),
        );
        assert_eq!(
            String::from_utf16(&after).unwrap(),
            "policy p {check c {predicate {set banner 😀; set}}}"
        );
    }

    #[test]
    fn opaque_command_authoring_public_entry_keeps_current_source_and_opt_in() {
        let source = "policy p {check c {predicate {set banner 😀; se}}}";
        let analysis = Analyser::new().analyse(source, "sslictcl");
        let profile = analysis.resolved_profile().unwrap();
        let cursor = u32::try_from(source.find("; se").unwrap() + 4).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
        let query = |text: &str, character: u32, offer: bool| {
            super::super::completions(
                text,
                0,
                character,
                &analysis,
                analysis.resolved_registry().filter(|_| offer),
                None,
                profile,
            )
        };
        assert!(
            query(source, position.character.get(), true)
                .iter()
                .any(|item| item.label == "set")
        );
        assert_eq!(
            query(source, position.character.get(), false),
            [] as [CompletionItem; 0]
        );
        assert_eq!(
            query(
                &source.replace("banner", "marker"),
                position.character.get(),
                true
            ),
            [] as [CompletionItem; 0]
        );
        // The astral character starts at UTF16 column41;42 splits its pair.
        assert_eq!(query(source, 42, true), [] as [CompletionItem; 0]);
    }
}
