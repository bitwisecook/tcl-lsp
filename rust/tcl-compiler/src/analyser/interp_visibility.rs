// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional interpreter source visibility, independently of native lookup.

use super::{Analyser, ResolvedAnalysisInput};
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceNameInput, SignatureSourceNameKey,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, NameBytes};
use tcl_lexer::{NativeWord, SourceImage, Span, Token};
use tcl_registry::{
    ChildInterpreterSafety, InterpreterTransition, StateTransition, TransitionSubject,
};
use tcl_syntax::naming::{
    NamePolicyProtocol, NativeNameContext, NativeNameProtocol, NativeNameQualification,
};

/// Reporting domain of a positively owned original child-interpreter body.
/// The complete declaration and input remain retained. This is a source label
/// domain, never a native interpreter, namespace token or selected callable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalInterpreterSourceDomain {
    label: String,
    input: ResolvedAnalysisInput,
    declaration: Vec<NativeWord>,
}
impl OriginalInterpreterSourceDomain {
    pub(super) fn reported_namespace(&self, namespace: &SignatureNamespaceScope) -> Option<String> {
        let SignatureNamespaceScope::C(path) = namespace else {
            return None;
        };
        let bytes = tcl_syntax::naming::native_namespace_full_name_bytes(path);
        let suffix = std::str::from_utf8(bytes.get(2..)?).ok()?;
        Some(if suffix.is_empty() {
            format!("::{}", self.label)
        } else {
            format!("::{}::{suffix}", self.label)
        })
    }

    pub(super) fn reported_command(&self, slot: &ByteCommandSlot) -> Option<String> {
        let bytes = tcl_syntax::naming::native_command_full_name_bytes(slot);
        let suffix = std::str::from_utf8(bytes.get(2..)?).ok()?;
        Some(format!("::{}::{suffix}", self.label))
    }
}

/// The original source assumptions of an interpreter visibility diagnostic.
/// They are retained as data; none establishes entered execution or a hidden slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpreterVisibilityObligation {
    /// The retained child creation and visibility operations complete successfully.
    SuccessfulSourceOperations,
    /// The child's initial commands agree with the retained C Registry context.
    RegistryInitialization,
    /// An unmodelled command or external mutation does not change this child.
    NoUnmodelledInterpreterMutation,
    /// Observers do not change visibility while original operands are evaluated.
    NoObserverInterference,
    /// The stored source body is entered under its retained child declaration.
    SourceBodyEntered,
    /// Original literal/list producers materialise without changing the command map.
    OriginalOperandMaterialization,
    /// The declared receiver body has no entered object namespace.
    UnknownExecutingReceiverNamespace,
    /// Conditional global fallback requires no earlier same-name receiver
    /// namespace binding or search-path target intercepting this source call.
    NoEarlierReceiverNamespaceBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceCommandOrigin {
    Registry(String),
    Declaration {
        name: SignatureSourceNameInput,
        kind: tcl_registry::CommandBindingDefinitionKind,
        operands: Vec<NativeWord>,
    },
    Alias {
        name: SignatureSourceNameInput,
        operands: Vec<NativeWord>,
    },
    Renamed {
        original: Box<SourceCommandOrigin>,
        operands: Vec<NativeWord>,
    },
    Exposed {
        original: Box<SourceCommandOrigin>,
        operands: Vec<NativeWord>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceVisibleSlot {
    Visible(SourceCommandOrigin),
    Unavailable(SourceCommandOrigin),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourceHiddenAllocation {
    previous_slot: ByteCommandSlot,
    origin: SourceCommandOrigin,
    operands: Vec<NativeWord>,
}

/// Hashable original source visibility context for an isolated body/cache.
/// Cloning retains the complete source and immutable editing generation.
/// No runtime hidden table, selected callable, native entry or effects are supplied.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceInterpreterVisibilitySnapshot {
    input: ResolvedAnalysisInput,
    policy: NamePolicyProtocol,
    declaration: Vec<NativeWord>,
    safe: bool,
    unavailable: BTreeMap<ByteCommandSlot, SourceCommandOrigin>,
    body: Option<NativeWord>,
    body_invocation: Vec<NativeWord>,
    slots: BTreeMap<ByteCommandSlot, SourceVisibleSlot>,
    hidden: BTreeMap<NameBytes, SourceHiddenAllocation>,
}
impl SourceInterpreterVisibilitySnapshot {
    fn initialize(
        input: ResolvedAnalysisInput,
        policy: NamePolicyProtocol,
        declaration: Vec<NativeWord>,
        safe: bool,
    ) -> Option<Self> {
        if !matches!(policy.recipe(), NativeNameProtocol::C(_)) || declaration.is_empty() {
            return None;
        }
        let context = input.context_registry();
        let mut slots = BTreeMap::new();
        let mut hidden = BTreeMap::new();
        let mut unavailable = BTreeMap::new();
        for command in context.commands().command_names() {
            let Some(spec) = context.context().resolve_spec(context.commands(), command) else {
                continue;
            };
            let slot = policy
                .recipe()
                .command_publication_slot(NativeNameContext::root(), command.as_bytes())
                .ok()?;
            let origin = SourceCommandOrigin::Registry(command.to_owned());
            if safe
                && slot.namespace.is_root()
                && spec
                    .traits
                    .contains(tcl_registry::Traits::SAFE_INTERP_HIDDEN)
            {
                let token = policy
                    .recipe()
                    .hidden_token_input(command.as_bytes())
                    .ok()?
                    .selected()
                    .into();
                hidden.insert(
                    token,
                    SourceHiddenAllocation {
                        previous_slot: slot.clone(),
                        origin: origin.clone(),
                        operands: declaration.clone(),
                    },
                );
                unavailable.insert(slot.clone(), origin.clone());
                slots.insert(slot, SourceVisibleSlot::Unavailable(origin));
            } else {
                slots.insert(slot, SourceVisibleSlot::Visible(origin));
            }
        }
        Some(Self {
            input,
            policy,
            declaration,
            safe,
            unavailable,
            body: None,
            body_invocation: Vec::new(),
            slots,
            hidden,
        })
    }
    pub(super) fn reporting_domain(&self, label: String) -> Arc<OriginalInterpreterSourceDomain> {
        Arc::new(OriginalInterpreterSourceDomain {
            label,
            input: self.input.clone(),
            declaration: self.declaration.clone(),
        })
    }
    fn image(&self) -> &SourceImage {
        self.declaration[0].image()
    }
    fn matches(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.image() == image
            && self.input == *input
            && self.policy.string_protocol().escape_syntax() == input.lexer_config().escapes
    }
    fn original_input(&self, word: &NativeWord) -> Option<SignatureSourceNameInput> {
        if word.image() != self.image() || word.config() != self.input.lexer_config() {
            return None;
        }
        SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            self.policy,
        )
        .map(SignatureSourceNameInput::OriginalWord)
    }
    fn hidden_token(&self, input: &SignatureSourceNameInput) -> Option<NameBytes> {
        if input.policy() != self.policy {
            return None;
        }
        let bytes = self
            .policy
            .recipe()
            .hidden_token_input(input.bytes())
            .ok()?
            .selected()
            .to_vec();
        (!bytes.windows(2).any(|pair| pair == b"::")).then(|| bytes.into())
    }
    fn hide(
        &mut self,
        visible: &SignatureSourceNameInput,
        hidden: &SignatureSourceNameInput,
        operands: &[NativeWord],
    ) -> Option<()> {
        let slot = self
            .policy
            .recipe()
            .command_lookup_slot(NativeNameContext::root(), visible.bytes())
            .ok()?;
        let Some(token) = self.hidden_token(hidden) else {
            return Some(());
        };
        if !slot.namespace.is_root() || self.hidden.contains_key(&token) {
            return Some(());
        }
        let Some(SourceVisibleSlot::Visible(origin)) = self.slots.get(&slot).cloned() else {
            return Some(());
        };
        self.unavailable.insert(slot.clone(), origin.clone());
        self.hidden.insert(
            token,
            SourceHiddenAllocation {
                previous_slot: slot.clone(),
                origin: origin.clone(),
                operands: operands.to_vec(),
            },
        );
        self.slots
            .insert(slot, SourceVisibleSlot::Unavailable(origin));
        Some(())
    }
    fn expose(
        &mut self,
        hidden: &SignatureSourceNameInput,
        visible: &SignatureSourceNameInput,
        operands: &[NativeWord],
    ) -> Option<()> {
        let token: NameBytes = self
            .policy
            .recipe()
            .hidden_token_input(hidden.bytes())
            .ok()?
            .selected()
            .into();
        let Some(allocation) = self.hidden.get(&token).cloned() else {
            return Some(());
        };
        let Some(destination) = self.hidden_token(visible) else {
            return Some(());
        };
        let slot = self
            .policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), destination.as_bytes())
            .ok()?;
        if !slot.namespace.is_root()
            || matches!(self.slots.get(&slot), Some(SourceVisibleSlot::Visible(_)))
        {
            return Some(());
        }
        self.hidden.remove(&token);
        self.unavailable.remove(&slot);
        self.slots.insert(
            slot,
            SourceVisibleSlot::Visible(SourceCommandOrigin::Exposed {
                original: Box::new(allocation.origin),
                operands: operands.to_vec(),
            }),
        );
        // The old source slot remains unavailable unless its independent new
        // declaration already installed another visible source allocation.
        Some(())
    }
    fn lookup_slot(
        &self,
        bytes: &[u8],
        namespace: &SignatureNamespaceScope,
    ) -> Option<ByteCommandSlot> {
        let context = namespace.context()?;
        let recipe = self.policy.recipe();
        let projection = recipe.command_lookup_input(context, bytes).ok()?;
        let mut slot = recipe.command_lookup_slot(context, bytes).ok()?;
        if !self.slots.contains_key(&slot)
            && projection.qualification() == NativeNameQualification::Unqualified
        {
            slot = recipe
                .command_lookup_slot(NativeNameContext::root(), bytes)
                .ok()?;
        }
        Some(slot)
    }
    fn observe(
        &self,
        name: SignatureSourceNameInput,
        namespace: SignatureNamespaceScope,
        words: Vec<NativeWord>,
    ) -> Option<ConditionalInterpreterVisibilitySubject> {
        let body = self.body.as_ref()?.content_span().ok()?;
        if words.is_empty()
            || words
                .iter()
                .any(|word| word.span().start() < body.start() || word.span().end() > body.end())
        {
            return None;
        }
        let slot = self.lookup_slot(name.bytes(), &namespace)?;
        if !slot.namespace.is_root()
            || !matches!(
                self.slots.get(&slot),
                Some(SourceVisibleSlot::Unavailable(_))
            )
        {
            return None;
        }
        Some(ConditionalInterpreterVisibilitySubject {
            snapshot: self.clone(),
            name,
            namespace: Some(namespace),
            declared_receiver: None,
            source_receiver_declaration: None,
            slot,
            words,
            obligations: vec![
                InterpreterVisibilityObligation::SuccessfulSourceOperations,
                InterpreterVisibilityObligation::RegistryInitialization,
                InterpreterVisibilityObligation::NoUnmodelledInterpreterMutation,
                InterpreterVisibilityObligation::NoObserverInterference,
                InterpreterVisibilityObligation::SourceBodyEntered,
                InterpreterVisibilityObligation::OriginalOperandMaterialization,
            ],
        })
    }
}

