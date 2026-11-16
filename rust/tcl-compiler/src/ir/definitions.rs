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

//! Procedure, iRules event body, method and class-relation definitions.

use super::command_tokens::CommandTokens;
use super::procedure_identity::{ProcedureBodyKind, ProcedureId};
use super::script::{ExecutionNamespace, Script};
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::definer::{CallableRole, MemberReceiver};

// Procedure and module types

/// A procedure definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Procedure {
    /// Short procedure name.
    pub name: String,
    /// Fully qualified name (e.g. `::ns::proc`).
    pub qualified_name: String,
    /// Original source publication geometry; no Native binding or body admission.
    pub publication: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    /// Original allocating instruction; source bytes and display labels cannot replace it.
    pub declaration_site: Option<crate::command_binding::CommandAllocationSite>,
    /// Body frame purpose, independent of the reporting label.
    pub body_kind: ProcedureBodyKind,
    /// Parameter names.
    pub params: Vec<String>,
    /// Source span of the definition.
    pub span: Span,
    /// Procedure body.
    pub body: Script,
    /// Raw parameter list text.
    pub params_raw: String,
    /// Source text of the body (`None` for synthetic procs like `when`).
    pub body_source: Option<String>,
    /// Offset of [`Self::body_source`]'s first byte in the unit source.
    ///
    /// A statement of the body carries an absolute span; subtracting this
    /// offset turns it into the body-relative position Tcl counts `errorLine`
    /// and `(procedure "p" line N)` from. `0` for a procedure with no body
    /// text of its own (a synthetic or lambda body unit).
    pub body_offset: u32,
    /// Whether defined inside `namespace eval`.
    pub namespace_scoped: bool,
    /// BIG-IP handler priority (`0..=1000`, default 500).
    pub base_priority: u32,
}

/// A Registry-selected iRules event body retained by its actual lowering
/// producer. This source contract supplies conditional frame policy only;
/// it establishes no worker, epoch, entered event, Normal or current value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceIrulesEventBody {
    event: String,
    event_word: Option<tcl_lexer::NativeWord>,
    pub(super) declaration: Span,
    pub(super) body: std::sync::Arc<Script>,
    pub(super) image: tcl_lexer::SourceImage,
    pub(super) config: LexerConfig,
    pub(super) registry: tcl_registry::RegistrySemanticKey,
}

impl SourceIrulesEventBody {
    pub(crate) fn from_lowering(
        event: &str,
        declaration: Span,
        body: &Script,
        image: tcl_lexer::SourceImage,
        config: LexerConfig,
        registry: &tcl_registry::CommandRegistry,
        event_word: Option<tcl_lexer::NativeWord>,
    ) -> Self {
        let event_word = event_word.filter(|word| {
            word.image() == &image
                && word.config() == config
                && declaration.start() <= word.span().start()
                && word.span().end() <= declaration.end()
        });
        Self {
            event: event.to_owned(),
            event_word,
            declaration,
            body: std::sync::Arc::new(body.clone()),
            image,
            config,
            registry: registry.snapshot().semantic_key(),
        }
    }

    /// Event identity selected at the real declaration, without name parsing.
    #[must_use]
    pub fn event(&self) -> &str {
        &self.event
    }

    /// Original complete event operand under its retained source generation.
    /// This is declaration geometry, without installation, activation or TMM facts.
    #[must_use]
    pub fn event_word(
        &self,
        image: &tcl_lexer::SourceImage,
        config: LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&tcl_lexer::NativeWord> {
        (self.matches_source(image, config) && self.registry == registry.snapshot().semantic_key())
            .then_some(self.event_word.as_ref())
            .flatten()
    }

    pub(crate) fn owns_procedure(
        &self,
        procedure: &Procedure,
        image: &tcl_lexer::SourceImage,
        config: LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        self.declaration == procedure.span
            && self.body.as_ref() == &procedure.body
            && self.matches_source(image, config)
            && self.registry == registry.snapshot().semantic_key()
    }

    /// Full original source/configuration correspondence, without event reachability.
    #[must_use]
    pub fn matches_source(&self, image: &tcl_lexer::SourceImage, config: LexerConfig) -> bool {
        &self.image == image && self.config == config
    }

    pub(crate) fn conditional_lattice_entry(
        &self,
        body: &Script,
        config: LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::var_resolve::ResolveContext> {
        if config != self.config.nested().normalized()
            || self.registry != registry.snapshot().semantic_key()
        {
            return None;
        }
        let mut expected = self.body.as_ref().clone();
        crate::lattice_rebase::rebase_script(&mut expected, -i64::from(self.declaration.start()));
        (expected == *body).then(|| self.conditional_entry())
    }

    pub(crate) fn conditional_entry(&self) -> crate::var_resolve::ResolveContext {
        crate::connection_scope::event_resolve_context(&self.event)
    }
}

/// A method definition within a class body.
///
/// Compiles like [`Procedure`] but carries class context for
/// interprocedural analysis and devirtualisation.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    /// Issued owner of the retained definition script; not a class allocation.
    pub definition_scope: Option<ProcedureId>,
    /// Enclosing class name.
    pub class_name: String,
    /// Method name.
    pub method_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Method body.
    pub body: Script,
    /// Current namespace used to resolve command heads in the method body.
    /// `TclOO` selects the receiver's object namespace at runtime; other
    /// registry definer families can name their definition target exactly.
    pub execution_namespace: ExecutionNamespace,
    /// Method kind.
    pub kind: MethodKind,
    /// Source span (may be absent for synthetic methods).
    pub span: Option<Span>,
    /// Instance-variable names in scope for this method (class-level
    /// `variable` declarations + the method's own `variable` decls).
    /// A method that *writes* any of these mutates object state and is
    /// therefore impure for O126 purposes, even though the write looks
    /// like a plain local `set`. Used by interprocedural method-purity.
    pub instance_vars: std::collections::HashSet<String>,
    /// Actual unanimous entered receiver state, independent of lexical class labels.
    pub(crate) original_receiver_context: Option<
        std::sync::Arc<
            crate::command_binding::original_receiver_body_context::OriginalReceiverBodyContext,
        >,
    >,
    /// Original counted `ParamList` producer; it grants no entered activation.
    pub(crate) original_parameters:
        Option<std::sync::Arc<crate::command_binding::formal_topology::OriginalFormalTopology>>,
}

/// The kind of a class method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MethodKind {
    /// Regular instance method.
    #[default]
    Method,
    /// Class-level method.
    ClassMethod,
    /// Constructor.
    Constructor,
    /// Destructor.
    Destructor,
}

