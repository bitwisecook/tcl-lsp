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
//! * A boolean word is rewritten **only** where the registry declares the
//!   role — [`tcl_registry::ArgRole::consumes_boolean`] is the single query,
//!   so every option that declares
//!   [`tcl_registry::ArgRole::Boolean`] is covered and a new one is covered
//!   the day it is declared (issue #1256). The word is consumed through
//!   `Tcl_GetBooleanFromObj` and its bytes are never otherwise observable. A
//!   value-definition site (`set flag yes`) keeps its bytes, because `$flag`
//!   may later meet `eq "yes"`, a `switch` arm, or a log line — `true` and
//!   `yes` are different strings even though both are truthy.
//! * `0`/`1` are also valid integers, so a
//!   [`tcl_registry::ArgRole::NumericOrBoolean`] position rewrites only the
//!   spellings that are unambiguously boolean: `-validate yes` normalises,
//!   `-validate 0` keeps its bytes because the command reads it as a number,
//!   and the `0/1` form has nothing safe to write there at all.
//! * The rewrite must hold across the document's target release range as well
//!   (issue #1257): the option word has to name the *same* option, carrying
//!   the *same* boolean role, in every release of the range. An option a
//!   later release removes, renames the prefix of, or re-roles is left alone.
//!
//! Both rewrites are idempotent: a canonical spelling resolves to itself, and
//! the configured boolean form maps to itself.

use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::hover::OptionSpec;
use tcl_registry::prelude::DialectSet;
use tcl_registry::{ArgRole, CommandRegistry, CommandSpec};

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

/// How much of the boolean vocabulary the formatter may rewrite at a value
/// position — decided by the position's **declared** [`ArgRole`], never by
/// what its value set happens to enumerate.
///
/// Ordered least- to most-permissive so a range check can `min` the releases'
/// verdicts and land on the most conservative one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BooleanSite {
    /// Not a boolean position at all: the bytes are data.
    None,
    /// Only the spellings that are unambiguously boolean. The position also
    /// accepts a number ([`ArgRole::NumericOrBoolean`]), so `0` and `1` belong
    /// to both languages and keep their bytes — and a numeric target form has
    /// nothing safe to write here.
    WordFormsOnly,
    /// Every spelling of the same truth value is interchangeable
    /// ([`ArgRole::Boolean`]): the word goes through `Tcl_GetBooleanFromObj`
    /// and its bytes are never otherwise observable.
    Interchangeable,
}

impl BooleanSite {
    /// The site a declared role describes.
    ///
    /// [`ArgRole::consumes_boolean`] is the single query for "is this word
    /// consumed as a boolean" (issue #1256) — reading it here is what makes
    /// every declared boolean option a rewrite site, and a newly declared one
    /// a site the day it lands, with no list to keep in step.
    fn of(role: Option<ArgRole>) -> Self {
        match role {
            Some(role) if role.consumes_boolean() => Self::Interchangeable,
            Some(ArgRole::NumericOrBoolean) => Self::WordFormsOnly,
            _ => Self::None,
        }
    }
}

