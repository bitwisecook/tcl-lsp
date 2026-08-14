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

//! Command-signature lookup.
//!
//! Per-handler dispatch consults the command registry to learn
//! how many arguments a command accepts and what role each
//! argument plays. The lookup returns one of three shapes:
//!
//! - [`CommandSig`] — a simple command (`set`, `proc`, `puts`).
//! - [`SubcommandSig`] — a command that dispatches on its first
//!   argument (`namespace eval`, `dict get`, `string length`,
//!   `info args`).
//! - `None` — the command isn't in the registry.

use std::collections::{BTreeSet, HashMap};

use tcl_registry::ProfileQueries;
use tcl_registry::abbrev::{Keyword, KeywordMatch, KeywordTable, PrefixMatching};
use tcl_registry::prelude::OptionSpec;
use tcl_registry::scoped::ScopedCommand;
use tcl_registry::{ArgRole, Arity, CommandRegistry, Traits};

/// Signature for a simple Tcl command.
///
/// No `PartialEq`/`Eq` — carries `leading_option_specs: Vec<&'static
/// OptionSpec>`, and `OptionSpec` has none (a `Hook` option's resolver has
/// no meaningful equality); nothing compares two `CommandSig`s anyway.
#[derive(Debug, Clone)]
pub struct CommandSig {
    /// Argument-count bounds.
    pub arity: Arity,
    /// Static arg-index → role map (0-based, after the command
    /// name). Args not listed default to ``ArgRole::Value``.
    pub arg_roles: HashMap<u8, ArgRole>,
    /// Behavioural trait flags, copied from the resolved
    /// [`tcl_registry::CommandSpec::traits`]. Consulted by
    /// [`crate::analyser::diagnostics::validity`]'s arity check to skip
    /// the generic E002/E003 floor/ceiling diagnostic for a command
    /// carrying [`Traits::STRUCTURALLY_CHECKED_ARITY`] (its own
    /// dedicated structural diagnostic owns arity instead).
    pub traits: Traits,

    /// Declared option / switch names valid in the active dialect.
    /// Leading arguments matching one of these are skipped before
    /// counting positional args for the E002 / E003 arity check.
    /// Populated from [`tcl_registry::CommandSpec::switch_names`]
    /// (dialect-filtered).
    pub leading_options: BTreeSet<String>,
    /// The declared option specs valid in the active dialect, carrying
    /// each option's *value* arity. The arity check consults these (via
    /// [`OptionSpec::value_word_count`]) so a value-taking option's value
    /// word (`regsub -start 0 …`) is skipped along with the flag rather
    /// than miscounted as a positional argument. Dialect-consistent with
    /// `leading_options`.
    pub leading_option_specs: Vec<&'static OptionSpec>,

    /// Registry-declared relationships between leading options. The generic
    /// validity pass reports conflicts without naming a command.
    pub option_constraints: Vec<&'static tcl_registry::OptionConstraint>,

    /// The resolved command / subcommand's primary invocation synopsis
    /// ([`tcl_registry::CommandSpec::primary_synopsis`] /
    /// [`tcl_registry::SubCommand::primary_synopsis`]), appended by the
    /// arity check as a "usage: …" suffix so an E002/E003/E005 message
    /// shows the expected shape, not just the counts. `None` when the
    /// spec declares no synopsis.
    pub synopsis: Option<&'static str>,

    /// Documented minimum abbreviation length for this subcommand's own
    /// name, from [`tcl_registry::SubCommand::min_abbrev`]. `None` (the norm)
    /// = uniqueness is the only constraint on an abbreviated spelling.
    pub min_abbrev: Option<u8>,
}

