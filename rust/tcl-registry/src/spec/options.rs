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

//! Option words: the prefix tables, resolving one option, counting a leading option run and the option table a call dispatches on.

use super::CommandSpec;
use super::subcommand::SubCommand;
use crate::abbrev::{Keyword, KeywordMatch, KeywordTable, PrefixMatching};
use crate::hover::OptionSpec;
use crate::invocation_words::InvocationArguments;
use tcl_dialect::model::{SpecSurface, SurfaceQuery};

/// Count the leading words of `args` that are declared options in
/// `options` — the `?-flag? ?-opt val?` prefix many commands/subcommands
/// accept before their positional arguments. Shared core of
/// [`CommandSpec::leading_option_word_count`] /
/// [`SubCommand::leading_option_word_count`], so every consumer that needs
/// "where do the true positionals start" (an `arg_types` hint, a
/// [`CommandSpec::defines_command_at`] / [`SubCommand::defines_command_at`]
/// name lookup, …) agrees with the one declared option table instead of
/// assuming a fixed offset that a `-safe`/`--`-style flag silently shifts.
///
/// A word counts as an option when it begins with `-`, is at least two
/// characters, and resolves to exactly one declared option — by exact name,
/// declared alias, or unique prefix (C Tcl's option tables accept any
/// unambiguous prefix of two or more characters: `string map -noc …`
/// behaves like `-nocase`, verified in `tclCmdMZ.c`'s
/// `strncmp(string, "-nocase", length)` loops). A value-taking option also
/// consumes its following word. Counting stops at the first
/// non-option-shaped word; an empty `options` table returns 0
/// unconditionally, so purely positional commands/subcommands are
/// untouched. Dialect-agnostic — callers that need dialect gating (e.g. an
/// option added in a later Tcl release) should filter `options` first.
/// Build an option [`KeywordTable`] from an option iterator, applying the
/// dialect and lifecycle filters and including declared aliases (Tcl's own
/// option table holds them, so `-bd` prefix-matches exactly like
/// `-borderwidth`).
fn option_table_from<'a>(
    options: impl Iterator<Item = &'a OptionSpec>,
    dialect: Option<SurfaceQuery<'_>>,
    parent_surface: Option<&'static [SpecSurface]>,
    package_version: Option<&str>,
    prefix_matching: PrefixMatching,
) -> KeywordTable<'static> {
    keyword_table_for_available_options(
        options.filter(|option| {
            option.supports_dialect(dialect, parent_surface)
                && option.available_for_version(package_version)
        }),
        prefix_matching,
    )
}

/// The shared keyword vocabulary after the caller selects exact availability.
pub(crate) fn keyword_table_for_available_options<'a>(
    options: impl Iterator<Item = &'a OptionSpec>,
    prefix_matching: PrefixMatching,
) -> KeywordTable<'static> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let mut keywords: Vec<Keyword<'static>> = Vec::new();
    for opt in options {
        for name in std::iter::once(opt.name).chain(opt.aliases.iter().copied()) {
            if !keywords.iter().any(|k| k.name == name) {
                keywords.push(Keyword {
                    name,
                    min_abbrev: opt.min_abbrev,
                });
            }
        }
    }
    KeywordTable::from_keywords(keywords, prefix_matching)
}

/// Resolve one leading option word against its descriptor table.
///
/// Tcl accepts an exact canonical spelling or alias, then an unambiguous
/// abbreviation of either spelling.  Keeping that rule beside
/// [`OptionSpec`] means the layout resolvers, option-value role walk, and
/// declaration helpers all consume precisely the words Tcl's option parser
/// consumes; no command consumer needs its own list of value-taking switches.
#[must_use]
pub fn resolve_option_prefix<'a>(options: &'a [OptionSpec], word: &str) -> Option<&'a OptionSpec> {
    resolve_option_prefix_with(options, word, PrefixMatching::Enabled)
}

/// [`resolve_option_prefix`] with the table's declared prefix policy.
#[must_use]
pub fn resolve_option_prefix_with<'a>(
    options: &'a [OptionSpec],
    word: &str,
    prefix_matching: PrefixMatching,
) -> Option<&'a OptionSpec> {
    resolve_option_prefix_in(options.iter(), word, prefix_matching)
}