impl MethodKind {
    /// The frame shape a callable member opens, read off its member-effect
    /// facts (`registry-consumer-contracts.md` § *The member-effect
    /// descriptor*): its [`CallableRole`] and the side its row resolves to
    /// after every wrapper shift.
    ///
    /// A method on the instances is a [`Self::Method`] and one on the class or
    /// type object a [`Self::ClassMethod`]; a namespace procedure
    /// ([`CallableRole::Procedure`], snit's `proc`) runs with no instance in
    /// frame, the class-method shape. A constructor or destructor exists only
    /// on the instances: the type-object spelling is no member at all (tclsh
    /// 8.6.18 and 9.0.4: `oo::class create X { self constructor {} {} }` →
    /// `invalid command name "constructor"`). An option accessor or mutator
    /// opens no method frame of its own, and nothing is a method on both
    /// sides at once, so each of those answers `None`.
    #[must_use]
    pub const fn from_effect(role: CallableRole, receiver: MemberReceiver) -> Option<Self> {
        match (role, receiver) {
            (CallableRole::Method, MemberReceiver::Instance) => Some(Self::Method),
            (CallableRole::Method, MemberReceiver::TypeObject) | (CallableRole::Procedure, _) => {
                Some(Self::ClassMethod)
            }
            (CallableRole::Constructor, MemberReceiver::Instance) => Some(Self::Constructor),
            (CallableRole::Destructor, MemberReceiver::Instance) => Some(Self::Destructor),
            (CallableRole::Method, MemberReceiver::Both)
            | (
                CallableRole::Constructor | CallableRole::Destructor,
                MemberReceiver::TypeObject | MemberReceiver::Both,
            )
            | (CallableRole::Accessor | CallableRole::Mutator, _) => None,
        }
    }

    /// Return the string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::ClassMethod => "classmethod",
            Self::Constructor => "constructor",
            Self::Destructor => "destructor",
        }
    }
}

/// Whole-module evidence about OO definitions the lowering could not
/// statically model, grouped so consumers widen on exactly the right
/// axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OoDefinitionEvidence {
    /// An `oo::class create` / `oo::define` targeted a class whose name
    /// is dynamic (`oo::define $cls { … }`): *any* class's methods may
    /// have been touched, so whole-module OO evidence is incomplete.
    pub dynamic_target: bool,
    /// A class-reference member carried a dynamic word (`superclass $b`):
    /// that class's ancestry is unknown, so hierarchy-scoped analysis
    /// must fall back to whole-module widening.
    pub dynamic_class_relations: bool,
    /// At least one executable definition/member body could not be published
    /// through [`Module::executable_script_roots`]. This includes dynamic or
    /// otherwise unsupported definer bodies, accessor/configuration scripts,
    /// and definitions nested in a method frame that the source-root post-pass
    /// does not revisit. Whole-module command-effect consumers must fail closed
    /// when this is set: absence from `methods` is not evidence that the runtime
    /// body cannot mutate the command table.
    pub unretained_executable_roots: bool,
}

/// One selected class-relation member and its original enclosing definer.
/// Source vectors retain their producers; class labels supply reporting only.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassRelation {
    pub(crate) declaring_class: String,
    pub(crate) definer: std::sync::Arc<CommandTokens>,
    pub(crate) member: std::sync::Arc<CommandTokens>,
    pub(crate) target_word: usize,
    pub(crate) body_word: usize,
}