/// Genuine conditional source visibility subject. This is not a callable
/// identity or a definite hidden command result; possible effects remain live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalInterpreterVisibilitySubject {
    snapshot: SourceInterpreterVisibilitySnapshot,
    name: SignatureSourceNameInput,
    namespace: Option<SignatureNamespaceScope>,
    declared_receiver: Option<Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>>,
    source_receiver_declaration: Option<Arc<super::types::OriginalSourceReceiverBodyDeclaration>>,
    slot: ByteCommandSlot,
    words: Vec<NativeWord>,
    obligations: Vec<InterpreterVisibilityObligation>,
}
impl ConditionalInterpreterVisibilitySubject {
    /// Original complete word/value producer, without a reporting-name lookup.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.name
    }
    /// Conditional original C root slot; no current table or namespace existence.
    #[must_use]
    pub const fn slot(&self) -> &ByteCommandSlot {
        &self.slot
    }
    /// Source namespace geometry retained separately from the reporting label.
    #[must_use]
    pub const fn namespace(&self) -> Option<&SignatureNamespaceScope> {
        self.namespace.as_ref()
    }
    /// Authentic declared receiver source scope. Its runtime object namespace
    /// and entry remain unknown; this cannot supply a callable or receiver.
    #[must_use]
    pub fn declared_receiver_body(
        &self,
    ) -> Option<&crate::command_binding::SourceDeclaredReceiverBodyEntry> {
        self.declared_receiver.as_deref()
    }
    /// Selected original method-body declaration without a native class or
    /// receiver allocation. This source scope supplies no executing namespace.
    #[must_use]
    pub fn source_receiver_body_declaration(
        &self,
    ) -> Option<&super::types::OriginalSourceReceiverBodyDeclaration> {
        self.source_receiver_declaration.as_deref()
    }
    /// Original whole invocation or list-producer vector, without an effective
    /// head borrowing the selector/outer call's span.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.words
    }
    /// Explicit applicability requirements; consumers must not parse the message.
    #[must_use]
    pub fn obligations(&self) -> &[InterpreterVisibilityObligation] {
        &self.obligations
    }
    /// Exact full source/channel, configuration and immutable editing generation.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.snapshot.matches(image, input)
            && self
                .source_receiver_declaration
                .as_ref()
                .is_none_or(|body| {
                    body.matches_source(image, input) && body.owns_invocation(&self.words, input)
                })
            && self.declared_receiver.as_ref().is_none_or(|body| {
                body.source().origin.source_image() == image
                    && matches!(
                        body.source().mapping,
                        crate::command_binding::ExecutedScriptMapping::Contiguous { .. }
                    )
            })
    }
}

struct OriginalInterpreterInvocation {
    native: Vec<NativeWord>,
    words: crate::registry_invocation::source_structure::OriginalRegistryWords,
    transitions: tcl_registry::StateTransitions,
}
impl OriginalInterpreterInvocation {
    fn input(&self, subject: &TransitionSubject) -> Option<SignatureSourceNameInput> {
        self.words
            .operands()
            .get(subject.argument_index()?)?
            .as_ref()?
            .input()
            .cloned()
    }
}

