// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Native source headers retain their independent source body/schema.

use super::source_procedure::OriginalSourceProcedureBody;
use crate::command_binding::formal_topology::OriginalFormalTopology;
use crate::command_binding::{SourceCommandBindings, SourceNamespaceKey};
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceNameInput, SignatureSourceNameKey,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct OriginalNativeSourceDeclaration {
    analysis_owner: Arc<crate::command_binding::OriginalSourceAnalysisOwner>,
    input: crate::analyser::ResolvedAnalysisInput,
    tape: Arc<super::OriginalSourceTransitionAdviceTape>,
    procedure: Arc<OriginalSourceProcedureBody>,
    name: SignatureSourceNameInput,
    formals: OriginalFormalTopology,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    namespace: SourceNamespaceKey,
    scope: SignatureNamespaceScope,
}
impl PartialEq for OriginalNativeSourceDeclaration {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.analysis_owner, &other.analysis_owner)
            && self.input == other.input
            && Arc::ptr_eq(&self.tape, &other.tape)
            && self.procedure == other.procedure
            && self.name == other.name
            && self.formals == other.formals
            && self.parameters == other.parameters
            && self.namespace == other.namespace
            && self.scope == other.scope
    }
}
impl Eq for OriginalNativeSourceDeclaration {}

