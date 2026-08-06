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

//! Keyword normalisation for the formatter (#1232, #1233).
//!
//! Two rewrites, both driven by the registry's abbreviation model
//! ([`tcl_registry::abbrev`]) so the formatter can only ever change a word
//! the analyser already resolved the same way:
//!
//! * **expand abbreviations** — `string le` → `string length`,
//!   `lsearch -noc` → `lsearch -nocase`, for subcommand and option words
//!   whose table resolves `Unique`;
//! * **canonical boolean form** — every word at a boolean-role consumption
//!   site becomes the configured pair (`true`/`false` by default), so
//!   `-nocase 1`, `-strict yes`, and `-underline on` stop reading as three
//!   conventions.
//!
//! Both are word-for-word replacements computed per command; the caller
//! substitutes them at emit time so the formatter's span bookkeeping is
//! untouched.
//!
//! # Safety
//!
//! * Ambiguous and unknown words are left byte-for-byte alone — the
//!   formatter never guesses. `string l` keeps its bytes and its W145.
//! * Strict tables are never touched.
//! * Command names are never touched: Tcl does not prefix-match them.
//! * A dynamic word (`$sub`, `[pick]`, `{*}`-expanded) abstains.
//! * A boolean word is rewritten **only** where the registry declares
//!   [`tcl_registry::ArgRole::Boolean`] — the word is consumed through
//!   `Tcl_GetBooleanFromObj` and its bytes are never otherwise observable
//!   (issue #1256). A value-definition site (`set flag yes`) keeps its
//!   bytes, because `$flag` may later meet `eq "yes"`, a `switch` arm, or a
//!   log line — `true` and `yes` are different strings even though both are
//!   truthy.
//! * `0`/`1` are also valid integers, so a
//!   [`tcl_registry::ArgRole::NumericOrBoolean`] position is a distinct
//!   declared fact and abstains rather than guess.
//!
//! Both rewrites are idempotent: a canonical spelling resolves to itself, and
//! the configured boolean form maps to itself.

use tcl_registry::CommandRegistry;
use tcl_registry::abbrev::KeywordMatch;
use tcl_registry::hover::OptionSpec;
use tcl_registry::prelude::DialectSet;

use super::config::{BooleanForm, FormatterConfig};

/// A word the formatter would rewrite: the argument index (0-based, after the
/// command name) and the replacement text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeywordRewrite {
    /// 0-based index into the command's argument list.
    pub index: usize,
    /// The canonical text to emit in place of the written word.
    pub text: String,
}

/// Whether `word` is a plain literal the formatter may rewrite — no
/// substitution, no expansion, non-empty.
fn is_static_word(word: &str) -> bool {
    !word.is_empty()
        && !word.contains('$')
        && !word.contains('[')
        && !word.starts_with("{*}")
        && !word.contains('\\')
}

/// The canonical spelling of a boolean `word` under `form`, or `None` when
/// the word is not an unambiguous boolean or already has the target spelling.
fn canonical_boolean(word: &str, form: BooleanForm) -> Option<String> {
    let (yes, no) = form.pair()?;
    let value = tcl_registry::abbrev::resolve_boolean(word)?;
    let target = if value { yes } else { no };
    (target != word).then(|| target.to_owned())
}

