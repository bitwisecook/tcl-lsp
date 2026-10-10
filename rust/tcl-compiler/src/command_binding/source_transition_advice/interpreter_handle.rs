// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Created-child script advice retains source lifetime and original producers.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, OriginalSourceCommandTransition,
    SourceAdviceNameInput, lookup_key, original_words, registry_words,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{ExecutablePart, ExecutableText, LexerConfig, NativeWord, SourceImage, WordKind};
use tcl_registry::InterpreterTransition;
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreatedInterpreterSourceCell {
    pub(super) creation: Arc<OriginalSourceCommandTransition>,
    transition: InterpreterTransition,
    pub(super) interpreter: SignatureSourceNameInput,
    pub(super) parent_key: ByteCommandSlot,
    pub(super) moves: Vec<Arc<OriginalSourceCommandTransition>>,
}

impl CreatedInterpreterSourceCell {
    pub(super) fn moved(
        mut self: Box<Self>,
        parent_key: ByteCommandSlot,
        receipt: &Arc<OriginalSourceCommandTransition>,
    ) -> Box<Self> {
        self.parent_key = parent_key;
        self.moves.push(Arc::clone(receipt));
        self
    }
}

/// Conditional script source of a canonical created child command.
/// The complete Create and call vectors retain independent source producers.
/// This supplies no parent allocation, selected callable, entered child,
/// runtime namespace, normal completion, effects or refactoring authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceInterpreterHandleBody {
    cell: CreatedInterpreterSourceCell,
    original: Arc<[NativeWord]>,
    body: NativeWord,
    context: ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    config: LexerConfig,
}
impl OriginalSourceInterpreterHandleBody {
    /// Original creation and its selected Registry transition recipe.
    #[must_use]
    pub fn creation(&self) -> &OriginalSourceCommandTransition {
        &self.cell.creation
    }
    /// Original child path, independently of the parent command's source key.
    #[must_use]
    pub const fn interpreter_input(&self) -> &SignatureSourceNameInput {
        &self.cell.interpreter
    }
    /// Current source parent key after the original creation and known moves.
    /// The child's path remains independent. No allocation or callability follows.
    #[must_use]
    pub const fn parent_key(&self) -> &ByteCommandSlot {
        &self.cell.parent_key
    }
    /// Original rename operations, in source order, selecting this parent key.
    /// These retain conditional successful-move obligations, not current tokens.
    #[must_use]
    pub fn move_lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.cell.moves
    }
    /// Complete original handle call, without reconstructed prefix operands.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Exact original single braced script operand.
    #[must_use]
    pub const fn body_word(&self) -> &NativeWord {
        &self.body
    }
    pub(super) fn invocation_offset(&self) -> u32 {
        self.original[0].span().start()
    }
    /// Complete source, full configuration and actual availability context.
    /// Successful Create, materialisation and body entry remain obligations.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.cell.creation.site().source.source_image() == image
            && self.config == config
            && self.context == *context.context()
            && self.registry == context.commands().snapshot().semantic_key()
            && self
                .cell
                .creation
                .original_words()
                .iter()
                .chain(self.original.iter())
                .all(|word| word.image() == image && word.config() == config)
            && self.cell.moves.iter().all(|receipt| {
                receipt.site().source.source_image() == image
                    && receipt
                        .original_words()
                        .iter()
                        .all(|word| word.image() == image && word.config() == config)
            })
            && self.original.contains(&self.body)
    }
}

