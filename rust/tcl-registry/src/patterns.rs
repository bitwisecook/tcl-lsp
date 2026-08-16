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

use crate::abbrev::PrefixMatching;
use crate::hover::OptionSpec;

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

/// Registry context supplied to a call-specific pattern resolver.
///
/// The caller filters [`options`](Self::options) for the document's resolved
/// profile before invoking the resolver.  Keeping the option grammar and the
/// reserved positional suffix together means an option-selected pattern
/// layout cannot accidentally recognise a switch that this Tcl release does
/// not have, nor scan a mandatory operand merely because it looks like one.
#[derive(Debug, Clone, Copy)]
pub struct PatternArgResolverContext<'a> {
    /// Available option descriptors in declaration order.
    pub options: &'a [&'static OptionSpec],
    /// Mandatory trailing operands excluded from the leading option scan.
    pub reserved_trailing_words: usize,
}

/// Resolve a command's call-specific pattern arguments.
///
/// Resolver callbacks receive the profile-filtered option metadata and the
/// command's reserved trailing operand boundary.  The callback must use both
/// rather than carrying a private option table or inferring the positional
/// boundary itself.
pub type PatternArgResolver = for<'a> fn(&[&str], PatternArgResolverContext<'a>) -> Vec<PatternArg>;

/// Resolve an option word against profile-filtered descriptor references.
///
/// Static option tables use [`crate::spec::resolve_option_prefix`]; this
/// counterpart preserves the same exact-or-unique-prefix rule after a
/// profile has selected only the options this invocation can actually use.
#[must_use]
pub(crate) fn resolve_available_option_prefix<'a>(
    options: &'a [&crate::hover::OptionSpec],
    word: &str,
) -> Option<&'a crate::hover::OptionSpec> {
    resolve_available_option_prefix_with(options, word, PrefixMatching::Enabled)
}

/// [`resolve_available_option_prefix`] with the command's declared prefix
/// policy.  A profile has already removed unavailable options from `options`,
/// so this one walk preserves the remaining exact-or-unique abbreviation
/// grammar without accidentally restoring an option from another release.
#[must_use]
pub(crate) fn resolve_available_option_prefix_with<'a>(
    options: &'a [&crate::hover::OptionSpec],
    word: &str,
    prefix_matching: PrefixMatching,
) -> Option<&'a crate::hover::OptionSpec> {
    if let Some(option) = options.iter().copied().find(|option| option.matches(word)) {
        return Some(option);
    }
    if !prefix_matching.accepts_prefixes() || !word.starts_with('-') || word.len() < 2 {
        return None;
    }
    let mut found = None;
    for option in options.iter().copied() {
        if std::iter::once(option.name)
            .chain(option.aliases.iter().copied())
            .any(|spelling| {
                spelling.starts_with(word)
                    && option
                        .min_abbrev
                        .is_none_or(|minimum| word.len() >= usize::from(minimum))
            })
        {
            match found {
                None => found = Some(option),
                Some(previous) if std::ptr::eq(previous, option) => {}
                Some(_) => return None,
            }
        }
    }
    found
}

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
