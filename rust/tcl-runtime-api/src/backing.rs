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

//! What a runtime backs, as the runtime itself reports it.
//!
//! A command spec *declares* how its behaviour reaches a runtime
//! (`CommandSpec::runtime_backing`); a runtime *reports* what it registered.
//! The two are separate facts, and a gate compares them: a declaration the
//! runtime does not bear out is drift. This module is the vocabulary of the
//! report, shared so that the bytecode VM and the WASM runtime answer in the
//! same words and a consumer (the `command-backing` gate, a rung-4 admission
//! check) reads both the same way.

use std::collections::BTreeMap;

/// How one runtime backs one command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegisteredBacking {
    /// A native handler is registered under the name.
    Builtin,
    /// A `TclOO` object command: a root the engine installs (`oo::object`,
    /// `oo::class`, …), or a command the object system binds in every
    /// object's namespace (`my`).
    Object,
    /// Defined by a Tcl library the runtime embeds, in the named file of that
    /// library, once the library is sourced.
    Stdlib {
        /// The library file, relative to the library root (`init.tcl`).
        file: &'static str,
    },
    /// Registered only to refuse: the runtime holds the name so that a call
    /// answers "not supported" instead of `invalid command name`.
    Unsupported,
    /// A handler that only a build with the numeric tower registers, in a
    /// build without it.
    NeedsNumericTower,
    /// Nothing answers to the name.
    Absent,
}

impl RegisteredBacking {
    /// The word a report prints for this answer.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Object => "object",
            Self::Stdlib { .. } => "stdlib",
            Self::Unsupported => "unsupported",
            Self::NeedsNumericTower => "needs-tower",
            Self::Absent => "absent",
        }
    }

    /// Whether the runtime registers something that executes the command: a
    /// handler, an object, a library definition, or (in a build without the
    /// numeric tower) the handler a tower build has. A refusal stub and an
    /// absence do not.
    #[must_use]
    pub const fn executes(self) -> bool {
        matches!(
            self,
            Self::Builtin | Self::Object | Self::Stdlib { .. } | Self::NeedsNumericTower
        )
    }
}

/// A runtime's answers, by command name.
///
/// A name is stored and looked up without a leading `::`: a runtime holds one
/// command per name whichever spelling a spec gives it, and the two spellings
/// of a command must not disagree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackingReport {
    entries: BTreeMap<String, RegisteredBacking>,
}

impl BackingReport {
    /// A report holding `entries`, each name without its leading `::`. The
    /// last answer for a name wins.
    #[must_use]
    pub fn from_entries(entries: impl IntoIterator<Item = (String, RegisteredBacking)>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(name, backing)| (canonical(&name).to_owned(), backing))
                .collect(),
        }
    }

    /// What the runtime reports for `name`: [`RegisteredBacking::Absent`] for a
    /// name it does not mention.
    #[must_use]
    pub fn of(&self, name: &str) -> RegisteredBacking {
        self.entries
            .get(canonical(name))
            .copied()
            .unwrap_or(RegisteredBacking::Absent)
    }

    /// Every name the runtime reports, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, RegisteredBacking)> {
        self.entries
            .iter()
            .map(|(name, backing)| (name.as_str(), *backing))
    }

    /// How many names the runtime reports.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the runtime reports nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn canonical(name: &str) -> &str {
    name.strip_prefix("::").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_one_command_whichever_spelling_asks() {
        let report = BackingReport::from_entries([
            ("set".to_owned(), RegisteredBacking::Builtin),
            ("::tcl::mathop::+".to_owned(), RegisteredBacking::Builtin),
            (
                "unknown".to_owned(),
                RegisteredBacking::Stdlib { file: "init.tcl" },
            ),
        ]);
        assert_eq!(report.of("set"), RegisteredBacking::Builtin);
        assert_eq!(report.of("::set"), RegisteredBacking::Builtin);
        assert_eq!(report.of("tcl::mathop::+"), RegisteredBacking::Builtin);
        assert_eq!(report.of("::tcl::mathop::+"), RegisteredBacking::Builtin);
        assert_eq!(
            report.of("::unknown"),
            RegisteredBacking::Stdlib { file: "init.tcl" }
        );
        assert_eq!(report.of("nonesuch"), RegisteredBacking::Absent);
        assert_eq!(report.len(), 3);
    }

    #[test]
    fn only_a_registered_handler_executes_the_command() {
        for (backing, executes) in [
            (RegisteredBacking::Builtin, true),
            (RegisteredBacking::Object, true),
            (RegisteredBacking::Stdlib { file: "init.tcl" }, true),
            (RegisteredBacking::NeedsNumericTower, true),
            (RegisteredBacking::Unsupported, false),
            (RegisteredBacking::Absent, false),
        ] {
            assert_eq!(backing.executes(), executes, "{}", backing.label());
        }
    }
}