/// A literal lambda's original source namespace under its child declaration.
/// This retains source materialisation and future-entry assumptions; it is not
/// an entered lambda, native frame or namespace allocation.
pub(super) struct OriginalInterpreterLambdaBody {
    input: ResolvedAnalysisInput,
    lambda: NativeWord,
    producer: Vec<NativeWord>,
    content: Span,
    namespace: SignatureNamespaceScope,
    domain: Option<Arc<OriginalInterpreterSourceDomain>>,
}
impl OriginalInterpreterLambdaBody {
    pub(super) fn for_body_token(
        &self,
        source: &str,
        input: &ResolvedAnalysisInput,
        token: Token,
    ) -> Option<(
        SignatureNamespaceScope,
        Option<Arc<OriginalInterpreterSourceDomain>>,
    )> {
        if self.input != *input
            || self.lambda.image() != &SourceImage::document(source)
            || self.lambda.config() != input.lexer_config()
            || self.producer.is_empty()
            || token.kind != tcl_lexer::TokenType::Str
            || token
                .span
                .start()
                .checked_add(u32::from(token.content_offset))
                != Some(self.content.start())
            || token.span.end() != self.content.end()
        {
            return None;
        }
        Some((self.namespace.clone(), self.domain.clone()))
    }
}
impl SourceInterpreterVisibilitySnapshot {
    fn literal_lambda_body(
        &self,
        invocation: &[NativeWord],
        producer: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        domain: Option<Arc<OriginalInterpreterSourceDomain>>,
    ) -> Option<OriginalInterpreterLambdaBody> {
        let schema = self.possible_source_schema(invocation, namespace)?;
        if schema.hook != Some(tcl_registry::hooks::AnalyserHookId::Apply) {
            return None;
        }
        let ordinal = schema
            .roles
            .iter()
            .find(|(_, role)| *role == tcl_registry::ArgRole::LambdaLiteral)?
            .0;
        let lambda = invocation.get(ordinal.checked_add(1)?)?;
        let input = self.original_input(lambda)?;
        let fields = crate::lambda_literal::split_original_lambda_literal(lambda)?;
        let content = fields.braced_body()?;
        input.original_list_element(0)?;
        input.original_list_element(1)?;
        if input.original_list_element(3).is_some()
            || producer.is_empty()
            || producer.iter().any(|word| {
                word.image() != self.image() || word.config() != self.input.lexer_config()
            })
        {
            return None;
        }
        let path = match input.original_list_element(2) {
            Some(original) => self
                .policy
                .recipe()
                .lambda_namespace_path(original.bytes())
                .ok()?,
            None => tcl_core_types::ByteNamespacePath::root(),
        };
        Some(OriginalInterpreterLambdaBody {
            input: self.input.clone(),
            lambda: lambda.clone(),
            producer: producer.to_vec(),
            content,
            namespace: SignatureNamespaceScope::C(path),
            domain,
        })
    }
}
impl Analyser {
    /// Namespace source geometry belongs to the original child ledger's
    /// selected Namespace Ensure schema. It supplies no entered namespace.
    pub(super) fn original_interp_namespace_source_scope(
        &self,
        offset: u32,
        arguments: &[Token],
        scope_path: &[usize],
    ) -> Option<SignatureNamespaceScope> {
        let snapshot = self.safe_interp_stack.last()?;
        let input = self.resolved_analysis_input();
        snapshot
            .matches(&SourceImage::document(&self.source), &input)
            .then_some(())?;
        let retained = self.retained_invocation_tokens(offset, arguments);
        let tokens = retained?;
        let native = crate::registry_invocation::original_native_compiler_words(
            snapshot.image(),
            tokens.words(),
            offset,
            input.lexer_config(),
        )?;
        let namespace = self.declaration_namespace_scope(scope_path)?;
        let schema = snapshot.possible_source_schema(&native, &namespace)?;
        if schema.hook != Some(tcl_registry::hooks::AnalyserHookId::NamespaceEval) {
            return None;
        }
        let mut ensure = schema.transitions.facts().iter().filter_map(|fact| {
            let StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
            }) = &fact.transition
            else {
                return None;
            };
            snapshot.mutation_operand(&native, subject)
        });
        let name = ensure.next()?;
        if ensure.any(|other| other != name) {
            return None;
        }
        namespace.child_from_input(&name)
    }

    pub(super) fn original_interp_procedure_source_name(
        &self,
        name: Token,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureSourceCommand> {
        let snapshot = self.safe_interp_stack.last()?;
        let input = self.resolved_analysis_input();
        snapshot
            .matches(&SourceImage::document(&self.source), &input)
            .then_some(())?;
        let namespace = self.declaration_namespace_scope(scope_path)?;
        let span = tcl_lexer::word_span_at(&self.source, name.span);
        let mut declarations = snapshot.slots.values().filter_map(|slot| {
            let SourceVisibleSlot::Visible(SourceCommandOrigin::Declaration {
                name,
                kind,
                operands,
            }) = slot
            else {
                return None;
            };
            if *kind != tcl_registry::CommandBindingDefinitionKind::Procedure
                || operands.is_empty()
                || operands.iter().any(|word| {
                    word.image() != snapshot.image() || word.config() != input.lexer_config()
                })
            {
                return None;
            }
            let key = name.original_word_key()?;
            (key.original_word().span() == span).then_some(key)
        });
        let key = declarations.next()?;
        if declarations.any(|other| other != key) {
            return None;
        }
        crate::signature_scan::scope::SignatureSourceCommand::procedure_from_key(&namespace, key)
    }

    pub(super) fn original_interp_apply_body(
        &self,
        head: Token,
        arguments: &[Token],
        scope_path: &[usize],
    ) -> Option<OriginalInterpreterLambdaBody> {
        let snapshot = self.safe_interp_stack.last()?;
        snapshot
            .matches(
                &SourceImage::document(&self.source),
                &self.resolved_analysis_input(),
            )
            .then_some(())?;
        let mut argv = vec![head];
        argv.extend_from_slice(arguments);
        let native = self.original_visibility_words(&argv)?;
        let namespace = self.declaration_namespace_scope(scope_path)?;
        let domain = super::scope::scope_at(&self.result.global_scope, scope_path)?
            .original_interpreter_source_domain
            .clone();
        snapshot.literal_lambda_body(&native, &native, &namespace, domain)
    }

    /// A typed source path selects its original Create vector independently of
    /// compatibility parent-command reporting keys. No runtime child is issued.
    fn original_interp_state_key(
        &self,
        invocation: &OriginalInterpreterInvocation,
        path: &SignatureSourceNameInput,
    ) -> Option<String> {
        if path.bytes().is_empty() {
            return self.interp_path_stack.last().map(|frame| frame.key.clone());
        }
        let input = self.resolved_analysis_input();
        let binding = self
            .head_identities
            .original_source_interpreter_path_binding(&input, &invocation.native, path)?;
        let image = SourceImage::document(&self.source);
        let mut matches = self.interpreters.iter().filter_map(|(key, state)| {
            let snapshot = state.visibility.as_ref()?;
            (!state.tainted
                && snapshot.matches(&image, &input)
                && snapshot.declaration == binding.creation().original_words())
            .then_some(key)
        });
        let key = matches.next()?;
        matches.next().is_none().then(|| key.clone())
    }

    pub(super) fn original_interp_eval_state_key(
        &self,
        head: Token,
        arguments: &[Token],
    ) -> Option<String> {
        let invocation = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpEval,
        )?;
        let path = invocation.words.operands().get(1)?.as_ref()?.input()?;
        self.original_interp_state_key(&invocation, path)
    }

    pub(super) fn handle_interp_delete_command_original(
        &mut self,
        args: &[String],
        head: Token,
        arguments: &[Token],
    ) {
        let invocation = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpDelete,
        );
        let keys = invocation
            .as_ref()
            .map(|invocation| {
                invocation
                    .transitions
                    .facts()
                    .iter()
                    .filter_map(|fact| {
                        let StateTransition::Interpreter(InterpreterTransition::Delete {
                            interpreter,
                        }) = &fact.transition
                        else {
                            return None;
                        };
                        self.original_interp_state_key(invocation, &invocation.input(interpreter)?)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for key in keys {
            if self.interpreters.remove(&key).is_some() {
                *self.interp_epochs.entry(key).or_insert(0) += 1;
            }
        }
        self.handle_interp_delete_command(args);
    }

    fn original_interpreter_invocation(
        &self,
        head: Token,
        arguments: &[Token],
        hook: tcl_registry::hooks::AnalyserHookId,
    ) -> Option<OriginalInterpreterInvocation> {
        let mut argv = vec![head];
        argv.extend_from_slice(arguments);
        let native = self.original_visibility_words(&argv)?;
        let end = native.last()?.span().end();
        let text = self.source.get(head.span.start() as usize..end as usize)?;
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            text,
            head.span.start(),
            self.lexer_config(),
        )
        .into_iter()
        .next()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        let context = self.resolved_analysis_input().context_registry();
        let transitions = words.with_source_schema(&context, |schema| {
            (schema.semantics.analyser_hook == Some(hook)).then(|| schema.state_transitions())
        })??;
        Some(OriginalInterpreterInvocation {
            native,
            words,
            transitions,
        })
    }
    fn original_visibility_words(&self, argv: &[Token]) -> Option<Vec<NativeWord>> {
        let input = self.resolved_analysis_input();
        let image = SourceImage::document(&self.source);
        if !self.head_identities.matches_resolved_analysis_input(&input)
            || !self
                .head_identities
                .matches_original_source_image(&image, self.lexer_config())
        {
            return None;
        }
        // Prefer the complete original child arena when it owns this exact
        // vector. Flattened invocation tokens cannot supply expansion markers.
        if let Some(original) = self.original_child_body_words(argv) {
            return Some(original);
        }
        let tokens = self.retained_invocation_tokens(argv.first()?.span.start(), argv);
        let Some(tokens) = tokens else {
            return self.original_child_body_words(argv);
        };
        if tokens.argv.len() != argv.len()
            || tokens
                .argv
                .iter()
                .zip(argv)
                .any(|(span, token)| *span != token.span)
        {
            return None;
        }
        crate::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            argv.first()?.span.start(),
            self.lexer_config(),
        )
        .or_else(|| self.original_child_body_words(argv))
    }

    /// Readonly lexical vectors of the privately retained original child root
    /// body. Only actual root commands and executable brackets are traversed;
    /// inert braced values do not become source scripts.
    fn original_child_body_words(&self, argv: &[Token]) -> Option<Vec<NativeWord>> {
        let snapshot = self.safe_interp_stack.last()?;
        let input = self.resolved_analysis_input();
        snapshot
            .matches(&SourceImage::document(&self.source), &input)
            .then_some(())?;
        let body = snapshot.body.as_ref()?;
        let mut pending = vec![body.content_span().ok()?];
        let mut visited = std::collections::HashSet::new();
        while let Some(span) = pending.pop() {
            if !visited.insert(span) {
                continue;
            }
            let plan = tcl_lexer::native_script_words_in(
                snapshot.image().clone(),
                span,
                input.lexer_config(),
            )
            .ok()?;
            for command in plan.commands {
                if command.words.len() == argv.len()
                    && command
                        .words
                        .iter()
                        .zip(argv)
                        .all(|(word, token)| word.group().span == token.span)
                {
                    return Some(command.words);
                }
                for word in command.words {
                    for part in word.executable_parts().all_parts() {
                        if let tcl_lexer::ExecutablePart::Command { body } = part.part {
                            pending.push(body);
                        }
                    }
                }
            }
        }
        None
    }
    pub(super) fn retain_interp_visibility_declaration(
        &mut self,
        path: &str,
        head: Token,
        arguments: &[Token],
    ) {
        let captured = (|| {
            let invocation = self.original_interpreter_invocation(
                head,
                arguments,
                tcl_registry::hooks::AnalyserHookId::InterpCreate,
            )?;
            let policy = self.declaration_name_policy()?;
            if !matches!(policy.recipe(), NativeNameProtocol::C(_)) {
                return None;
            }
            for fact in invocation.transitions.facts() {
                if let StateTransition::Interpreter(InterpreterTransition::Create {
                    interpreter: Some(interpreter),
                    safety,
                }) = &fact.transition
                {
                    let input = invocation.input(interpreter)?;
                    if input.bytes() != path.as_bytes() {
                        return None;
                    }
                    return SourceInterpreterVisibilitySnapshot::initialize(
                        self.resolved_analysis_input(),
                        policy,
                        invocation.native.clone(),
                        match safety {
                            ChildInterpreterSafety::Safe => true,
                            ChildInterpreterSafety::Inherited => self
                                .safe_interp_stack
                                .last()
                                .is_some_and(|parent| parent.safe),
                            ChildInterpreterSafety::Unknown => return None,
                        },
                    );
                }
            }
            None
        })();
        let key = self.qualified_interp_key(path);
        if let Some(state) = self.interpreters.get_mut(&key) {
            state.visibility = captured;
        }
    }
    pub(super) fn original_interp_visibility_body(
        &self,
        key: &str,
        body: Token,
        head: Token,
        arguments: &[Token],
    ) -> Option<SourceInterpreterVisibilitySnapshot> {
        let state = self.interpreters.get(key)?;
        if state.tainted {
            return None;
        }
        let mut snapshot = state.visibility.clone()?;
        let image = SourceImage::document(&self.source);
        if !snapshot.matches(&image, &self.resolved_analysis_input()) {
            return None;
        }
        let invocation = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpEval,
        )?;
        let path = invocation.words.operands().get(1)?.as_ref()?.input()?;
        if self.original_interp_state_key(&invocation, path).as_deref() != Some(key) {
            return None;
        }
        let context = self.resolved_analysis_input().context_registry();
        let requested = tcl_lexer::word_span(&tcl_lexer::SourceMap::new(&self.source), body);
        for region in invocation.words.source_script_bodies_for(
            &context,
            crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        ) {
            if region.original_container().span() != requested
                || !region.matches_source(&image, self.lexer_config())
                || !region.matches_context(&context)
            {
                continue;
            }
            snapshot.body = Some(region.original_container().clone());
            snapshot.body_invocation = invocation.native;
            return Some(snapshot);
        }
        None
    }
    pub(super) fn original_interp_handle_visibility_body(
        &self,
        body: Token,
        head: Token,
        arguments: &[Token],
    ) -> Option<(String, SourceInterpreterVisibilitySnapshot)> {
        let input = self.resolved_analysis_input();
        let image = SourceImage::document(&self.source);
        let mut argv = vec![head];
        argv.extend_from_slice(arguments);
        let original = self.original_visibility_words(&argv)?;
        let receipt = self
            .head_identities
            .original_source_interpreter_handle_body(&input, &original)?;
        let key = self
            .qualified_interp_key(std::str::from_utf8(receipt.interpreter_input().bytes()).ok()?);
        // The retained Create vector identifies the child independently of
        // its moved parent command's compatibility reporting key.
        let mut candidates = self.interpreters.values().filter_map(|state| {
            let snapshot = state.visibility.as_ref()?;
            (!state.tainted
                && snapshot.matches(&image, &input)
                && receipt.creation().original_words() == snapshot.declaration)
                .then_some(snapshot)
        });
        let mut snapshot = candidates.next()?.clone();
        if candidates.next().is_some() {
            return None;
        }
        if receipt.creation().original_words() != snapshot.declaration
            || receipt.body_word().span() != tcl_lexer::word_span_at(&self.source, body.span)
        {
            return None;
        }
        snapshot.body = Some(receipt.body_word().clone());
        snapshot.body_invocation = original;
        Some((key, snapshot))
    }

    pub(super) fn apply_original_interp_visibility_delta(
        &mut self,
        path: &str,
        head: Token,
        arguments: &[Token],
        hide: bool,
    ) {
        if crate::naming::is_dynamic_word(path) {
            self.dynamic_interp_ops = true;
            for state in self.interpreters.values_mut() {
                state.visibility = None;
                state.tainted = true;
            }
            return;
        }
        let hook = if hide {
            tcl_registry::hooks::AnalyserHookId::InterpHide
        } else {
            tcl_registry::hooks::AnalyserHookId::InterpExpose
        };
        let delta = self
            .original_interpreter_invocation(head, arguments, hook)
            .and_then(|invocation| {
                for fact in invocation.transitions.facts() {
                    if let StateTransition::Interpreter(
                        InterpreterTransition::Hide {
                            interpreter,
                            visible,
                            hidden,
                        }
                        | InterpreterTransition::Expose {
                            interpreter,
                            visible,
                            hidden,
                        },
                    ) = &fact.transition
                    {
                        if invocation.input(interpreter)?.bytes() != path.as_bytes() {
                            return None;
                        }
                        let key = self.original_interp_state_key(
                            &invocation,
                            &invocation.input(interpreter)?,
                        )?;
                        return Some((
                            key,
                            invocation.input(visible)?,
                            invocation.input(hidden)?,
                            invocation.native.clone(),
                        ));
                    }
                }
                None
            });
        let Some((key, visible, hidden, operands)) = delta else {
            return;
        };
        let Some(state) = self.interpreters.get_mut(&key) else {
            return;
        };
        let applied = (|| {
            let snapshot = state.visibility.as_mut()?;
            if hide {
                snapshot.hide(&visible, &hidden, &operands)
            } else {
                snapshot.expose(&hidden, &visible, &operands)
            }
        })();
        if applied.is_none() {
            state.tainted = true;
            state.visibility = None;
        }
    }
    fn original_interp_declared_receiver_body(
        &self,
        native: &[NativeWord],
        scope_path: &[usize],
    ) -> Option<Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>> {
        let scope = super::scope::scope_at(&self.result.global_scope, scope_path)?;
        if scope.kind != super::types::ScopeKind::Method || scope.naming_scope.is_some() {
            return None;
        }
        let input = self.resolved_analysis_input();
        let image = SourceImage::document(&self.source);
        self.head_identities
            .matches_resolved_analysis_input(&input)
            .then_some(())?;
        let bindings = self.head_identities.source_bindings_ref();
        let origin = bindings.source_origin()?;
        if origin.source_image() != &image {
            return None;
        }
        let body = bindings.declared_receiver_body_entry_at(origin, native.first()?.span().start());
        let body = body?;
        let source = body.source();
        let crate::command_binding::ExecutedScriptMapping::Contiguous { base } = source.mapping
        else {
            return None;
        };
        let end = base.checked_add(u32::try_from(source.text.bytes().len()).ok()?)?;
        (source.origin == *origin
            && native.iter().all(|word| {
                word.image() == &image
                    && word.config() == input.lexer_config()
                    && word.span().start() >= base
                    && word.span().end() <= end
            }))
        .then_some(body)
    }

    fn original_interp_source_receiver_declaration(
        &self,
        native: &[NativeWord],
        scope_path: &[usize],
    ) -> Option<Arc<super::types::OriginalSourceReceiverBodyDeclaration>> {
        let scope = super::scope::scope_at(&self.result.global_scope, scope_path)?;
        if scope.kind != super::types::ScopeKind::Method || scope.naming_scope.is_some() {
            return None;
        }
        let body = scope.original_receiver_body_declaration.as_ref()?;
        body.owns_invocation(native, &self.resolved_analysis_input())
            .then(|| Arc::clone(body))
    }

    /// Conditional source advice never skips an invocation or removes edges.
    pub(super) fn observe_interp_visibility(&mut self, argv: &[Token], scope_path: &[usize]) {
        self.retain_original_interp_source_loads(argv, scope_path);
        let observation = (|| {
            let snapshot = self.safe_interp_stack.last()?;
            if !snapshot.matches(
                &SourceImage::document(&self.source),
                &self.resolved_analysis_input(),
            ) {
                return None;
            }
            let native = self.original_visibility_words(argv);
            let native = native?;
            let name = snapshot.original_input(native.first()?)?;
            if let Some(namespace) = self.declaration_namespace_scope(scope_path) {
                return snapshot.observe(name, namespace, native);
            }
            let declared = self.original_interp_declared_receiver_body(&native, scope_path);
            let source_declaration =
                self.original_interp_source_receiver_declaration(&native, scope_path);
            if declared.is_none() && source_declaration.is_none() {
                return None;
            }
            // This is an explicit possible root-fallback hypothesis for the
            // actual declared body, never a root namespace stamped on a receiver.
            let mut subject = snapshot.observe(
                name,
                SignatureNamespaceScope::C(tcl_core_types::ByteNamespacePath::root()),
                native,
            )?;
            subject.namespace = None;
            subject.declared_receiver = declared;
            subject.source_receiver_declaration = source_declaration;
            subject.obligations.extend([
                InterpreterVisibilityObligation::UnknownExecutingReceiverNamespace,
                InterpreterVisibilityObligation::NoEarlierReceiverNamespaceBinding,
            ]);
            Some(subject)
        })();
        if let Some(subject) = observation {
            self.emit_interp_visibility(subject, argv[0].span);
        }
        let mutation = (|| {
            let snapshot = self.safe_interp_stack.last()?;
            let native = self.original_visibility_words(argv)?;
            let namespace = self.declaration_namespace_scope(scope_path)?;
            let schema = snapshot.possible_source_schema(&native, &namespace)?;
            let mut updated = snapshot.clone();
            let closed = updated
                .apply_command_transitions(&native, &namespace, &schema.transitions)
                .is_some();
            Some((updated, closed))
        })();
        if let Some((updated, closed)) = mutation {
            if closed {
                if let Some(current) = self.safe_interp_stack.last_mut() {
                    *current = updated.clone();
                }
            } else if let Some(current) = self.safe_interp_stack.last_mut() {
                current.slots.clear();
                current.hidden.clear();
                current.unavailable.clear();
            }
            if let Some(frame) = self.interp_path_stack.last()
                && let Some(state) = self.interpreters.get_mut(&frame.key)
            {
                state.visibility = closed.then_some(updated);
                state.tainted |= !closed;
            }
        }
    }
    fn retain_original_interp_source_loads(&mut self, argv: &[Token], scope_path: &[usize]) {
        let candidate = (|| {
            let snapshot = self.safe_interp_stack.last()?.clone();
            if !snapshot.matches(
                &SourceImage::document(&self.source),
                &self.resolved_analysis_input(),
            ) {
                return None;
            }
            let native = self.original_visibility_words(argv)?;
            let (namespace, declared_receiver, source_receiver_declaration) =
                if let Some(namespace) = self.declaration_namespace_scope(scope_path) {
                    (namespace, None, None)
                } else {
                    let declared = self.original_interp_declared_receiver_body(&native, scope_path);
                    let source_declaration =
                        self.original_interp_source_receiver_declaration(&native, scope_path);
                    if declared.is_none() && source_declaration.is_none() {
                        return None;
                    }
                    (
                        SignatureNamespaceScope::C(tcl_core_types::ByteNamespacePath::root()),
                        declared,
                        source_declaration,
                    )
                };
            let schema = snapshot.source_load_schema(&native, &namespace)?;
            let mut paths = Vec::new();
            for fact in schema.transitions.facts() {
                if let StateTransition::Package(
                    tcl_registry::model::binding::PackageTransition::SourceLoad { path },
                ) = &fact.transition
                {
                    let word = native.get(path.argument_index()?.checked_add(1)?)?.clone();
                    let input = snapshot.original_input(&word);
                    let literal = input
                        .as_ref()
                        .and_then(|input| std::str::from_utf8(input.bytes()).ok())
                        .map(str::to_owned);
                    let raw = literal.clone().or_else(|| {
                        self.source
                            .get(word.content_span().ok()?.as_range())
                            .map(str::to_owned)
                    })?;
                    paths.push((
                        raw,
                        literal.is_some(),
                        OriginalInterpreterSourceLoad {
                            snapshot: snapshot.clone(),
                            invocation: native.clone(),
                            path: word,
                            declared_receiver: declared_receiver.clone(),
                            source_receiver_declaration: source_receiver_declaration.clone(),
                        },
                    ));
                }
            }
            Some(paths)
        })();
        for (raw_path, is_literal, original) in candidate.unwrap_or_default() {
            let range = argv
                .iter()
                .find(|token| {
                    tcl_lexer::word_span_at(&self.source, token.span) == original.path.span()
                })
                .map_or(original.path.span(), |token| token.span);
            if self
                .result
                .source_targets
                .iter()
                .any(|source| source.range == range)
            {
                continue;
            }
            self.result
                .source_targets
                .push(crate::signature_scan::types::SignatureSource {
                    raw_path,
                    range,
                    is_literal,
                    site_namespace: self.command_resolution_namespace(scope_path),
                    original_interpreter_source_load: Some(Arc::new(original)),
                });
        }
    }

    fn emit_interp_visibility(
        &mut self,
        subject: ConditionalInterpreterVisibilitySubject,
        span: Span,
    ) {
        if self.structure_only {
            return;
        }
        let label = tcl_syntax::native_string::resident_name_label(subject.name_input().bytes());
        self.result.diagnostics.push(super::Diagnostic::new(tcl_core_types::DiagCode::W129,span,format!("'{label}' may be unavailable in this child interpreter under the retained source visibility assumptions"),super::types::Severity::Warning).with_subject(super::DiagnosticSubject::ConditionalInterpreterVisibility(Arc::new(subject))));
    }
    pub(super) fn safe_interp_ctx_snapshot(&self) -> Option<SourceInterpreterVisibilitySnapshot> {
        self.safe_interp_stack.last().cloned()
    }
}

