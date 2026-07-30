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

//! Is a byte offset in text Tcl never substitutes?
//!
//! The cursor-word scan ([`crate::hover::find_var_at_position`] /
//! [`crate::hover::find_word_span_at_position`]) is a delimiter-based
//! character scan: it happily matches a `$level`-shaped substring in a place
//! Tcl emits byte-for-byte unsubstituted, and hover / go-to-definition /
//! find-references / rename then resolved it to a real declaration —
//! contradicting the LSP's own semantic tokens (one opaque comment / string
//! token, no nested variable) and its own W220 "assignment is never read"
//! (which correctly treats those as non-reads). Issue #923 differential-audit
//! finding idx 24.
//!
//! Both tests here are **conservative**: they answer `true` only when the
//! position is provably inert, and `false` whenever anything is unclear, so a
//! caller that abstains on `true` can never lose a genuine reference.
//!
//! * [`offset_in_comment`] — a `#` that begins a command comments to the end
//!   of its line (`tclParse.c`'s `Tcl_ParseCommand`).
//! * [`offset_in_data_brace`] — a **braced** argument word whose registry
//!   [`ArgRole`] does not carry executable Tcl is data: `set t {plain $level
//!   here}` prints `$level` verbatim on tclsh 8.6 and 9.0. The role map is the
//!   registry's; nothing here names a command. A brace the registry says *does*
//!   carry a script (`proc`'s body, an `if` condition, `apply`'s lambda) is
//!   descended into, so a `$var` nested arbitrarily deep in real code stays
//!   live.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::TokenType;
use tcl_registry::{ArgRole, CommandRegistry};

/// Recursion cap for the descent through script-bearing braced arguments.
const MAX_DEPTH: u32 = 32;

/// Whether `off` sits inside a `#` comment.
///
/// A `#` begins a comment only in **command position** — at the start of a
/// line, or after a `;` / `{` (`tclParse.c`). Conservative in two ways: the
/// scan stops at the offset's own line (a backslash-continued comment
/// therefore under-reports, never over-reports), and a `#` on a line with an
/// odd number of preceding unescaped `"` is treated as literal text inside a
/// quoted word rather than a comment.
#[must_use]
pub fn offset_in_comment(source: &str, off: u32) -> bool {
    let off = off as usize;
    if off > source.len() {
        return false;
    }
    let line_start = source[..off].rfind('\n').map_or(0, |nl| nl + 1);
    let line = &source[line_start..off];
    let mut quotes = 0u32;
    let mut command_position = true;
    let mut it = line.char_indices();
    while let Some((_idx, c)) = it.next() {
        match c {
            '\\' => {
                let _ = it.next();
                command_position = false;
            }
            '"' => {
                quotes += 1;
                command_position = false;
            }
            '#' if command_position && quotes.is_multiple_of(2) => return true,
            ' ' | '\t' => {}
            ';' | '{' => command_position = true,
            _ => command_position = false,
        }
    }
    false
}

/// Whether `off` sits inside a **braced argument word that carries no
/// executable Tcl** — a data brace, whose `$`/`[` contents Tcl never
/// substitutes.
///
/// Descends through braced arguments the registry marks as script-bearing
/// ([`ArgRole::carries_script`]) so nesting is handled; abstains (`false`)
/// on anything it cannot classify — an unknown command, a command with a
/// [`case_list`](tcl_registry::CommandSpec::case_list) clause block (whose
/// bodies are scripts a role cannot express), a partial command, or a
/// non-braced word.
#[must_use]
pub fn offset_in_data_brace(
    source: &str,
    off: u32,
    registry: &CommandRegistry,
    dialect: &str,
) -> bool {
    let config = tcl_lexer::LexerConfig::for_dialect(dialect);
    classify(source, source, 0, off, registry, config, 0)
}

/// Walk `text` (anchored at absolute byte `base`) looking for the innermost
/// braced argument word covering `off`; `true` once one is found whose role
/// carries no script.
fn classify(
    full_source: &str,
    text: &str,
    base: u32,
    off: u32,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    depth: u32,
) -> bool {
    if depth >= MAX_DEPTH {
        return false;
    }
    for seg in segment_commands_with_offset_and_config(text, base, config) {
        if seg.argv.is_empty() || seg.span.start() > off || off >= seg.span.end() {
            continue;
        }
        // A partial (unclosed-delimiter) command has no trustworthy word
        // layout; claiming inertness there could hide a real reference.
        if seg.is_partial {
            return false;
        }
        let Some(spec) = registry.get(seg.name()) else {
            return false;
        };
        // A clause list holds bodies no `ArgRole` describes — be conservative.
        if spec.case_list.is_some() {
            return false;
        }
        let arg_texts: Vec<&str> = seg.texts.iter().skip(1).map(String::as_str).collect();
        let mut script_indices: Vec<usize> = Vec::new();
        for role in [ArgRole::Body, ArgRole::Expr, ArgRole::LambdaLiteral] {
            script_indices.extend(registry.arg_indices_for_role(seg.name(), &arg_texts, role));
        }
        for (arg_idx, tok) in seg.argv.iter().enumerate().skip(1) {
            if tok.span.start() > off || off >= tok.span.end() {
                continue;
            }
            match tok.kind {
                // A braced word: script-bearing → descend; data → inert.
                TokenType::Str => {
                    if !script_indices.contains(&(arg_idx - 1)) {
                        return true;
                    }
                    let (start, end) = brace_content(full_source, *tok);
                    let Some(inner) = full_source.get(start as usize..end as usize) else {
                        return false;
                    };
                    return classify(full_source, inner, start, off, registry, config, depth + 1);
                }
                // A `[…]` substitution is a fresh script wherever it appears.
                TokenType::Cmd => {
                    let (start, end) = brace_content(full_source, *tok);
                    let Some(inner) = full_source.get(start as usize..end as usize) else {
                        return false;
                    };
                    return classify(full_source, inner, start, off, registry, config, depth + 1);
                }
                _ => return false,
            }
        }
        return false;
    }
    false
}

