// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original child paths retain creation independently of parent command moves.

use super::interpreter_handle::CreatedInterpreterSourceCell;
use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext,
    OriginalSourceCommandTransition, OriginalSourceTransitionAdviceTape,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use std::sync::Arc;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_registry::{
    BodyInterpreter, CommandBindingTransition, InterpreterTransition, StateTransition,
};

/// A conditional original child path and its independent created parent key.
/// Typed Registry interpreter selectors own the path operand. This supplies no
/// child allocation, entered frame, successful transition or selected callable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceInterpreterPathBinding {
    cell: CreatedInterpreterSourceCell,
    path: SignatureSourceNameInput,
    original: Arc<[NativeWord]>,
    context: ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    config: LexerConfig,
}
impl OriginalSourceInterpreterPathBinding {
    /// Genuine original creation, retaining its selected transition recipe.
    #[must_use]
    pub fn creation(&self) -> &OriginalSourceCommandTransition {
        &self.cell.creation
    }
    /// Genuine selected original path producer, independent of parent spelling.
    #[must_use]
    pub const fn path_input(&self) -> &SignatureSourceNameInput {
        &self.path
    }
    /// Complete original source operation, without reconstructed operands.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Full source/channel/configuration and the actual complete Registry context.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.config == config
            && self.context == *context.context()
            && self.registry == context.commands().snapshot().semantic_key()
            && self.cell.creation.site().source.source_image() == image
            && self
                .cell
                .creation
                .original_words()
                .iter()
                .chain(self.original.iter())
                .all(|word| word.image() == image && word.config() == config)
            && self.cell.moves.iter().all(|receipt| {
                receipt
                    .original_words()
                    .iter()
                    .all(|word| word.image() == image && word.config() == config)
            })
    }
}

fn selected_path_indices(schema: &tcl_registry::ResolvedInvocation<'_, '_>) -> Vec<usize> {
    let mut indices = Vec::new();
    if let BodyInterpreter::Argument(index) = schema.semantics.body_interpreter {
        indices.push(usize::from(index));
    }
    for fact in schema.state_transitions().facts() {
        match &fact.transition {
            StateTransition::Interpreter(
                InterpreterTransition::Delete { interpreter }
                | InterpreterTransition::MarkTrusted { interpreter }
                | InterpreterTransition::SetRecursionLimit { interpreter, .. }
                | InterpreterTransition::SetBackgroundError { interpreter, .. }
                | InterpreterTransition::Hide { interpreter, .. }
                | InterpreterTransition::Expose { interpreter, .. },
            ) => {
                indices.extend(interpreter.argument_index());
            }
            StateTransition::CommandBinding(CommandBindingTransition::Alias {
                source_interpreter,
                target_interpreter,
                ..
            }) => {
                indices.extend(source_interpreter.argument_index());
                indices.extend(target_interpreter.argument_index());
            }
            StateTransition::CommandBinding(CommandBindingTransition::Delete {
                interpreter: Some(interpreter),
                ..
            }) => indices.extend(interpreter.argument_index()),
            _ => {}
        }
    }
    indices.sort_unstable();
    indices.dedup();
    indices
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_interpreter_paths(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        // naming.interpreter.original-created-parent-command-move
        // naming.interpreter.original-child-path-source-binding
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-path-source-binding.md
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command-move.md
        // The CLI path/parent distinction is observed independently. These
        // bindings are conditional source projections, not native identities.
        if !graph.uncertain_operations.is_empty() {
            return;
        }
        let Some(offset) = invocation.native.first().map(|word| word.span().start()) else {
            return;
        };
        let mut bindings = Vec::new();
        for index in selected_path_indices(schema) {
            let Some(path) = invocation
                .arguments
                .get(index)
                .and_then(|word| word.input.as_ref())
                .and_then(super::SourceAdviceNameInput::native_input)
            else {
                continue;
            };
            if path.bytes().is_empty() {
                continue;
            }
            let mut cells = graph.cells.values().filter_map(|cell| {
                let AdviceCell::CreatedInterpreter(cell) = cell else {
                    return None;
                };
                (cell.interpreter.bytes() == path.bytes()).then_some(cell)
            });
            let Some(cell) = cells.next() else {
                continue;
            };
            if cells.next().is_some() {
                continue;
            }
            bindings.push(OriginalSourceInterpreterPathBinding {
                cell: cell.as_ref().clone(),
                path: path.clone(),
                original: Arc::from(invocation.native),
                context: self.context.context().clone(),
                registry: self.context.commands().snapshot().semantic_key(),
                config: self.config,
            });
        }
        if !bindings.is_empty() {
            tape.interpreter_paths.insert(offset, bindings);
        }
    }
}
