// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual receiver entry state, separate from lexical method declaration facts.

use super::formal_topology::OriginalFormalTopology;
use super::receiver_self::CalledBodyReceiver;
use super::{DeferredSourceBody, ExecutedScriptSource, SourceCommandTarget};
use crate::var_resolve::{ResolveContext, VariableExecutionFrame};
use std::sync::Arc;

/// An entered original method retains its selected declaring provider and
/// receiver axis before body operands execute. A later declaration, class
/// label or source preview cannot issue this receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalReceiverBodyContext {
    source: ExecutedScriptSource,
    provider: SourceCommandTarget,
    receiver: CalledBodyReceiver,
    frame: VariableExecutionFrame,
    parameters: OriginalFormalTopology,
    config: tcl_lexer::LexerConfig,
    context: Arc<ResolveContext>,
}

impl OriginalReceiverBodyContext {
    pub(super) fn at_entered_call(
        body: &DeferredSourceBody,
        provider: &SourceCommandTarget,
        receiver: CalledBodyReceiver,
        frame: &VariableExecutionFrame,
        context: &ResolveContext,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Self> {
        if !body.receiver_method
            || provider.identity.is_none()
            || context.execution_name_policy.is_none()
            || !matches!(
                frame.layout(),
                VariableExecutionFrame::ReceiverMethod { .. }
            )
        {
            return None;
        }
        let (_, _, source) = body.executed_script.as_ref()?;
        let parameters = body.original_parameters.as_ref()?;
        let VariableExecutionFrame::ReceiverMethod { identity } = frame.layout() else {
            return None;
        };
        if context.activation.as_ref() != Some(identity)
            || context.original_formal_topology.as_deref() != Some(parameters)
        {
            return None;
        }
        Some(Self {
            source: source.clone(),
            provider: provider.clone(),
            receiver,
            frame: frame.clone(),
            parameters: parameters.clone(),
            config,
            context: Arc::new(context.clone()),
        })
    }

    pub(crate) fn context(&self) -> &ResolveContext {
        &self.context
    }
}
