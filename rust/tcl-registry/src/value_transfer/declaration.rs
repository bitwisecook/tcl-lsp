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

//! The three declaration states and their resolution
//! (`docs/design/compiler/value-transfers.md` § *One invocation, one
//! context*).
//!
//! For each specialisation on a spec, a subcommand, or a form: *inherited*
//! from the enclosing level, *declared* explicitly, or *declined*
//! explicitly. With `Option<…>` meaning "derive when absent", a more
//! specific form could not suppress a parent's evaluator; the third state
//! makes abstention a declaration. When every scope inherits, a semantics
//! is derived only from a descriptor that states the same operation:
//! `NativeLowering::CellReadModifyWrite(u)` states a read-modify-write and
//! `Traits::DESTROYS_VARIABLE` states an unbind. `VarWriteTyping::ElementsOf`
//! states a type relationship and `LOOP_LIST_HEADER` a CFG shape; neither
//! states iteration, so neither derives anything.

use crate::forms::CommandForm;
use crate::native_lowering::NativeLowering;
use crate::spec::{CommandSpec, SubCommand};
use crate::traits::Traits;

use super::cell_update::CellUpdateSemantics;
use super::route::{EvalRoute, NativeEvalId};
use super::unbind::UnbindSemantics;
use super::{
    AnalysisInputs, Budget, CommandSemantics, EvalAnswer, FactDomain, PlanAnswer, TransferAnswer,
};

/// The value-transfer declaration at one scope of a spec.
#[derive(Clone, Copy)]
pub enum SemanticsDeclaration {
    /// Say nothing at this scope: the enclosing scope's declaration
    /// applies, and at the outermost scope the derivation from descriptors
    /// that state the same operation.
    Inherited,
    /// An explicit registry-owned specialisation.
    Declared(&'static dyn CommandSemantics),
    /// `semantics none`: an explicit abstention that stops inheritance and
    /// derivation at this scope.
    Declined,
}

impl std::fmt::Debug for SemanticsDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inherited => f.write_str("Inherited"),
            Self::Declared(semantics) => f
                .debug_tuple("Declared")
                .field(&semantics.identity())
                .finish(),
            Self::Declined => f.write_str("Declined"),
        }
    }
}

impl SemanticsDeclaration {
    /// Whether this scope says nothing.
    #[must_use]
    pub const fn is_inherited(&self) -> bool {
        matches!(self, Self::Inherited)
    }
}

/// Which scope of a spec a declaration came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeclarationScope {
    /// The command itself.
    Command,
    /// The selected subcommand.
    Subcommand,
    /// The selected form.
    Form,
}

impl DeclarationScope {
    /// Stable spelling for the inventory.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Subcommand => "subcommand",
            Self::Form => "form",
        }
    }
}

/// A specialisation derived from a descriptor that states the same
/// operation. Owned, because it carries the descriptor's own facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedSemantics {
    /// From `NativeLowering::CellReadModifyWrite`.
    CellUpdate(CellUpdateSemantics),
    /// From `Traits::DESTROYS_VARIABLE`.
    Unbind(UnbindSemantics),
}

impl DerivedSemantics {
    /// The specialisation as a trait object.
    #[must_use]
    pub fn as_semantics(&self) -> &dyn CommandSemantics {
        match self {
            Self::CellUpdate(semantics) => semantics,
            Self::Unbind(semantics) => semantics,
        }
    }
}

impl CommandSemantics for DerivedSemantics {
    fn identity(&self) -> &'static str {
        self.as_semantics().identity()
    }

    fn route(&self) -> EvalRoute {
        self.as_semantics().route()
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        self.as_semantics().structure(input)
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        budget: &mut Budget,
    ) -> TransferAnswer {
        self.as_semantics().transfer(domain, input, budget)
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        self.as_semantics().evaluate(input, budget)
    }
}

/// How a resolved invocation came by its semantics: the inventory's
/// "declared semantics" column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticsOrigin {
    /// Declared explicitly at the named scope.
    Declared(DeclarationScope),
    /// Derived from a descriptor stating the same operation.
    Derived,
    /// Declined explicitly at the named scope.
    Declined(DeclarationScope),
    /// Nothing declared, nothing derivable.
    None,
}

impl SemanticsOrigin {
    /// Stable spelling for the inventory.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared(DeclarationScope::Command) => "declared (command)",
            Self::Declared(DeclarationScope::Subcommand) => "declared (subcommand)",
            Self::Declared(DeclarationScope::Form) => "declared (form)",
            Self::Derived => "derived",
            Self::Declined(DeclarationScope::Command) => "declined (command)",
            Self::Declined(DeclarationScope::Subcommand) => "declined (subcommand)",
            Self::Declined(DeclarationScope::Form) => "declined (form)",
            Self::None => "none",
        }
    }
}