/// Signature for a command that dispatches on a subcommand word.
///
/// No `PartialEq`/`Eq` — carries `CommandSig`s, which have none.
#[derive(Debug, Clone)]
pub struct SubcommandSig {
    /// Subcommand name → [`CommandSig`] mapping. Empty for
    /// commands that haven't yet had their subcommand table
    /// populated in the registry.
    pub subcommands: HashMap<String, CommandSig>,
    /// When `true`, unknown subcommands are not flagged as
    /// diagnostics — used for generated dialect packs.
    pub allow_unknown: bool,
    /// Whether invoking the command with no subcommand word at all is
    /// itself an arity error (E001). Mirrors the parent
    /// [`tcl_registry::CommandSpec::arity`]'s minimum: `true` when it is
    /// at least 1 (the overwhelming majority of ensemble-shaped
    /// commands — `string`, `dict`, `info`, `array`, … — all error
    /// "wrong # args" when called bare), `false` for the rare command
    /// whose spec declares a zero minimum because a bare call has a
    /// well-defined default — e.g. `history` with no arguments is
    /// `history info` per history(n), confirmed against tclsh 9.0.4.
    /// This is registry data, not a hardcoded command name: any future
    /// spec that sets `arity.min == 0` on a `WithSubcommands` command
    /// gets the same treatment automatically.
    pub subcommand_required: bool,
    /// The value shape a non-subcommand first word may take to select the
    /// command's *default* form (`after 200 …`), copied from
    /// [`tcl_registry::CommandSpec::default_form_first_word`]. `None` =
    /// every first word must be a known subcommand.
    pub default_form_first_word: Option<tcl_registry::DefaultFormFirstWord>,
    /// Whether this ensemble's dispatch honours unique-prefix abbreviation,
    /// from [`tcl_registry::CommandSpec::prefix_matching`] — `Strict` for a
    /// `TCL_INDEX_STRICT` table.
    pub prefix_matching: PrefixMatching,
}

impl SubcommandSig {
    /// This signature's subcommand words as a [`KeywordTable`].
    ///
    /// The map already holds only the dialect-available subcommands, so the
    /// table is dialect-correct and prefix uniqueness matches the profile.
    #[must_use]
    pub fn keyword_table(&self) -> KeywordTable<'_> {
        KeywordTable::from_keywords(
            self.subcommands.iter().map(|(name, sig)| Keyword {
                name: name.as_str(),
                min_abbrev: sig.min_abbrev,
            }),
            self.prefix_matching,
        )
    }

    /// Resolve a subcommand word three-valued — unique, ambiguous, or
    /// unknown — through the shared registry abbreviation API, so the
    /// analyser cannot drift from what the registry, formatter, and minifier
    /// think an abbreviation means.
    #[must_use]
    pub fn resolve_word(&self, word: &str) -> KeywordMatch<'_> {
        self.keyword_table().resolve(word)
    }

    /// Resolve a subcommand word to its canonical [`CommandSig`], accepting a
    /// unique non-empty prefix the way Tcl's `Tcl_GetIndexFromObj` ensemble
    /// dispatch does (`string le` ⇒ `length`). An exact match wins; an
    /// ambiguous prefix resolves to `None`. Only the dialect-available
    /// subcommands are in the map, so prefix resolution is dialect-correct.
    #[must_use]
    pub fn resolve(&self, word: &str) -> Option<&CommandSig> {
        let canonical = self.resolve_word(word).unique()?;
        self.subcommands.get(canonical)
    }

    /// Whether `word` resolves to a known subcommand (exact or unique prefix).
    #[must_use]
    pub fn is_known(&self, word: &str) -> bool {
        self.resolve(word).is_some()
    }

    /// Whether `word` matches the spec-declared default-form value shape
    /// (`after 200 …` — an integer first word is the default form, not an
    /// unknown subcommand). `false` when the spec declares no default form.
    #[must_use]
    pub fn matches_default_form(&self, word: &str) -> bool {
        self.default_form_first_word
            .is_some_and(|shape| shape.matches(word))
    }
}

/// What ``signature_for_command`` returned.
///
/// No `PartialEq`/`Eq` — both variants carry a type with none.
#[derive(Debug, Clone)]
pub enum CommandSignature {
    /// A simple command.
    Simple(CommandSig),
    /// A command with subcommands.
    WithSubcommands(SubcommandSig),
}