/// Resolve an option in an already availability-filtered descriptor table.
/// Exact names and aliases win before declared minimum unique abbreviations.
#[must_use]
pub(crate) fn resolve_available_option_prefix_with<'a>(
    options: &[&'a OptionSpec],
    word: &str,
    prefix_matching: PrefixMatching,
) -> Option<&'a OptionSpec> {
    resolve_option_prefix_in(options.iter().copied(), word, prefix_matching)
}

fn resolve_option_prefix_in<'a>(
    options: impl Iterator<Item = &'a OptionSpec> + Clone,
    word: &str,
    prefix_matching: PrefixMatching,
) -> Option<&'a OptionSpec> {
    if let Some(option) = options.clone().find(|option| option.matches(word)) {
        return Some(option);
    }
    if !prefix_matching.accepts_prefixes() || !word.starts_with('-') || word.len() < 2 {
        return None;
    }
    let mut found = None;
    for option in options {
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

/// Prove where a leading option grammar ends in evaluated source words.
/// Unknown words while option scanning is live leave the boundary unknown;
/// a known positional prefix or declared terminator permits unknown trailing
/// values. Every declared fixed option value must be present. Expansion never
/// supplies an exact argv boundary.
#[must_use]
pub fn leading_option_word_count_for_arguments(
    options: &[&OptionSpec],
    args: crate::InvocationArguments<'_>,
    prefix_matching: PrefixMatching,
    reserved_trailing_words: usize,
) -> Option<usize> {
    let count = args.exact_argv_len()?;
    if options.is_empty() {
        return Some(0);
    }
    let scan_end = count.saturating_sub(reserved_trailing_words);
    let mut index = 0;
    while index < scan_end {
        let argument = args.get(index)?;
        if argument.proves_non_option() {
            return Some(index);
        }
        let word = argument.literal()?;
        if !word.starts_with('-') || word.len() < 2 {
            return Some(index);
        }
        let option = resolve_available_option_prefix_with(options, word, prefix_matching)?;
        let width = match option.value {
            crate::hover::OptionValue::Flag => 0,
            crate::hover::OptionValue::Takes(value) => match value.arity {
                crate::hover::OptionArity::One => 1,
                crate::hover::OptionArity::Fixed(width) => usize::from(width),
                crate::hover::OptionArity::OptionalOneOf(_)
                | crate::hover::OptionArity::OptionalNotOption
                | crate::hover::OptionArity::Hook(_) => {
                    option.value_word_count_for_arguments(args, index)?
                }
            },
        };
        index = index.checked_add(1)?.checked_add(width)?;
        if index > count {
            return None;
        }
        if option.name == "--" {
            return Some(index);
        }
    }
    Some(index)
}

/// Return the index immediately after a declared leading option prefix.
///
/// The descriptor-driven walk honours unique abbreviations, aliases, each
/// option's actual value arity, and a declared `--` terminator.  It is the
/// common layout primitive for commands whose positional grammar starts only
/// after their option prefix.
#[must_use]
pub fn leading_option_word_count(options: &[OptionSpec], args: &[&str]) -> usize {
    leading_option_word_count_with(options, args, PrefixMatching::Enabled)
}

/// [`leading_option_word_count`] with the table's declared prefix policy.
#[must_use]
pub fn leading_option_word_count_with(
    options: &[OptionSpec],
    args: &[&str],
    prefix_matching: PrefixMatching,
) -> usize {
    leading_option_run_with(options, args, prefix_matching).0
}

/// The leading option run of `args`: how many words its options (and their
/// values) cover, and whether the declared end-of-options word ended it.
fn leading_option_run_with(
    options: &[OptionSpec],
    args: &[&str],
    prefix_matching: PrefixMatching,
) -> (usize, bool) {
    if options.is_empty() {
        return (0, false);
    }
    let mut i = 0;
    while let Some(&word) = args.get(i) {
        if !word.starts_with('-') || word.len() < 2 {
            break;
        }
        let resolved = resolve_option_prefix_with(options, word, prefix_matching);
        let Some(opt) = resolved else { break };
        i += 1 + opt.value_word_count(args, i);
        if opt.name == "--" {
            // `--` ends option parsing (Tcl's own `Tcl_ParseArgsObjv`-style
            // convention: every subsequent word — even one that looks like a
            // declared option, e.g. `interp create -- -safe` — is positional).
            // Without this, the loop above would keep matching option-shaped
            // words past the terminator and swallow a literal name meant to
            // land in a positional slot such as `defines_command_at`.
            return (i, true);
        }
    }
    (i, false)
}

impl CommandSpec {
    /// This command's option keyword table (canonical spellings and declared
    /// aliases), filtered to what exists for `dialect` and `package_version`.
    #[must_use]
    pub fn option_table(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordTable<'static> {
        option_table_from(
            self.options
                .iter()
                .chain(self.command_forms.iter().flat_map(|f| f.options.iter())),
            dialect,
            self.surface,
            package_version,
            prefix_override.unwrap_or(self.prefix_matching),
        )
    }

    /// The canonical names of the switches in front of this call's first
    /// positional argument.
    ///
    /// Words resolve through the command's own option table, so an
    /// abbreviation counts exactly when the real command accepts one
    /// (`lsearch -al` is `-all`; `regexp -inl` is an error, because
    /// `Tcl_RegexpObjCmd` reads its table with `TCL_EXACT`), and a value word
    /// is consumed by the switch that takes it. The scan also stops short of
    /// the operands C Tcl reserves ([`Self::reserved_trailing_words`]), so a
    /// mandatory word is never read as a switch: `Tcl_LsearchObjCmd` scans
    /// `i < objc - 2`, which makes `lsearch -all foo` a search for `foo` in
    /// the one-element list `-all` — it returns -1, an int, not the list
    /// `-all` would otherwise select (verified on tclsh 9.0.4).
    #[must_use]
    pub fn leading_switch_names(&self, args: &[&str]) -> Vec<&'static str> {
        args[..self.switch_word_count(args)]
            .iter()
            .filter_map(|word| resolve_option_prefix_with(self.options, word, self.prefix_matching))
            .map(|option| option.name)
            .collect()
    }

    /// How many positional words this call supplies — every word that is not
    /// a leading switch or one of their values. The optional-trailing-argument
    /// split (`regsub`'s `varName`, `pid`'s `fileId`) reads this.
    #[must_use]
    pub fn positional_word_count(&self, args: &[&str]) -> usize {
        args.len() - self.switch_word_count(args)
    }

    /// Words consumed by the leading switch run, values included.
    pub(crate) fn switch_word_count(&self, args: &[&str]) -> usize {
        let option_scan_end = args.len().saturating_sub(self.reserved_trailing_words);
        leading_option_word_count_with(self.options, &args[..option_scan_end], self.prefix_matching)
    }

    /// Resolve an option word against this command's option table.
    #[must_use]
    pub fn resolve_option_word(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordMatch<'static> {
        self.option_table(dialect, package_version, prefix_override)
            .resolve(word)
    }

    /// Declared option / switch names valid in `dialect`, in
    /// declaration order with duplicates removed.
    ///
    /// Walks the command's
    /// declared options (both the flat [`Self::options`] list and
    /// every [`CommandForm`]'s options) and keeps
    /// only those whose [`OptionSpec::supports_dialect`] holds for
    /// `dialect`, inheriting the command's own [`Self::dialects`] as
    /// the parent set. `dialect == None` means "no dialect filter"
    /// (every declared option is returned).
    ///
    /// Used by the analyser's option-aware arity check: leading
    /// arguments that match one of these names are skipped before
    /// counting positional args, so option flags introduced in a
    /// later Tcl release don't leak into an earlier dialect's
    /// signature and get wrongly skipped (e.g. `regsub -command` is
    /// 9.0-only).
    #[must_use]
    pub fn switch_names(&self, dialect: Option<SurfaceQuery<'_>>) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let consider = |opt: &OptionSpec, names: &mut Vec<&'static str>| {
            if opt.supports_dialect(dialect, self.surface) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        };
        for opt in self.options {
            consider(opt, &mut names);
        }
        for form in self.command_forms {
            for opt in form.options {
                consider(opt, &mut names);
            }
        }
        names
    }

    /// The declared [`OptionSpec`]s available in *dialect* (canonical
    /// options plus every command-form option), deduped by name.
    ///
    /// The name-only [`Self::switch_names`] is enough to *recognise* a
    /// leading option, but the analyser's arity check also needs to know
    /// how many *value* words each option consumes (`-start 0` is two
    /// words, not one), so a value word is not miscounted as a positional
    /// argument. Returning the specs lets a consumer call
    /// [`OptionSpec::value_word_count`]. Kept next to `switch_names` so the
    /// two stay dialect-consistent.
    #[must_use]
    pub fn option_specs(&self, dialect: Option<SurfaceQuery<'_>>) -> Vec<&'static OptionSpec> {
        let mut specs: Vec<&'static OptionSpec> = Vec::new();
        let mut consider = |opt: &'static OptionSpec| {
            if opt.supports_dialect(dialect, self.surface)
                && !specs.iter().any(|o| o.name == opt.name)
            {
                specs.push(opt);
            }
        };
        for opt in self.options {
            consider(opt);
        }
        for form in self.command_forms {
            for opt in form.options {
                consider(opt);
            }
        }
        specs
    }

    /// The option-effect answer for one call of this command
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *Options with
    /// semantic effects*): the generic walk of
    /// [`crate::option_effect::option_effects`] over this command's
    /// [`Self::option_specs`] available at `dialect`, its
    /// [`Self::option_effect_families`], its
    /// [`Self::reserved_trailing_words`], and its prefix policy.
    #[must_use]
    pub fn option_effects(
        &self,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> crate::option_effect::OptionEffects {
        self.option_effects_over(&self.option_specs(dialect), args, dialect)
    }

    /// [`Self::option_effects`] over an option table the caller has already
    /// filtered for its profile.
    #[must_use]
    pub(crate) fn option_effects_over(
        &self,
        options: &[&OptionSpec],
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> crate::option_effect::OptionEffects {
        crate::option_effect::option_effects_over(
            options,
            self.option_effect_families,
            args,
            self.reserved_trailing_words,
            dialect,
            self.prefix_matching,
        )
    }

    /// Whether an option of this command selects the language of its pattern
    /// operand — an effect on the [`crate::option_effect::EffectAxis::PatternLanguage`]
    /// axis (`lsearch -regexp`). Such a command's pattern layout is the
    /// projection of [`Self::option_effects`], not a static role.
    #[must_use]
    pub fn option_selects_pattern_language(&self) -> bool {
        self.options.iter().any(|option| {
            option.effect.is_some_and(|effect| {
                matches!(
                    effect.kind.axis(),
                    Some(crate::option_effect::EffectAxis::PatternLanguage(_))
                )
            })
        })
    }

    /// The spelling of the option whose declared effect selects `axis`, when
    /// one does — the generic way to name, say, the regex-mode switch of a
    /// case-list command without spelling it.
    #[must_use]
    pub fn option_selecting(&self, axis: crate::option_effect::EffectAxis) -> Option<&'static str> {
        self.options
            .iter()
            .find(|option| {
                option.effect.is_some_and(|effect| {
                    effect.kind == crate::option_effect::OptionEffectKind::Selects(axis)
                })
            })
            .map(|option| option.name)
    }

    /// [`leading_option_word_count`] against this command's own
    /// [`Self::options`] — how many of `args`' leading words are declared
    /// flags/options, so a positional argument index (e.g.
    /// [`Self::defines_command_at`]) can be shifted past them.
    #[must_use]
    pub fn leading_option_word_count(&self, args: &[&str]) -> usize {
        leading_option_word_count_with(self.options, args, self.prefix_matching)
    }

    /// Whether the end-of-options word ended [`Self::leading_option_word_count`]'s
    /// run — after which every word is positional, so a name slot may hold a
    /// word shaped like an option (`interp create -- -safe` names the child
    /// `-safe`).
    #[must_use]
    pub fn leading_option_run_is_terminated(&self, args: &[&str]) -> bool {
        leading_option_run_with(self.options, args, self.prefix_matching).1
    }

    /// Like [`Self::switch_names`], but optionally including documented
    /// abbreviation aliases (`-bd` for `-borderwidth`) and filtering by the
    /// resolved package version (dropping options not yet introduced at, or
    /// already retired by, *`package_version`*).
    ///
    /// `include_aliases` is for validation callers that must accept `-bd`;
    /// completion passes `false` so only canonical spellings are offered.
    /// `package_version` is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]);
    /// `None` keeps every option.
    #[must_use]
    pub fn switch_names_ext(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        include_aliases: bool,
        package_version: Option<&str>,
    ) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let consider = |opt: &OptionSpec, names: &mut Vec<&'static str>| {
            if !opt.supports_dialect(dialect, self.surface) {
                return;
            }
            if !opt.available_for_version(package_version) {
                return;
            }
            if !names.contains(&opt.name) {
                names.push(opt.name);
            }
            if include_aliases {
                for alias in opt.aliases {
                    if !names.contains(alias) {
                        names.push(alias);
                    }
                }
            }
        };
        for opt in self.options {
            consider(opt, &mut names);
        }
        for form in self.command_forms {
            for opt in form.options {
                consider(opt, &mut names);
            }
        }
        names
    }

    /// Look up an option by its canonical name or any documented alias,
    /// honouring the `dialect` and `package_version` gates.
    #[must_use]
    pub fn find_option(
        &self,
        option_name: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Option<&OptionSpec> {
        let matches = |opt: &&OptionSpec| {
            opt.matches(option_name)
                && opt.supports_dialect(dialect, self.surface)
                && opt.available_for_version(package_version)
        };
        self.options.iter().find(matches).or_else(|| {
            self.command_forms
                .iter()
                .flat_map(|f| f.options.iter())
                .find(matches)
        })
    }
}

