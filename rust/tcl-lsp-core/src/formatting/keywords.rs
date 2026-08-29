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
use tcl_registry::{ArgRole, CommandRegistry, CommandSpec};

use super::config::{BooleanForm, FormatterConfig};
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::model::SurfaceQuery;

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
/// tclsh-proof (8.6.14): `string is boolean tru` → `1` and `expr {tru ? 1 :
/// 0}` → `1`, so `tru` is the same truth value as `true`; `string is boolean
/// o` → `0` (and `expr {o ? 1 : 0}` is an error), so the one ambiguous prefix
/// is not a boolean at all and keeps its bytes.
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
    if site == BooleanSite::WordFormsOnly
        && (is_numeric_spelling(word) || is_numeric_spelling(target))
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
/// An empty range (the default, and every vendor dialect with no core release
/// row) means "no range was declared", so the target's own answer stands — the
/// pre-existing behaviour. A release that no longer carries the command
/// contributes an empty table, which can never vouch for the word, so the
/// rewrite is abandoned. The keyword table `scope` names on `cmd_name`'s spec
/// in one specific `release`, filtered by that release's own dialect bit —
/// empty when `registry` has no such spec (or, for
/// [`RangeTable::SubcommandOptions`], no such named subcommand), which can
/// never vouch for any word.
///
/// Takes the release's registry as a parameter rather than reaching for
/// [`crate::context_for_dialect`] itself, so the range machinery can
/// be exercised against a synthetic spec in tests — the process-wide cache
/// only ever holds the real registry data, never a fixture (issue #1268).
fn release_table(
    release: &str,
    registry: &CommandRegistry,
    cmd_name: &str,
    scope: RangeTable<'_>,
) -> tcl_registry::abbrev::KeywordTable<'static> {
    // The release's own authoring point, derived from its environment
    // rather than re-parsed from the name (ledger C1/C2): for the five
    // core ladder names this is exactly the single release bit
    // `DialectProfile::find` produced, sweep-pinned in the registry model.
    let point = Some(crate::document_context_for_dialect(release).authoring_query());
    let empty =
        || tcl_registry::abbrev::KeywordTable::new(std::iter::empty(), PrefixMatching::Enabled);
    // Each release's table is filtered by *its own* dialect bit — a keyword
    // added in 9.0 is a candidate in 9.0's table and not in 8.6's, which is
    // what makes the range question meaningful.
    let Some(spec) = registry.get(cmd_name) else {
        return empty();
    };
    match scope {
        RangeTable::Subcommands => spec.subcommand_table(point, None, None),
        RangeTable::CommandOptions => spec.option_table(point, None, None),
        RangeTable::SubcommandOptions(name) => spec
            .subcommands
            .iter()
            .find(|s| s.name == name)
            .map_or_else(empty, |sub| sub.option_table(point, None, None)),
    }
}

