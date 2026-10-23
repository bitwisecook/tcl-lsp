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

//! Conditional variable-reference candidates in independently selected text.

use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, SubstFlags};
use tcl_lexer::{LexerConfig, SourceImage, Span};

/// Possible reference roots, in source order with duplicates retained.
/// This lexical inventory supplies no variable cell, read or execution proof.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubstitutionVariableCandidates {
    names: Vec<String>,
    unknown: bool,
}

impl SubstitutionVariableCandidates {
    /// Original reference names, without treating literal sigils as syntax.
    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Whether syntax, depth or byte-name coverage remained unavailable.
    #[must_use]
    pub const fn has_unknown_residual(&self) -> bool {
        self.unknown
    }

    /// Conditional membership; unknown coverage cannot establish absence.
    #[must_use]
    pub fn may_contain(&self, name: &str) -> bool {
        self.unknown || self.names.iter().any(|candidate| candidate == name)
    }
}

/// Scan independently supplied substitution text under its actual grammar.
/// Array-index children and nested command/expression text retain source order.
/// Braces in command text do not establish execution: these are MAY candidates.
/// Malformed syntax or unsupported byte names retain an explicit residual.
#[must_use]
pub fn substitution_variable_candidates(
    source: &[u8],
    config: LexerConfig,
) -> SubstitutionVariableCandidates {
    let mut out = SubstitutionVariableCandidates::default();
    let Ok(length) = u32::try_from(source.len()) else {
        out.unknown = true;
        return out;
    };
    let image = SourceImage::native(source);
    let mut pending = vec![(Span::new(0, length), 0u16)];
    let mut names = Vec::new();
    while let Some((span, depth)) = pending.pop() {
        if depth >= 256 {
            out.unknown = true;
            continue;
        }
        let Ok(arena) =
            ExecutablePartArena::decompose(image.clone(), span, SubstFlags::default(), config)
        else {
            out.unknown = true;
            continue;
        };
        for part in arena.all_parts() {
            match part.part {
                ExecutablePart::Variable { .. } => {
                    let name = arena
                        .bytes(part.span)
                        .and_then(|bytes| {
                            super::variable_reference_root_bytes(bytes, config)
                                .ok()
                                .flatten()
                        })
                        .and_then(|bytes| std::str::from_utf8(bytes).ok());
                    if let Some(name) = name {
                        out.unknown |= name.contains('\0');
                        names.push((part.span.start(), name.to_owned()));
                    } else {
                        out.unknown = true;
                    }
                }
                ExecutablePart::Command { body } => pending.push((body, depth + 1)),
                ExecutablePart::Expression { expression } => pending.push((expression, depth + 1)),
                ExecutablePart::ParseError(_) => out.unknown = true,
                ExecutablePart::Text(_) => {}
            }
        }
    }
    names.sort_by_key(|(offset, _)| *offset);
    out.names = names.into_iter().map(|(_, name)| name).collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_candidates_keep_index_children_order_duplicates_and_literal_names() {
        let config = LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
        let names = substitution_variable_candidates(
            br"$arr([foo $index])/$tail/$tail/${$literal}/${}",
            config,
        );
        assert!(!names.has_unknown_residual());
        assert_eq!(
            names.names(),
            ["arr", "index", "tail", "tail", "$literal", ""]
        );
        let escaped = substitution_variable_candidates(br"\$inert/\\$live", config);
        assert_eq!(escaped.names(), ["live"]);
    }

    #[test]
    fn configured_candidates_keep_release_closers_and_unavailable_residual() {
        for (profile, name) in [("tcl8.6", "a{b"), ("tcl9.1", "a{b}c")] {
            let config = LexerConfig::for_profile(tcl_dialect::DialectProfile::find(profile));
            let names = substitution_variable_candidates(b"${a{b}c}/$tail", config);
            assert_eq!(names.names(), [name, "tail"]);
            assert!(!names.has_unknown_residual());
        }
        let config = LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
        let malformed = substitution_variable_candidates(b"[missing $value", config);
        assert!(malformed.has_unknown_residual());
        assert!(malformed.may_contain("unrepresented"));
        let nul = substitution_variable_candidates(b"${x\0tail}", config);
        assert_eq!(nul.names(), ["x\0tail"]);
        assert!(nul.has_unknown_residual());
    }
}