/// Compute every keyword rewrite for one command invocation.
///
/// `args` are the written argument words (after the command name).
/// `dynamic` is parallel to `args`: `true` for a word the formatter must not
/// touch (a substitution, a `{*}` expansion, a braced/quoted word whose bytes
/// are data rather than a keyword).
pub(crate) fn rewrites_for_command(
    registry: &CommandRegistry,
    dialect: Option<DialectSet>,
    config: &FormatterConfig,
    cmd_name: &str,
    args: &[String],
    dynamic: &[bool],
) -> Vec<KeywordRewrite> {
    if !config.expand_abbreviations && config.boolean_form == BooleanForm::Preserve {
        return Vec::new();
    }
    let Some(spec) = registry.get(cmd_name) else {
        return Vec::new();
    };
    let touchable = |i: usize| {
        !dynamic.get(i).copied().unwrap_or(false) && args.get(i).is_some_and(|a| is_static_word(a))
    };

    let mut out: Vec<KeywordRewrite> = Vec::new();

    // The subcommand word, and the option table it selects.
    let mut options: &'static [OptionSpec] = spec.options;
    let mut start = 0usize;
    let mut sub_prefix = spec.prefix_matching;
    if !spec.subcommands.is_empty() {
        if !touchable(0) {
            return out;
        }
        let word = &args[0];
        match spec.resolve_subcommand_word(word, dialect, None, None) {
            KeywordMatch::Unique(canonical) => {
                if config.expand_abbreviations && canonical != word {
                    out.push(KeywordRewrite {
                        index: 0,
                        text: canonical.to_owned(),
                    });
                }
                if let Some(sub) = spec.subcommands.iter().find(|s| s.name == canonical) {
                    options = sub.options;
                    sub_prefix = sub.prefix_matching;
                }
            }
            // Ambiguous or unknown: the formatter never guesses, and it
            // cannot know which option table applies either.
            KeywordMatch::Ambiguous(_) | KeywordMatch::Unknown => return out,
        }
        start = 1;
    }

    if options.is_empty() {
        return out;
    }
    let table = tcl_registry::abbrev::KeywordTable::from_keywords(
        options
            .iter()
            .filter(|opt| opt.supports_dialect(dialect, spec.dialects))
            .flat_map(|opt| {
                std::iter::once(opt.name)
                    .chain(opt.aliases.iter().copied())
                    .map(move |name| tcl_registry::abbrev::Keyword {
                        name,
                        min_abbrev: opt.min_abbrev,
                    })
            }),
        sub_prefix,
    );

    let mut i = start;
    while i < args.len() {
        let word = args[i].as_str();
        if word == "--" {
            break;
        }
        if !word.starts_with('-') || word.len() < 2 || !touchable(i) {
            i += 1;
            continue;
        }
        // A negative-number literal is a positional value, not an option.
        let digits = word[1..].trim_start_matches('-');
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit() || c == '.') {
            i += 1;
            continue;
        }
        let Some(canonical) = table.resolve(word).unique() else {
            i += 1;
            continue;
        };
        if config.expand_abbreviations && canonical != word {
            out.push(KeywordRewrite {
                index: i,
                text: canonical.to_owned(),
            });
        }
        let Some(opt) = options.iter().find(|o| o.matches(canonical)) else {
            i += 1;
            continue;
        };
        let consumed = opt.value_word_count(args, i);
        // The option's value word, when the registry proves it is consumed as
        // a boolean and nothing else.
        if consumed == 1
            && config.boolean_form != BooleanForm::Preserve
            && opt.value_is_boolean()
            && touchable(i + 1)
            && let Some(value) = args.get(i + 1)
            && let Some(text) = canonical_boolean(value, config.boolean_form)
        {
            out.push(KeywordRewrite { index: i + 1, text });
        }
        i += 1 + consumed;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(expand: bool, form: BooleanForm) -> FormatterConfig {
        FormatterConfig {
            expand_abbreviations: expand,
            boolean_form: form,
            ..FormatterConfig::default()
        }
    }

    fn rewrites(cmd: &str, args: &[&str], config: &FormatterConfig) -> Vec<(usize, String)> {
        let registry = tcl_registry::registry_for_dialect("tcl8.6");
        let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        let dynamic = vec![false; owned.len()];
        rewrites_for_command(
            registry,
            DialectSet::parse("tcl8.6"),
            config,
            cmd,
            &owned,
            &dynamic,
        )
        .into_iter()
        .map(|r| (r.index, r.text))
        .collect()
    }

    #[test]
    fn a_unique_subcommand_prefix_expands() {
        let cfg = config(true, BooleanForm::Preserve);
        assert_eq!(
            rewrites("string", &["le", "$s"], &cfg),
            vec![(0, "length".to_owned())]
        );
        // Already canonical — idempotent, no rewrite.
        assert!(rewrites("string", &["length", "$s"], &cfg).is_empty());
    }

    #[test]
    fn an_ambiguous_or_unknown_subcommand_is_left_alone() {
        let cfg = config(true, BooleanForm::Preserve);
        assert!(rewrites("string", &["l", "$s"], &cfg).is_empty());
        assert!(rewrites("string", &["zzz", "$s"], &cfg).is_empty());
    }

    #[test]
    fn a_unique_option_prefix_expands() {
        let cfg = config(true, BooleanForm::Preserve);
        assert_eq!(
            rewrites("lsearch", &["-noc", "-al", "$x", "$p"], &cfg),
            vec![(0, "-nocase".to_owned()), (1, "-all".to_owned())]
        );
        // An ambiguous option prefix stays.
        assert!(rewrites("lsearch", &["-a", "$x", "$p"], &cfg).is_empty());
    }

    #[test]
    fn expansion_can_be_turned_off() {
        let cfg = config(false, BooleanForm::Preserve);
        assert!(rewrites("string", &["le", "$s"], &cfg).is_empty());
        assert!(rewrites("lsearch", &["-noc", "$x", "$p"], &cfg).is_empty());
    }

    #[test]
    fn a_dynamic_word_abstains() {
        let cfg = config(true, BooleanForm::Preserve);
        let registry = tcl_registry::registry_for_dialect("tcl8.6");
        let args = vec!["le".to_owned(), "$s".to_owned()];
        // Marked dynamic by the caller (a `{*}`-expanded word).
        let out = rewrites_for_command(
            registry,
            DialectSet::parse("tcl8.6"),
            &cfg,
            "string",
            &args,
            &[true, false],
        );
        assert!(out.is_empty());
    }

    #[test]
    fn a_command_name_is_never_rewritten() {
        // `str` is not `string` — the rewrite map is keyed on argument
        // indices only, and an unknown command yields nothing at all.
        let cfg = config(true, BooleanForm::Preserve);
        assert!(rewrites("str", &["length", "$s"], &cfg).is_empty());
    }

    #[test]
    fn boolean_words_normalise_to_the_configured_form() {
        for (form, yes, no) in [
            (BooleanForm::TrueFalse, "true", "false"),
            (BooleanForm::YesNo, "yes", "no"),
            (BooleanForm::OnOff, "on", "off"),
            (BooleanForm::ZeroOne, "1", "0"),
        ] {
            assert_eq!(
                canonical_boolean("t", form).as_deref(),
                Some(yes),
                "{form:?}"
            );
            assert_eq!(
                canonical_boolean("fals", form).as_deref(),
                Some(no),
                "{form:?}"
            );
            // Already in the target form — idempotent.
            assert_eq!(canonical_boolean(yes, form), None, "{form:?}");
            assert_eq!(canonical_boolean(no, form), None, "{form:?}");
        }
        // `preserve` never rewrites.
        assert_eq!(canonical_boolean("yes", BooleanForm::Preserve), None);
        // A non-boolean, and the one ambiguous boolean prefix, abstain.
        assert_eq!(canonical_boolean("x", BooleanForm::TrueFalse), None);
        assert_eq!(canonical_boolean("o", BooleanForm::TrueFalse), None);
    }
}