impl OriginalNativeSourceDeclaration {
    pub(crate) fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        self.procedure.declaration.site()
    }
    pub(crate) fn name_input(&self) -> &SignatureSourceNameInput {
        &self.name
    }
    pub(crate) fn declaration_slot(&self) -> &tcl_core_types::ByteCommandSlot {
        &self.procedure.declaration_slot
    }
    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.namespace
    }
    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        std::str::from_utf8(self.formals.original_input().bytes()).ok()
    }
    pub(crate) fn body_span(&self) -> tcl_lexer::Span {
        self.procedure.body.content_span()
    }
    pub(crate) fn body_text(&self) -> Option<&str> {
        self.site()
            .source
            .source_image()
            .try_text()
            .ok()?
            .get(self.body_span().as_range())
    }
    pub(crate) fn label(&self) -> Option<String> {
        std::str::from_utf8(&tcl_syntax::naming::native_command_full_name_bytes(
            self.declaration_slot(),
        ))
        .ok()
        .map(str::to_owned)
    }
    pub(crate) fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        input == &self.input
            && !input.has_logical_source_name_context()
            && !input.has_hosted_source_name_context()
            && self
                .procedure
                .declaration
                .matches_source(image, input.lexer_config())
            && self
                .procedure
                .declaration
                .matches_context(input.borrowed_context_registry())
            && self
                .procedure
                .body
                .matches_source(image, input.lexer_config())
            && self
                .procedure
                .body
                .matches_context(input.borrowed_context_registry())
            && self.formals.original_input().policy() == self.name.policy()
            && self.procedure.declaration.dialect().parameter_grammar()
                == Some(self.formals.parameter_grammar())
            && self.procedure.declaration_slot == self.procedure.slot
            && self.namespace.display().as_deref() == self.scope.display().as_deref()
    }
}
impl SourceCommandBindings {
    pub(crate) fn original_native_source_declarations(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<Arc<OriginalNativeSourceDeclaration>>> {
        if input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || self.source_metadata_owner().source_analysis_input() != Some(input)
        {
            return None;
        }
        let analysis_owner = self.original_analysis_owner()?;
        analysis_owner.matches(self, input).then_some(())?;
        let seed = self.original_source_transition_seed()?;
        let tape = Arc::new(seed.prepare_for(
            input.borrowed_context_registry(),
            super::OriginalSourceCataloguePurpose::CompletePrefix,
        )?);
        let inventory = tape.procedure_publications()?;
        if !inventory.matches_source_context(
            self.source_origin()?.source_image(),
            input.lexer_config(),
            input.borrowed_context_registry(),
        ) {
            return None;
        }
        let image = self.source_origin()?.source_image();
        let cards = inventory
            .original_declarations()
            .iter()
            .filter_map(|procedure| {
                let name = procedure.name.native_input()?;
                let roles = procedure.declaration.roles()?;
                let mut parameters = roles
                    .iter()
                    .filter(|(_, role)| *role == tcl_registry::ArgRole::ParamList);
                let argument = parameters.next()?.0;
                if parameters.next().is_some() {
                    return None;
                }
                let word = &procedure.declaration.arguments().get(argument)?.original;
                let producer = SignatureSourceNameKey::from_original_native_word(
                    word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
                    name.policy(),
                )?;
                let formals = OriginalFormalTopology::from_original_key(
                    producer,
                    procedure.declaration.dialect(),
                )?;
                let scope = match name.policy().recipe() {
                    tcl_syntax::naming::NativeNameProtocol::C(_) => {
                        SignatureNamespaceScope::C(procedure.declaration_slot.namespace.clone())
                    }
                    recipe @ tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                        SignatureNamespaceScope::Jim(
                            recipe
                                .jim_procedure_namespace(
                                    procedure.declaration_slot.simple.as_bytes(),
                                )
                                .ok()?
                                .into(),
                        )
                    }
                };
                let namespace = SourceNamespaceKey::authored(scope.display()?);
                let card = OriginalNativeSourceDeclaration {
                    analysis_owner: Arc::clone(analysis_owner),
                    input: input.clone(),
                    tape: Arc::clone(&tape),
                    procedure: Arc::clone(procedure),
                    name: name.clone(),
                    parameters: formals.advisory_parameters(),
                    formals,
                    namespace,
                    scope,
                };
                card.matches_source(image, input).then(|| Arc::new(card))
            })
            .collect();
        Some(cards)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalNativeSourceMayCommand {
    pub(super) original: Arc<[tcl_lexer::NativeWord]>,
    pub(super) head: super::SourceAdviceNameInput,
    pub(super) body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
}

/// Whole original declaration/tape used only for readonly child consumers.
#[derive(Debug)]
pub(crate) struct OriginalNativeSourceBodyChildren {
    declaration: Arc<OriginalNativeSourceDeclaration>,
    input: crate::analyser::ResolvedAnalysisInput,
    tape: Arc<super::OriginalSourceTransitionAdviceTape>,
    commands: std::collections::BTreeMap<u32, Arc<[tcl_lexer::NativeWord]>>,
}
impl PartialEq for OriginalNativeSourceBodyChildren {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.tape, &other.tape)
            && self.declaration == other.declaration
            && self.input == other.input
            && self.commands == other.commands
    }
}
impl Eq for OriginalNativeSourceBodyChildren {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalNativeSourceCommand {
    owner: Arc<OriginalNativeSourceBodyChildren>,
    original: Arc<[tcl_lexer::NativeWord]>,
}
/// A positive original command occurrence under an authentic procedure body.
/// This retains only source phase/caller membership, never dispatch or census closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalNativeProcedureBodyCommand {
    owner: Arc<OriginalNativeSourceBodyChildren>,
    original: Arc<[tcl_lexer::NativeWord]>,
}
impl OriginalNativeProcedureBodyCommand {
    /// Complete original configured words retained by the shared body issuer.
    #[must_use]
    pub fn original_words(&self) -> &[tcl_lexer::NativeWord] {
        &self.original
    }
    /// Literal source head data, independent of actual lookup or selected handler.
    #[must_use]
    pub fn static_head(&self) -> Option<Vec<u8>> {
        let head = self.original.first()?;
        (!head.group().expand).then_some(())?;
        tcl_syntax::word_rules::original_static_word_unicode_value(head)
    }
    /// Current declaration from this exact retained source body allocation.
    #[must_use]
    pub fn caller_declaration<'a>(
        &self,
        analysis: &'a crate::analyser::AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        (analysis.resolved_input.as_ref() == Some(&self.owner.input)).then_some(())?;
        let realm = analysis.retained_command_realm()?;
        let current = realm.source_bindings_ref().original_analysis_owner()?;
        Arc::ptr_eq(current, &self.owner.declaration.analysis_owner).then_some(())?;
        self.owner
            .declaration
            .analysis_owner
            .matches(realm.source_bindings_ref(), &self.owner.input)
            .then_some(())?;
        self.owner.declaration.procedure.source_procedure(analysis)
    }
}
impl SourceCommandBindings {
    pub(crate) fn original_native_procedure_body_commands(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<OriginalNativeProcedureBodyCommand>> {
        let mut commands = Vec::new();
        for declaration in self.original_native_source_declarations(input)? {
            let Some(owner) = declaration.children(self, input) else {
                continue;
            };
            for original in owner.commands.values() {
                commands.push(OriginalNativeProcedureBodyCommand {
                    owner: Arc::clone(&owner),
                    original: Arc::clone(original),
                });
            }
        }
        Some(commands)
    }
}

impl OriginalNativeSourceDeclaration {
    pub(crate) fn children(
        self: &Arc<Self>,
        bindings: &SourceCommandBindings,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Arc<OriginalNativeSourceBodyChildren>> {
        if bindings
            .original_analysis_owner()
            .is_none_or(|owner| !Arc::ptr_eq(owner, &self.analysis_owner))
            || !self.analysis_owner.matches(bindings, input)
            || bindings.source_metadata_owner().source_analysis_input() != Some(input)
            || !Arc::ptr_eq(bindings.source_origin()?, &self.site().source)
            || !self.matches_source(bindings.source_origin()?.source_image(), input)
        {
            return None;
        }
        let tape = Arc::clone(&self.tape);
        let plan = tcl_lexer::native_script_words_in(
            self.site().source.source_image().clone(),
            self.body_span(),
            input.lexer_config(),
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        let mut commands = std::collections::BTreeMap::new();
        for command in plan.commands {
            let words = command.words.as_slice();
            commands.insert(words.first()?.span().start(), Arc::from(words));
        }
        for advice in tape
            .advice
            .values()
            .chain(tape.body_catalogue.values().flatten())
        {
            if advice.original_procedure_source_body() != Some(self.procedure.body.as_ref())
                || !advice.matches_source(self.site().source.source_image(), input.lexer_config())
                || !advice.matches_context(input.borrowed_context_registry())
            {
                continue;
            }
            let original = Arc::<[tcl_lexer::NativeWord]>::from(advice.original_words());
            let offset = advice.site().offset;
            if commands
                .get(&offset)
                .is_some_and(|words| words != &original)
            {
                return None;
            }
            commands.insert(offset, original);
        }
        for receipt in tape.native_procedure_may_commands.values().flatten() {
            if receipt.body.as_ref() != self.procedure.body.as_ref() {
                continue;
            }
            let original = &receipt.original;
            let offset = original.first()?.span().start();
            if original.iter().any(|word| {
                word.image() != self.site().source.source_image()
                    || word.config() != input.lexer_config()
                    || word.group().expand
                    || word.span().start() < self.body_span().start()
                    || word.span().end() > self.body_span().end()
            }) || commands.get(&offset).is_some_and(|held| held != original)
            {
                return None;
            }
            commands.insert(offset, Arc::clone(original));
        }
        Some(Arc::new(OriginalNativeSourceBodyChildren {
            declaration: Arc::clone(self),
            input: input.clone(),
            tape,
            commands,
        }))
    }
}
impl OriginalNativeSourceBodyChildren {
    pub(crate) fn stamp_tokens(
        self: &Arc<Self>,
        tokens: &mut crate::ir::CommandTokens,
    ) -> Option<()> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let original = self.commands.get(&offset)?;
        if crate::registry_invocation::original_native_compiler_words(
            self.declaration.site().source.source_image(),
            tokens.words(),
            offset,
            self.input.lexer_config(),
        )?
        .as_slice()
            != original.as_ref()
        {
            return None;
        }
        let attachment = tokens
            .source_binding
            .as_ref()?
            .original_analysis_owner
            .as_ref()?;
        if !Arc::ptr_eq(attachment, &self.declaration.analysis_owner) {
            return None;
        }
        for (&child_offset, child) in &self.commands {
            if child_offset == offset
                || !tokens.words().iter().any(|word| {
                    word.source().span.start() <= child_offset
                        && child
                            .last()
                            .is_some_and(|last| last.span().end() <= word.source().span.end())
                })
            {
                continue;
            }
            let Some(Some(receipt)) = self.tape.native_procedure_may_commands.get(&child_offset)
            else {
                continue;
            };
            if receipt.original.as_ref() != child.as_ref()
                || receipt.body.as_ref() != self.declaration.procedure.body.as_ref()
            {
                return None;
            }
            let position = tokens
                .nested_bindings
                .iter()
                .position(|(at, _)| *at == child_offset);
            let binding = if let Some(position) = position {
                &mut tokens.nested_bindings[position].1
            } else {
                let mut binding = crate::command_binding::SourceInvocationBinding::unknown();
                binding.original_analysis_owner = Some(Arc::clone(attachment));
                tokens.nested_bindings.push((child_offset, binding));
                &mut tokens.nested_bindings.last_mut()?.1
            };
            if binding
                .original_analysis_owner
                .as_ref()
                .is_none_or(|held| !Arc::ptr_eq(held, attachment))
            {
                return None;
            }
            let command = OriginalNativeSourceCommand {
                owner: Arc::clone(self),
                original: Arc::clone(child),
            };
            if binding
                .original_native_source_command
                .as_ref()
                .is_some_and(|held| held.as_ref() != &command)
                || binding.invocation_site().is_some_and(|site| {
                    site.offset != child_offset
                        || !Arc::ptr_eq(&site.source, &self.declaration.site().source)
                })
            {
                return None;
            }
            if let Some((_, recorded)) = binding.original_recorded_command()
                && crate::registry_invocation::original_native_compiler_words(
                    self.declaration.site().source.source_image(),
                    recorded.words(),
                    child_offset,
                    self.input.lexer_config(),
                )
                .is_none_or(|words| words.as_slice() != child.as_ref())
            {
                return None;
            }
            binding.original_native_source_command = Some(Arc::new(command));
        }
        tokens
            .source_binding
            .as_mut()?
            .original_native_source_command = Some(Arc::new(OriginalNativeSourceCommand {
            owner: Arc::clone(self),
            original: Arc::clone(original),
        }));
        Some(())
    }
}
impl OriginalNativeSourceCommand {
    pub(crate) fn matches_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        input == &self.owner.input
            && registry.snapshot().semantic_key()
                == input
                    .borrowed_context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
            && tokens.synthetic.is_none()
            && tokens.original_words_match_argv_text()
            && self
                .owner
                .declaration
                .matches_source(self.owner.declaration.site().source.source_image(), input)
            && self.original.first().is_some_and(|head| {
                crate::registry_invocation::original_native_compiler_words(
                    self.owner.declaration.site().source.source_image(),
                    tokens.words(),
                    head.span().start(),
                    input.lexer_config(),
                )
                .is_some_and(|words| words.as_slice() == self.original.as_ref())
            })
    }
    fn syntax_offset(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<u32> {
        if !Arc::ptr_eq(&self.owner.tape, &self.owner.declaration.tape)
            || !self.matches_tokens(tokens, input, registry)
            || tokens
                .source_binding
                .as_ref()?
                .original_analysis_owner
                .as_ref()
                .is_none_or(|held| !Arc::ptr_eq(held, &self.owner.declaration.analysis_owner))
        {
            return None;
        }
        let offset = self.original.first()?.span().start();
        (self.owner.commands.get(&offset)? == &self.original).then_some(offset)
    }
    pub(crate) fn matches_module(
        &self,
        tokens: &crate::ir::CommandTokens,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let Some(input) = module.source_metadata_input.as_ref() else {
            return false;
        };
        self.matches_tokens(tokens, input, registry)
            && module
                .original_native_declaration_body_units
                .iter()
                .any(|(id, card)| {
                    module.procedure_ids.owns(*id)
                        && card.owns_source_declaration(&self.owner.declaration)
                        && module
                            .body_units
                            .get(id)
                            .is_some_and(|procedure| card.matches_procedure(module, procedure))
                })
    }
    fn may_matches(
        &self,
        tokens: &crate::ir::CommandTokens,
        word: &str,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        if !self.matches_module(tokens, module, registry) {
            return false;
        }
        let Some(head) = self.original.first() else {
            return false;
        };
        let Some(key) = SignatureSourceNameKey::from_original_native_word(
            head,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.owner.input.lexer_config()),
            self.owner.declaration.name_input().policy(),
        ) else {
            return false;
        };
        if SignatureSourceNameInput::OriginalWord(key).bytes() != word.as_bytes() {
            return false;
        }
        let offset = head.span().start();
        let Some(Some(may)) = self.owner.tape.native_procedure_may_commands.get(&offset) else {
            return false;
        };
        if may.original.as_ref() != self.original.as_ref()
            || may.body.as_ref() != self.owner.declaration.procedure.body.as_ref()
            || may
                .head
                .native_input()
                .is_none_or(|input| input.bytes() != word.as_bytes())
        {
            return false;
        }
        if self.owner.tape.blocks_registry_source(offset) {
            return false;
        }
        let advice = if let Some(advice) = self.owner.tape.body_catalogue(offset) {
            Some(advice)
        } else {
            match self.owner.tape.lookup(offset) {
                super::OriginalSourceTransitionAdviceLookup::Advice(advice) => Some(advice),
                super::OriginalSourceTransitionAdviceLookup::Refused
                | super::OriginalSourceTransitionAdviceLookup::Unrepresented => None,
            }
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        if binding.lookup_state.is_some() {
            if binding
                .original_invocation_metadata_for_module(tokens, module, registry)
                .is_none()
            {
                return false;
            }
            let Some(advice) = advice else {
                return false;
            };
            if advice.original_words() != self.original.as_ref()
                || !crate::command_binding::source_targets_match_advice(
                    self.owner.input.borrowed_context_registry(),
                    binding,
                    advice,
                )
            {
                return false;
            }
        }
        true
    }
    pub(crate) fn words(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        if !self.matches_tokens(tokens, input, registry) {
            return None;
        }
        let offset = self.original.first()?.span().start();
        if self.owner.tape.blocks_registry_source(offset) {
            return None;
        }
        let advice = if let Some(advice) = self.owner.tape.body_catalogue(offset) {
            advice
        } else {
            match self.owner.tape.lookup(offset) {
                super::OriginalSourceTransitionAdviceLookup::Advice(advice) => advice,
                super::OriginalSourceTransitionAdviceLookup::Refused
                | super::OriginalSourceTransitionAdviceLookup::Unrepresented => return None,
            }
        };
        if advice.original_words() != self.original.as_ref()
            || advice.original_procedure_source_body()
                != Some(self.owner.declaration.procedure.body.as_ref())
            || tokens.source_binding.as_ref().is_none_or(|binding| {
                binding.lookup_state.is_some()
                    && !crate::command_binding::source_targets_match_advice(
                        input.borrowed_context_registry(),
                        binding,
                        advice,
                    )
            })
        {
            return None;
        }
        crate::registry_invocation::source_structure::source_transition_words_from_advice(
            self.owner
                .declaration
                .site()
                .source
                .source_image()
                .try_text()
                .ok()?,
            input.lexer_config(),
            input.borrowed_context_registry(),
            advice.clone(),
        )
    }
}
impl crate::command_binding::SourceInvocationBinding {
    pub(crate) fn original_native_source_owner_for_body(
        &self,
        tokens: &crate::ir::CommandTokens,
        module: &crate::ir::Module,
        body: crate::ir::ProcedureId,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        if tokens.source_binding.as_ref() != Some(self) {
            return false;
        }
        let Some(command) = self.original_native_source_command.as_ref() else {
            return false;
        };
        command.matches_module(tokens, module, registry)
            && module
                .original_native_declaration_body_units
                .get(&body)
                .is_some_and(|card| card.owns_source_declaration(&command.owner.declaration))
    }
    /// Exact held child syntax, independently of handler/schema applicability.
    /// Unknown handlers remain opaque; no invocation or executed effect is issued.
    pub(crate) fn original_native_source_syntax_offset(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<u32> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let command = self.original_native_source_command.as_ref()?;
        let held = &command.owner.input;
        if input != held && input != &held.for_nested_source() {
            return None;
        }
        command.syntax_offset(tokens, held, registry)
    }
    /// Original held source grammar only; no external current Module is selected.
    pub(crate) fn original_native_source_input_for_name_roles(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let command = self.original_native_source_command.as_ref()?;
        command.words(tokens, &command.owner.input, registry)?;
        Some(&command.owner.input)
    }
    pub(crate) fn original_native_source_words(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        self.original_native_source_command
            .as_ref()?
            .words(tokens, input, registry)
    }
    pub(crate) fn has_original_native_source_command(&self) -> bool {
        self.original_native_source_command.is_some()
    }
    pub(crate) fn original_native_source_may_owner(
        &self,
        tokens: &crate::ir::CommandTokens,
        word: &str,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        tokens.source_binding.as_ref() == Some(self)
            && self
                .original_native_source_command
                .as_ref()
                .is_some_and(|command| command.may_matches(tokens, word, module, registry))
    }
}

#[cfg(test)]
mod may_terminal_tests {
    use super::*;

