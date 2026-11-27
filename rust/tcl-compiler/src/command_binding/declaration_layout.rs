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

//! Original operand layouts observed in an original body's own frame.
//!
//! A layout observation precedes operand evaluation. It preserves candidate
//! lookup and logical grammar for diagnostics. Declared frames and original
//! entered frames retain their independent identities; this never publishes compiler
//! admission, successful dispatch, an entered body, or closed runtime effects.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{
    CommandAllocationSite, OriginalCompilationLookupAdvice, SourceCommandBindings,
    SourceConditionalBodyEntry, SourceLookupSnapshot,
};

mod invocation_advice;
mod layout_recording;
mod metadata_context;
mod script_activation;
mod source_headers;
mod source_operand_advice;
mod variable_frame;
mod variable_symbols;

pub(super) use invocation_advice::{
    direct_declaration_operand_owner, local_read_scope_is_excluded, symbolic_version,
};
pub(super) use variable_symbols::root_diagnostic_namespace;

/// Original diagnostic frame ownership. A root script retains its actual
/// global frame; it never receives a synthetic procedure allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OriginalDiagnosticFrameEntry {
    Body(Arc<SourceConditionalBodyEntry>),
    DeclaredReceiver(Arc<super::SourceDeclaredReceiverBodyEntry>),
    LogicalReceiver(Arc<OriginalLogicalReceiverSourceBody>),
    DeclaredProcedure(Arc<super::declaration_preview::DeclaredProcedureBody>),
    RootScript {
        source: super::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        namespace: super::SourceNamespaceKey,
    },
    ScriptActivation {
        source: super::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        namespace: super::SourceNamespaceKey,
        config: tcl_lexer::LexerConfig,
    },
    /// A conditional Logical source frame, with no entered Native activation.
    LogicalScriptActivation {
        source: super::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        namespace: super::SourceNamespaceKey,
        config: tcl_lexer::LexerConfig,
    },
}

/// Original procedure source scope for parameter reports. The actual row
/// retains either its allocated body or its uninstalled declaration; neither
/// variant supplies entered-frame or Native invocation authority.
pub(crate) struct OriginalBodyParameterSourceScope {
    entry: Arc<OriginalDiagnosticFrameEntry>,
    layout: OriginalCompilationLookupAdvice,
}
impl OriginalBodyParameterSourceScope {
    pub(crate) fn layout(&self) -> &OriginalCompilationLookupAdvice {
        &self.layout
    }
    pub(crate) fn matches_parameters(&self, parameters: &[&str]) -> bool {
        self.layout
            .closed_body_parameter_source_entry_lookup(&self.entry)
            && self
                .entry
                .original_formal_topology()
                .is_some_and(|formals| {
                    formals
                        .parameters()
                        .iter()
                        .map(|formal| formal.name.as_slice())
                        .eq(parameters.iter().map(|name| name.as_bytes()))
                })
    }
    pub(crate) fn original_words(
        &self,
        tokens: &crate::ir::CommandTokens,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        if let OriginalDiagnosticFrameEntry::Body(body) = self.entry.as_ref() {
            return self
                .layout
                .original_body_parameter_source_words(tokens, body, config, registry);
        }
        (self.layout.config == config).then_some(())?;
        self.layout
            .closed_body_parameter_source_entry_lookup(&self.entry)
            .then_some(())?;
        self.layout
            .original_declaration_metadata_context(registry, tokens)
            .ok()?;
        let original = self.layout.site.source.original_lexical_command(
            tokens.words(),
            self.layout.site.offset,
            config,
        )?;
        Some(original.native_words().to_vec())
    }
}

impl OriginalCompilationLookupAdvice {
    fn closed_body_parameter_source_entry_lookup(
        &self,
        entry: &OriginalDiagnosticFrameEntry,
    ) -> bool {
        let namespace = match entry {
            OriginalDiagnosticFrameEntry::Body(body) => body.namespace_context(),
            OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => Some(&body.namespace),
            _ => return false,
        };
        self.closed_body_parameter_source_parts(
            entry.owns_source(&self.site.source, self.site.offset),
            entry.frame(),
            namespace,
        )
    }
}

