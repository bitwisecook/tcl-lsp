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

//! Literal data versus deferred substitutions in the compiled-word protocol.

/// Whether the compiled-word runtime scanner must inspect this representation.
/// This selects no command, variable, frame, source geometry or successful result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompiledWordSubstitution {
    /// The bytes are already the final value, including braces and bare dollars.
    LiteralValue,
    /// The configured compiled-word scanner retains the remaining substitutions.
    Deferred,
}

/// Select the compiler/VM word protocol for already normalized word bytes.
/// This is not Tcl source parsing: bare `$name` is data, `${...}` and `[...]`
/// request the runtime scanner, and `$(...)` does so only for Jim's grammar.
/// Escapes have already been decoded by the producer; a selected marker retains
/// the scanner even if its eventual parse fails. Written source and finished
/// values must use their separate APIs rather than infer this representation.
/// The VM's legacy whole-braced-word rule remains an independent earlier gate.
#[must_use]
pub fn compiled_word_substitution(
    word: &[u8],
    config: tcl_lexer::LexerConfig,
) -> CompiledWordSubstitution {
    if word.windows(2).any(|pair| pair == b"${")
        || word.contains(&b'[')
        || (config.var_syntax.has_expr_sugar() && word.windows(2).any(|pair| pair == b"$("))
    {
        CompiledWordSubstitution::Deferred
    } else {
        CompiledWordSubstitution::LiteralValue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_word_protocol_retains_only_its_configured_runtime_markers() {
        for (dialect, expression) in [("tcl8.6", false), ("tcl9.0", false), ("jim", true)] {
            let config = tcl_lexer::LexerConfig::for_dialect(dialect);
            for word in [b"".as_slice(), b"{}", b"$bare\\n", b"$\0data"] {
                assert_eq!(
                    compiled_word_substitution(word, config),
                    CompiledWordSubstitution::LiteralValue
                );
            }
            for word in [
                b"${value}".as_slice(),
                b"pre[command]post",
                b"[unclosed",
                b"${unfinished",
            ] {
                assert_eq!(
                    compiled_word_substitution(word, config),
                    CompiledWordSubstitution::Deferred
                );
            }
            assert_eq!(
                compiled_word_substitution(b"pre$(1+2)", config),
                if expression {
                    CompiledWordSubstitution::Deferred
                } else {
                    CompiledWordSubstitution::LiteralValue
                }
            );
        }
    }
}