impl AdviceGraph {
    pub(super) fn apply_interpreter_transition(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        transition: &InterpreterTransition,
    ) -> Option<()> {
        match transition {
            InterpreterTransition::Create {
                interpreter: Some(path),
                ..
            } => {
                let Some(policy) = self.policy.native() else {
                    return Some(());
                };
                if !matches!(policy.recipe(), NativeNameProtocol::C(_)) {
                    return Some(());
                }
                let Some(parent) = transition.created_parent_command(self.dialect) else {
                    return Some(());
                };
                let interpreter = receipt.input(path)?.native_input()?.clone();
                let name = interpreter.original_list_element(0)?;
                if interpreter.original_list_element(1).is_some()
                    || name.bytes() != parent.as_bytes()
                {
                    return None;
                }
                let key = policy
                    .recipe()
                    .command_c_api_publication_slot(NativeNameContext::root(), name.bytes())
                    .ok()?;
                // Qualified parent publication needs an independent namespace
                // existence owner. This bounded source recipe retains root only.
                if !key.namespace.is_root() {
                    return None;
                }
                if self
                    .cells
                    .get(&key)
                    .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
                {
                    return None;
                }
                self.cells.insert(
                    key.clone(),
                    AdviceCell::CreatedInterpreter(Box::new(CreatedInterpreterSourceCell {
                        creation: Arc::clone(receipt),
                        transition: transition.clone(),
                        interpreter,
                        parent_key: key,
                        moves: Vec::new(),
                    })),
                );
            }
            InterpreterTransition::Delete { interpreter } => {
                let path = receipt.input(interpreter)?;
                self.cells.retain(|_, cell| !matches!(cell,
                    AdviceCell::CreatedInterpreter(created) if created.interpreter.bytes() == path.bytes()));
            }
            _ => {}
        }
        Some(())
    }
}