/// Look up signature metadata for a command.
///
/// Returns:
///
/// - [`CommandSignature::WithSubcommands`] when the spec has
///   non-empty subcommands.
/// - [`CommandSignature::Simple`] when the spec exists but
///   has no subcommands.
/// - `None` when the registry doesn't know the command.
///
/// The `profile` argument selects which dialect-specific subcommand and
/// option sets are materialised (availability mask + version ceiling +
/// the subtractive iRules disable filter); pass
/// `DialectProfile::plain_tcl()` when the caller has no specific dialect
/// context.
#[must_use]
pub fn signature_for_command(
    registry: &CommandRegistry,
    cmd_name: &str,
    profile: &tcl_dialect::DialectProfile,
) -> Option<CommandSignature> {
    let spec = profile.resolve_command(registry, cmd_name)?;

    if !spec.subcommands.is_empty() {
        let mut subs: HashMap<String, CommandSig> = HashMap::new();
        for sub in spec.subcommands {
            // The profile filters out subcommands not available in the
            // current dialect (own gate falling back to the parent's,
            // intersected with the availability mask).
            if !profile.is_subcommand_available(spec, sub) {
                continue;
            }
            let arg_roles = sub
                .arg_roles
                .iter()
                .map(|(idx, role)| (*idx, *role))
                .collect();
            // Per-subcommand options (e.g. `-symbolic` / `-hard` on
            // `file link`) feed the subcommand arity check's leading-
            // option skip.  The option dialect inherits from the
            // subcommand (falling back to the parent command) when it
            // does not pin its own (§5.2 gating: intersects + ceiling).
            let leading_options = profile
                .available_sub_option_names(spec, sub)
                .into_iter()
                .map(str::to_string)
                .collect();
            let leading_option_specs = profile.available_sub_option_specs(spec, sub);
            subs.insert(
                sub.name.to_string(),
                CommandSig {
                    arity: sub.arity,
                    arg_roles,
                    traits: sub.traits,
                    leading_options,
                    leading_option_specs,
                    option_constraints: sub
                        .option_constraints
                        .iter()
                        .chain(
                            sub.subcommand_forms
                                .iter()
                                .flat_map(|form| form.option_constraints.iter()),
                        )
                        .filter(|constraint| {
                            constraint.supports_dialect(
                                Some(profile.availability_mask),
                                sub.dialects.or(spec.dialects),
                            )
                        })
                        .collect(),
                    synopsis: sub.primary_synopsis(),
                    min_abbrev: sub.min_abbrev,
                },
            );
        }
        return Some(CommandSignature::WithSubcommands(SubcommandSig {
            subcommands: subs,
            allow_unknown: spec.allow_unknown_subcommands,
            subcommand_required: spec.arity.min > 0,
            default_form_first_word: spec.default_form_first_word,
            prefix_matching: spec.prefix_matching,
        }));
    }

    let arg_roles = spec
        .arg_roles
        .iter()
        .map(|(idx, role)| (*idx, *role))
        .collect();
    let leading_options = profile
        .available_option_names(spec)
        .into_iter()
        .map(str::to_string)
        .collect();
    let leading_option_specs = profile.available_option_specs(spec);
    Some(CommandSignature::Simple(CommandSig {
        arity: spec.arity,
        arg_roles,
        traits: spec.traits,
        leading_options,
        leading_option_specs,
        option_constraints: spec
            .option_constraints
            .iter()
            .chain(
                spec.command_forms
                    .iter()
                    .flat_map(|form| form.option_constraints.iter()),
            )
            .filter(|constraint| {
                constraint.supports_dialect(Some(profile.availability_mask), spec.dialects)
            })
            .collect(),
        // The walk cannot know the file's resolved package-version floor
        // yet (`package require` may appear anywhere), so form selection
        // stays permissive here — the post-walk gate is what version-aware
        // reporting goes through.
        synopsis: spec.primary_synopsis(None),
        // A command name is never prefix-matched, so a simple command's
        // signature carries no abbreviation floor.
        min_abbrev: None,
    }))
}

