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

//! Namespace and ensemble transitions: exports, imports, paths and ensemble dispatch tables.

use super::TransitionSubject;

/// The namespace selected by a namespace-state transition.
///
/// The current namespace is intentionally distinct from a literal namespace
/// word.  Keeping that distinction in the registry lets target-neutral
/// consumers retain the Tcl execution context without treating the spelling
/// of an unqualified word as its fully-qualified runtime value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceTransitionTarget {
    /// The namespace current at the invocation site.
    Current,
    /// A namespace selected by a command operand.
    Named(TransitionSubject),
}

/// Exact namespace ensemble management operation selected by the Registry.
/// Prefix spellings and other literal words remain `Other` until the selected
/// sub-subcommand resolver supplies a unique operation receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsembleManagementOperation {
    /// Create a command linked to the current namespace.
    Create,
    /// Query or update an existing ensemble command.
    Configure,
    /// Test whether a command is an ensemble (read-only; normally no transition).
    Exists,
    /// Literal but unsupported or abbreviated operation spelling.
    Other,
}

/// The one command slot a literal `namespace ensemble create` installs.
/// Nothing else in the command table changes; subcommands dispatch at call
/// time through the ensemble's own configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsembleCommandName {
    /// No `-command`: the current namespace's own qualified name, published
    /// in its parent namespace (`::` itself for the global namespace).
    CurrentNamespace,
    /// The last `-command` value; a relative name resolves against the
    /// current namespace.
    Named(TransitionSubject),
}

/// Where one subcommand of a literal ensemble dispatches.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnsembleDispatchTarget {
    /// A command prefix whose head is qualified in the ensemble's namespace
    /// when relative, then looked up exactly (`-map`, and `-subcommands`
    /// before 8.6).
    Qualified(Vec<String>),
    /// A name looked up relative to the ensemble's namespace at call time
    /// (`-subcommands` from 8.6).
    Relative(String),
}

/// The dispatch table a literal `namespace ensemble create` freezes: its
/// subcommands in table order, whether unique prefixes select them, and
/// whether an `-unknown` handler answers a miss. Absent when dispatch
/// depends on later namespace exports or on `-parameters`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnsembleDispatchTable {
    /// Subcommand names and their targets.
    pub entries: Vec<(String, EnsembleDispatchTarget)>,
    /// `-prefixes`: unique prefixes select a subcommand.
    pub prefixes: bool,
    /// A non-empty `-unknown` handler runs on a miss.
    pub unknown_handler: bool,
}

impl EnsembleDispatchTable {
    /// The entry C Tcl's ensemble selects for `word`: an exact name, else a
    /// unique prefix when `-prefixes` allows it. `None` is a miss.
    #[must_use]
    pub fn select(&self, word: &str) -> Option<&EnsembleDispatchTarget> {
        let names = self
            .entries
            .iter()
            .map(|(name, _)| name.as_bytes())
            .collect::<Vec<_>>();
        tcl_cmd_core::ensemble::resolve_subcommand(&names, word.as_bytes(), self.prefixes)
            .map(|index| &self.entries[index].1)
    }
}