/// A retained Logical member/body issuer with original source geometry only.
/// Its frame identity describes the written receiver body, without a Native
/// method allocation, formal ABI, receiver namespace or entered activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalLogicalReceiverSourceBody {
    body: Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
    input: crate::analyser::ResolvedAnalysisInput,
    source: super::ExecutedScriptSource,
    frame: crate::var_resolve::VariableExecutionFrame,
}

impl OriginalDiagnosticFrameEntry {
    pub(super) fn owns_original_context(
        &self,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        let namespace = match self {
            Self::Body(body) => body.namespace_context(),
            Self::DeclaredReceiver(_) => None,
            Self::LogicalReceiver(_) => return false,
            Self::DeclaredProcedure(body) => Some(&body.namespace),
            Self::RootScript { namespace, .. }
            | Self::ScriptActivation { namespace, .. }
            | Self::LogicalScriptActivation { namespace, .. } => Some(namespace),
        };
        SourceCommandBindings::context_owns_frame(context, self.frame(), namespace)
    }

    pub(super) fn source(&self) -> &super::ExecutedScriptSource {
        match self {
            Self::Body(body) => body.source(),
            Self::DeclaredReceiver(body) => body.source(),
            Self::LogicalReceiver(body) => &body.source,
            Self::DeclaredProcedure(body) => &body.source,
            Self::RootScript { source, .. }
            | Self::ScriptActivation { source, .. }
            | Self::LogicalScriptActivation { source, .. } => source,
        }
    }

    pub(super) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        match self {
            Self::Body(body) => body.frame(),
            Self::DeclaredReceiver(body) => body.preview_frame(),
            Self::LogicalReceiver(body) => &body.frame,
            Self::DeclaredProcedure(body) => &body.frame,
            Self::RootScript { frame, .. }
            | Self::ScriptActivation { frame, .. }
            | Self::LogicalScriptActivation { frame, .. } => frame,
        }
    }

    pub(super) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        match self {
            Self::Body(body) => body.parameters(),
            Self::DeclaredReceiver(body) => body.parameters(),
            Self::DeclaredProcedure(body) => &body.parameters,
            Self::LogicalReceiver(_)
            | Self::RootScript { .. }
            | Self::ScriptActivation { .. }
            | Self::LogicalScriptActivation { .. } => &[],
        }
    }

    pub(super) fn original_formal_topology(
        &self,
    ) -> Option<&super::formal_topology::OriginalFormalTopology> {
        match self {
            Self::Body(body) => body.original_formal_topology(),
            Self::DeclaredReceiver(body) => body.original_formal_topology(),
            Self::DeclaredProcedure(body) => body.original_parameters.as_ref(),
            Self::LogicalReceiver(_)
            | Self::RootScript { .. }
            | Self::ScriptActivation { .. }
            | Self::LogicalScriptActivation { .. } => None,
        }
    }

    pub(super) fn owns_source(&self, origin: &Arc<super::SourceOriginId>, offset: u32) -> bool {
        let source = self.source();
        &source.origin == origin
            && offset >= source.base()
            && u64::from(offset) < u64::from(source.base()) + source.text.len() as u64
    }

    /// A readonly insertion cursor immediately before an unchanged body's
    /// closer belongs to that body's naming frame. Source occurrences remain
    /// half-open; materialised body values cannot borrow this end affinity.
    fn owns_source_cursor(&self, origin: &Arc<super::SourceOriginId>, offset: u32) -> bool {
        if self.owns_source(origin, offset) {
            return true;
        }
        let source = self.source();
        let super::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
            return false;
        };
        let Some(end) = u32::try_from(source.text.len())
            .ok()
            .and_then(|length| base.checked_add(length))
        else {
            return false;
        };
        &source.origin == origin
            && offset == end
            && origin
                .source_image()
                .bytes()
                .get(base as usize..end as usize)
                == Some(source.text.bytes())
    }

    #[cfg(test)]
    pub(super) fn owns_invocation(&self, binding: &super::SourceInvocationBinding) -> bool {
        binding.variable_frame == *self.frame()
    }

    pub(super) fn owns_frame(
        &self,
        bindings: &SourceCommandBindings,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) -> bool {
        match self {
            Self::Body(body) => bindings.original_body_owns_frame(body, frame),
            Self::DeclaredReceiver(body) => body.preview_frame() == frame,
            Self::LogicalReceiver(_) => false,
            Self::DeclaredProcedure(body) => &body.frame == frame,
            Self::RootScript {
                frame: expected, ..
            }
            | Self::ScriptActivation {
                frame: expected, ..
            }
            | Self::LogicalScriptActivation {
                frame: expected, ..
            } => expected == frame,
        }
    }
}