/// A possible file-source edge under a retained child source declaration.
/// The original path and full visibility lineage remain owned; file existence,
/// actual dispatch, file execution and Normal completion are not established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalInterpreterSourceLoad {
    snapshot: SourceInterpreterVisibilitySnapshot,
    invocation: Vec<NativeWord>,
    path: NativeWord,
    declared_receiver: Option<Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>>,
    source_receiver_declaration: Option<Arc<super::types::OriginalSourceReceiverBodyDeclaration>>,
}
impl OriginalInterpreterSourceLoad {
    /// Independent declared receiver source scope when its runtime namespace
    /// is unknown. This is a possible global-source hypothesis only.
    #[must_use]
    pub fn declared_receiver_body(
        &self,
    ) -> Option<&crate::command_binding::SourceDeclaredReceiverBodyEntry> {
        self.declared_receiver.as_deref()
    }
    /// Selected original method-body declaration without a native class or
    /// receiver allocation. This source scope supplies no executing namespace.
    #[must_use]
    pub fn source_receiver_body_declaration(
        &self,
    ) -> Option<&super::types::OriginalSourceReceiverBodyDeclaration> {
        self.source_receiver_declaration.as_deref()
    }
    /// Original path operand, independently of its reporting text.
    #[must_use]
    pub const fn path_word(&self) -> &NativeWord {
        &self.path
    }
    /// Complete original invocation vector, including the real source head.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.invocation
    }
    /// Same retained analysis and full input, without rebuilding source facts.
    #[must_use]
    pub fn matches_analysis(&self, analysis: &super::AnalysisResult) -> bool {
        analysis.resolved_input.as_ref().is_some_and(|input| {
            self.matches_source(self.path.image(), input)
                && analysis.matches_original_source_image(self.path.image(), input.lexer_config())
        })
    }

    /// Whole original source/channel and complete immutable input join.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.snapshot.matches(image, input)
            && self
                .source_receiver_declaration
                .as_ref()
                .is_none_or(|body| {
                    body.matches_source(image, input)
                        && body.owns_invocation(&self.invocation, input)
                })
            && self.declared_receiver.as_ref().is_none_or(|body| {
                body.source().origin.source_image() == image
                    && matches!(
                        body.source().mapping,
                        crate::command_binding::ExecutedScriptMapping::Contiguous { .. }
                    )
            })
            && self.path.image() == image
            && self.path.config() == input.lexer_config()
            && self.invocation.contains(&self.path)
            && self
                .invocation
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalSourceAsciiListValue {
    operand: NativeWord,
    producer: Vec<NativeWord>,
    input: ResolvedAnalysisInput,
    bytes: Vec<u8>,
}
impl OriginalSourceAsciiListValue {
    fn matches(
        &self,
        snapshot: &SourceInterpreterVisibilitySnapshot,
        operand: &NativeWord,
    ) -> bool {
        self.operand == *operand
            && self.input == snapshot.input
            && self.producer.iter().all(|word| {
                word.image() == snapshot.image() && word.config() == self.input.lexer_config()
            })
            && crate::command_binding::original_single_command_substitution_words(operand)
                .is_some_and(|original| original.command().words == self.producer)
    }
}

