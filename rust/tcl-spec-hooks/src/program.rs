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

//! What the host is given: one pack's hook bodies, and what it gives back.
//!
//! Deliberately not the loader's `HookDecl`. The host takes plain data — a
//! family, what the hook hangs off, the body text, and the declared inputs —
//! so it depends on no loader type and a loader depends on no host type; the
//! adapter between them is a `From` impl in whichever crate owns both, and is
//! a dozen lines.

use tcl_registry::pack_hooks::{HookFamily, HookInputs, HookSlot};

/// What a hook hangs off, which is what fixes the words it sees: a command
/// hook sees the words after the command name, a subcommand hook the words
/// after the subcommand word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOwner {
    /// A property on the command itself.
    Command,
    /// A property on one of its subcommands.
    Subcommand(String),
    /// The `-arity-hook` flag of one option row.
    Option {
        /// The owning subcommand, when the option is not a command-level one.
        subcommand: Option<String>,
        /// The option word as written.
        option: String,
    },
}

impl HookOwner {
    /// The subcommand word this hook belongs to, for `ctx`'s `subcommand` key.
    #[must_use]
    pub fn subcommand(&self) -> Option<&str> {
        match self {
            Self::Command => None,
            Self::Subcommand(name) => Some(name),
            Self::Option { subcommand, .. } => subcommand.as_deref(),
        }
    }
}

/// One hook body to host: everything the calling convention needs, and
/// nothing about where it was parsed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookProgram {
    /// The command whose spec carries the hook — `ctx`'s `command`.
    pub command: String,
    /// What the hook hangs off.
    pub owner: HookOwner,
    /// The family, which fixes the emitter verbs and what silence means.
    pub family: HookFamily,
    /// The declared parameter list, conventionally `words ctx`.
    pub parameters: Vec<String>,
    /// The body source.
    pub body: String,
    /// The inputs the hook declared it reads — the cacheability rule's input.
    pub inputs: HookInputs,
    /// The slot this hook is already bound to, when the caller allocated one.
    ///
    /// A slot is baked into a leaked `CommandSpec` as a function pointer and
    /// is therefore *process*-wide, while a host is per **thread** — so a
    /// second thread building its own host for the same pack must bind the
    /// slots the spec already carries rather than allocate fresh ones. `None`
    /// means "allocate one", which is what a caller owning both the spec and
    /// the host in one thread wants.
    pub slot: Option<HookSlot>,
}

impl HookProgram {
    /// A hook on the command itself, declaring no inputs (so: legal,
    /// uncacheable).
    #[must_use]
    pub fn new(command: impl Into<String>, family: HookFamily, body: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            owner: HookOwner::Command,
            family,
            parameters: vec!["words".to_owned(), "ctx".to_owned()],
            body: body.into(),
            inputs: HookInputs::unrestricted(),
            slot: None,
        }
    }

    /// The same hook, attached to a subcommand.
    #[must_use]
    pub fn on_subcommand(mut self, subcommand: impl Into<String>) -> Self {
        self.owner = HookOwner::Subcommand(subcommand.into());
        self
    }

    /// The same hook, declaring the inputs it reads.
    #[must_use]
    pub fn with_inputs(mut self, inputs: HookInputs) -> Self {
        self.inputs = inputs;
        self
    }

    /// The parameters the body is actually compiled with.
    ///
    /// [`Self::parameters`] is the conventional `words ctx`, but a hook that
    /// declared its inputs and did **not** name `words` must not be handed
    /// them: the shape cache keys on word *shape*, so such a hook reading word
    /// content would silently receive another call's answer. Dropping the
    /// parameter turns that into a "no such variable" from the sandbox the
    /// first time the body reads `$words`, which the host reports and
    /// quarantines like any other hook failure — a loud wrong instead of a
    /// quiet one.
    #[must_use]
    pub fn effective_parameters(&self) -> Vec<&str> {
        let binds_words = self.inputs.binds_words();
        self.parameters
            .iter()
            .map(String::as_str)
            .filter(|name| binds_words || *name != "words")
            .collect()
    }

    /// The same hook, bound to a slot the caller already allocated.
    #[must_use]
    pub fn with_slot(mut self, slot: HookSlot) -> Self {
        self.slot = Some(slot);
        self
    }

    /// A stable identity for diagnostics and crash records.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.owner {
            HookOwner::Command => format!("{} {}", self.command, self.family.field()),
            HookOwner::Subcommand(subcommand) => {
                format!("{} {} {}", self.command, subcommand, self.family.field())
            }
            HookOwner::Option { subcommand, option } => match subcommand {
                Some(subcommand) => format!(
                    "{} {} {option} {}",
                    self.command,
                    subcommand,
                    self.family.field()
                ),
                None => format!("{} {option} {}", self.command, self.family.field()),
            },
        }
    }
}

/// One pack's hooks, as the host receives them.
///
/// The unit of isolation: each pack gets its own engine instance with no
/// shared interpreter state, so a crash or quarantine in one pack cannot
/// degrade another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackPrograms {
    /// The pack name from `speclib <pack-name> <dsl-version> { … }`.
    pub pack: String,
    /// The pack source's content hash, reported in a crash record so a bug
    /// report names the exact bytes.
    pub content_hash: String,
    /// The DSL vocabulary version the pack declared.
    pub dsl_version: String,
    /// Every hook body the pack declares.
    pub programs: Vec<HookProgram>,
}

impl PackPrograms {
    /// An empty pack of that name.
    #[must_use]
    pub fn new(pack: impl Into<String>) -> Self {
        Self {
            pack: pack.into(),
            content_hash: String::new(),
            dsl_version: String::new(),
            programs: Vec::new(),
        }
    }

    /// Add one hook body.
    #[must_use]
    pub fn with(mut self, program: HookProgram) -> Self {
        self.programs.push(program);
        self
    }
}

/// One installed hook: the registry slot whose thunk now answers for it.
///
/// The caller turns this into the spec field it fills — `const_fold_fn(slot)`
/// for a fold, `arg_role_resolver_fn(slot)` for a resolver — and builds the
/// `CommandSpec` with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookInstallation {
    /// The hook's command.
    pub command: String,
    /// What it hangs off.
    pub owner: HookOwner,
    /// Its family.
    pub family: HookFamily,
    /// The registry slot, when one could be allocated and the body compiled.
    /// `None` means the hook is not installed at all: the command keeps its
    /// declarative facts, which is the same degradation an abstaining hook
    /// produces, minus the per-call cost.
    pub slot: Option<HookSlot>,
    /// Why the hook is not installed, when `slot` is `None`.
    pub declined: Option<String>,
}
