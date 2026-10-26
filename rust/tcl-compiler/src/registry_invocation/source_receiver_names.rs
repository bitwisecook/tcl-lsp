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

//! Original receiver syntax is name data or a scalar reference, never both.

pub(crate) enum SourceReceiverNameOperand {
    Literal(String),
    ScalarVariable(String),
}

/// Lexical receiver data only; callers retain the selected receiver role and
/// current whole-source owner independently. No value or object is resolved.
pub(crate) fn original_source_receiver_name_operand(
    word: &tcl_lexer::NativeWord,
) -> Option<SourceReceiverNameOperand> {
    if word.group().expand {
        return None;
    }
    if let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word) {
        return String::from_utf8(value)
            .ok()
            .map(SourceReceiverNameOperand::Literal);
    }
    let arena = word.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Variable { name, index: None } = part.part else {
        return None;
    };
    let name = arena.bytes(name)?;
    if tcl_syntax::naming::split_element_ref_bytes(name).is_some() {
        return None;
    }
    std::str::from_utf8(name)
        .ok()
        .map(|name| SourceReceiverNameOperand::ScalarVariable(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_receiver_syntax_preserves_literal_data_and_scalar_reference_names() {
        let source = "receiver {$obj} ${obj} {two words} {} {[obj]} {obj(key)} $obj(key) ${obj(key)} [pick] x$obj";
        let image = tcl_lexer::SourceImage::document(source);
        let words = tcl_lexer::native_script_words_in(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap()
        .commands
        .remove(0)
        .words;
        assert!(
            matches!(original_source_receiver_name_operand(&words[1]), Some(SourceReceiverNameOperand::Literal(name)) if name == "$obj")
        );
        assert!(
            matches!(original_source_receiver_name_operand(&words[2]), Some(SourceReceiverNameOperand::ScalarVariable(name)) if name == "obj")
        );
        for (ordinal, expected) in [(3, "two words"), (4, ""), (5, "[obj]"), (6, "obj(key)")] {
            assert!(
                matches!(original_source_receiver_name_operand(&words[ordinal]), Some(SourceReceiverNameOperand::Literal(name)) if name == expected)
            );
        }
        for word in &words[7..] {
            assert!(original_source_receiver_name_operand(word).is_none());
        }
    }
}