struct ConditionalSourceCommandSchema {
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    scripts: Vec<usize>,
    source_values: Vec<Option<OriginalSourceAsciiListValue>>,
    traits: tcl_registry::Traits,
    hook: Option<tcl_registry::hooks::AnalyserHookId>,
    transitions: tcl_registry::StateTransitions,
}
impl SourceCommandOrigin {
    fn registry_command(&self) -> Option<&str> {
        match self {
            Self::Registry(command) => Some(command),
            Self::Exposed { original, .. } | Self::Renamed { original, .. } => {
                original.registry_command()
            }
            Self::Declaration { .. } | Self::Alias { .. } => None,
        }
    }
}
impl SourceInterpreterVisibilitySnapshot {
    fn possible_source_schema(
        &self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
    ) -> Option<ConditionalSourceCommandSchema> {
        let input = self.original_input(native.first()?)?;
        let slot = self.lookup_slot(input.bytes(), namespace)?;
        let SourceVisibleSlot::Visible(origin) = self.slots.get(&slot)? else {
            return None;
        };
        self.schema_for_origin(native, origin, namespace)
    }

    fn source_load_schema(
        &self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
    ) -> Option<ConditionalSourceCommandSchema> {
        let input = self.original_input(native.first()?)?;
        let slot = self.lookup_slot(input.bytes(), namespace)?;
        let origin = match self.slots.get(&slot)? {
            SourceVisibleSlot::Visible(origin) | SourceVisibleSlot::Unavailable(origin) => origin,
        };
        self.schema_for_origin(native, origin, namespace)
    }

