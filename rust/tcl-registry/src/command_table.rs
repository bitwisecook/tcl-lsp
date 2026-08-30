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

//! Command-table mutation **selectors**.
//!
//! A handful of commands mutate the interpreter's *command table* itself —
//! they define, move, or alias command names.  Consumers that model
//! command-name bindings (the flow-sensitive lattice in
//! `tcl_compiler::command_binding`, the lowerer's alias table, the
//! analyser's rename / alias records) need to know *which* calls do this
//! and *what they did*; before this descriptor existed each of them matched
//! `proc` / `rename` / `interp alias` by name.
//!
//! ## What this is now (centralisation ledger C8)
//!
//! This enum was for a time a **second** transition vocabulary: the
//! registry declared the coarse effect, consumers dispatched on it, and
//! each then re-destructured the argument layout for itself — beside
//! [`crate::state_transition`]'s `CommandBindingTransition`, which already
//! said the same thing precisely.  It is now a one-word **selector** for a
//! stock [`crate::StateTransitionDescriptor`]
//! ([`CommandTableEffect::transitions`]), and nothing else:
//!
//! - a shipped spec names the stock descriptor directly and does not stamp
//!   the selector as well;
//! - a `SpecTcl` pack, which cannot supply a Rust resolver, writes
//!   `command_table_effect` and the registry resolves it to the same stock
//!   descriptor;
//! - **every** consumer reads
//!   [`crate::CommandRegistry::command_binding_transitions`], never this
//!   enum.  The shape destructuring lives with the resolver, once.

/// How a command mutates the interpreter's command table.
///
/// Written by a `SpecTcl` pack on
/// [`crate::CommandSpec::command_table_effect`] (or the
/// [`crate::SubCommand`] twin for a subcommand-shaped mutator such as
/// `interp alias`), and resolved to its stock transition descriptor by
/// [`Self::transitions`].  A shipped spec names the descriptor itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandTableEffect {
    /// The call binds its first argument as a procedure — `proc name
    /// params body` (`Tcl_ProcObjCmd`, `generic/tclProc.c`).  Narrower
    /// than [`crate::Traits::DEFINES_PROCEDURE`], which also marks the
    /// `TclOO` metaclasses whose *name* argument sits behind a
    /// `create` / `new` subcommand word — a binding-lattice consumer
    /// reading argument 0 as the defined name must only see the
    /// `proc`-shaped form.
    DefinesProcedure,
    /// The call moves (or, with an empty target, deletes) a command —
    /// `rename oldName newName` (`Tcl_RenameObjCmd`,
    /// `generic/tclCmdMZ.c`, dispatching to `TclRenameCommand` in
    /// `generic/tclBasic.c`).
    RenamesCommands,
    /// The call creates (or, in the shorter forms, queries / deletes)
    /// a command alias — `interp alias` (`AliasCreate`,
    /// `generic/tclInterp.c`).  Stamped on `interp`'s `alias`
    /// subcommand.
    CreatesAliases,
}

impl CommandTableEffect {
    /// The stock [`crate::StateTransitionDescriptor`] this selector stands
    /// for — the *same* descriptor the shipped `proc` / `rename` / `interp
    /// alias` specs name, so a pack's shorthand and a shipped declaration
    /// produce one vocabulary through one resolver.
    #[must_use]
    pub const fn transitions(self) -> crate::state_transition::StateTransitionDescriptor {
        use crate::state_transition::command_binding;
        match self {
            Self::DefinesProcedure => command_binding::DEFINES_PROCEDURE,
            Self::RenamesCommands => command_binding::RENAMES_COMMANDS,
            Self::CreatesAliases => command_binding::CREATES_ALIASES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CommandTableEffect;
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::state_transition::{CommandBindingTransition, StateTransition};

    /// The selector a `SpecTcl` pack writes resolves to the *same* facts the
    /// shipped specs' descriptors produce — one vocabulary, one resolver
    /// (ledger C8). A pack cannot supply a Rust resolver, so this is the
    /// only route its declaration has, and it must not be a second one.
    #[test]
    fn the_pack_selector_resolves_to_the_stock_transitions() {
        let defined = CommandTableEffect::DefinesProcedure
            .transitions()
            .resolve(InvocationArguments::literals(&["greet", "", ""]));
        assert!(matches!(
            defined.facts(),
            [fact] if matches!(
                &fact.transition,
                StateTransition::CommandBinding(CommandBindingTransition::Define { .. })
            )
        ));

        let moved = CommandTableEffect::RenamesCommands
            .transitions()
            .resolve(InvocationArguments::literals(&["format", "origfmt"]));
        assert!(matches!(
            moved.facts(),
            [fact] if matches!(
                &fact.transition,
                StateTransition::CommandBinding(CommandBindingTransition::Move { .. })
            )
        ));

        let aliased = CommandTableEffect::CreatesAliases.transitions().resolve(
            InvocationArguments::literals(&["alias", "", "myfmt", "", "format"]),
        );
        assert!(matches!(
            aliased.facts(),
            [fact] if matches!(
                &fact.transition,
                StateTransition::CommandBinding(CommandBindingTransition::Alias { .. })
            )
        ));
    }

    /// A pack that stamps the alias selector on a command whose words are
    /// nothing like `interp alias` states no alias, rather than inventing
    /// one out of that command's own arguments.
    #[test]
    fn the_alias_selector_states_nothing_for_an_unshaped_call() {
        let stated = CommandTableEffect::CreatesAliases
            .transitions()
            .resolve(InvocationArguments::literals(&["myTree", "", "x", ""]));
        assert!(stated.facts().is_empty());
    }

    /// A dynamic operand never reaches a consumer as a source spelling: it
    /// is a typed unknown plus the descriptor's declared widening.
    #[test]
    fn a_dynamic_operand_is_typed_unknown_and_widened() {
        let words = [InvocationWord::Dynamic, InvocationWord::Literal("new")];
        let stated = CommandTableEffect::RenamesCommands
            .transitions()
            .resolve(InvocationArguments::structured(&words));
        assert!(stated.touches_command_bindings());
        // The move is still stated — its *source* is simply a typed unknown
        // carrying the argument index it came from, never the word `$old`.
        assert!(stated.command_bindings().all(|binding| matches!(
            binding,
            CommandBindingTransition::Move { from, to }
                if from.literal().is_none() && to.literal() == Some("new")
        )));
    }
}
