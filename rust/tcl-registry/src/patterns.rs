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

//! Pattern- and format-string language classification.
//!
//! These two enums tag a command (or subcommand) argument that carries
//! an embedded mini-language — a glob/regex pattern, or a
//! `format`/`clock`/`binary`/`regsub` format string — so the LSP can
//! emit *sub-tokens* (semantic-token splitting inside the string
//! literal) and run pattern-specific validation.

/// Kind of pattern language an argument uses, for semantic tokens and
/// validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternType {
    /// Glob pattern (`string match`, `glob`, `lsearch` default,
    /// `switch` default).
    Glob,
    /// Regular expression (`regexp`, `regsub`, `lsearch -regexp`,
    /// `switch -regexp`).
    Regex,
}

/// One concrete pattern-bearing argument of an invocation.
///
/// Most commands use one static [`PatternType`] plus an [`ArgRole::Pattern`]
/// position. Commands such as `lsearch` select the language with an option,
/// so their registry resolver returns this paired fact rather than forcing an
/// LSP consumer to understand `-regexp` itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternArg {
    /// Index into the post-head argument list.
    pub index: u8,
    /// Embedded language accepted at this position.
    pub kind: PatternType,
}

/// Resolve a command's call-specific pattern arguments.
pub type PatternArgResolver = fn(&[&str]) -> Vec<PatternArg>;

impl PatternType {
    /// Stable lowercase tag (`"glob"` / `"regex"`) — used by the audit
    /// dumper so both sides normalise identically.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Glob => "glob",
            Self::Regex => "regex",
        }
    }
}

/// Kind of format string an argument uses, for inlay-hint parsing and
/// semantic tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatType {
    /// `printf`-style conversion string (`format`, `scan`).
    Sprintf,
    /// `clock` format/scan field string (`clock format`, `clock scan`).
    Clock,
    /// `binary` format/scan field string (`binary format`, `binary scan`).
    Binary,
    /// `regsub` replacement string (`\&` / `\N` backrefs).
    Regsub,
}

impl FormatType {
    /// Stable lowercase tag
    /// (`"sprintf"` / `"clock"` / `"binary"` / `"regsub"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sprintf => "sprintf",
            Self::Clock => "clock",
            Self::Binary => "binary",
            Self::Regsub => "regsub",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::CommandRegistry;

    #[test]
    fn enum_tags_have_expected_str_values() {
        assert_eq!(PatternType::Glob.as_str(), "glob");
        assert_eq!(PatternType::Regex.as_str(), "regex");
        assert_eq!(FormatType::Sprintf.as_str(), "sprintf");
        assert_eq!(FormatType::Regsub.as_str(), "regsub");
    }

    /// `regexp` / `regsub` carry `PatternType::Regex`.
    #[test]
    fn regexp_and_regsub_are_regex_patterns() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            registry.get("regexp").unwrap().pattern_type,
            Some(PatternType::Regex)
        );
        assert_eq!(
            registry.get("regsub").unwrap().pattern_type,
            Some(PatternType::Regex)
        );
        // A non-pattern command stays `None`.
        assert_eq!(registry.get("puts").unwrap().pattern_type, None);
    }
}