    fn original_ascii_list_value(
        &self,
        word: &NativeWord,
        namespace: &SignatureNamespaceScope,
    ) -> Option<OriginalSourceAsciiListValue> {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        if word.group().expand
            || word.image() != self.image()
            || word.config() != self.input.lexer_config()
        {
            return None;
        }
        let original = crate::command_binding::original_single_command_substitution_words(word)?;
        let native = &original.command().words;
        let head = self.original_input(native.first()?)?;
        let slot = self.lookup_slot(head.bytes(), namespace)?;
        let SourceVisibleSlot::Visible(origin) = self.slots.get(&slot)? else {
            return None;
        };
        let values = native
            .iter()
            .map(|word| self.original_ascii_list_value(word, namespace))
            .collect::<Vec<_>>();
        let bytes = self.with_schema_for_origin(native, origin, &values, |schema| {
            schema.authored_source_ascii_list_result()
        })?;
        Some(OriginalSourceAsciiListValue {
            operand: word.clone(),
            producer: native.clone(),
            input: self.input.clone(),
            bytes,
        })
    }
    fn schema_for_origin(
        &self,
        native: &[NativeWord],
        origin: &SourceCommandOrigin,
        namespace: &SignatureNamespaceScope,
    ) -> Option<ConditionalSourceCommandSchema> {
        let source_values = native
            .iter()
            .map(|word| self.original_ascii_list_value(word, namespace))
            .collect::<Vec<_>>();
        self.with_schema_for_origin(native, origin, &source_values, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            if !complete {
                return None;
            }
            let roles = roles
                .into_iter()
                .map(|(ordinal, role)| {
                    Some((
                        schema
                            .semantics
                            .argument_offset
                            .checked_add(usize::from(ordinal))?,
                        role,
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(ConditionalSourceCommandSchema {
                roles,
                scripts: schema.authored_source_script_arguments()?,
                source_values: source_values.clone(),
                traits: schema.semantics.traits,
                hook: schema.semantics.analyser_hook,
                transitions: schema.state_transitions(),
            })
        })
    }
    fn with_schema_for_origin<R>(
        &self,
        native: &[NativeWord],
        origin: &SourceCommandOrigin,
        source_values: &[Option<OriginalSourceAsciiListValue>],
        consume: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<R>,
    ) -> Option<R> {
        if native.len() != source_values.len()
            || native.iter().any(|word| {
                word.image() != self.image() || word.config() != self.input.lexer_config()
            })
        {
            return None;
        }
        let canonical = origin.registry_command()?;
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            native,
            self.policy.string_protocol(),
        )
        .ok()?;
        let arguments = native
            .iter()
            .enumerate()
            .skip(1)
            .map(|(ordinal, word)| {
                if word.group().expand {
                    return Some(tcl_registry::InvocationWord::Expanded);
                }
                let literal = if let Some(value) = &source_values[ordinal] {
                    if !value.matches(self, word) {
                        return None;
                    }
                    Some(value.bytes.as_slice())
                } else {
                    captured.literal(ordinal)
                };
                Some(
                    crate::registry_invocation::source_structure::source_schema_word(
                        literal.map_or(
                            tcl_registry::InvocationWord::Dynamic,
                            tcl_registry::InvocationWord::KnownBytes,
                        ),
                    ),
                )
            })
            .collect::<Option<Vec<_>>>()?;
        let context = self.input.context_registry();
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(canonical),
                    &arguments,
                )
                .with_dialect(tcl_registry::InvocationDialect::of_profile(
                    self.input.unit_profile(),
                )),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        consume(&resolution.resolved()?)
    }
}
impl Analyser {
    /// List quoting preserves its original source producer and possible schema.
    /// An ensemble's reporting map supplies no corresponding target receipt.
    pub(super) fn observe_indirect_interp_visibility(
        &mut self,
        argv: &[Token],
        scope_path: &[usize],
    ) {
        let Some(snapshot) = self.safe_interp_stack.last().cloned() else {
            return;
        };
        let native = self.original_visibility_words(argv);
        let namespace = self.declaration_namespace_scope(scope_path);
        let Some(native) = native else {
            return;
        };
        let Some(namespace) = namespace else {
            return;
        };
        if !snapshot.matches(
            &SourceImage::document(&self.source),
            &self.resolved_analysis_input(),
        ) {
            return;
        }
        if native.first().is_some_and(|word| word.group().expand) {
            self.observe_original_list_quote(&snapshot, &native[0], &namespace, scope_path);
        }
        let Some(schema) = snapshot.possible_source_schema(&native, &namespace) else {
            return;
        };
        if schema
            .source_values
            .iter()
            .enumerate()
            .any(|(ordinal, value)| {
                value
                    .as_ref()
                    .is_some_and(|value| !value.matches(&snapshot, &native[ordinal]))
            })
        {
            return;
        }
        for ordinal in schema.scripts {
            if let Some(word) = native.get(ordinal + 1) {
                self.observe_original_list_quote(&snapshot, word, &namespace, scope_path);
            }
        }
    }
    fn observe_original_list_quote(
        &mut self,
        snapshot: &SourceInterpreterVisibilitySnapshot,
        word: &NativeWord,
        namespace: &SignatureNamespaceScope,
        scope_path: &[usize],
    ) {
        let Some(producer) =
            crate::command_binding::original_single_command_substitution_words(word)
        else {
            return;
        };
        let original = producer.original_operand();
        if original.image() != snapshot.image()
            || original.config() != snapshot.input.lexer_config()
        {
            return;
        }
        let command = producer.command();
        let body = command.span;
        let native = &command.words;
        let Some(schema) = snapshot.possible_source_schema(native, namespace) else {
            return;
        };
        if !schema
            .traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
        {
            return;
        }
        let Some(head) = native.get(1) else {
            return;
        };
        let Some(name) = snapshot.original_input(head) else {
            return;
        };
        if let Some(subject) = snapshot.observe(name, namespace.clone(), native.clone()) {
            self.emit_interp_visibility(subject, head.span());
        }
        // A lambda's body is source structure under this exact original list
        // producer. No effective dispatch or actual frame is inferred here.
        if snapshot
            .possible_source_schema(&native[1..], namespace)
            .is_some_and(|schema| schema.hook == Some(tcl_registry::hooks::AnalyserHookId::Apply))
        {
            let Some(text) = self.source.get(body.as_range()) else {
                return;
            };
            let Some(segment) = crate::segmenter::segment_commands_with_offset_and_config(
                text,
                body.start(),
                word.config(),
            )
            .into_iter()
            .next() else {
                return;
            };
            if segment.argv.len() != native.len() {
                return;
            }
            let domain = super::scope::scope_at(&self.result.global_scope, scope_path)
                .and_then(|scope| scope.original_interpreter_source_domain.clone());
            let original = snapshot.literal_lambda_body(&native[1..], native, namespace, domain);
            self.handle_apply_command_with_source_namespace(
                &segment.texts[2..],
                &segment.argv[2..],
                scope_path,
                original,
            );
        }
    }
}

impl SourceInterpreterVisibilitySnapshot {
    fn mutation_operand(
        &self,
        native: &[NativeWord],
        subject: &TransitionSubject,
    ) -> Option<SignatureSourceNameInput> {
        self.original_input(native.get(subject.argument_index()?.checked_add(1)?)?)
    }
    fn withdraw_visible_slot(&mut self, slot: &ByteCommandSlot) {
        self.slots.remove(slot);
        if let Some(origin) = self.unavailable.get(slot) {
            self.slots
                .insert(slot.clone(), SourceVisibleSlot::Unavailable(origin.clone()));
        }
    }
    fn apply_move(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        from: &TransitionSubject,
        to: &TransitionSubject,
    ) -> Option<()> {
        let from = self.mutation_operand(native, from)?;
        let to = self.mutation_operand(native, to)?;
        let selected = self
            .policy
            .recipe()
            .rename_source_input(namespace.context()?, from.bytes())
            .ok()?;
        let slot = self.lookup_slot(selected.selected(), namespace)?;
        let Some(SourceVisibleSlot::Visible(origin)) = self.slots.get(&slot).cloned() else {
            return Some(());
        };
        let destination = if to.bytes().is_empty() {
            None
        } else {
            Some(
                self.policy
                    .recipe()
                    .rename_destination_slot(namespace.context()?, to.bytes())
                    .ok()?,
            )
        };
        if destination.as_ref().is_some_and(|destination| {
            matches!(
                self.slots.get(destination),
                Some(SourceVisibleSlot::Visible(_))
            )
        }) {
            return Some(());
        }
        self.withdraw_visible_slot(&slot);
        if let Some(destination) = destination {
            self.slots.insert(
                destination,
                SourceVisibleSlot::Visible(SourceCommandOrigin::Renamed {
                    original: Box::new(origin),
                    operands: native.to_vec(),
                }),
            );
        }
        Some(())
    }
    fn apply_binding_transition(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        transition: &tcl_registry::CommandBindingTransition,
    ) -> Option<()> {
        use tcl_registry::CommandBindingTransition as Binding;
        match transition {
            Binding::Define { name, kind } => {
                let input = self.mutation_operand(native, name)?;
                let slot = self
                    .policy
                    .recipe()
                    .command_publication_slot(namespace.context()?, input.bytes())
                    .ok()?;
                self.slots.insert(
                    slot,
                    SourceVisibleSlot::Visible(SourceCommandOrigin::Declaration {
                        name: input,
                        kind: *kind,
                        operands: native.to_vec(),
                    }),
                );
            }
            Binding::Move { from, to } => self.apply_move(native, namespace, from, to)?,
            Binding::Delete { interpreter, name } => {
                if let Some(interpreter) = interpreter
                    && !self
                        .mutation_operand(native, interpreter)?
                        .bytes()
                        .is_empty()
                {
                    return Some(());
                }
                let input = self.mutation_operand(native, name)?;
                let slot = self.lookup_slot(input.bytes(), namespace)?;
                self.withdraw_visible_slot(&slot);
            }
            Binding::Alias {
                source_interpreter,
                alias,
                target,
                ..
            } => {
                if !self
                    .mutation_operand(native, source_interpreter)?
                    .bytes()
                    .is_empty()
                {
                    return Some(());
                }
                let input = self.mutation_operand(native, alias)?;
                self.mutation_operand(native, target)?;
                let slot = self
                    .policy
                    .recipe()
                    .alias_publication_slot(namespace.context()?, input.bytes())
                    .ok()?;
                self.slots.insert(
                    slot,
                    SourceVisibleSlot::Visible(SourceCommandOrigin::Alias {
                        name: input,
                        operands: native.to_vec(),
                    }),
                );
            }
            Binding::Unknown { .. } => return None,
        }
        Some(())
    }
    fn apply_namespace_transition(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        transition: &tcl_registry::NamespaceTransition,
    ) -> Option<()> {
        use tcl_registry::{NamespaceTransition as Namespace, NamespaceTransitionTarget as Target};
        match transition {
            Namespace::Ensure { .. } | Namespace::Export { .. } => Some(()),
            Namespace::Delete { namespace: target } => {
                let path = match target {
                    Target::Current => match namespace {
                        SignatureNamespaceScope::C(path) => path.clone(),
                        _ => return None,
                    },
                    Target::Named(subject) => {
                        let input = self.mutation_operand(native, subject)?;
                        self.policy
                            .recipe()
                            .namespace_address_path(namespace.context()?, input.bytes())
                            .ok()?
                    }
                };
                if path.is_root() {
                    return None;
                }
                self.slots.retain(|slot, _| {
                    !slot.namespace.as_segments().starts_with(path.as_segments())
                });
                Some(())
            }
            Namespace::Import { .. }
            | Namespace::Forget { .. }
            | Namespace::SetPath { .. }
            | Namespace::SetUnknown { .. }
            | Namespace::Ensemble { .. } => None,
        }
    }
    fn apply_command_transitions(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        transitions: &tcl_registry::StateTransitions,
    ) -> Option<()> {
        for fact in transitions.facts() {
            match &fact.transition {
                StateTransition::CommandBinding(transition) => {
                    self.apply_binding_transition(native, namespace, transition)?;
                }
                StateTransition::Namespace(transition) => {
                    self.apply_namespace_transition(native, namespace, transition)?;
                }
                _ => {}
            }
        }
        if transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
        {
            return None;
        }
        Some(())
    }
}

impl Analyser {
    pub(super) fn retain_interp_visibility_alias(&mut self, head: Token, arguments: &[Token]) {
        let Some(invocation) = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpAlias,
        ) else {
            for state in self.interpreters.values_mut() {
                state.visibility = None;
                state.tainted = true;
            }
            return;
        };
        for transition in invocation.transitions.command_bindings() {
            if let tcl_registry::CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target,
                ..
            } = transition
            {
                let owned = (|| {
                    let path = invocation.input(source_interpreter)?;
                    let key = self.original_interp_state_key(&invocation, &path)?;
                    let alias = invocation.input(alias)?;
                    invocation.input(target)?;
                    let policy = alias.policy();
                    let slot = policy
                        .recipe()
                        .alias_publication_slot(NativeNameContext::root(), alias.bytes())
                        .ok()?;
                    Some((key, slot, alias, invocation.native.clone()))
                })();
                if let Some((key, slot, name, operands)) = owned {
                    if let Some(state) = self.interpreters.get_mut(&key)
                        && let Some(snapshot) = state.visibility.as_mut()
                    {
                        snapshot.slots.insert(
                            slot,
                            SourceVisibleSlot::Visible(SourceCommandOrigin::Alias {
                                name,
                                operands,
                            }),
                        );
                    }
                } else {
                    for state in self.interpreters.values_mut() {
                        state.visibility = None;
                        state.tainted = true;
                    }
                }
            } else {
                for state in self.interpreters.values_mut() {
                    state.visibility = None;
                    state.tainted = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subjects(
        result: &super::super::AnalysisResult,
    ) -> Vec<&ConditionalInterpreterVisibilitySubject> {
        result
            .diagnostics
            .iter()
            .filter_map(super::super::Diagnostic::conditional_interpreter_visibility)
            .collect()
    }

    #[test]
    fn original_child_script_positions_retain_list_values_and_reference_timing() {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for body in [
                "uplevel [list source b.tcl]",
                "uplevel [list source {a b.tcl}]",
                "uplevel [list source [list b.tcl]]",
            ] {
                let source = format!("interp create -safe s; interp eval s {{{body}}}");
                let analysis = Analyser::new().analyse(&source, dialect);
                let observed = subjects(&analysis);
                assert_eq!(
                    observed.len(),
                    1,
                    "{dialect} {body}: {:?}",
                    analysis.diagnostics
                );
                assert_eq!(observed[0].name_input().bytes(), b"source");
                assert_eq!(observed[0].original_words().len(), 3);
                assert!(observed[0].matches_source(
                    &SourceImage::document(&source),
                    analysis.resolved_input.as_ref().unwrap()
                ));
                assert!(
                    observed[0].original_words()[1].span().start()
                        > u32::try_from(source.find("uplevel").unwrap()).unwrap()
                );
            }
            for body in [
                "uplevel [list $head b.tcl]",
                "uplevel [lindex {source b.tcl} 0]",
                "proc list args {return data}; uplevel [list source b.tcl]",
                "set data [list source b.tcl]",
                "trace remove variable x write [list exec ls]",
                "trace add $kind x write [list exec ls]",
            ] {
                let source = format!("interp create -safe s; interp eval s {{{body}}}");
                let analysis = Analyser::new().analyse(&source, dialect);
                assert!(
                    subjects(&analysis).is_empty(),
                    "{dialect} {body}: {:?}",
                    analysis.diagnostics
                );
            }
        }
    }

    #[test]
    fn original_child_receiver_visibility_keeps_unknown_namespace_and_body_owner_explicit() {
        // naming.interpreter.original-source-visibility-advice
        // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
        // naming.tcloo.original-source-receiver-body-declaration
        // docs/design/analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md
        let source = "interp create -safe s; interp eval s {oo::class create C {method m {} {source a.tcl}}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let observed = subjects(&analysis);
        assert_eq!(observed.len(), 1, "{:?}", analysis.diagnostics);
        assert!(observed[0].namespace().is_none());
        assert!(observed[0].source_receiver_body_declaration().is_some());
        assert!(observed[0].declared_receiver_body().is_none());
        assert!(
            observed[0]
                .obligations()
                .contains(&InterpreterVisibilityObligation::UnknownExecutingReceiverNamespace)
        );
        assert!(
            observed[0]
                .obligations()
                .contains(&InterpreterVisibilityObligation::NoEarlierReceiverNamespaceBinding)
        );
        assert!(observed[0].matches_source(
            &SourceImage::document(source),
            analysis.resolved_input.as_ref().unwrap()
        ));
        assert!(observed[0].slot().namespace.is_root());
        assert_eq!(observed[0].slot().simple.as_bytes(), b"source");
        assert!(
            analysis.source_targets[0]
                .original_interpreter_source_load
                .as_ref()
                .unwrap()
                .source_receiver_body_declaration()
                .is_some()
        );
        let inert = "interp create -safe s; interp eval s {set data {method m {} {source a.tcl}}}";
        assert!(subjects(&Analyser::new().analyse(inert, "tcl8.6")).is_empty());
    }

    #[test]
    fn original_created_handle_visibility_uses_the_same_child_source_snapshot() {
        // naming.interpreter.original-created-handle-source-body
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
        let handle = "interp create -safe s; s eval {source a.tcl}";
        let path = "interp create -safe s; interp eval s {source a.tcl}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let by_handle = Analyser::new().analyse(handle, dialect);
            let by_path = Analyser::new().analyse(path, dialect);
            assert_eq!(
                subjects(&by_handle).len(),
                1,
                "{dialect}: {:?}",
                by_handle.diagnostics
            );
            assert_eq!(
                subjects(&by_path).len(),
                1,
                "{dialect}: {:?}",
                by_path.diagnostics
            );
            assert_eq!(subjects(&by_handle)[0].slot(), subjects(&by_path)[0].slot());
            assert_eq!(
                subjects(&by_handle)[0].obligations(),
                subjects(&by_path)[0].obligations()
            );
            assert!(subjects(&by_handle)[0].matches_source(
                &SourceImage::document(handle),
                by_handle.resolved_input.as_ref().unwrap()
            ));
            assert_eq!(by_handle.source_targets.len(), 1);
            assert!(
                by_handle.source_targets[0]
                    .original_interpreter_source_load
                    .is_some()
            );
        }
        for source in [
            "interp create -safe s; rename s moved; s eval {source a.tcl}",
            "interp create -safe s; rename s moved; interp delete s; moved eval {source a.tcl}",
            "interp create -safe s; rename s moved; rename moved {}; moved eval {source a.tcl}",
            "interp create -safe s; mystery; s eval {source a.tcl}",
            "interp create -safe s; proc s args {}; s eval {source a.tcl}",
        ] {
            let result = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                subjects(&result).is_empty(),
                "{source}: {:?}",
                result.diagnostics
            );
        }
    }