/// The `[start, end)` byte range of a braced / bracketed token's **content**,
/// honouring the inner-end token convention (`Token.end` is the last inner
/// character, except for an empty `{}` / `[]` where it sits on the closer).
fn brace_content(source: &str, tok: tcl_lexer::Token) -> (u32, u32) {
    let start = tok.span.start().saturating_add(1);
    let end = tok.span.end().saturating_sub(1);
    if end <= start || end as usize > source.len() {
        return (start, start);
    }
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> &'static CommandRegistry {
        tcl_registry::registry_for_dialect("tcl8.6")
    }

    fn offset_of(src: &str, needle: &str) -> u32 {
        u32::try_from(src.find(needle).expect("needle present")).expect("fits u32")
    }

    /// TP: the `$level` inside the audit's own inert comment.
    #[test]
    fn a_dollar_ref_inside_a_comment_is_inert() {
        let src = "proc p {} {\n    set level 42\n    # so $level ...\n}\n";
        assert!(offset_in_comment(src, offset_of(src, "$level")));
    }

    /// TN: a `#` that is not in command position is ordinary text, and a `#`
    /// inside a quoted word is not a comment either.
    #[test]
    fn a_non_command_position_hash_is_not_a_comment() {
        let src = "puts a#b $v\n";
        assert!(!offset_in_comment(src, offset_of(src, "$v")));
        let quoted = "puts \"a # b $v\"\n";
        assert!(!offset_in_comment(quoted, offset_of(quoted, "$v")));
        // A trailing comment after `;` *is* one.
        let trailing = "set x 1 ;# note $v\n";
        assert!(offset_in_comment(trailing, offset_of(trailing, "$v")));
    }

    /// TP: a braced *data* word — `set`'s value argument.
    #[test]
    fn a_dollar_ref_inside_a_data_brace_is_inert() {
        let src = "set t {plain $level here}\n";
        assert!(offset_in_data_brace(
            src,
            offset_of(src, "$level"),
            reg(),
            "tcl8.6"
        ));
        // Nested inside a proc body, which is descended into.
        let nested = "proc p {} {\n    set t {plain $level here}\n}\n";
        assert!(offset_in_data_brace(
            nested,
            offset_of(nested, "$level"),
            reg(),
            "tcl8.6"
        ));
    }

    /// TN: every braced word that really does carry executable Tcl.
    #[test]
    fn a_dollar_ref_inside_a_script_brace_is_live() {
        for src in [
            "proc p {} {\n    puts $level\n}\n",
            "if {$level > 1} { puts hi }\n",
            "while {$level} { break }\n",
            "foreach i {1 2} { puts $level }\n",
            "set x [expr {$level * 2}]\n",
            "apply {{a} {puts $level}} 1\n",
            "eval {puts $level}\n",
        ] {
            assert!(
                !offset_in_data_brace(src, offset_of(src, "$level"), reg(), "tcl8.6"),
                "should be live: {src:?}"
            );
        }
    }

    /// TN: a `switch` clause list is a shape no `ArgRole` describes, so the
    /// walk abstains rather than claiming its bodies are data.
    #[test]
    fn a_clause_list_abstains() {
        let src = "switch $v {\n    a { puts $level }\n}\n";
        assert!(!offset_in_data_brace(
            src,
            offset_of(src, "$level"),
            reg(),
            "tcl8.6"
        ));
    }

    /// TN: an unquoted / quoted word substitutes, and an unknown command is
    /// never classified.
    #[test]
    fn unbraced_and_unknown_shapes_abstain() {
        let quoted = "set t \"plain $level here\"\n";
        assert!(!offset_in_data_brace(
            quoted,
            offset_of(quoted, "$level"),
            reg(),
            "tcl8.6"
        ));
        let unknown = "zzznotacommand {plain $level here}\n";
        assert!(!offset_in_data_brace(
            unknown,
            offset_of(unknown, "$level"),
            reg(),
            "tcl8.6"
        ));
    }
}
