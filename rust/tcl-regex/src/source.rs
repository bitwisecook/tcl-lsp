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

//! Source component ranges reported by successful native pattern compilers.

use std::ops::Range;

/// The semantic role selected by the pattern compiler.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PatternRole {
    Options,
    Comment,
    Anchor,
    CharacterClass,
    Alternation,
    CaptureOpen,
    NoncaptureOpen,
    PositiveLookahead,
    NegativeLookahead,
    GroupClose,
    Quantifier,
    Escape,
    Backreference,
}

/// A non-literal pattern component. Ranges address the exact input to the
/// corresponding entry point, in C character units or Jim original bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatternComponent {
    pub range: Range<usize>,
    pub role: PatternRole,
}

/// Compile the C regular expression and retain its original character-unit
/// ranges. Compilation errors yield no partial claims.
///
/// # Errors
/// Returns the same syntax error as the C pattern compiler.
pub fn are_components(
    input: &[u32],
    flags: i32,
) -> Result<Vec<PatternComponent>, crate::ErrorCode> {
    let mut components = Vec::new();
    crate::parser::compile_traced(input, flags, Some(&mut components))?;
    Ok(components)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defs::REG_ADVANCED, jim};

    #[test]
    fn the_compiler_owns_directors_escapes_and_aggregate_ranges() {
        let source = "é(a[[:alpha:]]){1,2}?\\1\\b(?#end)";
        let units: Vec<_> = source.chars().map(u32::from).collect();
        let components = are_components(&units, REG_ADVANCED).unwrap();
        let observed: Vec<_> = components
            .into_iter()
            .map(|item| {
                let spelling: String = units[item.range]
                    .iter()
                    .copied()
                    .map(|u| char::from_u32(u).unwrap())
                    .collect();
                (spelling, item.role)
            })
            .collect();
        assert_eq!(
            observed
                .iter()
                .map(|(s, r)| (s.as_str(), *r))
                .collect::<Vec<_>>(),
            [
                ("(", PatternRole::CaptureOpen),
                ("[[:alpha:]]", PatternRole::CharacterClass),
                (")", PatternRole::GroupClose),
                ("{1,2}?", PatternRole::Quantifier),
                ("\\1", PatternRole::Backreference),
                ("\\b", PatternRole::Escape),
                ("(?#end)", PatternRole::Comment),
            ]
        );
        for text in ["***=a.*", "(?q)a.*"] {
            let units: Vec<_> = text.chars().map(u32::from).collect();
            assert_eq!(
                are_components(&units, REG_ADVANCED).unwrap(),
                [PatternComponent {
                    range: 0..4,
                    role: PatternRole::Options
                }]
            );
        }
        let invalid: Vec<_> = "(?>a)".chars().map(u32::from).collect();
        assert!(are_components(&invalid, REG_ADVANCED).is_err());
    }

    #[test]
    fn jim_reports_the_original_byte_extent_and_its_own_escape_grammar() {
        let text = "é(?:a|b)[[:alpha:]]{1,2}?\\1\0(z)";
        let observed = jim::Regex::components(text.as_bytes(), jim::Flags::new()).unwrap();
        assert_eq!(
            observed
                .into_iter()
                .map(|c| (&text[c.range], c.role))
                .collect::<Vec<_>>(),
            [
                ("(?:", PatternRole::NoncaptureOpen),
                ("|", PatternRole::Alternation),
                (")", PatternRole::GroupClose),
                ("[[:alpha:]]", PatternRole::CharacterClass),
                ("{1,2}?", PatternRole::Quantifier),
                ("\\1", PatternRole::Escape),
            ]
        );
        let text = " x # [discarded]\n ()";
        let parts =
            jim::Regex::components(text.as_bytes(), jim::Flags::new().with_expanded(true)).unwrap();
        assert_eq!(
            parts
                .into_iter()
                .map(|c| &text[c.range])
                .collect::<Vec<_>>(),
            ["(", ")"]
        );
        for pattern in [b"(?>a)".as_slice(), b"***=a.*"] {
            assert!(jim::Regex::components(pattern, jim::Flags::new()).is_err());
        }
    }
}
