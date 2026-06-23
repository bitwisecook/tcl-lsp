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
    fn tags_match_python_enum_values() {
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