/// The declaration state of one resolved invocation, resolved once with
/// the selected subcommand and form.
#[derive(Clone, Copy)]
pub enum ResolvedSemantics {
    /// Declared explicitly at `scope`.
    Declared {
        /// The specialisation.
        semantics: &'static dyn CommandSemantics,
        /// The scope that declared it.
        scope: DeclarationScope,
    },
    /// Derived from a descriptor stating the same operation.
    Derived(DerivedSemantics),
    /// Declined explicitly at `scope`.
    Declined {
        /// The scope that declined.
        scope: DeclarationScope,
    },
    /// Nothing declared, nothing derivable: the generic conservative
    /// behaviour.
    None,
}

impl std::fmt::Debug for ResolvedSemantics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Declared { semantics, scope } => f
                .debug_struct("Declared")
                .field("identity", &semantics.identity())
                .field("scope", scope)
                .finish(),
            Self::Derived(derived) => f.debug_tuple("Derived").field(derived).finish(),
            Self::Declined { scope } => f.debug_struct("Declined").field("scope", scope).finish(),
            Self::None => f.write_str("None"),
        }
    }
}

impl ResolvedSemantics {
    /// The specialisation, when there is one.
    #[must_use]
    pub fn semantics(&self) -> Option<&dyn CommandSemantics> {
        match self {
            Self::Declared { semantics, .. } => Some(*semantics),
            Self::Derived(derived) => Some(derived.as_semantics()),
            Self::Declined { .. } | Self::None => None,
        }
    }

    /// How the semantics came about.
    #[must_use]
    pub const fn origin(&self) -> SemanticsOrigin {
        match self {
            Self::Declared { scope, .. } => SemanticsOrigin::Declared(*scope),
            Self::Derived(_) => SemanticsOrigin::Derived,
            Self::Declined { scope } => SemanticsOrigin::Declined(*scope),
            Self::None => SemanticsOrigin::None,
        }
    }

    /// The declared route, when there is a specialisation.
    #[must_use]
    pub fn route(&self) -> Option<EvalRoute> {
        self.semantics().map(CommandSemantics::route)
    }

    /// Whether the specialisation is the direct route's one-target write of
    /// a value word ([`super::cell_write::CellWriteSemantics`], `set name
    /// value`): the command stores its last word, as Tcl substitutes it,
    /// into the place its `VarWrite` word names, and returns it. What a
    /// consumer reads to bind the written variable to the value word — the
    /// analyser's assignment binding and constant-string environment, the
    /// search-path record — in place of the command's spelling or an
    /// analyser hook.
    #[must_use]
    pub fn writes_value_word(&self) -> bool {
        self.route()
            == Some(EvalRoute::Direct {
                id: NativeEvalId::CellWrite,
            })
    }
}

/// Resolve the declaration states of `spec`, the selected `sub`, and the
/// selected `form` into one answer: the innermost explicit declaration or
/// abstention wins; when every scope inherits, the derivation applies.
#[must_use]
pub fn resolve_semantics(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    form: Option<&CommandForm>,
) -> ResolvedSemantics {
    resolve_semantics_scoped(spec, sub, form, true)
}

/// [`resolve_semantics`] with the command scope in or out of reach: an
/// instance invocation (`$obj method`) resolves against a class spec whose
/// command-level declaration and descriptors describe the class command,
/// not the method, so only the method's and form's scopes apply.
#[must_use]
pub fn resolve_semantics_scoped(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    form: Option<&CommandForm>,
    inherit_command: bool,
) -> ResolvedSemantics {
    let command = if inherit_command {
        spec.semantics
    } else {
        SemanticsDeclaration::Inherited
    };
    let scopes = [
        (
            DeclarationScope::Form,
            form.map_or(SemanticsDeclaration::Inherited, |form| form.semantics),
        ),
        (
            DeclarationScope::Subcommand,
            sub.map_or(SemanticsDeclaration::Inherited, |sub| sub.semantics),
        ),
        (DeclarationScope::Command, command),
    ];
    for (scope, declaration) in scopes {
        match declaration {
            SemanticsDeclaration::Inherited => {}
            SemanticsDeclaration::Declared(semantics) => {
                return ResolvedSemantics::Declared { semantics, scope };
            }
            SemanticsDeclaration::Declined => return ResolvedSemantics::Declined { scope },
        }
    }
    if inherit_command {
        derive(spec, sub)
    } else {
        ResolvedSemantics::None
    }
}

/// Derive a specialisation from a descriptor that states the same
/// operation, or nothing.
fn derive(spec: &CommandSpec, sub: Option<&SubCommand>) -> ResolvedSemantics {
    if sub.is_none()
        && let Some(NativeLowering::CellReadModifyWrite(update)) = spec.native_lowering
    {
        return ResolvedSemantics::Derived(DerivedSemantics::CellUpdate(CellUpdateSemantics {
            update,
            creates_absent: spec.safe_on_uninit,
        }));
    }
    let traits = spec.traits | sub.map_or_else(Traits::empty, |sub| sub.traits);
    if traits.contains(Traits::DESTROYS_VARIABLE) {
        return ResolvedSemantics::Derived(DerivedSemantics::Unbind(UnbindSemantics));
    }
    ResolvedSemantics::None
}
