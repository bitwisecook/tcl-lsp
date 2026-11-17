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

//! The `# tcl-lsp: stub` directive records and the declared surface built from them.

use super::StubFlags;
use tcl_lexer::Span;

/// One parameter declared inside a ``# tcl-lsp: stub NAME {ARGS}``
/// brace block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubArgDef {
    /// Argument name as written.  Optional markers (``?…?``) are
    /// stripped before storage.
    pub name: String,
    /// Argument role — one of ``body`` / ``expr`` / ``var`` /
    /// ``var_read`` / ``name`` / ``pattern`` / ``channel`` /
    /// ``value`` (the default when no ``:role`` annotation is
    /// supplied).
    pub role: String,
    /// ``true`` when the source token is wrapped in ``?…?``.
    pub optional: bool,
}

/// Inline `# stub: NAME ARGS BODY` directive capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubCommandDef {
    /// Stub command name.
    pub name: String,
    /// Parsed parameter list from the ``{ARGS}`` brace block
    /// (empty when the block is ``{}``).
    pub args: Vec<StubArgDef>,
    /// Span of the comment line carrying the directive.
    pub range: Span,
    /// Trailing flag set (``-barrier`` / ``-loop`` / ``-pure``
    /// / ``-mutator`` / ``-unsafe`` / ``-scope_alias`` /
    /// ``-extension``).
    pub flags: StubFlags,
    /// The frame effect ``-frame own|none|caller`` states, in the registry's
    /// vocabulary ([`tcl_registry::model::DeclaredFrameEffect`]); unstated
    /// without one.
    pub frame: tcl_registry::model::DeclaredFrameEffect,
    /// `true` when this declaration came from a workspace sidecar rather than
    /// the analysed document. Such declarations participate in resolution but
    /// cannot produce source-positioned shadow diagnostics.
    pub from_sidecar: bool,
}

impl StubCommandDef {
    /// Ingest this directive as a provenance-tagged
    /// [`tcl_registry::model::DeclaredCommand`].
    ///
    /// The source span stays here — the directive keeps it for diagnostic
    /// emitters — and each argument's role word is canonicalised through
    /// the registry's own [`tcl_registry::model::role_for_word`], so a
    /// declared argument and a catalogue argument are the same fact.
    ///
    /// [`Self::from_sidecar`] chooses the §6.4 provenance label: a workspace
    /// `.tcl.stubs` file is [`Provenance::WorkspaceUntrusted`], an inline
    /// block in the analysed buffer is [`Provenance::Document`].
    ///
    /// The trailing flags land on the fields a catalogue command states the
    /// same facts on ([`Self::declared_traits`],
    /// [`Self::declared_side_effects`]), so every consumer that reads those
    /// fields through [`tcl_registry::model::DocumentCommandSurface`] sees a
    /// stubbed command the way it sees a catalogued one.
    ///
    /// A declaration flagged ``-extension`` starts from the conservative
    /// default for a command native code registers
    /// ([`tcl_registry::model::DeclaredCommand::extension`]), and the facts the
    /// other flags state narrow it, each on its own axis
    /// ([`tcl_registry::model::DeclaredCommand::narrowed_by`]); an unflagged
    /// declaration starts from nothing, as it always has.
    ///
    /// [`Provenance::WorkspaceUntrusted`]: tcl_dialect::model::Provenance::WorkspaceUntrusted
    /// [`Provenance::Document`]: tcl_dialect::model::Provenance::Document
    #[must_use]
    pub fn to_declared_command(&self) -> tcl_registry::model::DeclaredCommand {
        use tcl_dialect::model::Provenance;
        use tcl_registry::model::{DeclaredArgument, DeclaredCommand, role_for_word};
        let arguments = self
            .args
            .iter()
            .map(|a| DeclaredArgument {
                name: a.name.clone(),
                role: role_for_word(&a.role),
                optional: a.optional,
            })
            .collect();
        let provenance = if self.from_sidecar {
            Provenance::WorkspaceUntrusted
        } else {
            Provenance::Document
        };
        if self.flags.contains(StubFlags::EXTENSION) {
            return DeclaredCommand::extension(self.name.clone(), arguments, provenance)
                .narrowed_by(
                    self.flags.contains(StubFlags::PURE),
                    self.declared_traits(),
                    self.declared_side_effects(),
                )
                .with_frame_effect(self.frame);
        }
        DeclaredCommand::new(self.name.clone(), arguments, provenance)
            .with_pure(self.flags.contains(StubFlags::PURE))
            .with_traits(self.declared_traits())
            .with_side_effects(self.declared_side_effects())
            .with_frame_effect(self.frame)
    }