// Derived equality compares in field order: the whole lookup snapshot last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationLayoutObservation {
    pub(super) issuer: DeclarationLayoutIssuer,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) words: Arc<[tcl_source::words::WordExpr]>,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) entry: Arc<OriginalDiagnosticFrameEntry>,
    pub(super) snapshot: Arc<SourceLookupSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum DeclarationLayoutIssuer {
    OriginalDeclaration,
    EnteredActivation,
    /// Independent source preview, without entered receiver authority.
    ConditionalReceiverSource,
}

/// A sealed conditional source declaration whose original command has no
/// dispatch point. The saved raw binding retains unknown/absence and every
/// source/world attachment; its targets remain separate from runtime lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalLogicalDeclarationTargets {
    original_binding: Box<super::SourceInvocationBinding>,
    targets: Vec<super::SourceCommandTarget>,
}

impl OriginalLogicalDeclarationTargets {
    fn matches(
        &self,
        binding: &super::SourceInvocationBinding,
        advice: &OriginalCompilationLookupAdvice,
    ) -> bool {
        let mut original = binding.clone();
        original.original_logical_declaration_targets = None;
        original.lookup_state.is_none()
            && &original == self.original_binding.as_ref()
            && self.targets.as_slice() == advice.targets()
    }
}

/// Select the immutable declaration owner without joining entered activations.
/// A conflicting original receipt withdraws the entire declaration projection.
pub(super) fn original_declaration_layouts(
    observations: &[DeclarationLayoutObservation],
) -> Option<impl Iterator<Item = &DeclarationLayoutObservation> + Clone> {
    select_declaration_layouts(observations, false)
}

/// Catalogue source advice also admits independently collected receiver bodies.
/// Those rows cannot supply the original execution or read projection.
pub(super) fn original_source_catalogue_layouts(
    observations: &[DeclarationLayoutObservation],
) -> Option<impl Iterator<Item = &DeclarationLayoutObservation> + Clone> {
    select_declaration_layouts(observations, true)
}

fn select_declaration_layouts(
    observations: &[DeclarationLayoutObservation],
    catalogue: bool,
) -> Option<impl Iterator<Item = &DeclarationLayoutObservation> + Clone> {
    let original = observations.iter().filter(move |observation| {
        observation.issuer == DeclarationLayoutIssuer::OriginalDeclaration
            || (catalogue
                && observation.issuer == DeclarationLayoutIssuer::ConditionalReceiverSource)
    });
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some()
        && original.clone().next().is_none()
    {
        eprintln!(
            "ORIGINAL_DECLARATION stage=no-original observations={}",
            observations.len()
        );
    }
    let first = super::trace_rbs_operand_option("rows.first_original", original.clone().next())?;
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
        for observation in original.clone() {
            eprintln!(
                "ORIGINAL_DECLARATION offset={} stage=owner entry={} config={} words={} frame={} origin={} context={}",
                observation
                    .words
                    .first()
                    .map_or(0, |word| word.source().span.start()),
                observation.entry == first.entry,
                observation.config == first.config,
                observation.words == first.words,
                observation.snapshot.state.variable_frame == *observation.entry.frame(),
                observation.snapshot.state.current_source_origin.as_ref()
                    == Some(&observation.entry.source().origin),
                observation
                    .entry
                    .owns_original_context(&observation.snapshot.state.source_variables),
            );
        }
    }
    if original.clone().any(|observation| {
        // The first row is equal to itself; only later rows need the join.
        (!std::ptr::eq(observation, first)
            && (observation.entry != first.entry
                || observation.config != first.config
                || observation.words != first.words))
            || observation.snapshot.state.current_source_origin.as_ref()
                != Some(&observation.entry.source().origin)
            || (catalogue
                && observation.words.first().is_none_or(|word| {
                    !observation.entry.owns_source(
                        &observation.entry.source().origin,
                        word.source().span.start(),
                    )
                }))
            || (!catalogue
                && (observation.snapshot.state.variable_frame != *observation.entry.frame()
                    || !observation
                        .entry
                        .owns_original_context(&observation.snapshot.state.source_variables)))
    }) {
        return super::trace_rbs_operand_option("rows.owner_join", None);
    }
    Some(original)
}

