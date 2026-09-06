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
use crate::documentation::{DocumentationAnnotation, DocumentationCarrier, DocumentationExample};
use crate::hover::OptionSpec;

/// One worked example whose carrier is the command that owns the embedded
/// mini-language argument.
macro_rules! templated {
    ($code:literal; carrier ($cline:literal, $cneedle:literal); $(($line:literal, $needle:literal, $label:literal)),+ $(,)?) => {
        {
            const ANNOTATIONS: &[DocumentationAnnotation] =
                &[$(DocumentationAnnotation::new($line, $needle, $label)),+];
            DocumentationExample::with_carrier($code, DocumentationCarrier::new($cline, $cneedle), ANNOTATIONS)
        }
    };
}

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
    /// Every pattern language, in declaration order.
    pub const ALL: &'static [Self] = &[Self::Glob, Self::Regex];

    /// Stable lowercase tag (`"glob"` / `"regex"`) — used by the audit
    /// dumper so both sides normalise identically.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Glob => "glob",
            Self::Regex => "regex",
        }
    }

    /// Registry-owned program showing a shipped command whose `pattern_type`
    /// is this language, the pattern word it reads, and how the LSP then
    /// tokenises and checks that word. The carrier is the pattern-taking
    /// command. This exhaustive match is the compile gate for pattern-language
    /// documentation.
    #[must_use]
    pub const fn example(self) -> DocumentationExample {
        match self {
            Self::Glob => {
                templated!("set name [file tail $path]\nset is_script [string match *.tcl $name]\nif {$is_script} { source $path }"; carrier (1, "string match"); (1, "string match", "takes the pattern as its first argument"), (1, "*.tcl", "is tokenised as wildcards, never as regex syntax"), (2, "$is_script", "carries the match result"))
            }
            Self::Regex => {
                templated!("set version 8.6.14\nregexp {^(\\d+)\\.(\\d+)} $version -> major minor\nputs $major.$minor"; carrier (1, "regexp"); (1, "regexp", "takes the pattern as its first non-option argument"), (1, "{^(\\d+)\\.(\\d+)}", "is highlighted as regex syntax and checked for ReDoS (W303) and quoting pitfalls (W306)"), (2, "$major.$minor", "uses the captured groups"))
            }
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
    /// Every format-string family, in declaration order.
    pub const ALL: &'static [Self] = &[Self::Sprintf, Self::Clock, Self::Binary, Self::Regsub];

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

    /// Registry-owned program showing a shipped command whose
    /// `format_string_type` is this family, the template word it reads, and
    /// how the LSP then tokenises, hints, or version-gates that word. The
    /// carrier is the template-taking command. This exhaustive match is the
    /// compile gate for format-family documentation.
    #[must_use]
    pub const fn example(self) -> DocumentationExample {
        match self {
            Self::Sprintf => {
                templated!("set line [format {%-10s %5d} $name $count]\nputs $line"; carrier (0, "format"); (0, "format", "takes the template as its first argument"), (0, "%-10s %5d", "each conversion is highlighted and hinted (str, int); one newer than the target Tcl raises W138"), (1, "$line", "holds the rendered text"))
            }
            Self::Clock => {
                templated!("set stamp [clock format [clock seconds] -format {%Y-%m-%d %b}]\nputs $stamp"; carrier (0, "clock format"); (0, "clock format", "takes the template after -format"), (0, "%Y-%m-%d %b", "fields are highlighted as clock conversions: %b is a month name here, not format's %b, so W138 stays quiet"), (1, "$stamp", "holds the rendered timestamp"))
            }
            Self::Binary => {
                templated!("set packet [binary format Sa4 $port $tag]\nbinary scan $packet Sa4 port tag"; carrier (0, "binary format"); (0, "binary format", "takes the template as its first argument"), (0, "Sa4", "is tokenised as cursor fields, one per packed value, never as printf conversions"), (1, "binary scan", "reads the same field language to unpack the bytes"))
            }
            Self::Regsub => {
                templated!("regsub -all {(\\w+)@(\\w+)} $text {\\2 at \\1} swapped\nputs $swapped"; carrier (0, "regsub"); (0, "regsub", "takes the replacement template after its pattern"), (0, "{\\2 at \\1}", "is read as a replacement: \\1 and \\2 are backreferences, not printf conversions"), (1, "$swapped", "holds the rewritten text"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::CommandRegistry;
    use crate::types::example_checks::assert_examples_valid;

    #[test]
    fn every_pattern_type_has_a_distinct_source_aligned_example() {
        let examples: Vec<_> = PatternType::ALL
            .iter()
            .map(|&kind| (format!("{kind:?}"), kind.example()))
            .collect();
        assert_examples_valid("PatternType", &examples);
    }

    #[test]
    fn every_format_type_has_a_distinct_source_aligned_example() {
        let examples: Vec<_> = FormatType::ALL
            .iter()
            .map(|&kind| (format!("{kind:?}"), kind.example()))
            .collect();
        assert_examples_valid("FormatType", &examples);
    }

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