    /// The traits the flags state, each on the field its catalogue
    /// counterpart uses: `-barrier` is [`Traits::CREATES_DYNAMIC_BARRIER`],
    /// `-loop` [`Traits::HAS_LOOP_BODY`], `-unsafe`
    /// [`Traits::UNSAFE`] with [`Traits::SAFE_INTERP_HIDDEN`] (as `exec`
    /// states them), `-scope_alias` [`Traits::CREATES_SCOPE_ALIAS`], and
    /// `-mutator` [`Traits::READS_BEFORE_WRITE`] — the read half of a
    /// read-modify-write, which `lset` states beside the effect
    /// [`Self::declared_side_effects`] gives.
    ///
    /// [`Traits::CREATES_DYNAMIC_BARRIER`]: tcl_registry::Traits::CREATES_DYNAMIC_BARRIER
    /// [`Traits::HAS_LOOP_BODY`]: tcl_registry::Traits::HAS_LOOP_BODY
    /// [`Traits::UNSAFE`]: tcl_registry::Traits::UNSAFE
    /// [`Traits::SAFE_INTERP_HIDDEN`]: tcl_registry::Traits::SAFE_INTERP_HIDDEN
    /// [`Traits::CREATES_SCOPE_ALIAS`]: tcl_registry::Traits::CREATES_SCOPE_ALIAS
    /// [`Traits::READS_BEFORE_WRITE`]: tcl_registry::Traits::READS_BEFORE_WRITE
    #[must_use]
    pub fn declared_traits(&self) -> tcl_registry::Traits {
        use tcl_registry::Traits;
        const FIELDS: [(StubFlags, Traits); 5] = [
            (StubFlags::BARRIER, Traits::CREATES_DYNAMIC_BARRIER),
            (StubFlags::LOOP, Traits::HAS_LOOP_BODY),
            (
                StubFlags::UNSAFE,
                Traits::UNSAFE.union(Traits::SAFE_INTERP_HIDDEN),
            ),
            (StubFlags::SCOPE_ALIAS, Traits::CREATES_SCOPE_ALIAS),
            (StubFlags::MUTATOR, Traits::READS_BEFORE_WRITE),
        ];
        FIELDS
            .iter()
            .filter(|(flag, _)| self.flags.contains(*flag))
            .fold(Traits::empty(), |traits, (_, field)| traits.union(*field))
    }

    /// The side effects the flags state: `-mutator` reads and writes a
    /// variable — the [`SideEffectTarget::Variable`] effect `lset` and
    /// `lappend` declare for the target they mutate.
    ///
    /// [`SideEffectTarget::Variable`]: tcl_registry::side_effects::SideEffectTarget::Variable
    #[must_use]
    pub fn declared_side_effects(&self) -> Vec<tcl_registry::side_effects::SideEffect> {
        use tcl_registry::side_effects::{SideEffect, SideEffectTarget};
        if !self.flags.contains(StubFlags::MUTATOR) {
            return Vec::new();
        }
        vec![SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]
    }
}

/// Ingest a slice of [`StubCommandDef`] records as the document's
/// [`tcl_registry::model::DeclaredSurface`].
///
/// Declarations are ingested in slice order and a later one of the same
/// name replaces an earlier one, which is both the directive's documented
/// "last one wins" rule and what makes an inline block override a
/// workspace sidecar (the caller ingests sidecar declarations first).
#[must_use]
pub fn build_declared_surface(defs: &[StubCommandDef]) -> tcl_registry::model::DeclaredSurface {
    let mut surface = tcl_registry::model::DeclaredSurface::new();
    for def in defs {
        surface.declare(def.to_declared_command());
    }
    surface
}

/// Inline `# stub-expr: NAME ARGS` directive capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubExprDef {
    /// Stub expression-function name.
    pub name: String,
    /// Either ``"function"`` (``stub expr-func``) or
    /// ``"operator"`` (``stub expr-op``).
    pub kind: String,
    /// Number of arguments (functions) or operands (operators).
    /// Defaults to 1 for functions and 2 for operators when the
    /// trailing arity word is absent.
    pub arity: u32,
    /// Span of the comment line carrying the directive.
    pub range: Span,
    /// See [`StubCommandDef::from_sidecar`].
    pub from_sidecar: bool,
}
