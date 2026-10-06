// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original method-name operands at an accepted native definition phase.

use super::{
    AllocationIncarnation, BindingKind, CommandAllocationSite, ExecutedScriptSource,
    SourceCommandTarget, SourceMethodReceiver, SourceReceiverMethodEntry,
    executed_script_source::source_text,
};
use crate::ir::{WordExpr, WordPart};
use tcl_lexer::{LexerConfig, Span};

/// A method-name reference retained independently of callable method lookup.
/// The entry, when present, is the original declaration at this definition
/// phase. A missing entry cannot borrow a later declaration with the same name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceDefinitionMethodReference {
    class: SourceCommandTarget,
    declaring_class: SourceCommandTarget,
    phase: SourceDefinitionMethodReferencePhase,
}

impl SourceDefinitionMethodReference {
    /// Exact class incarnation whose definition accepted this operand.
    #[must_use]
    pub fn class(&self) -> &SourceCommandTarget {
        &self.class
    }

    /// Original owner of the retained declaration, independently of inheritance.
    /// This does not provide a declaration when `method_entry()` is absent.
    #[must_use]
    pub fn declaring_class(&self) -> &SourceCommandTarget {
        &self.declaring_class
    }

    /// Instance or class-object member table selected by the original worker.
    #[must_use]
    pub fn receiver(&self) -> SourceMethodReceiver {
        self.phase.receiver
    }

    /// Actual original private definition worker that consumed this name.
    #[must_use]
    pub fn worker(&self) -> &SourceCommandTarget {
        &self.phase.worker
    }

    /// Full original source instance and worker invocation offset.
    #[must_use]
    pub fn invocation(&self) -> &CommandAllocationSite {
        &self.phase.invocation
    }

    /// Original unchanged written name operand.
    #[must_use]
    pub fn operand(&self) -> &WordExpr {
        &self.phase.operand
    }

    /// Frozen name consumed at the definition phase.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.phase.name
    }

    /// Exact original declaration available when this operand was consumed.
    #[must_use]
    pub fn method_entry(&self) -> Option<&SourceReceiverMethodEntry> {
        self.phase.entry.as_ref()
    }

    /// Unchanged name bytes within the original operand's source instance.
    /// Consumers separately validate that instance against their document.
    #[must_use]
    pub fn name_span(&self) -> Span {
        self.phase.name_span
    }
}

/// Inputs from the native definition owner after private-worker attestation.
pub(super) struct DefinitionMethodReferenceCapture {
    pub(super) receiver: SourceMethodReceiver,
    pub(super) worker: SourceCommandTarget,
    pub(super) invocation: CommandAllocationSite,
    pub(super) operand: WordExpr,
    pub(super) name: String,
    pub(super) entry: Option<SourceReceiverMethodEntry>,
    pub(super) config: LexerConfig,
}

/// Name and original method inventory before class registration publishes the
/// created command token. This cannot itself provide a class edit identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceDefinitionMethodReferencePhase {
    receiver: SourceMethodReceiver,
    worker: SourceCommandTarget,
    invocation: CommandAllocationSite,
    operand: WordExpr,
    name: String,
    name_span: Span,
    entry: Option<SourceReceiverMethodEntry>,
}

impl SourceDefinitionMethodReferencePhase {
    pub(super) fn capture(input: DefinitionMethodReferenceCapture) -> Option<Self> {
        if !input.worker.registry_backed
            || !input.worker.prepended.is_empty()
            || input.worker.implementation_generation != 0
            || !is_static_operand(&input.operand)
            || input.entry.as_ref().is_some_and(|entry| {
                entry.name() != input.name || entry.receiver() != input.receiver
            })
        {
            return None;
        }
        let start = ExecutedScriptSource::literal_word_base(
            source_text(&input.invocation.source)?,
            &input.operand,
            &input.name,
            input.config,
        )?;
        let end = start.checked_add(u32::try_from(input.name.len()).ok()?)?;
        Some(Self {
            receiver: input.receiver,
            worker: input.worker,
            invocation: input.invocation,
            operand: input.operand,
            name: input.name,
            name_span: Span::new(start, end),
            entry: input.entry,
        })
    }

    pub(super) fn with_created_class(
        self,
        class: &SourceCommandTarget,
    ) -> Option<SourceDefinitionMethodReference> {
        let allocation = class.identity.as_ref()?.allocation.as_ref()?;
        if class.kind != BindingKind::Class
            || !class.prepended.is_empty()
            || allocation.incarnation == AllocationIncarnation::RepeatedFresh
        {
            return None;
        }
        let declaring_class = self
            .entry
            .as_ref()
            .and_then(SourceReceiverMethodEntry::declaring_class)
            .unwrap_or(class)
            .clone();
        Some(SourceDefinitionMethodReference {
            class: class.clone(),
            declaring_class,
            phase: self,
        })
    }
}

fn is_static_operand(operand: &WordExpr) -> bool {
    match operand {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
        WordExpr::Template { parts, .. } => parts
            .iter()
            .all(|part| matches!(part, WordPart::Text { .. })),
        _ => false,
    }
}