// Snapshot hashing is cached. Omitting the declaration recipe permits harmless
// hash collisions; structural equality still compares the complete owner.
impl Hash for DeclarationLayoutObservation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.issuer.hash(state);
        self.snapshot.hash(state);
        self.namespace.hash(state);
        self.config.hash(state);
        self.words.hash(state);
    }
}

pub(super) type DeclarationLayouts =
    BTreeMap<CommandAllocationSite, Vec<DeclarationLayoutObservation>>;

/// Immutable original local-frame naming owner. This is a conditional source
/// declaration identity, not an entered activation or a physical local table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOriginalVariableFrame {
    source: super::ExecutedScriptSource,
    frame: crate::var_resolve::VariableExecutionFrame,
    config: tcl_lexer::LexerConfig,
    logical_lambda: Option<Arc<super::OriginalLogicalLambdaBody>>,
}

/// Candidate original source calls retain declaration allocations separately
/// from Registry metadata. This grants native formal layout advice only.
pub(crate) struct OriginalDeclarationCallLayoutAdvice {
    layout: OriginalCompilationLookupAdvice,
}

/// An original operand read in an accepted declaration's local frame.
/// This is diagnostic occurrence evidence, not a physical read, contents
/// verdict or represented SSA use. Unknown runtime alternatives remain open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarationReadOccurrenceAdvice {
    invocation: CommandAllocationSite,
    name: String,
    source: tcl_source::words::SourceSite,
    spelling: String,
}

/// A decoded variable operand in the original declaration-local layout.
/// Its symbolic value is conditional advice, never a physical address/read.
pub(crate) struct DeclarationVariableOperandAdvice {
    invocation: CommandAllocationSite,
    word: tcl_source::words::WordExpr,
    name: String,
}

/// Sealed original source layout for conditional diagnostics. This carrier
/// has no conversion to Native lookup/body/occurrence or execution facts.
pub(crate) struct DeclarationSourceOperandLayoutAdvice {
    pub(crate) config: tcl_lexer::LexerConfig,
    kind: DeclarationSourceOperandLayoutKind,
}

enum DeclarationSourceOperandLayoutKind {
    Original(OriginalCompilationLookupAdvice),
    Configured {
        layout: Arc<super::declaration_preview::OriginalDeclarationSourceOperandLayout>,
        owner: std::sync::Weak<super::OriginalSourceAnalysisOwner>,
        cache: std::sync::Weak<super::declaration_preview::OriginalDeclarationSourceOperandCache>,
    },
}

/// One complete written scalar source operand and its symbolic diagnostic use.
/// Folded source bytes acquire no ordinary Native occurrence or cell address.
pub(crate) struct DeclarationSourceVariableOperandAdvice {
    site: CommandAllocationSite,
    word: tcl_source::words::WordExpr,
    name: String,
    source_operations: Arc<[super::declaration_preview::OriginalDeclarationSourceOperation]>,
}

/// Common immutable declaration-vector join, independent of target selection.
struct OriginalDeclarationWordSource<'a> {
    snapshot: &'a Arc<super::SourceLookupSnapshot>,
    site: &'a CommandAllocationSite,
    words: &'a [tcl_source::words::WordExpr],
    config: tcl_lexer::LexerConfig,
    dialect: tcl_native_model::InvocationDialect,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_nested_root_activation_purpose_tests;

#[cfg(test)]
#[path = "logical_purpose_tests.rs"]
mod logical_purpose_tests;

#[cfg(test)]
mod original_logical_declaration_receipt_tests;

#[cfg(test)]
mod authored_source_parent_tests;