/// Whether `word` resolves to `canonical` in one specific `release`'s table —
/// the single-release primitive [`resolves_across_range`] folds over the
/// whole target range, and [`positional_site_across_range`] reuses it to gate
/// a subcommand-scoped positional site exactly the way an option's value is
/// gated by its own `dialects` field (issue #1268).
fn resolves_in_release(
    release: &str,
    registry: &CommandRegistry,
    cmd_name: &str,
    scope: RangeTable<'_>,
    word: &str,
    canonical: &str,
) -> bool {
    release_table(release, registry, cmd_name, scope)
        .resolve(word)
        .unique()
        == Some(canonical)
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
/// An empty range (the default, and every vendor dialect with no core release
/// row) means "no range was declared", so the target's own answer stands — the
/// pre-existing behaviour: `releases` is already empty in that case
/// ([`tcl_registry::version_range::core_releases_in`]), and `.all()` over an
/// empty iterator is vacuously `true`. A release that no longer carries the
/// command contributes an empty table, which can never vouch for the word, so
/// the rewrite is abandoned.
fn resolves_across_range(
    releases: &[(&str, &CommandRegistry)],
    cmd_name: &str,
    scope: RangeTable<'_>,
    word: &str,
    canonical: &str,
) -> bool {
    releases.iter().all(|&(release, registry)| {
        resolves_in_release(release, registry, cmd_name, scope, word, canonical)
    })
}

/// The option specs `scope` names on `spec`, for a release other than the
/// document's own. Empty when that release's spec has no such table.
fn options_for_scope<'a>(spec: &'a CommandSpec, scope: RangeTable<'_>) -> &'a [OptionSpec] {
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
/// An empty `releases` (the default, and every vendor dialect with no core
/// release row) means "no range was declared", so the target's own verdict
/// stands — `fold` over an empty slice returns `target` unchanged.
fn boolean_site_across_range(
    releases: &[(&str, &CommandRegistry)],
    cmd_name: &str,
    scope: RangeTable<'_>,
    canonical: &str,
    target: BooleanSite,
) -> BooleanSite {
    releases.iter().fold(target, |site, &(release, registry)| {
        let point = Some(crate::document_context_for_dialect(release).authoring_query());
        let found = registry.get(cmd_name).and_then(|spec| {
            options_for_scope(spec, scope)
                .iter()
                .find(|opt| opt.matches(canonical) && opt.supports_dialect(point, spec.surface))
                .map(|opt| BooleanSite::of(opt.value_role()))
        });
        site.min(found.unwrap_or(BooleanSite::None))
    })
}

/// The [`BooleanSite`] [`CommandRegistry::arg_indices_for_role`] declares at
/// `index` of a call to `cmd_name` with `args` — the one place a release's own
/// verdict comes from, whether `index` is a plain positional word, a
/// repeated-tail word, or an option's value (issue #1268).
///
/// Reads straight off the registry's uniform role query rather than
/// re-deriving the fact from an option lookup, so a spec that declares
/// [`ArgRole::Boolean`]/[`ArgRole::NumericOrBoolean`] on a **positional**
/// argument is covered the day it is declared, with no option-scan special
/// case. `args` must already have the subcommand word and any recognised
/// option word in their *canonical* spelling — `arg_indices_for_role`'s
/// option-value matching is an exact-name check, so an abbreviated `-noc`
/// would otherwise go unrecognised where the option scan's own abbreviation
/// table already resolved it.
fn declared_boolean_site_at(
    registry: &CommandRegistry,
    cmd_name: &str,
    args: &[&str],
    index: usize,
) -> BooleanSite {
    if registry
        .arg_indices_for_role(cmd_name, args, ArgRole::Boolean)
        .contains(&index)
    {
        BooleanSite::Interchangeable
    } else if registry
        .arg_indices_for_role(cmd_name, args, ArgRole::NumericOrBoolean)
        .contains(&index)
    {
        BooleanSite::WordFormsOnly
    } else {
        BooleanSite::None
    }
}

/// The boolean site every release of the document's target range agrees on
/// for a **positional** argument index — [`boolean_site_across_range`]'s
/// counterpart for a plain positional or repeated-tail word rather than an
/// option's value (issue #1268).
///
/// [`CommandRegistry::arg_indices_for_role`] has no dialect awareness of its
/// own: it resolves a subcommand word by exact-or-abbreviated match with no
/// dialect filter at all, so a subcommand a given release does not carry
/// would still be "found" by an exact match on `canonical_args[0]`. The
/// [`resolves_in_release`] check below, over [`RangeTable::Subcommands`], is
/// what actually enforces the per-release dialect gate here — the positional
/// counterpart of an option's own `supports_dialect` check in
/// [`boolean_site_across_range`]. A command with no subcommands has no such
/// word to gate: a positional role declared directly on the `CommandSpec` is
/// release-invariant static data, so only the command's own presence in the
/// release can disagree.
///
/// An empty `releases` (the default, and every vendor dialect with no core
/// release row) means "no range was declared", so the target's own verdict
/// stands, exactly as [`boolean_site_across_range`] does.
fn positional_site_across_range(
    releases: &[(&str, &CommandRegistry)],
    cmd_name: &str,
    canonical_args: &[String],
    index: usize,
    target: BooleanSite,
) -> BooleanSite {
    let arg_refs: Vec<&str> = canonical_args.iter().map(String::as_str).collect();
    let sub_word = canonical_args.first().map(String::as_str);
    releases.iter().fold(target, |site, &(release, registry)| {
        let Some(release_spec) = registry.get(cmd_name) else {
            return BooleanSite::None;
        };
        if !release_spec.subcommands.is_empty() {
            let Some(word) = sub_word else {
                return BooleanSite::None;
            };
            if !resolves_in_release(
                release,
                registry,
                cmd_name,
                RangeTable::Subcommands,
                word,
                word,
            ) {
                return BooleanSite::None;
            }
        }
        site.min(declared_boolean_site_at(
            registry, cmd_name, &arg_refs, index,
        ))
    })
}

/// The facts every boolean-rewrite helper below needs about the current call,
/// bundled so passing them around does not blow past a sane argument count —
/// [`scan_options`] and [`push_positional_boolean_rewrites`] both take one of
/// these plus only what is specific to their own half of the scan.
struct RewriteCtx<'a> {
    registry: &'a CommandRegistry,
    dialect: Option<SurfaceQuery<'a>>,
    /// The command's own dialect set, for the option table's
    /// `supports_dialect` filter — a `SubCommand` inherits it via `None`.
    spec_surface: Option<&'static [SpecSurface]>,
    config: &'a FormatterConfig,
    releases: &'a [(&'a str, &'a CommandRegistry)],
    cmd_name: &'a str,
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
/// one applies, and return the option scope it selects along with the word's
/// canonical spelling — the caller seeds its canonicalized-argument copy from
/// this even when `expand_abbreviations` never pushes the rewrite, since the
/// positional boolean pass (issue #1268) needs it to name the same subcommand
/// [`CommandRegistry::arg_indices_for_role`] would resolve by exact match.
///
/// `None` when the word is ambiguous or unknown — the formatter never guesses,
/// and it cannot know which option table applies either.
fn subcommand_scope(
    spec: &tcl_registry::CommandSpec,
    dialect: Option<SurfaceQuery<'_>>,
    config: &FormatterConfig,
    releases: &[(&str, &CommandRegistry)],
    cmd_name: &str,
    word: &str,
    out: &mut Vec<KeywordRewrite>,
) -> Option<(OptionScope, &'static str)> {
    let canonical = spec
        .resolve_subcommand_word(word, dialect, None, None)
        .unique()?;
    if config.expand_abbreviations
        && canonical != word
        && resolves_across_range(releases, cmd_name, RangeTable::Subcommands, word, canonical)
    {
        out.push(KeywordRewrite {
            index: 0,
            text: canonical.to_owned(),
        });
    }
    let sub = spec.subcommands.iter().find(|s| s.name == canonical);
    let scope = OptionScope {
        options: sub.map_or(spec.options, |sub| sub.options),
        prefix: sub.map_or(spec.prefix_matching, |sub| sub.prefix_matching),
        range_table: sub.map_or(RangeTable::CommandOptions, |sub| {
            RangeTable::SubcommandOptions(sub.name)
        }),
        start: 1,
    };
    Some((scope, canonical))
}

/// The option-word half of [`rewrites_for_command`]: expands an abbreviated
/// option name (issues #1232/#1233) and classifies each option's value word
/// as a boolean site by the same [`declared_boolean_site_at`] query the
/// positional pass uses (issue #1268), preserving the dialect and
/// target-range gates (#1256/#1257) around it.
///
/// Every word index an option consumes as its value is marked `claimed`, so
/// [`push_positional_boolean_rewrites`] never reclassifies it as a bare
/// positional. Mutates `canonical_args` in place: every recognised option
/// word is replaced by its canonical spelling so the positional pass's
/// exact-match role query can still see an abbreviated `-noc`.
fn scan_options(
    ctx: &RewriteCtx<'_>,
    scope: &OptionScope,
    args: &[String],
    touchable: &impl Fn(usize) -> bool,
    canonical_args: &mut [String],
    claimed: &mut [bool],
    out: &mut Vec<KeywordRewrite>,
) {
    if scope.options.is_empty() {
        return;
    }
    let registry = ctx.registry;
    let config = ctx.config;
    let releases = ctx.releases;
    let cmd_name = ctx.cmd_name;
    let options = scope.options;
    let table = tcl_registry::abbrev::KeywordTable::from_keywords(
        options
            .iter()
            .filter(|opt| opt.supports_dialect(ctx.dialect, ctx.spec_surface))
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
        canonical.clone_into(&mut canonical_args[i]);
        if config.expand_abbreviations
            && canonical != word
            && resolves_across_range(releases, cmd_name, scope.range_table, word, canonical)
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
        for value_idx in (i + 1)..(i + 1 + consumed).min(claimed.len()) {
            claimed[value_idx] = true;
        }
        // The option's value word, when the registry *declares* the position
        // boolean (issue #1256, driven from `arg_indices_for_role` per issue
        // #1268) and every release of the target range agrees about both the
        // option and its role (issue #1257).
        if consumed == 1 && config.boolean_form != BooleanForm::Preserve && touchable(i + 1) {
            let value_index = i + 1;
            let canonical_refs: Vec<&str> = canonical_args.iter().map(String::as_str).collect();
            let target_site =
                declared_boolean_site_at(registry, cmd_name, &canonical_refs, value_index);
            if target_site != BooleanSite::None
                && resolves_across_range(releases, cmd_name, scope.range_table, word, canonical)
                && let Some(value) = args.get(value_index)
                && let Some(text) = canonical_boolean(
                    value,
                    config.boolean_form,
                    boolean_site_across_range(
                        releases,
                        cmd_name,
                        scope.range_table,
                        canonical,
                        target_site,
                    ),
                )
            {
                out.push(KeywordRewrite {
                    index: value_index,
                    text,
                });
            }
        }
        i += 1 + consumed;
    }
}

/// The positional / repeated-tail half of the gap (issue #1268):
/// [`CommandRegistry::arg_indices_for_role`] answers uniformly across
/// positional roles, repeated tails, and option values; [`scan_options`]
/// already claimed every option-value index, so whatever boolean-role index
/// it did *not* claim is a genuine positional word.
fn push_positional_boolean_rewrites(
    ctx: &RewriteCtx<'_>,
    args: &[String],
    canonical_args: &[String],
    claimed: &[bool],
    touchable: &impl Fn(usize) -> bool,
    out: &mut Vec<KeywordRewrite>,
) {
    if ctx.config.boolean_form == BooleanForm::Preserve {
        return;
    }
    let registry = ctx.registry;
    let cmd_name = ctx.cmd_name;
    let canonical_refs: Vec<&str> = canonical_args.iter().map(String::as_str).collect();
    let mut candidates: Vec<(usize, BooleanSite)> = registry
        .arg_indices_for_role(cmd_name, &canonical_refs, ArgRole::Boolean)
        .into_iter()
        .map(|i| (i, BooleanSite::Interchangeable))
        .collect();
    candidates.extend(
        registry
            .arg_indices_for_role(cmd_name, &canonical_refs, ArgRole::NumericOrBoolean)
            .into_iter()
            .map(|i| (i, BooleanSite::WordFormsOnly)),
    );
    for (index, target_site) in candidates {
        if claimed.get(index).copied().unwrap_or(false) || !touchable(index) {
            continue;
        }
        let site = positional_site_across_range(
            ctx.releases,
            cmd_name,
            canonical_args,
            index,
            target_site,
        );
        if site == BooleanSite::None {
            continue;
        }
        let Some(value) = args.get(index) else {
            continue;
        };
        if let Some(text) = canonical_boolean(value, ctx.config.boolean_form, site) {
            out.push(KeywordRewrite { index, text });
        }
    }
}

/// Compute every keyword rewrite for one command invocation.
///
/// `args` are the written argument words (after the command name).
/// `dynamic` is parallel to `args`: `true` for a word the formatter must not
/// touch (a substitution, a `{*}` expansion, a braced/quoted word whose bytes
/// are data rather than a keyword).
pub(crate) fn rewrites_for_command(
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
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

    // The registry packs the document's target range spans (issue #1257),
    // computed once so every per-release check below — subcommand words,
    // option words, and now positional/repeated-tail boolean sites (issue
    // #1268) — walks the same list.
    let release_names = tcl_registry::version_range::core_releases_in(config.target_range());
    let releases: Vec<(&str, &CommandRegistry)> = release_names
        .iter()
        .map(|&release| (release, crate::registry_for_dialect(release)))
        .collect();

    let mut out: Vec<KeywordRewrite> = Vec::new();
    // A copy of `args` used only to query `CommandRegistry::arg_indices_for_role`
    // (issue #1268): the subcommand word and any recognised option word are
    // replaced by their canonical spelling so the registry's exact-match role
    // query sees an abbreviated `-noc`/subcommand prefix the way the option
    // scan's own abbreviation table already did. Genuine positional words are
    // never touched here.
    let mut canonical_args: Vec<String> = args.to_vec();
    // Argument indices the option scan below already classified (an option's
    // value word) — the positional pass past it must not reclassify them.
    let mut claimed = vec![false; args.len()];

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
        match subcommand_scope(
            spec, dialect, config, &releases, cmd_name, &args[0], &mut out,
        ) {
            Some((scope, canonical)) => {
                canonical.clone_into(&mut canonical_args[0]);
                scope
            }
            // Ambiguous or unknown: the formatter never guesses, and it
            // cannot know which option table applies either.
            None => return out,
        }
    };

    let ctx = RewriteCtx {
        registry,
        dialect,
        spec_surface: spec.surface,
        config,
        releases: &releases,
        cmd_name,
    };
    scan_options(
        &ctx,
        &scope,
        args,
        &touchable,
        &mut canonical_args,
        &mut claimed,
        &mut out,
    );
    push_positional_boolean_rewrites(&ctx, args, &canonical_args, &claimed, &touchable, &mut out);

    out
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family};
    use tcl_dialect::model::{SpecSurface, SurfaceQuery};
    use super::*;

    fn config(expand: bool, form: BooleanForm) -> FormatterConfig {
        FormatterConfig {
            expand_abbreviations: expand,
            boolean_form: form,
            ..FormatterConfig::default()
        }
    }

    fn rewrites(cmd: &str, args: &[&str], config: &FormatterConfig) -> Vec<(usize, String)> {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        let dynamic = vec![false; owned.len()];
        rewrites_for_command(
            registry,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
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
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let args = vec!["le".to_owned(), "$s".to_owned()];
        // Marked dynamic by the caller (a `{*}`-expanded word).
        let out = rewrites_for_command(
            registry,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
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
    /// tclsh-proof (8.6.14): `expr {yes ? 1 : 0}` → `1`, so `yes` at such a
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

    /// Issue #1268: the option scan only ever reaches a boolean value through
    /// an `-option`; a plain positional argument was never classified at all.
    /// `CommandRegistry::arg_indices_for_role` already answers the role
    /// question for a positional the same way it does for an option's value,
    /// so a spec that declares `ArgRole::Boolean` at a fixed argument index
    /// is now a rewrite site the day it is declared — no core built-in does
    /// this today, so the fixture is synthetic, same convention as the
    /// numeric-or-boolean *option* test above.
    #[test]
    fn a_positional_boolean_role_rewrites_through_the_registry() {
        let spec = CommandSpec {
            name: "boolpos26",
            arg_roles: &[(0, ArgRole::Boolean), (1, ArgRole::Value)],
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(spec);
        let run = |args: &[&str]| {
            let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
            let dynamic = vec![false; owned.len()];
            rewrites_for_command(
                &registry,
                None,
                &config(false, BooleanForm::TrueFalse),
                "boolpos26",
                &owned,
                &dynamic,
            )
            .into_iter()
            .map(|r| (r.index, r.text))
            .collect::<Vec<_>>()
        };
        // TP: a bare positional word at a declared `Boolean` index rewrites —
        // the option scan never touches it at all.
        assert_eq!(run(&["yes", "yes"]), vec![(0, "true".to_owned())]);
        // FP guard: the neighbouring `Value`-role position keeps its bytes
        // even though the written word is itself a boolean spelling.
        assert_eq!(run(&["no", "on"]), vec![(0, "false".to_owned())]);
    }

    /// The `NumericOrBoolean` licence (word-forms only, never a numeric
    /// spelling) applies identically at a positional site (issue #1268) —
    /// the positional counterpart of
    /// [`a_numeric_or_boolean_option_value_is_rewritten_through_the_registry`].
    #[test]
    fn a_positional_numeric_or_boolean_role_keeps_numeric_spellings() {
        let spec = CommandSpec {
            name: "boolnum26",
            arg_roles: &[(0, ArgRole::NumericOrBoolean)],
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(spec);
        let run = |args: &[&str]| {
            let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
            let dynamic = vec![false; owned.len()];
            rewrites_for_command(
                &registry,
                None,
                &config(false, BooleanForm::TrueFalse),
                "boolnum26",
                &owned,
                &dynamic,
            )
            .into_iter()
            .map(|r| (r.index, r.text))
            .collect::<Vec<_>>()
        };
        // TP: a word spelling normalises.
        assert_eq!(run(&["yes"]), vec![(0, "true".to_owned())]);
        // TN: `0`/`1` are numbers here too — byte-identical round trip.
        assert!(run(&["0"]).is_empty());
        assert!(run(&["1"]).is_empty());
    }

    /// A subcommand-scoped positional boolean must honour both preserved
    /// guarantees (issue #1268): the per-release **dialect** gate (a
    /// subcommand a release does not carry is never a rewrite site there) and
    /// the target-**range** agreement check (#1257) — even though
    /// `CommandRegistry::arg_indices_for_role` itself has no dialect
    /// awareness at all and would resolve the subcommand word by exact match
    /// regardless of release.
    #[test]
    fn a_subcommand_scoped_positional_boolean_honours_the_dialect_and_range_gates() {
        static GATED: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
            name: "flag",
            surface: Some(SpecSurface::TCL90_PLUS),
            arg_roles: &[(0, ArgRole::Boolean)],
            ..tcl_registry::SubCommand::DEFAULT
        }];
        let spec = CommandSpec {
            name: "wm26probe",
            subcommands: GATED,
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(spec);

        let run = |dialect: Option<SurfaceQuery<'_>>, target_range: Option<&'static [&'static str]>| {
            let cfg = FormatterConfig {
                target_range_override: target_range,
                ..config(false, BooleanForm::TrueFalse)
            };
            let owned = vec!["flag".to_owned(), "yes".to_owned()];
            let dynamic = vec![false, false];
            rewrites_for_command(&registry, dialect, &cfg, "wm26probe", &owned, &dynamic)
                .into_iter()
                .map(|r| (r.index, r.text))
                .collect::<Vec<_>>()
        };

        // TP: the document's own release carries the subcommand, and no range
        // was declared (the empty default) — the target's own verdict stands.
        assert_eq!(
            run(Some(SurfaceQuery::core(Family::Tcl, "9.0")), None),
            vec![(1, "true".to_owned())]
        );
        // FN guard (dialect): a document targeting a release that does not
        // carry the subcommand never reaches the positional pass at all — the
        // subcommand scan itself abstains, same as an ambiguous/unknown word.
        assert!(run(Some(SurfaceQuery::core(Family::Tcl, "8.6")), None).is_empty());

        // The range-agreement half (#1257), exercised directly: a range
        // spanning a release that drops the subcommand drags the verdict to
        // `None` even though the *target* release (9.0) alone accepts it —
        // `arg_indices_for_role` would otherwise find the subcommand by exact
        // match regardless of that release's dialect bit.
        let canonical_args = vec!["flag".to_owned(), "yes".to_owned()];
        let releases_disagreeing: Vec<(&str, &CommandRegistry)> =
            vec![("tcl8.6", &registry), ("tcl9.0", &registry)];
        assert_eq!(
            positional_site_across_range(
                &releases_disagreeing,
                "wm26probe",
                &canonical_args,
                1,
                BooleanSite::Interchangeable,
            ),
            BooleanSite::None
        );
        // A range confined to releases that all carry the subcommand agrees.
        let releases_agreeing: Vec<(&str, &CommandRegistry)> = vec![("tcl9.0", &registry)];
        assert_eq!(
            positional_site_across_range(
                &releases_agreeing,
                "wm26probe",
                &canonical_args,
                1,
                BooleanSite::Interchangeable,
            ),
            BooleanSite::Interchangeable
        );
    }

    /// The option path's own dialect gate keeps working post-refactor
    /// (issue #1268 preserves #1257's guarantee): a range spanning a release
    /// that does not carry the option drags the verdict to `None`, and a
    /// range confined to releases that do all agree.
    #[test]
    fn an_option_boolean_site_across_the_range_honours_the_options_own_dialect_gate() {
        use tcl_registry::hover::{OptionSpec, OptionValue};

        static OPTIONS_90: &[OptionSpec] = &[OptionSpec {
            name: "-validate",
            value: OptionValue::boolean(),
            surface: Some(SpecSurface::TCL90_PLUS),
            detail: "Only from Tcl 9.0.",
            ..OptionSpec::DEFAULT
        }];
        let spec = CommandSpec {
            name: "opt26probe",
            options: OPTIONS_90,
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(spec);

        let releases_disagreeing: Vec<(&str, &CommandRegistry)> =
            vec![("tcl8.6", &registry), ("tcl9.0", &registry)];
        assert!(!resolves_across_range(
            &releases_disagreeing,
            "opt26probe",
            RangeTable::CommandOptions,
            "-validate",
            "-validate",
        ));
        assert_eq!(
            boolean_site_across_range(
                &releases_disagreeing,
                "opt26probe",
                RangeTable::CommandOptions,
                "-validate",
                BooleanSite::Interchangeable,
            ),
            BooleanSite::None
        );

        let releases_agreeing: Vec<(&str, &CommandRegistry)> = vec![("tcl9.0", &registry)];
        assert!(resolves_across_range(
            &releases_agreeing,
            "opt26probe",
            RangeTable::CommandOptions,
            "-validate",
            "-validate",
        ));
        assert_eq!(
            boolean_site_across_range(
                &releases_agreeing,
                "opt26probe",
                RangeTable::CommandOptions,
                "-validate",
                BooleanSite::Interchangeable,
            ),
            BooleanSite::Interchangeable
        );
    }
}
