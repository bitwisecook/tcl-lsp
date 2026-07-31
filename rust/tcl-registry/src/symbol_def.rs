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

//! Symbol-definer descriptors — commands that introduce a *named
//! definition* into the document outline.
//!
//! `proc` (→ a function symbol) and the `TclOO` / snit / itcl class
//! definers already have bespoke analyser records ([`crate::spec::CommandSpec`]'s
//! [`Traits::DEFINES_PROCEDURE`] / [`Traits::IS_OO_METACLASS`] /
//! [`crate::spec::CommandSpec::definition_body`]) because they carry rich
//! structure (parameter lists, method tables, MRO).  A great many *other*
//! commands, though, do nothing more than bind a bare name that a reader wants
//! to jump to from the outline: `tcltest::test NAME …` names a test case,
//! `benchmark NAME …` names a benchmark, and so on.
//!
//! Rather than teach the analyser / signature-scanner / document-symbol
//! provider a new `match cmd_name` arm for each of these, a command declares —
//! as registry *data* — that one of its arguments is a definition name via
//! [`CommandSpec::defines_symbol`].  Every consumer then discovers the symbol
//! generically: it asks the registry which argument holds the name and what
//! outline category to file it under.  Adding the next such command is a
//! one-line spec change, never a compiler edit.
//!
//! [`Traits::DEFINES_PROCEDURE`]: crate::traits::Traits::DEFINES_PROCEDURE
//! [`Traits::IS_OO_METACLASS`]: crate::traits::Traits::IS_OO_METACLASS
//! [`CommandSpec::defines_symbol`]: crate::spec::CommandSpec::defines_symbol

/// The outline category a [`SymbolDef`] files its name under.
///
/// Deliberately coarse — it maps to an editor `SymbolKind` in the
/// document-symbol / workspace-symbol layer, and to a human label for hover /
/// completion detail.  Kept as an enum (not a raw `SymbolKind`) so the registry
/// crate does not depend on any LSP type, and so the mapping to a concrete
/// editor kind lives with the provider that owns the wire contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefinedSymbolKind {
    /// A test case defined by a testing command (`tcltest::test`), named for
    /// outline navigation.  Rendered as a function-like entry in the outline —
    /// a named, runnable unit, matching how other language servers surface test
    /// definitions.
    Test,
    /// A named test *constraint* registered by `tcltest::testConstraint NAME
    /// ?value?` — a boolean condition a test's `-constraints` gate on.  A
    /// named, immutable condition, so it reads as a constant in the outline.
    Constraint,
    /// A custom result-*matcher* registered by `tcltest::customMatch MODE
    /// command` — a new value accepted by `test -match`, backed by a comparison
    /// command.  A custom comparison strategy, so it reads as an operator.
    Matcher,
    /// An event handler bound by an event-handler command (`when EVENT { … }`
    /// in iRules), named for its event.  The top-level structure of an iRule
    /// is its handlers, so they are the entries the outline exists to show.
    Event,
}

impl DefinedSymbolKind {
    /// A short lowercase label for hover / completion detail.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Constraint => "constraint",
            Self::Matcher => "match mode",
            Self::Event => "event",
        }
    }
}

/// Declares that a command binds a definition *name* the outline should list.
///
/// Attached to a command via [`CommandSpec::defines_symbol`].  The name is read
/// from argument [`Self::name_arg`] (0-based, *after* the command word) and
/// filed under [`Self::kind`].  Consumers resolve the argument's value through
/// the compiler's constant-propagation machinery, so a name written as a
/// literal, a constant `$var`, or a foldable `[cmd …]` all resolve identically.
///
/// [`CommandSpec::defines_symbol`]: crate::spec::CommandSpec::defines_symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolDef {
    /// 0-based index, *after* the command word, of the argument holding the
    /// definition name (`0` for `test NAME description …`).
    pub name_arg: u8,
    /// 0-based index of an argument holding a short human description to show
    /// as the outline entry's detail (`1` for `test name DESCRIPTION …`), or
    /// `None` when the command has no description argument.  Only used when the
    /// argument resolves to a constant — a substituted description is skipped.
    pub detail_arg: Option<u8>,
    /// 0-based index of the argument that makes this call a *definition* rather
    /// than a reference.  `Some(1)` for `testConstraint NAME value` — the
    /// setter defines the constraint, but the one-argument getter form
    /// (`testConstraint NAME`) only reads it, so a symbol is recorded only when
    /// that argument is present.  `None` = every call is a definition
    /// (`test`, `customMatch`).
    pub requires_arg: Option<u8>,
    /// The outline category the name is filed under.
    pub kind: DefinedSymbolKind,
}

impl SymbolDef {
    /// Construct a [`SymbolDef`] naming the symbol at `name_arg` with `kind`,
    /// no description argument, and every call treated as a definition.
    #[must_use]
    pub const fn new(name_arg: u8, kind: DefinedSymbolKind) -> Self {
        Self {
            name_arg,
            detail_arg: None,
            requires_arg: None,
            kind,
        }
    }

    /// Set the description argument index (see [`Self::detail_arg`]).
    #[must_use]
    pub const fn with_detail(mut self, detail_arg: u8) -> Self {
        self.detail_arg = Some(detail_arg);
        self
    }

    /// Record a definition only when the argument at `idx` is present (see
    /// [`Self::requires_arg`]).
    #[must_use]
    pub const fn defined_when_present(mut self, idx: u8) -> Self {
        self.requires_arg = Some(idx);
        self
    }
}