/// Like [`signature_for_command`] but dialect-AGNOSTIC: every declared
/// subcommand and option is materialised regardless of dialect gates.
/// For existence checks of the form "is this a subcommand in ANY
/// dialect" (the W002 disabled-subcommand hint), where filtering by the
/// active profile would defeat the purpose.
#[must_use]
pub fn signature_for_command_any_dialect(
    registry: &CommandRegistry,
    cmd_name: &str,
) -> Option<CommandSignature> {
    let spec = registry.get(cmd_name)?;
    if !spec.subcommands.is_empty() {
        let mut subs: HashMap<String, CommandSig> = HashMap::new();
        for sub in spec.subcommands {
            let arg_roles = sub
                .arg_roles
                .iter()
                .map(|(idx, role)| (*idx, *role))
                .collect();
            let leading_options = sub
                .switch_names(None, spec.dialects)
                .into_iter()
                .map(str::to_string)
                .collect();
            let leading_option_specs = sub.option_specs(None, spec.dialects);
            subs.insert(
                sub.name.to_string(),
                CommandSig {
                    arity: sub.arity,
                    arg_roles,
                    traits: sub.traits,
                    leading_options,
                    leading_option_specs,
                    option_constraints: sub
                        .option_constraints
                        .iter()
                        .chain(
                            sub.subcommand_forms
                                .iter()
                                .flat_map(|form| form.option_constraints.iter()),
                        )
                        .collect(),
                    synopsis: sub.primary_synopsis(),
                    min_abbrev: sub.min_abbrev,
                },
            );
        }
        return Some(CommandSignature::WithSubcommands(SubcommandSig {
            subcommands: subs,
            allow_unknown: spec.allow_unknown_subcommands,
            subcommand_required: spec.arity.min > 0,
            default_form_first_word: spec.default_form_first_word,
            prefix_matching: spec.prefix_matching,
        }));
    }
    let arg_roles = spec
        .arg_roles
        .iter()
        .map(|(idx, role)| (*idx, *role))
        .collect();
    let leading_options = spec
        .switch_names(None)
        .into_iter()
        .map(str::to_string)
        .collect();
    let leading_option_specs = spec.option_specs(None);
    Some(CommandSignature::Simple(CommandSig {
        arity: spec.arity,
        arg_roles,
        traits: spec.traits,
        leading_options,
        leading_option_specs,
        option_constraints: spec
            .option_constraints
            .iter()
            .chain(
                spec.command_forms
                    .iter()
                    .flat_map(|form| form.option_constraints.iter()),
            )
            .collect(),
        // Deliberately permissive, like every other gate on this
        // dialect-agnostic path: the question it answers is "does this exist
        // in ANY dialect", so filtering forms by a version floor would defeat
        // the purpose.
        synopsis: spec.primary_synopsis(None),
        // A command name is never prefix-matched, so a simple command's
        // signature carries no abbreviation floor.
        min_abbrev: None,
    }))
}