    fn fixture() -> (crate::ir::Module, crate::analyser::ResolvedAnalysisInput) {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        // Unentered native declarations need an explicit unknown entry.
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_resolved_analysis_input(input.clone())
                .with_explicit_unknown_entry(),
            "puts ROOT\nproc f {} {not-declared VALUE}\n",
        );
        (module, input)
    }

    #[test]
    fn native_source_may_preserves_terminal_tape_and_reached_owner_refusals() {
        let (module, input) = fixture();
        let registry = input.borrowed_context_registry().commands();
        let (&id, _) = module
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let tokens = module.body_units.get(&id).unwrap().body.statements[0]
            .tokens()
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.original_native_source_may_owner(
            tokens,
            "not-declared",
            &module,
            registry
        ));
        let command = binding.original_native_source_command.as_ref().unwrap();
        let offset = command.original[0].span().start();
        for barrier in [false, true] {
            let mut tape = super::super::OriginalSourceTransitionAdviceTape::default();
            if barrier {
                tape.native_procedure_may_commands.insert(
                    offset,
                    command
                        .owner
                        .tape
                        .native_procedure_may_commands
                        .get(&offset)
                        .unwrap()
                        .clone(),
                );
                tape.registry_barriers.insert(offset);
            } else {
                tape.native_procedure_may_commands.insert(offset, None);
            }
            let owner = Arc::new(OriginalNativeSourceBodyChildren {
                declaration: Arc::clone(&command.owner.declaration),
                input: input.clone(),
                tape: Arc::new(tape),
                commands: command.owner.commands.clone(),
            });
            let mut changed = tokens.clone();
            changed
                .source_binding
                .as_mut()
                .unwrap()
                .original_native_source_command = Some(Arc::new(OriginalNativeSourceCommand {
                owner,
                original: Arc::clone(&command.original),
            }));
            assert!(
                !changed
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_native_source_may_owner(&changed, "not-declared", &module, registry)
            );
            assert!(
                changed
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_native_source_syntax_offset(&changed, &input, registry)
                    .is_none()
            );
            assert!(crate::registry_invocation::original_procedure_may_dependencies_for_word_for_module(
                &changed, "not-declared", &module, registry).is_none());
        }
        let reached = module.top_level.statements[0]
            .tokens()
            .unwrap()
            .source_binding
            .as_ref()
            .unwrap();
        assert!(reached.lookup_state.is_some());
        let mut changed = tokens.clone();
        changed.source_binding.as_mut().unwrap().lookup_state = reached.lookup_state.clone();
        changed.source_binding.as_mut().unwrap().targets = reached.targets.clone();
        changed.source_binding.as_mut().unwrap().unknown = reached.unknown;
        assert!(
            !changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_native_source_may_owner(&changed, "not-declared", &module, registry)
        );
    }
}
