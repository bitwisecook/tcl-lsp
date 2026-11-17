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

//! A script operand built by one command substitution of literal words.
//!
//! `namespace eval ::n [list proc writer {} {...}]` hands its handler a
//! script whose text the walk evaluated from literal words alone. With that
//! value exact, the body walker accounts for the script command by command,
//! as it does for a braced literal; a variable or a compound word does not
//! qualify.

use crate::ir::WordExpr;

/// Whether `word` is exactly one command substitution of a single command
/// whose every word is a bare or braced literal.
pub(super) fn is_literal_word_substitution(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let WordExpr::CommandSubstitution { spelling, .. } = word else {
        return false;
    };
    let Some(script) = spelling
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
    else {
        return false;
    };
    let commands = crate::segmenter::segment_commands_with_offset_and_config(script, 0, config);
    let [command] = commands.as_slice() else {
        return false;
    };
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(script),
        config,
        command,
    );
    !tokens.words().is_empty()
        && tokens.words().iter().all(|word| {
            matches!(
                word,
                WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
            )
        })
}

/// Whether the script operand at `argument` is a literal-word substitution
/// whose produced value the walk retained exactly.
pub(super) fn is_exact_literal_word_script(
    native: super::SourceNativeInvocation<'_>,
    argument: usize,
    config: tcl_lexer::LexerConfig,
) -> bool {
    native
        .script_operands()
        .written_word(argument)
        .is_some_and(|word| is_literal_word_substitution(word, config))
        && native
            .script_operands()
            .arguments
            .literal_at(argument)
            .is_some()
}

#[cfg(test)]
mod tests {
    use super::is_literal_word_substitution;
    use crate::ir::WordExpr;

    fn word(source: &str) -> WordExpr {
        let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        )
        .words()[1]
            .clone()
    }

    /// Dialect-invariant word shape, checked under Tcl 9.0.
    #[test]
    fn only_one_command_of_literal_words_qualifies() {
        let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        for (source, expected) in [
            ("x [list set a 1]", true),
            ("x [list proc w {} {upvar 1 x y}]", true),
            ("x {set a 1}", false),
            ("x $s", false),
            ("x [list set a $v]", false),
            ("x [list set a [b]]", false),
            ("x [list a; list b]", false),
            ("x [list {*}{a b}]", false),
            ("x a[list b]", false),
            ("x []", false),
        ] {
            assert_eq!(
                is_literal_word_substitution(&word(source), config),
                expected,
                "{source}"
            );
        }
    }
}