    #[test]
    fn original_moved_handle_visibility_uses_the_original_child_path() {
        // naming.interpreter.original-created-handle-source-body
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
        // Independent CLI observations: naming.interpreter.original-created-parent-command-move
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command-move.md
        for source in [
            "interp create -safe s; rename s moved; moved eval {source a.tcl}",
            "interp create -safe s; rename s first; rename first second; second eval {source a.tcl}",
            "interp create -safe s; namespace eval N {}; rename s ::N::held; ::N::held eval {source a.tcl}",
        ] {
            for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
                let result = Analyser::new().analyse(source, dialect);
                assert_eq!(subjects(&result).len(), 1, "{dialect}: {source}");
                assert_eq!(result.source_targets.len(), 1);
                let subject = subjects(&result)[0];
                assert!(subject.matches_source(
                    &SourceImage::document(source),
                    result.resolved_input.as_ref().unwrap()
                ));
                assert_eq!(subject.name_input().bytes(), b"source");
            }
        }
    }

    #[test]
    fn original_child_path_visibility_survives_parent_command_moves() {
        // naming.interpreter.original-child-path-source-binding
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-path-source-binding.md
        // naming.interpreter.original-created-parent-command-move
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command-move.md
        // Native paths are measured independently; diagnostics and source edges
        // below remain conditional on successful original source operations.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [
                "interp create -safe s; rename s moved; interp eval s {source a.tcl}",
                "interp create -safe s; rename s first; rename first second; interp eval s {source a.tcl}",
                "interp create -safe s; namespace eval N {}; rename s ::N::held; interp eval s {source a.tcl}",
            ] {
                let result = Analyser::new().analyse(source, dialect);
                assert_eq!(subjects(&result).len(), 1, "{dialect}: {source}");
                assert_eq!(result.source_targets.len(), 1);
                assert!(
                    !result
                        .diagnostics
                        .iter()
                        .any(|diag| diag.code == tcl_core_types::DiagCode::W140)
                );
            }
            let exposed = "interp create -safe s; rename s moved; interp expose s source; interp eval s {source a.tcl}";
            assert!(subjects(&Analyser::new().analyse(exposed, dialect)).is_empty());
            let deleted =
                "interp create -safe s; rename s moved; interp delete s; moved eval {source a.tcl}";
            assert!(subjects(&Analyser::new().analyse(deleted, dialect)).is_empty());
        }
        let unknown =
            "interp create -safe s; rename s moved; mystery; interp eval s {source a.tcl}";
        assert!(subjects(&Analyser::new().analyse(unknown, "tcl8.6")).is_empty());
    }

    #[test]
    fn original_child_visibility_is_scoped_conditional_and_retains_possible_source_edges() {
        // Implementation contract: naming.interpreter.original-source-visibility-advice
        // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
        // Empirical C source controls: naming.interpreter.safe-visibility-source-controls
        // docs/design/analysis/name-resolution-proofs/interpreter-safe-visibility-source-controls.md
        // These assertions test source projections. CLI observations do not
        // supply a hidden allocation, entered interpreter or selected call.
        let source =
            "interp create -safe s\ninterp eval s {source a.tcl; ::source b.tcl; ::::source c.tcl}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            let subjects = subjects(&result);
            assert_eq!(subjects.len(), 3, "{dialect}: {:?}", result.diagnostics);
            assert_eq!(result.source_targets.len(), 3, "{dialect}");
            let input = result.resolved_input.as_ref().unwrap();
            for target in &result.source_targets {
                let original = target.original_interpreter_source_load.as_ref().unwrap();
                assert!(original.matches_source(&SourceImage::document(source), input));
                assert_eq!(original.original_words().len(), 2);
                assert_eq!(original.path_word(), &original.original_words()[1]);
                assert!(
                    !original.matches_source(&SourceImage::document("source foreign.tcl"), input)
                );
            }
            for subject in subjects {
                assert!(subject.matches_source(&SourceImage::document(source), input));
                assert!(subject.slot().namespace.is_root());
                assert_eq!(subject.slot().simple.as_bytes(), b"source");
                assert_eq!(subject.obligations().len(), 6);
                assert_eq!(
                    subject.original_words().first().unwrap().span(),
                    subject.name_input().original_word_key().unwrap().span()
                );
            }
        }
        let jim = Analyser::new().analyse(source, "jimtcl");
        assert!(
            subjects(&jim).is_empty(),
            "Jim does not inherit the C child surface"
        );
    }

    #[test]
    fn original_child_visibility_preserves_literal_colon_and_independent_allocations() {
        // Implementation contract: naming.interpreter.original-source-visibility-advice
        // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
        // Exact colon and hidden/visible allocation CLI controls are recorded
        // in interpreter-safe-visibility-source-controls.md. This source ledger
        // remains conditional and never replaces an actual hidden-slot gate.
        let source = "interp create -safe s\ninterp eval s {proc :source {} {return literal}; namespace eval :ns {proc source {} {return scoped}}; :source; :ns::source; source a.tcl}\ninterp expose s source :visible\ninterp eval s {:visible a.tcl; source b.tcl}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            assert_eq!(
                subjects(&result).len(),
                2,
                "{dialect}: {:?}",
                result.diagnostics
            );
            assert!(
                subjects(&result)
                    .iter()
                    .all(|subject| subject.name_input().bytes() == b"source")
            );
        }
        let source = "interp create -safe s\ninterp eval s {proc source {} {return visible}; source}\ninterp expose s source :original\ninterp eval s {source; :original a.tcl; rename source {}; source b.tcl}";
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(subjects(&result).len(), 1, "{:?}", result.diagnostics);
        assert_eq!(subjects(&result)[0].name_input().bytes(), b"source");
    }

    #[test]
    fn original_child_lambda_visibility_uses_its_own_literal_namespace() {
        // Implementation contract: naming.interpreter.original-source-visibility-advice
        // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
        // Independent original source controls: naming.procedure.original-root-and-colon-holder-publication
        // docs/design/analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md
        // This checks a retained original literal lambda and source lookup
        // premises, never an entered lambda or runtime namespace allocation.
        let source = "interp create -safe s\ninterp eval s {namespace eval :ns {proc source {} {return local}; apply {{} {source a.tcl} :ns}; apply {{} {source b.tcl}}}}";
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            let visibility = subjects(&result);
            // C85 publishes the procedure in reparsed ns, matching Apply's
            // independently ::-prefixed :ns. C86+ publishes it in :ns while
            // Apply still selects ns, so both original source calls may miss.
            let expected = if dialect == "tcl8.5" {
                vec!["source b.tcl"]
            } else {
                vec!["source a.tcl", "source b.tcl"]
            };
            let actual = visibility
                .iter()
                .map(|subject| {
                    subject
                        .name_input()
                        .original_word_key()
                        .unwrap()
                        .span()
                        .start()
                })
                .collect::<Vec<_>>();
            let expected = expected
                .iter()
                .map(|text| u32::try_from(source.find(text).unwrap()).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{dialect}");
        }
        let shadow = "interp create -safe s\ninterp eval s {proc apply {args} {return}; apply {{} {source data.tcl}}}";
        let result = Analyser::new().analyse(shadow, "tcl8.6");
        assert!(
            subjects(&result).is_empty(),
            "a source-defined callable supplies no Apply schema"
        );
    }

    #[test]
    fn original_child_visibility_rejects_foreign_source_configuration_and_generation() {
        // Implementation contract: naming.interpreter.original-source-visibility-advice
        // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
        let source = "interp create -safe s\ninterp eval s {source a.tcl}";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let original = subjects(&result)[0];
        let input = result.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        assert!(original.matches_source(&image, &input.clone()));
        assert!(
            !original.matches_source(&SourceImage::document(&(source.to_owned() + "\n")), input)
        );
        let mut foreign = input.clone();
        foreign.context = Arc::new(
            input
                .context
                .with_command_store(Arc::clone(input.context.commands())),
        );
        assert!(!original.matches_source(&image, &foreign));
        let mut config = input.clone();
        config.config.leading_bom = if config.config.leading_bom == tcl_lexer::LeadingBom::Skip {
            tcl_lexer::LeadingBom::Content
        } else {
            tcl_lexer::LeadingBom::Skip
        };
        assert!(!original.matches_source(&image, &config));
        let native_image = SourceImage::native(source.as_bytes());
        assert!(!original.matches_source(&native_image, input));
    }
}