impl AdviceInvocationContext<'_> {
    pub(super) fn interpreter_handle_body(
        &self,
        graph: &AdviceGraph,
        head: &SourceAdviceNameInput,
        native: &[NativeWord],
    ) -> Option<OriginalSourceInterpreterHandleBody> {
        let key = lookup_key(self.policy, head.bytes())?;
        let AdviceCell::CreatedInterpreter(cell) = graph.cells.get(&key)? else {
            return None;
        };
        // A current parent key retains its original child path and known move
        // lineage. Alias, computed and stale heads supply no corresponding cell.
        if key != cell.parent_key || !graph.uncertain_operations.is_empty() {
            return None;
        }
        let written = original_words(native, self.policy)?;
        let arguments = registry_words(written.get(1..)?);
        let selected = cell.transition.created_handle_eval_body(
            self.dialect,
            tcl_registry::InvocationArguments::Structured(&arguments).with_dialect(self.dialect),
        )?;
        let body = native.get(selected.argument_index()?.checked_add(1)?)?;
        if body.group().kind != WordKind::Braced
            || body.group().expand
            || body
                .executable_parts()
                .all_parts()
                .any(|part| !matches!(part.part, ExecutablePart::Text(ExecutableText::Original)))
        {
            return None;
        }
        body.content_span().ok()?;
        if native
            .iter()
            .any(|word| word.image() != self.origin.source_image() || word.config() != self.config)
        {
            return None;
        }
        Some(OriginalSourceInterpreterHandleBody {
            cell: cell.as_ref().clone(),
            original: Arc::from(native),
            body: body.clone(),
            context: self.context.context().clone(),
            registry: self.context.commands().snapshot().semantic_key(),
            config: self.config,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use tcl_lexer::SourceImage;

    #[test]
    fn original_created_handle_body_retains_source_order_and_refuses_unowned_entries() {
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command.md

        // naming.interpreter.original-created-handle-source-body
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
        // Native CLI answers basic creation/handle/path/delete equivalence in
        // naming.interpreter.original-created-parent-command. This assertion
        // tests a separate conditional source recipe, not entered dispatch.
        let source = "interp create -safe s; interp hide s format; s eval {source a.tcl}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let input = analysis.resolved_input.as_ref().unwrap();
            let image = SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                input.lexer_config(),
            )
            .unwrap();
            let call = &plan.commands.last().unwrap().words;
            let realm = analysis.retained_command_realm().unwrap();
            let receipt = realm
                .original_source_interpreter_handle_body(input, call)
                .unwrap();
            assert_eq!(receipt.parent_key().simple.as_bytes(), b"s", "{dialect}");
            assert_eq!(receipt.interpreter_input().bytes(), b"s");
            assert_eq!(receipt.original_words(), call);
            assert_eq!(receipt.body_word(), &call[2]);
            assert_eq!(receipt.creation().original_words(), plan.commands[0].words);
            assert!(receipt.matches_source_context(
                &image,
                input.lexer_config(),
                &input.context_registry()
            ));
            assert!(!receipt.matches_source_context(
                &SourceImage::document("foreign"),
                input.lexer_config(),
                &input.context_registry()
            ));
            let mut foreign_config = input.lexer_config();
            foreign_config.strict_quoting ^= true;
            assert!(!receipt.matches_source_context(
                &image,
                foreign_config,
                &input.context_registry()
            ));
            assert!(
                realm
                    .original_source_interpreter_handle_body(input, &call[..2])
                    .is_none()
            );
        }
        for source in [
            "s eval {source a.tcl}",
            "interp create -safe s; interp delete s; s eval {source a.tcl}",
            "interp create -safe s; rename s moved; s eval {source a.tcl}",
            "interp create -safe s; rename s moved; interp delete s; moved eval {source a.tcl}",
            "interp create -safe s; rename s moved; rename moved {}; moved eval {source a.tcl}",
            "interp create -safe s; rename s moved; proc moved args {}; moved eval {source a.tcl}",
            "interp create -safe s; rename s moved; mystery; moved eval {source a.tcl}",
            "interp create -safe s; proc s args {}; s eval {source a.tcl}",
            "interp create -safe s; mystery; s eval {source a.tcl}",
            "interp create -safe s; interp alias {} call {} s; call eval {source a.tcl}",
            "interp create -safe s; s eval {source} a.tcl",
            "interp create -safe s; s e {source a.tcl}",
            "interp create -safe s; s eval $script",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let input = analysis.resolved_input.as_ref().unwrap();
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                input.lexer_config(),
            )
            .unwrap();
            let realm = analysis.retained_command_realm().unwrap();
            assert!(
                realm
                    .original_source_interpreter_handle_body(
                        input,
                        &plan.commands.last().unwrap().words
                    )
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_created_handle_moves_keep_child_path_and_parent_lineage_separate() {
        // naming.interpreter.original-created-handle-source-body
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
        // Independent CLI observations: naming.interpreter.original-created-parent-command-move
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command-move.md
        // Source receipts retain the original path and exact successful-move
        // premises. The CLI does not donate a runtime handle or entered child.
        for (source, parent, moves) in [
            (
                "interp create -safe s; rename s moved; moved eval {source a.tcl}",
                b"moved".as_slice(),
                1,
            ),
            (
                "interp create -safe s; rename s first; rename first second; second eval {source a.tcl}",
                b"second".as_slice(),
                2,
            ),
            (
                "interp create -safe s; namespace eval N {}; rename s ::N::held; ::N::held eval {source a.tcl}",
                b"held".as_slice(),
                1,
            ),
        ] {
            for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
                let analysis = Analyser::new().analyse(source, dialect);
                let input = analysis.resolved_input.as_ref().unwrap();
                let image = SourceImage::document(source);
                let plan = tcl_lexer::native_script_words_in(
                    image.clone(),
                    tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                    input.lexer_config(),
                )
                .unwrap();
                let call = &plan.commands.last().unwrap().words;
                let receipt = analysis
                    .retained_command_realm()
                    .unwrap()
                    .original_source_interpreter_handle_body(input, call)
                    .unwrap();
                assert_eq!(
                    receipt.parent_key().simple.as_bytes(),
                    parent,
                    "{dialect}: {source}"
                );
                assert_eq!(receipt.interpreter_input().bytes(), b"s");
                assert_eq!(receipt.move_lineage().len(), moves);
                assert_eq!(receipt.creation().original_words(), plan.commands[0].words);
                assert_eq!(receipt.original_words(), call);
                assert!(receipt.matches_source_context(
                    &image,
                    input.lexer_config(),
                    &input.context_registry()
                ));
                for movement in receipt.move_lineage() {
                    assert!(
                        plan.commands
                            .iter()
                            .any(|command| command.words == movement.original_words())
                    );
                }
            }
        }
    }
}