/// The option table one call dispatches on, as resolved by
/// [`SubCommand::option_scope`] — the table itself, the dialect gate its
/// options inherit, and which second-level operation (if any) supplied it.
#[derive(Debug, Clone, Copy)]
pub struct OptionScope {
    /// The options this call recognises.
    pub options: &'static [OptionSpec],
    /// The gate an option declaring none of its own inherits.
    pub surface: Option<&'static [SpecSurface]>,
    /// Positional words after the outer selector before this option table.
    pub option_prefix_words: usize,
    /// The second-level operation whose own table this is; `None` when the
    /// answer came from the subcommand's table.
    pub sub_subcommand: Option<&'static str>,
}

impl SubCommand {
    /// This subcommand's option keyword table, filtered to what exists for
    /// `dialect` and `package_version`. See
    /// [`CommandSpec::option_table`].
    #[must_use]
    pub fn option_table(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordTable<'static> {
        option_table_from(
            self.options
                .iter()
                .chain(self.subcommand_forms.iter().flat_map(|f| f.options.iter())),
            dialect,
            self.surface,
            package_version,
            prefix_override.unwrap_or(self.prefix_matching),
        )
    }

    /// Which option table a call against this subcommand actually dispatches
    /// on, once the word *after* the subcommand name is taken into account.
    ///
    /// For the overwhelmingly common single-level subcommand this is just
    /// [`Self::options`]. For a two-level ensemble whose operations disagree
    /// about their options — `namespace ensemble create` vs `configure` —
    /// it is the resolved [`SubSubCommand::options`] instead,
    /// and *instead* is the point: merging the two tables reintroduces the
    /// bug, since each table's distinctive entry is the other's error.
    ///
    /// `next_word` is the literal word after the subcommand name, or `None`
    /// when the caller has no literal there (nothing typed yet, a `$var`, a
    /// `{*}` expansion). `None` and an unresolvable word fall back to this
    /// subcommand's own table — the permissive answer, so a dynamic dispatch
    /// word never narrows what is offered or accepted.
    ///
    /// A resolved operation answers with whatever it declares, **including an
    /// explicitly empty table**: `Some(&[])` means "no options here" and must
    /// not fall back. Only
    /// [`SubSubCommand::options`] `== None` — declaring nothing either way —
    /// inherits.
    ///
    /// `spec_surface` is the owning [`CommandSpec::dialects`], for the
    /// standard option-gate inheritance chain (option → sub-sub → sub →
    /// command).
    #[must_use]
    pub fn option_scope(
        &self,
        next_word: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        spec_surface: Option<&'static [SpecSurface]>,
    ) -> OptionScope {
        let inherited = self.surface.or(spec_surface);
        if let Some(word) = next_word.filter(|w| !w.is_empty())
            && !self.sub_subcommands.is_empty()
            && let Some(op) = self.resolve_sub_subcommand_gated(word, dialect, package_version)
            && let Some(options) = op.options
        {
            return OptionScope {
                options,
                surface: op.surface.or(inherited),
                sub_subcommand: Some(op.name),
                option_prefix_words: 1 + op.option_prefix_words,
            };
        }
        OptionScope {
            options: self.options,
            surface: inherited,
            sub_subcommand: None,
            option_prefix_words: self.option_prefix_words,
        }
    }

