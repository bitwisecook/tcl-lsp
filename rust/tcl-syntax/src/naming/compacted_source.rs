// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Newly authored short source names, independently of existing-name lookup.

/// One newly constructed short name under the complete selected source grammar.
/// It proves only scalar/reference and literal-word spelling, never uniqueness,
/// source ownership, lookup, Native binding or transformation permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredCompactName {
    name: String,
    config: tcl_lexer::LexerConfig,
}

impl AuthoredCompactName {
    /// The exact grammar used to construct this new spelling.
    #[must_use]
    pub fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }

    /// Consume this proposed spelling as new source data, without a name identity.
    #[must_use]
    pub fn into_name(self) -> String {
        self.name
    }
}

/// Length-first lowercase ASCII proposals: `a` through `z`, then `aa`, `ab`, ….
/// The caller retains its own scope, coverage, edit and observability purposes.
#[derive(Debug, Clone)]
pub struct AuthoredCompactNameSequence {
    config: tcl_lexer::LexerConfig,
    indices: Vec<u8>,
}

impl AuthoredCompactNameSequence {
    /// Retain the whole original grammar for every newly authored candidate.
    #[must_use]
    pub fn new(config: tcl_lexer::LexerConfig) -> Self {
        Self {
            config,
            indices: vec![0],
        }
    }

    /// Reject observed collisions while consuming at most `attempts` proposals.
    /// `None` preserves an unbounded authoring search. A false predicate means
    /// only no supplied observation rejected the spelling; it supplies no
    /// complete inventory, unused-name verdict, availability or edit permission.
    /// Grammar refusal is terminal and never replaced with another profile.
    #[must_use]
    pub fn next_avoiding_observed(
        &mut self,
        mut observed_collision: impl FnMut(&str) -> bool,
        attempts: Option<usize>,
    ) -> Option<AuthoredCompactName> {
        let mut remaining = attempts;
        loop {
            if let Some(left) = remaining.as_mut() {
                *left = left.checked_sub(1)?;
            }
            let name = self
                .indices
                .iter()
                .map(|index| char::from(b'a' + *index))
                .collect::<String>();
            self.advance();
            let spelling = crate::word_rules::lexical_ascii_source_word(&name, self.config)?;
            if spelling != name
                || super::scalar_source_reference(&name, self.config, false).is_none()
            {
                return None;
            }
            if !observed_collision(&name) {
                return Some(AuthoredCompactName {
                    name,
                    config: self.config,
                });
            }
        }
    }

    fn advance(&mut self) {
        for index in self.indices.iter_mut().rev() {
            if *index < 25 {
                *index += 1;
                return;
            }
            *index = 0;
        }
        self.indices.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_compact_names_preserve_length_order_and_observed_collisions() {
        let mut sequence = AuthoredCompactNameSequence::new(tcl_lexer::LexerConfig::default());
        assert_eq!(sequence.next_avoiding_observed(|_| false, Some(0)), None);
        assert_eq!(
            sequence
                .next_avoiding_observed(|_| false, Some(1))
                .unwrap()
                .into_name(),
            "a"
        );
        assert_eq!(
            sequence
                .next_avoiding_observed(|name| name != "z", Some(25))
                .unwrap()
                .into_name(),
            "z"
        );
        assert_eq!(
            sequence
                .next_avoiding_observed(|_| false, Some(1))
                .unwrap()
                .into_name(),
            "aa"
        );
        assert_eq!(
            sequence
                .next_avoiding_observed(|name| name != "az", Some(25))
                .unwrap()
                .into_name(),
            "az"
        );
        assert_eq!(
            sequence
                .next_avoiding_observed(|_| false, Some(1))
                .unwrap()
                .into_name(),
            "ba"
        );
        assert_eq!(sequence.next_avoiding_observed(|_| true, Some(1000)), None);
        assert!(
            sequence
                .next_avoiding_observed(|_| false, Some(1))
                .is_some()
        );
    }

    #[test]
    fn authored_compact_names_retain_custom_whole_grammar_without_lookup() {
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Content,
            base_offset: 41,
            base_line: 3,
            base_col: 7,
            ..tcl_lexer::LexerConfig::default()
        };
        let mut sequence = AuthoredCompactNameSequence::new(config);
        let name = sequence
            .next_avoiding_observed(
                |name| ["a", "b", "a::b", "a(b)", "$a", "é"].contains(&name),
                Some(3),
            )
            .unwrap();
        assert_eq!(name.config(), config);
        let spelling = name.into_name();
        assert_eq!(spelling, "c");
        assert_eq!(
            crate::word_rules::lexical_ascii_source_word(&spelling, config).as_deref(),
            Some("c")
        );
        assert_eq!(
            super::super::scalar_source_reference(&spelling, config, false).as_deref(),
            Some("$c")
        );
        assert_ne!(config, tcl_lexer::LexerConfig::default());
    }
}