/// A mutation of namespace-owned state.
///
/// These are deliberately semantic operations, rather than command names or
/// option spellings.  Registry resolvers select the applicable variant; the
/// compiler only projects the resulting typed state operation.  Pattern and
/// list operands remain whole Tcl values, never host-language token streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceTransition {
    /// Ensure a namespace exists, creating missing ancestors as Tcl does for
    /// `namespace eval`.
    Ensure {
        /// Namespace that must exist before the evaluated script begins.
        namespace: NamespaceTransitionTarget,
    },
    /// Delete a namespace and every namespace, command, variable, and
    /// namespace-local handler below it.
    Delete {
        /// Root of the recursively deleted namespace tree.
        namespace: NamespaceTransitionTarget,
    },
    /// Change the exported-command pattern list of a namespace.
    Export {
        /// Namespace whose export list changes.
        namespace: NamespaceTransitionTarget,
        /// Export patterns or the `-clear` control word, as Tcl values.
        patterns: Vec<TransitionSubject>,
    },
    /// Install command imports into a namespace.
    Import {
        /// Namespace receiving imported command bindings.
        namespace: NamespaceTransitionTarget,
        /// Whether existing commands may be replaced. An unresolved leading
        /// option leaves this unknown rather than assuming a non-forcing import.
        force: Option<bool>,
        /// Import patterns, retained as whole Tcl values.
        patterns: Vec<TransitionSubject>,
    },
    /// Remove imported command bindings from a namespace.
    Forget {
        /// Namespace losing imported command bindings.
        namespace: NamespaceTransitionTarget,
        /// Import patterns, retained as whole Tcl values.
        patterns: Vec<TransitionSubject>,
    },
    /// Change the command-resolution path of a namespace.
    SetPath {
        /// Namespace whose path changes.
        namespace: NamespaceTransitionTarget,
        /// Tcl list value containing the path entries.
        path: TransitionSubject,
    },
    /// Change the namespace-local fallback command handler.
    SetUnknown {
        /// Namespace whose handler changes.
        namespace: NamespaceTransitionTarget,
        /// Command-prefix Tcl value installed as the handler.
        handler: TransitionSubject,
    },
    /// Create or reconfigure an ensemble dispatch surface in a namespace.
    Ensemble {
        /// Namespace owning the ensemble dispatch state.
        namespace: NamespaceTransitionTarget,
        /// Exact selected operation; dynamic dispatch remains unknown.
        operation: Option<EnsembleManagementOperation>,
        /// Complete post-head argv, retaining each original operand ordinal.
        /// Consumers must use the selected Registry option schema before
        /// assigning map or subcommand meaning to these values.
        arguments: Vec<TransitionSubject>,
        /// The command a `create` whose every option and value is literal,
        /// known and valid for the selected release installs; `None` for any
        /// other operation or form.
        created: Option<EnsembleCommandName>,
        /// The created ensemble's frozen dispatch table, when `created` is
        /// known and its subcommands do not depend on later namespace state.
        dispatch: Option<EnsembleDispatchTable>,
    },
}

impl NamespaceTransition {
    /// Split the sole exact export control word from the retained pattern
    /// operands. Other dash-prefixed values, including `--`, are patterns.
    /// An unknown leading value leaves both clearing and pattern layout unknown.
    #[must_use]
    pub fn export_pattern_operands(
        patterns: &[TransitionSubject],
    ) -> Option<(bool, &[TransitionSubject])> {
        let clear = patterns.first().map_or(Some(false), |first| {
            first.literal().map(|word| word == "-clear")
        })?;
        Some((clear, &patterns[usize::from(clear)..]))
    }

    /// Registry-authored global helper invoked before C namespace import.
    /// Absence or actual body effects require an independent current-table proof.
    #[must_use]
    pub const fn import_preload_command(
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<&'static str> {
        match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(_) => Some("auto_import"),
            tcl_syntax::naming::NativeNameProtocol::Jim084 => None,
        }
    }

    /// Select the exact leading export control using the independently
    /// selected native pattern extent. Remaining operands retain whole values.
    #[must_use]
    pub fn export_pattern_byte_operands<'a>(
        patterns: &'a [&'a [u8]],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(bool, &'a [&'a [u8]])> {
        if !matches!(
            policy.recipe(),
            tcl_syntax::naming::NativeNameProtocol::C(_)
        ) {
            return None;
        }
        let clear = patterns.first().map_or(Some(false), |first| {
            policy.recipe().namespace_export_clears(first).ok()
        })?;
        Some((clear, &patterns[usize::from(clear)..]))
    }

    /// Select the leading import option using the same independently selected
    /// pattern extent as actual import lookup; every remaining value is intact.
    #[must_use]
    pub fn import_pattern_byte_operands<'a>(
        patterns: &'a [&'a [u8]],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(bool, &'a [&'a [u8]])> {
        let forced = patterns.first().map_or(Some(false), |first| {
            policy.recipe().namespace_import_forces(first).ok()
        })?;
        Some((forced, &patterns[usize::from(forced)..]))
    }

    /// Whether this transition can change command lookup in the current
    /// interpreter.
    ///
    /// Creation alone installs no command, and changing an export pattern only
    /// affects a later import operation. Every other variant can add, remove,
    /// redirect, or provide fallback command resolution immediately.
    #[must_use]
    pub const fn affects_command_resolution(&self) -> bool {
        matches!(
            self,
            Self::Delete { .. }
                | Self::Import { .. }
                | Self::Forget { .. }
                | Self::SetPath { .. }
                | Self::SetUnknown { .. }
                | Self::Ensemble { .. }
        )
    }
}