    /// Resolve an option word against this subcommand's option table.
    #[must_use]
    pub fn resolve_option_word(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordMatch<'static> {
        self.option_table(dialect, package_version, prefix_override)
            .resolve(word)
    }

    /// [`leading_option_word_count`] against this subcommand's own
    /// [`Self::options`] — how many of `args`' leading words (*after* the
    /// subcommand word itself) are declared flags/options, so a positional
    /// argument index (e.g. [`Self::defines_command_at`]) can be shifted
    /// past them. The subcommand counterpart of
    /// [`CommandSpec::leading_option_word_count`].
    ///
    /// Capped by [`Self::max_leading_option_words`] when the subcommand
    /// declares one: a subcommand that hand-parses a single optional flag
    /// treats every further option-shaped word as positional, so counting
    /// them all would shift a positional index past real arguments.
    #[must_use]
    pub fn leading_option_word_count(&self, args: &[&str]) -> usize {
        let counted = leading_option_word_count_with(self.options, args, self.prefix_matching);
        match self.max_leading_option_words {
            Some(cap) => counted.min(usize::from(cap)),
            None => counted,
        }
    }

    /// Whether the end-of-options word ended [`Self::leading_option_word_count`]'s
    /// run within the subcommand's cap — after which every word is
    /// positional, so a name slot may hold a word shaped like an option.
    #[must_use]
    pub fn leading_option_run_is_terminated(&self, args: &[&str]) -> bool {
        let (counted, terminated) =
            leading_option_run_with(self.options, args, self.prefix_matching);
        terminated
            && self
                .max_leading_option_words
                .is_none_or(|cap| counted <= usize::from(cap))
    }