/// Whether `word`'s bytes read as a number — the half of the boolean
/// vocabulary a [`BooleanSite::WordFormsOnly`] position consumes as a count
/// rather than a truth value.
///
/// Deliberately generous about what counts as numeric: only the boolean
/// table's own spellings ever reach here, of which `0` and `1` are the
/// numeric ones, and erring towards "numeric" errs towards leaving bytes
/// alone.
fn is_numeric_spelling(word: &str) -> bool {
    let digits = word.strip_prefix(['-', '+']).unwrap_or(word);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// The canonical spelling of a boolean `word` under `form` at `site`, or
/// `None` when the word is not an unambiguous boolean, already has the target
/// spelling, or the site does not license the rewrite.
///
/// tclsh-proof (8.6.16): `string is boolean tru` → `1` and `expr {tru ? 1 :
/// 0}` → `1`, so `tru` is the same truth value as `true`; `string is boolean
/// o` → `0`, so the one ambiguous prefix is not a boolean at all and keeps its
/// bytes.
fn canonical_boolean(word: &str, form: BooleanForm, site: BooleanSite) -> Option<String> {
    if site == BooleanSite::None {
        return None;
    }
    let (yes, no) = form.pair()?;
    let value = tcl_registry::abbrev::resolve_boolean(word)?;
    let target = if value { yes } else { no };
    // A numeric-or-boolean position reads `0`/`1` as an integer, so neither
    // the written word nor the replacement may be one: rewriting `-validate 0`
    // to `false`, or `-validate yes` to `1`, would move the value between the
    // two languages the position accepts.
    if site == BooleanSite::WordFormsOnly && (is_numeric_spelling(word) || is_numeric_spelling(target))
    {
        return None;
    }
    (target != word).then(|| target.to_owned())
}

/// Which keyword table of a command the version-range check should build.
#[derive(Debug, Clone, Copy)]
enum RangeTable<'a> {
    /// The ensemble's subcommand words.
    Subcommands,
    /// The command's own option words (a command with no subcommands).
    CommandOptions,
    /// The named subcommand's option words.
    SubcommandOptions(&'a str),
}

/// Whether `word` still resolves to `canonical` in **every** release of the
/// document's target range (issue #1257).
///
/// The target release has already answered `Unique(canonical)` — this is the
/// forward-compatibility half: a prefix unique today can become ambiguous
/// when a later Tcl adds a keyword (`string cat` arrived in 8.6.2 and
/// shortened what `string c…` could mean), and expanding it would rewrite
/// source that a newer interpreter reads differently.
///
/// An empty range (the default, and every vendor dialect with no core version
/// bit) means "no range was declared", so the target's own answer stands —
/// the pre-existing behaviour. A release that no longer carries the command
/// contributes an empty table, which can never vouch for the word, so the
/// rewrite is abandoned.
fn resolves_across_range(
    config: &FormatterConfig,
    cmd_name: &str,
    scope: RangeTable<'_>,
    word: &str,
    canonical: &str,
) -> bool {
    let releases = tcl_registry::version_range::core_releases_in(config.target_range);
    if releases.is_empty() {
        return true;
    }
    let empty =
        || tcl_registry::abbrev::KeywordTable::new(std::iter::empty(), PrefixMatching::Enabled);
    let tables: Vec<_> = releases
        .iter()
        .map(|release| {
            // Each release's table is filtered by *its own* dialect bit — a
            // keyword added in 9.0 is a candidate in 9.0's table and not in
            // 8.6's, which is what makes the range question meaningful.
            let bits = DialectSet::parse(release);
            let Some(spec) = tcl_registry::registry_for_dialect(release).get(cmd_name) else {
                return empty();
            };
            match scope {
                RangeTable::Subcommands => spec.subcommand_table(bits, None, None),
                RangeTable::CommandOptions => spec.option_table(bits, None, None),
                RangeTable::SubcommandOptions(name) => spec
                    .subcommands
                    .iter()
                    .find(|s| s.name == name)
                    .map_or_else(empty, |sub| sub.option_table(bits, None, None)),
            }
        })
        .collect();
    tcl_registry::abbrev::resolve_over_versions(&tables, word).unique() == Some(canonical)
}

/// The option specs `scope` names on `spec`, for a release other than the
/// document's own. Empty when that release's spec has no such table.
fn options_for_scope(spec: &'static CommandSpec, scope: RangeTable<'_>) -> &'static [OptionSpec] {
    match scope {
        // Subcommand *words* are not a value position; nothing to say.
        RangeTable::Subcommands => &[],
        RangeTable::CommandOptions => spec.options,
        RangeTable::SubcommandOptions(name) => spec
            .subcommands
            .iter()
            .find(|s| s.name == name)
            .map_or(&[][..], |sub| sub.options),
    }
}

/// The boolean site every release of the document's target range agrees on for
/// the value of `canonical` (issue #1257), starting from the target release's
/// own verdict in `target`.
///
/// The word already resolves to `canonical` in the target, and
/// [`resolves_across_range`] separately proves it still names that option in
/// every release. This is the other half: the *role* has to hold too. A
/// release that dropped the option, or that declares its value numeric-or-
/// boolean where the target declares it boolean, drags the verdict down to the
/// most conservative one — the formatter rewrites what is safe everywhere in
/// the range or it rewrites nothing.
///
/// An empty range (the default, and every vendor dialect with no core version
/// bit) means "no range was declared", so the target's own verdict stands.
fn boolean_site_across_range(
    config: &FormatterConfig,
    cmd_name: &str,
    scope: RangeTable<'_>,
    canonical: &str,
    target: BooleanSite,
) -> BooleanSite {
    let releases = tcl_registry::version_range::core_releases_in(config.target_range);
    if releases.is_empty() {
        return target;
    }
    releases.iter().fold(target, |site, release| {
        let bits = DialectSet::parse(release);
        let found = tcl_registry::registry_for_dialect(release)
            .get(cmd_name)
            .and_then(|spec| {
                options_for_scope(spec, scope)
                    .iter()
                    .find(|opt| opt.matches(canonical) && opt.supports_dialect(bits, spec.dialects))
                    .map(|opt| BooleanSite::of(opt.value_role()))
            });
        site.min(found.unwrap_or(BooleanSite::None))
    })
}