/// Build the signature for a [`ScopedCommand`] — a command available only
/// inside a scoped body (`report::defstyle`'s `top` / `data` / `columns` / …).
///
/// An ensemble scoped command (non-empty `subcommands`) yields a
/// [`CommandSignature::WithSubcommands`] so the per-subcommand arity + W001
/// checks apply exactly as for a registry ensemble; a plain scoped command
/// yields a [`CommandSignature::Simple`].  Scoped commands are not
/// dialect-gated, so no dialect filtering is applied.
#[must_use]
pub fn signature_for_scoped_command(scoped: &ScopedCommand) -> CommandSignature {
    if !scoped.subcommands.is_empty() {
        let mut subs: HashMap<String, CommandSig> = HashMap::new();
        for sub in scoped.subcommands {
            let arg_roles = sub
                .arg_roles
                .iter()
                .map(|(idx, role)| (*idx, *role))
                .collect();
            subs.insert(
                sub.name.to_string(),
                CommandSig {
                    arity: sub.arity,
                    arg_roles,
                    traits: sub.traits,
                    // Scoped ensemble operations declare no option flags.
                    leading_options: BTreeSet::new(),
                    leading_option_specs: Vec::new(),
                    option_constraints: Vec::new(),
                    synopsis: sub.primary_synopsis(),
                    min_abbrev: sub.min_abbrev,
                },
            );
        }
        return CommandSignature::WithSubcommands(SubcommandSig {
            subcommands: subs,
            allow_unknown: scoped.allow_unknown_subcommands,
            subcommand_required: scoped.arity.min > 0,
            // Scoped ensembles declare no non-subcommand default form.
            default_form_first_word: None,
            // Scoped ensembles dispatch like any other Tcl ensemble.
            prefix_matching: PrefixMatching::Enabled,
        });
    }
    CommandSignature::Simple(CommandSig {
        arity: scoped.arity,
        arg_roles: HashMap::new(),
        // `ScopedCommand` itself carries no `traits` — only its
        // (reused-`SubCommand`) ensemble operations do.
        traits: Traits::empty(),
        leading_options: BTreeSet::new(),
        leading_option_specs: Vec::new(),
        option_constraints: Vec::new(),
        synopsis: scoped
            .hover
            .as_ref()
            .and_then(|h| h.synopsis.iter().copied().find(|s| !s.is_empty())),
        min_abbrev: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn unknown_command_returns_none() {
        let reg = registry();
        let sig = signature_for_command(
            &reg,
            "definitely_not_a_command_xyz",
            tcl_dialect::DialectProfile::plain_tcl(),
        );
        assert!(sig.is_none());
    }

    #[test]
    fn simple_command_returns_simple_sig() {
        let reg = registry();
        let sig = signature_for_command(&reg, "set", tcl_dialect::DialectProfile::plain_tcl())
            .expect("set should be in registry");
        let CommandSignature::Simple(cs) = sig else {
            panic!("expected Simple, got {sig:?}");
        };
        // `set var ?value?` — arity is 1..=2.
        assert!(cs.arity.accepts(1) || cs.arity.accepts(2));
    }

    #[test]
    fn subcommand_command_returns_with_subcommands() {
        let reg = registry();
        let sig = signature_for_command(&reg, "string", tcl_dialect::DialectProfile::plain_tcl())
            .expect("string should be in registry");
        let CommandSignature::WithSubcommands(scs) = sig else {
            panic!("expected WithSubcommands, got {sig:?}");
        };
        // `string length`, `string index`, etc. should be
        // populated for any 8.5+ dialect.
        assert!(
            !scs.subcommands.is_empty(),
            "expected non-empty subcommands for `string`"
        );
    }

    #[test]
    fn proc_returns_simple_sig_with_arity() {
        let reg = registry();
        let sig = signature_for_command(&reg, "proc", tcl_dialect::DialectProfile::plain_tcl())
            .expect("proc should be there");
        let CommandSignature::Simple(cs) = sig else {
            panic!("proc should be Simple");
        };
        // `proc name args body` — exactly 3 args.
        assert!(cs.arity.accepts(3));
    }

    #[test]
    fn dialect_filter_changes_subcommand_visibility() {
        let reg = registry();
        // `info` exists in every Tcl dialect; we just verify the
        // helper returns a non-empty subcommand map under a
        // narrow dialect.
        let sig =
            signature_for_command(&reg, "info", tcl_dialect::DialectProfile::by_name("tcl8.4"))
                .expect("info present in 8.4");
        let CommandSignature::WithSubcommands(scs) = sig else {
            panic!("info should have subcommands");
        };
        assert!(scs.subcommands.contains_key("body"));
    }
}