    /// Declared option-flag names for this subcommand, filtered by
    /// *dialect*.
    ///
    /// Per-subcommand options
    /// (e.g. `-symbolic` / `-hard` on `file link`) flow into the
    /// subcommand's [`crate::analyser`]-side `leading_options` so the
    /// arity check skips them before counting positionals.  An option's
    /// dialect membership inherits from this subcommand's `dialects`
    /// (falling back to *`parent_surface`*, the parent
    /// [`CommandSpec::dialects`]) when the option itself does not pin a
    /// dialect.
    #[must_use]
    pub fn switch_names(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        parent_surface: Option<&'static [SpecSurface]>,
    ) -> Vec<&'static str> {
        let effective_parent = self.surface.or(parent_surface);
        let mut names: Vec<&'static str> = Vec::new();
        for opt in self.options {
            if opt.supports_dialect(dialect, effective_parent) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        }
        names
    }

    /// The declared [`OptionSpec`]s for this subcommand available in
    /// *dialect* (see [`CommandSpec::option_specs`]). Carries the value
    /// arity the analyser's arity check needs so a value-taking option's
    /// value word (`file link -symbolic dst src`) is not miscounted as a
    /// positional argument.
    #[must_use]
    pub fn option_specs(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        parent_surface: Option<&'static [SpecSurface]>,
    ) -> Vec<&'static OptionSpec> {
        let effective_parent = self.surface.or(parent_surface);
        let mut specs: Vec<&'static OptionSpec> = Vec::new();
        for opt in self.options {
            if opt.supports_dialect(dialect, effective_parent)
                && !specs.iter().any(|o| o.name == opt.name)
            {
                specs.push(opt);
            }
        }
        specs
    }
}
