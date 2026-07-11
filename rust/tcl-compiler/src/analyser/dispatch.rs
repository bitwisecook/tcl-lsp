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

use tcl_registry::prelude::{DialectSet, OptionSpec};
use tcl_registry::scoped::ScopedCommand;
use tcl_registry::{ArgRole, Arity, CommandRegistry, Traits};

/// Signature for a simple Tcl command.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

/// Signature for a command that dispatches on a subcommand word.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl SubcommandSig {
    /// Resolve a subcommand word to its canonical [`CommandSig`], accepting a
    /// unique non-empty prefix the way Tcl's `Tcl_GetIndexFromObj` ensemble
    /// dispatch does (`string le` ⇒ `length`). An exact match wins; an
    /// ambiguous prefix resolves to `None`. Only the dialect-available
    /// subcommands are in the map, so prefix resolution is dialect-correct.
    #[must_use]
    pub fn resolve(&self, word: &str) -> Option<&CommandSig> {
        if word.is_empty() {
            return None;
        }
        if let Some(exact) = self.subcommands.get(word) {
            return Some(exact);
        }
        let mut hits = self
            .subcommands
            .iter()
            .filter(|(name, _)| name.starts_with(word));
        let (_, first) = hits.next()?;
        if hits.next().is_some() {
            return None; // ambiguous prefix
        }
        Some(first)
    }

    /// Whether `word` resolves to a known subcommand (exact or unique prefix).
    #[must_use]
    pub fn is_known(&self, word: &str) -> bool {
        self.resolve(word).is_some()
    }
}

/// What ``signature_for_command`` returned.
#[derive(Debug, Clone, PartialEq, Eq)]
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
/// The `dialect` argument selects which dialect-specific subcommand
/// set is materialised; pass `DialectSet::ALL_TCL` when the
/// caller has no specific dialect context.
#[must_use]
pub fn signature_for_command(
    registry: &CommandRegistry,
    cmd_name: &str,
    dialect: DialectSet,
) -> Option<CommandSignature> {
    let spec = registry.get_for_dialect(cmd_name, dialect)?;

    if !spec.subcommands.is_empty() {
        let mut subs: HashMap<String, CommandSig> = HashMap::new();
        for sub in spec.subcommands {
            // `dialects` filters out subcommands not available in
            // the current dialect.
            if let Some(spec_dialects) = sub.dialects
                && !spec_dialects.intersects(dialect)
            {
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
            // does not pin its own.
            let leading_options = sub
                .switch_names(Some(dialect), spec.dialects)
                .into_iter()
                .map(str::to_string)
                .collect();
            let leading_option_specs = sub.option_specs(Some(dialect), spec.dialects);
            subs.insert(
                sub.name.to_string(),
                CommandSig {
                    arity: sub.arity,
                    arg_roles,
                    traits: sub.traits,
                    leading_options,
                    leading_option_specs,
                },
            );
        }
        return Some(CommandSignature::WithSubcommands(SubcommandSig {
            subcommands: subs,
            allow_unknown: spec.allow_unknown_subcommands,
            subcommand_required: spec.arity.min > 0,
        }));
    }

    let arg_roles = spec
        .arg_roles
        .iter()
        .map(|(idx, role)| (*idx, *role))
        .collect();
    let leading_options = spec
        .switch_names(Some(dialect))
        .into_iter()
        .map(str::to_string)
        .collect();
    let leading_option_specs = spec.option_specs(Some(dialect));
    Some(CommandSignature::Simple(CommandSig {
        arity: spec.arity,
        arg_roles,
        traits: spec.traits,
        leading_options,
        leading_option_specs,
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
                },
            );
        }
        return CommandSignature::WithSubcommands(SubcommandSig {
            subcommands: subs,
            allow_unknown: scoped.allow_unknown_subcommands,
            subcommand_required: scoped.arity.min > 0,
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
        let sig = signature_for_command(&reg, "definitely_not_a_command_xyz", DialectSet::ALL_TCL);
        assert!(sig.is_none());
    }

    #[test]
    fn simple_command_returns_simple_sig() {
        let reg = registry();
        let sig = signature_for_command(&reg, "set", DialectSet::ALL_TCL)
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
        let sig = signature_for_command(&reg, "string", DialectSet::ALL_TCL)
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
        let sig =
            signature_for_command(&reg, "proc", DialectSet::ALL_TCL).expect("proc should be there");
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
            signature_for_command(&reg, "info", DialectSet::TCL84).expect("info present in 8.4");
        let CommandSignature::WithSubcommands(scs) = sig else {
            panic!("info should have subcommands");
        };
        assert!(scs.subcommands.contains_key("body"));
    }
}