/// The option table a command's leading subcommand word selects, and where the
/// option scan starts.
struct OptionScope {
    /// The option specs to resolve `-option` words against.
    options: &'static [OptionSpec],
    /// The table's prefix-matching strictness.
    prefix: PrefixMatching,
    /// Which table the version-range check should rebuild per release.
    range_table: RangeTable<'static>,
    /// The first argument index the option scan looks at.
    start: usize,
}

/// Resolve an ensemble's subcommand word, pushing its expansion rewrite when
/// one applies, and return the option scope it selects.
///
/// `None` when the word is ambiguous or unknown — the formatter never guesses,
/// and it cannot know which option table applies either.
fn subcommand_scope(
    spec: &tcl_registry::CommandSpec,
    dialect: Option<DialectSet>,
    config: &FormatterConfig,
    cmd_name: &str,
    word: &str,
    out: &mut Vec<KeywordRewrite>,
) -> Option<OptionScope> {
    let canonical = spec
        .resolve_subcommand_word(word, dialect, None, None)
        .unique()?;
    if config.expand_abbreviations
        && canonical != word
        && resolves_across_range(config, cmd_name, RangeTable::Subcommands, word, canonical)
    {
        out.push(KeywordRewrite {
            index: 0,
            text: canonical.to_owned(),
        });
    }
    let sub = spec.subcommands.iter().find(|s| s.name == canonical);
    Some(OptionScope {
        options: sub.map_or(spec.options, |sub| sub.options),
        prefix: sub.map_or(spec.prefix_matching, |sub| sub.prefix_matching),
        range_table: sub.map_or(RangeTable::CommandOptions, |sub| {
            RangeTable::SubcommandOptions(sub.name)
        }),
        start: 1,
    })
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
    let scope = if spec.subcommands.is_empty() {
        OptionScope {
            options: spec.options,
            prefix: spec.prefix_matching,
            range_table: RangeTable::CommandOptions,
            start: 0,
        }
    } else {
        if !touchable(0) {
            return out;
        }
        match subcommand_scope(spec, dialect, config, cmd_name, &args[0], &mut out) {
            Some(scope) => scope,
            // Ambiguous or unknown: the formatter never guesses, and it
            // cannot know which option table applies either.
            None => return out,
        }
    };

    if scope.options.is_empty() {
        return out;
    }
    let options = scope.options;
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
        scope.prefix,
    );

    let mut i = scope.start;
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
        if config.expand_abbreviations
            && canonical != word
            && resolves_across_range(config, cmd_name, scope.range_table, word, canonical)
        {
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
        // The option's value word, when the registry *declares* the position
        // boolean (issue #1256) and every release of the target range agrees
        // about both the option and its role (issue #1257).
        let site = BooleanSite::of(opt.value_role());
        if consumed == 1
            && config.boolean_form != BooleanForm::Preserve
            && site != BooleanSite::None
            && touchable(i + 1)
            && resolves_across_range(config, cmd_name, scope.range_table, word, canonical)
            && let Some(value) = args.get(i + 1)
            && let Some(text) = canonical_boolean(
                value,
                config.boolean_form,
                boolean_site_across_range(config, cmd_name, scope.range_table, canonical, site),
            )
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
        let site = BooleanSite::Interchangeable;
        for (form, yes, no) in [
            (BooleanForm::TrueFalse, "true", "false"),
            (BooleanForm::YesNo, "yes", "no"),
            (BooleanForm::OnOff, "on", "off"),
            (BooleanForm::ZeroOne, "1", "0"),
        ] {
            assert_eq!(
                canonical_boolean("t", form, site).as_deref(),
                Some(yes),
                "{form:?}"
            );
            assert_eq!(
                canonical_boolean("fals", form, site).as_deref(),
                Some(no),
                "{form:?}"
            );
            // Already in the target form — idempotent.
            assert_eq!(canonical_boolean(yes, form, site), None, "{form:?}");
            assert_eq!(canonical_boolean(no, form, site), None, "{form:?}");
        }
        // `preserve` never rewrites.
        assert_eq!(canonical_boolean("yes", BooleanForm::Preserve, site), None);
        // A non-boolean, and the one ambiguous boolean prefix, abstain.
        assert_eq!(canonical_boolean("x", BooleanForm::TrueFalse, site), None);
        assert_eq!(canonical_boolean("o", BooleanForm::TrueFalse, site), None);
        // A position that is not a boolean site at all never rewrites.
        assert_eq!(
            canonical_boolean("yes", BooleanForm::TrueFalse, BooleanSite::None),
            None
        );
    }

    #[test]
    fn the_site_comes_from_the_declared_role() {
        assert_eq!(
            BooleanSite::of(Some(ArgRole::Boolean)),
            BooleanSite::Interchangeable
        );
        assert_eq!(
            BooleanSite::of(Some(ArgRole::NumericOrBoolean)),
            BooleanSite::WordFormsOnly
        );
        assert_eq!(BooleanSite::of(Some(ArgRole::Value)), BooleanSite::None);
        assert_eq!(BooleanSite::of(None), BooleanSite::None);
        // The whole role space is covered: exactly one role is a boolean
        // consumption site, and it is the one `consumes_boolean()` names.
        for role in ArgRole::ALL {
            assert_eq!(
                BooleanSite::of(Some(*role)) == BooleanSite::Interchangeable,
                role.consumes_boolean(),
                "{role:?}"
            );
        }
        // Most conservative wins when releases disagree.
        assert_eq!(
            BooleanSite::Interchangeable.min(BooleanSite::WordFormsOnly),
            BooleanSite::WordFormsOnly
        );
        assert_eq!(
            BooleanSite::WordFormsOnly.min(BooleanSite::None),
            BooleanSite::None
        );
    }

    /// A numeric-or-boolean position rewrites the word spellings and leaves
    /// the numeric ones alone.
    ///
    /// tclsh-proof (8.6.16): `expr {yes ? 1 : 0}` → `1`, so `yes` at such a
    /// position can only be the truth value — while `0` there is equally a
    /// count, and `string is integer 0` → `1`. Moving a value between the two
    /// languages the position accepts is the one thing the rewrite must not
    /// do, in either direction.
    #[test]
    fn a_numeric_or_boolean_site_rewrites_only_word_spellings() {
        let site = BooleanSite::WordFormsOnly;
        // TP: an unambiguously boolean spelling normalises.
        assert_eq!(
            canonical_boolean("yes", BooleanForm::TrueFalse, site).as_deref(),
            Some("true")
        );
        assert_eq!(
            canonical_boolean("of", BooleanForm::TrueFalse, site).as_deref(),
            Some("false")
        );
        // TN: `1`/`0` are numbers here — byte-identical round trip.
        assert_eq!(canonical_boolean("1", BooleanForm::TrueFalse, site), None);
        assert_eq!(canonical_boolean("0", BooleanForm::TrueFalse, site), None);
        // TN: the `0/1` form has nothing safe to write at such a position, so
        // even a word spelling is left alone.
        for word in ["yes", "no", "true", "off"] {
            assert_eq!(
                canonical_boolean(word, BooleanForm::ZeroOne, site),
                None,
                "{word}"
            );
        }
        // The same words at a purely boolean position do rewrite, including
        // to the numeric form.
        assert_eq!(
            canonical_boolean("1", BooleanForm::TrueFalse, BooleanSite::Interchangeable).as_deref(),
            Some("true")
        );
        assert_eq!(
            canonical_boolean("yes", BooleanForm::ZeroOne, BooleanSite::Interchangeable).as_deref(),
            Some("1")
        );
    }

    #[test]
    fn a_numeric_or_boolean_option_value_is_rewritten_through_the_registry() {
        // No core command declares a bare numeric-or-boolean *option* value
        // today, so the end-to-end path is exercised against a synthetic spec
        // — the point being that the formatter reads the declared role and
        // nothing else.
        use tcl_registry::hover::{OptionSpec, OptionValue};

        static OPTIONS: &[OptionSpec] = &[
            OptionSpec {
                name: "-validate",
                value: OptionValue::numeric_or_boolean("level"),
                detail: "A level count, or a boolean.",
                ..OptionSpec::DEFAULT
            },
            OptionSpec {
                name: "-strict",
                value: OptionValue::boolean(),
                detail: "Purely boolean.",
                ..OptionSpec::DEFAULT
            },
        ];
        let spec = CommandSpec {
            name: "g26probe",
            options: OPTIONS,
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(spec);
        let run = |args: &[&str], form: BooleanForm| {
            let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
            let dynamic = vec![false; owned.len()];
            rewrites_for_command(
                &registry,
                None,
                &config(false, form),
                "g26probe",
                &owned,
                &dynamic,
            )
            .into_iter()
            .map(|r| (r.index, r.text))
            .collect::<Vec<_>>()
        };
        // TP: a word spelling at the numeric-or-boolean position normalises.
        assert_eq!(
            run(&["-validate", "yes"], BooleanForm::TrueFalse),
            vec![(1, "true".to_owned())]
        );
        // TN: the numeric spelling is a count there, and keeps its bytes.
        assert!(run(&["-validate", "0"], BooleanForm::TrueFalse).is_empty());
        // The purely boolean option beside it rewrites either spelling.
        assert_eq!(
            run(&["-strict", "0"], BooleanForm::TrueFalse),
            vec![(1, "false".to_owned())]
        );
    }
}
