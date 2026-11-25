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

//! Source-facing projection of the shared command-binding owner. This module
//! adapts rich positioned results to legacy head-only consumers; it never
//! interprets an import, alias, rename, namespace path, or shadow itself.

use std::borrow::Cow;
use std::collections::BTreeMap;

mod declared_source;
mod original_bindings;
mod original_tokens;
pub(crate) use declared_source::document_realm_with_declared_contracts;
mod source_transition_advice;

use crate::command_binding::{SourceCommandBindings, SourceInvocationBinding};
use tcl_registry::CommandRegistry;
use tcl_registry::model::{BindingKnowledge, BindingTarget, ResolvedContext, SpecKey};

/// What a command head resolves to at one point in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealmBinding<'a> {
    /// The head names this registry command — the head text itself when no
    /// fact applies, or the effective target of a proven import / alias /
    /// rename.
    Command(&'a str),
    /// The head's binding was provably taken over by something the registry
    /// does not model: a `rename` moved the built-in away, a user `proc`
    /// redefined it, or an alias bound it to an unresolvable target.  No
    /// registry grammar applies.
    Rebound,
}

/// A binding fact without borrowing the head spelling supplied by the caller.
///
/// This is the cross-crate form of [`CommandBindingRealm`] lookup: a consumer can
/// preserve its own written text when no fact applies, while a resolved target
/// borrows from the map. That separation avoids making a syntax/registry
/// caller manufacture compiler-owned copies merely to compare a command head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealmBindingFact<'a> {
    /// No applicable fact; the written spelling remains the identity.
    Unchanged,
    /// A proven alias/import/rename target.
    Command(&'a str),
    /// The spelling was provably rebound away from registry semantics.
    Rebound,
}

impl<'a> RealmBinding<'a> {
    /// The name to resolve registry grammar against.
    ///
    /// [`Self::Rebound`] answers with the empty string, which
    /// [`CommandRegistry::get`] never resolves — so every registry query a
    /// consumer already makes (`arg_indices_for_role`, `format_string_args`,
    /// `handle_binding`, …) answers "not a known command" without that
    /// consumer testing the variant.
    #[must_use]
    pub fn spec_name(self) -> &'a str {
        match self {
            Self::Command(name) => name,
            Self::Rebound => "",
        }
    }

    /// Whether the head's registry binding was provably taken over.
    #[must_use]
    pub fn is_rebound(self) -> bool {
        matches!(self, Self::Rebound)
    }
}

/// A command head in its two forms — the spelling the source wrote, and the
/// registry name that spelling effectively resolves to.
///
/// The two are the same for almost every head, and differ once the document
/// rebinds a name.  Which of the two a test must read is a real distinction,
/// not a convenience:
///
/// * a **global command** lookup reads [`Self::resolved`], so `::snit::type`,
///   a proven alias of it, and the bare spelling all answer alike (and a
///   rebound spelling answers nothing);
/// * a **lexical** test reads [`Self::written`] — a class-body member
///   sub-keyword (`method`, `constructor`) or a `$var` head is not a command
///   binding at all, so a top-level `rename method …` says nothing about the
///   word inside an `oo::define`.
#[derive(Debug, Clone, Copy)]
pub struct HeadWords<'a> {
    /// The head exactly as the source spells it.
    pub written: &'a str,
    /// The registry name the spelling resolves to — empty when the head was
    /// provably rebound, which every registry query then answers "unknown" for.
    pub resolved: &'a str,
}

impl<'a> HeadWords<'a> {
    /// A head with nothing proven about it: written and resolved alike.
    #[must_use]
    pub fn plain(written: &'a str) -> Self {
        Self {
            written,
            resolved: written,
        }
    }
}

/// Legacy head projection over the same command-table interpretation used by
/// lowering and executable effect consumers.
#[derive(Debug, Clone, Default)]
pub struct CommandBindingRealm {
    bindings: SourceCommandBindings,
    source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    specs: BTreeMap<String, SpecKey>,
    original_tokens: original_tokens::OriginalCommandTokenCache,
    original_bindings: original_bindings::OriginalInvocationBindingCache,
    source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache,
    source_call_advice: source_transition_advice::OriginalSourceTransitionAdviceCache,
    query_memo: crate::command_binding::original_name_value::OriginalRealmQueryMemo,
    stamped: StampedTokenMemo,
    math_functions: crate::command_binding::OriginalMathFunctionWordMemo,
}

/// [`CommandBindingRealm::stamp_original_tokens`] is a pure function of the
/// realm and the unstamped tokens, and the analyser asks it for the same
/// command many times per walk. A copy starts empty, so a realm rebuilt by a
/// `with_*` step never answers from its predecessor's interpretation.
#[derive(Debug, Default)]
struct StampedTokenMemo(
    std::sync::Mutex<std::collections::HashMap<crate::ir::CommandTokens, crate::ir::CommandTokens>>,
);

impl Clone for StampedTokenMemo {
    fn clone(&self) -> Self {
        Self::default()
    }
}

const MAX_STAMPED_COMMANDS: usize = 65_536;

// Derived preparation does not change the immutable retained interpretation.
impl PartialEq for CommandBindingRealm {
    fn eq(&self, other: &Self) -> bool {
        self.bindings == other.bindings
            && self.specs == other.specs
            && self.source_input == other.source_input
    }
}

impl Eq for CommandBindingRealm {}

static EMPTY_REALM: std::sync::LazyLock<CommandBindingRealm> =
    std::sync::LazyLock::new(CommandBindingRealm::default);

impl CommandBindingRealm {
    pub(crate) fn with_analyser_source_settings(
        mut self,
        settings: std::sync::Arc<crate::analyser::source_settings::OriginalAnalyserSourceSettings>,
    ) -> Self {
        self.bindings.retain_analyser_source_settings(settings);
        self.stamped = StampedTokenMemo::default();
        self
    }

    /// Seal the independently supplied editing inputs with this new source
    /// interpretation. Changing a public result projection cannot replace them.
    pub(crate) fn with_resolved_analysis_input(
        mut self,
        input: crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        self.source_input = Some(input);
        self.stamped = StampedTokenMemo::default();
        self
    }

    /// Exact original profiles, grammar, hosted policy and immutable context
    /// generation. This is input correspondence, not a lookup or execution.
    /// Source bytes require independent current-image validation.
    #[must_use]
    pub fn matches_resolved_analysis_input(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        self.source_input.as_ref() == Some(input)
    }
    /// The complete input this realm's source interpretation was sealed with.
    pub(crate) fn resolved_source_input(&self) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.source_input.as_ref()
    }

    /// Borrow the independently retained complete metadata generation.
    /// This does not authenticate source bytes or confer entered-frame facts.
    pub(crate) fn source_metadata_context(
        &self,
    ) -> Option<std::sync::Arc<tcl_registry::model::ContextRegistry>> {
        Some(self.source_input.as_ref()?.context_registry())
    }

    /// Retain the same document execution proof when lowering a positioned
    /// body for a secondary analysis. State snapshots share their allocation.
    pub(crate) fn source_bindings(&self) -> SourceCommandBindings {
        self.bindings.clone()
    }

    /// Original callee caller-alias arguments for one invocation, sharing the
    /// invocation-independent callee survey across this realm's queries.
    pub(crate) fn declaration_caller_alias_arguments(
        &self,
        invocation: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<Vec<String>> {
        self.bindings.declaration_caller_alias_arguments_with(
            invocation,
            registry,
            self.bindings.source_metadata_owner(),
            Some(&self.query_memo),
        )
    }

    /// [`SourceCommandBindings::original_math_functions_in_source`], sharing
    /// each word's occurrences across this realm's regions.
    pub(crate) fn original_math_functions_in_source(
        &self,
        registry: &CommandRegistry,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        region: tcl_lexer::Span,
    ) -> Vec<crate::command_binding::OriginalMathFunctionOccurrence> {
        self.bindings.original_math_functions_in_source_with(
            registry,
            image,
            config,
            region,
            Some(&self.math_functions),
        )
    }

    /// Borrow the retained inventory for syntax navigation in the same source
    /// root. Reconstructed/entered sources must select their own origin first.
    #[doc(hidden)]
    pub fn source_bindings_ref(&self) -> &SourceCommandBindings {
        &self.bindings
    }

    /// Complete original source bytes and channel retained by this realm.
    /// Readonly correspondence only: this supplies no entered frame, command
    /// selection, normal completion or native execution capability.
    #[must_use]
    pub fn original_source_image(&self) -> Option<&tcl_lexer::SourceImage> {
        self.source_bindings_ref()
            .source_origin()
            .map(|origin| origin.source_image())
    }

    /// Complete retained original source/channel and full lexer configuration.
    /// Correspondence grants no lookup, entered frame, evaluation or edit.
    #[must_use]
    pub fn matches_original_source_image(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.bindings.matches_original_source_image(image, config)
    }

    /// Logical authoring metadata from unanimous original pre-operand rows.
    /// The retained full editing input, source image and configuration must
    /// agree. Complete static original argv and unchanged authored table cells
    /// are required; dynamic words, shadows, deletion and unknown mutations
    /// abstain. This supplies no Native lookup, dispatch, frame or completion.
    #[must_use]
    pub fn lexical_source_header_unpositioned(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
        name: &str,
    ) -> Option<String> {
        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        if !self.matches_resolved_analysis_input(input)
            || input.has_hosted_source_name_context()
            || tcl_registry::InvocationDialect::of_profile(input.analyser_profile())
                .authored_name_policy()
                .is_some()
        {
            return None;
        }
        let head =
            self.bindings
                .lexical_source_header_unpositioned(image, input.lexer_config(), name)?;
        let context = input.context_registry();
        context
            .context()
            .resolve_spec_in_realm(
                context.commands(),
                &head,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .map(|spec| spec.name.to_owned())
    }

    /// Positioned Logical source candidate over the complete original vector.
    /// The retained image, full input, configuration and before-argv table
    /// must agree. Unknown argument values stay unknown; this grants no
    /// preservation across their effects, Native lookup or runtime dispatch.
    #[must_use]
    pub fn lexical_source_header_for_words(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
        words: &[tcl_lexer::NativeWord],
    ) -> Option<String> {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        if !self.matches_resolved_analysis_input(input)
            || input.has_hosted_source_name_context()
            || tcl_registry::InvocationDialect::of_profile(input.analyser_profile())
                .authored_name_policy()
                .is_some()
        {
            return None;
        }
        let head =
            self.bindings
                .lexical_source_header_for_words(image, input.lexer_config(), words)?;
        let context = input.context_registry();
        context
            .context()
            .resolve_spec_in_realm(
                context.commands(),
                &head,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .map(|spec| spec.name.to_owned())
    }

    /// Exact dispatch proof from the shared source interpreter.
    #[must_use]
    pub fn invocation_at_source(&self, head: &str, offset: u32) -> SourceInvocationBinding {
        // The source query selects every dispatch point at `offset` whatever
        // the presentation head, so the realm's per-offset memo is the same
        // binding. Joining every activation's point is linear in the number
        // of activations; repeated queries must not repeat it.
        let _ = head;
        self.retained_original_invocation(offset).as_ref().clone()
    }

    /// Original static naming producer from this realm's existing original
    /// layout inventory. This supplies neither execution nor edit authority.
    #[must_use]
    pub fn original_source_name_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        rules: tcl_syntax::word_rules::WordValueRules,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        self.bindings
            .original_source_name_at_span(span, config, rules, policy)
    }

    /// Exact retained written value input, including computed readonly values.
    /// Complete source/configuration/argv ownership remains on the shared realm.
    #[must_use]
    pub fn original_written_name_input_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        self.bindings
            .original_written_name_input_at_span_with(span, config, Some(&self.query_memo))
    }

    /// Retained original input with exact caller-source and channel currency.
    #[must_use]
    pub fn original_written_name_input_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        self.bindings
            .original_written_name_input_at_span_in_source(image, span, config)
    }

    /// Original ordinal-zero whole head with its genuine retained source site.
    /// This is geometry only; it grants no Native handler or selected target.
    #[must_use]
    pub fn original_written_head_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(
        tcl_lexer::NativeWord,
        crate::command_binding::CommandAllocationSite,
    )> {
        self.bindings.original_written_head_at_span_in_source_with(
            image,
            span,
            config,
            Some(&self.query_memo),
        )
    }

    /// Genuine whole original word and parent head under exact source currency.
    /// This is source geometry only, independently of values and selection.
    #[must_use]
    pub fn original_written_word_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(tcl_lexer::NativeWord, u32)> {
        let written = self.bindings.original_written_word_at_span_in_source_with(
            image,
            span,
            config,
            Some(&self.query_memo),
        );
        if !self
            .source_input
            .as_ref()
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context)
        {
            return written;
        }
        let catalogue = self.original_logical_catalogue_word_at_span(image, span, config);
        match (written, catalogue) {
            (Some(written), Some(catalogue)) if written != catalogue => None,
            (Some(written), _) => Some(written),
            (None, catalogue) => catalogue,
        }
    }

    /// Contents the interpretation froze for the whole written word at `span`.
    /// Advice only; see the binding owner.
    #[must_use]
    pub fn frozen_written_word_value_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<String> {
        self.bindings
            .frozen_written_word_value_at_span_in_source_with(
                image,
                span,
                config,
                Some(&self.query_memo),
            )
    }

    /// Purpose-specific readonly C array operand root. Genuine whole-vector
    /// ownership and the unknown index remain independent of value/cell lookup.
    #[must_use]
    pub fn original_c_array_operand_root_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        self.bindings
            .original_c_array_operand_root_at_span_in_source_with(
                image,
                span,
                config,
                policy,
                Some(&self.query_memo),
            )
    }

    /// Original operand and independent caller namespace geometry retained by
    /// the shared complete-vector source owner. This forwarding supplies no
    /// namespace existence, argument role, import or execution authority.
    #[must_use]
    pub fn original_namespace_operand_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNamespaceOperand> {
        self.bindings
            .original_namespace_operand_at_span_in_source_with(
                image,
                span,
                config,
                Some(&self.query_memo),
            )
    }

    /// Attach retained original-source receipts to unchanged command tokens.
    /// Each consumer must select its own purpose projection afterwards;
    /// attachment itself grants neither dispatch nor body-entry authority.
    pub fn stamp_original_tokens(&self, tokens: &mut crate::ir::CommandTokens) {
        let retained = self
            .stamped
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&*tokens)
            .cloned();
        if let Some(stamped) = retained {
            *tokens = stamped;
            return;
        }
        let unstamped = tokens.clone();
        self.stamp_original_tokens_uncached(tokens);
        let mut entries = self
            .stamped
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if entries.len() < MAX_STAMPED_COMMANDS {
            entries.insert(unstamped, tokens.clone());
        }
    }

    fn stamp_original_tokens_uncached(&self, tokens: &mut crate::ir::CommandTokens) {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        self.bindings
            .stamp_original_tokens_with_bindings(tokens, |offset| {
                self.retained_original_invocation(offset).as_ref().clone()
            });
        if let Some(input) = self.source_input.as_ref()
            && input.has_logical_source_name_context()
            && tokens
                .source_binding
                .as_ref()
                .is_some_and(|binding| binding.lookup_state.is_none())
            && let Some(advice) =
                self.original_source_transition_advice(input.borrowed_context_registry(), tokens)
            && let Some(binding) = tokens.source_binding.as_mut()
        {
            binding.retain_original_logical_source_advice(advice);
        }
        self.retain_original_registered_instance_source(tokens);
        for (_, binding) in tokens.nested_bindings.iter() {
            if let Some(mut child) = binding.original_recorded_command_tokens() {
                child.source_binding = Some(binding.clone());
                self.retain_original_registered_instance_source(&child);
            }
        }
        if let Some(input) = self.source_input.as_ref()
            && input.has_logical_source_name_context()
        {
            for (_, binding) in tokens.nested_bindings_mut() {
                if binding.has_original_source_layout_owner() {
                    continue;
                }
                let Some(mut child) = binding.original_recorded_command_tokens() else {
                    continue;
                };
                child.source_binding = Some(binding.clone());
                if let Some(advice) = self
                    .original_source_transition_advice(input.borrowed_context_registry(), &child)
                {
                    binding.retain_original_logical_source_advice(advice);
                }
            }
        }
    }

    fn retain_original_registered_instance_source(&self, tokens: &crate::ir::CommandTokens) {
        let Some(input) = self.source_input.as_ref() else {
            return;
        };
        if !self.matches_resolved_analysis_input(input) {
            return;
        }
        let Some(binding) = tokens.source_binding.as_ref() else {
            return;
        };
        let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
            return;
        };
        let Some(site) = binding.invocation_site() else {
            return;
        };
        if !self.matches_original_source_image(site.source.source_image(), config)
            || config != input.lexer_config()
        {
            return;
        }
        let Some(original) =
            site.source
                .original_lexical_command(tokens.words(), site.offset, config)
        else {
            return;
        };
        let Some(receipt) =
            self.original_source_registered_instance_words(input, original.native_words())
        else {
            return;
        };
        let _ = self
            .bindings
            .retain_original_registered_instance_source(input, receipt);
    }

    /// Cursor geometry from an actual rejected procedure's body-delimiter cut.
    /// Complete caller image and configuration must match the retained source;
    /// this conditional region grants no complete body, entry, child lookup,
    /// Normal completion or native compiler authority.
    #[must_use]
    pub fn original_incomplete_body_region_at(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        cursor: u32,
        registry: &CommandRegistry,
    ) -> Option<tcl_lexer::Span> {
        // Implementation contract: naming.source.incomplete-body-header-metadata
        // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
        self.bindings
            .original_incomplete_body_region_at(image, config, cursor, registry)
    }

    /// Original declaration grammar and written positions for navigation.
    /// This conditional projection cannot establish executed aliases or bodies.
    #[must_use]
    pub fn original_declaration_assistance(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::OriginalDeclarationAssistance> {
        let advice = self
            .bindings
            .declaration_operand_layout_advice(tokens)
            .or_else(|| {
                tokens
                    .source_binding
                    .as_ref()?
                    .original_compilation_lookup_advice(tokens)
            })?;
        crate::registry_invocation::original_declaration_assistance(registry, tokens, &advice)
    }

    /// Exact call/allocation/frame receipt for scoped caller-name navigation.
    /// Conditional own-body frames grant symbolic identity, never execution,
    /// physical cells, values or completed writes.
    #[must_use]
    pub fn caller_frame_invocation_template_at(
        &self,
        tokens: &crate::ir::CommandTokens,
        read_offset: u32,
        registry: &CommandRegistry,
    ) -> Option<crate::command_binding::SourceCallerFrameInvocationTemplate> {
        self.bindings
            .caller_frame_invocation_template_at(tokens, read_offset, registry)
    }

    /// Original definition-name operands and their exact class incarnation.
    /// Missing coverage cannot provide a reference or editable method name.
    pub fn definition_method_reference_inventories(
        &self,
    ) -> impl Iterator<
        Item = (
            &crate::command_binding::SourceCommandTarget,
            Option<&[crate::command_binding::SourceDefinitionMethodReference]>,
        ),
    > {
        self.bindings.definition_method_reference_inventories()
    }

    /// Possible child dispatches from retained entered-body observations.
    /// These preserve actual lookup contexts without asserting parent purity.
    #[must_use]
    pub(crate) fn possible_entered_body_invocations(
        &self,
        invocation: &crate::command_binding::CommandAllocationSite,
    ) -> Vec<SourceInvocationBinding> {
        self.bindings.possible_entered_body_invocations(invocation)
    }

    /// Actual reads directly owned by this original invocation's argv. Nested
    /// evaluations and body phases retain their separate temporal owners.
    #[must_use]
    pub fn variable_accesses_for_invocation_args(
        &self,
        offset: u32,
    ) -> Vec<crate::command_binding::SourceVariableAccess> {
        self.bindings.variable_accesses_for_invocation_args(offset)
    }

    /// Positioned slot advice, independently of executable binding facts.
    pub fn diagnostic_slot_presence_at(
        &self,
        offset: u32,
    ) -> crate::command_binding::SourceCommandSlotPresence {
        self.bindings.diagnostic_slot_presence_at(offset)
    }

    /// Exact native command-table math lookup after the original operands.
    pub fn diagnostic_math_function_presence_at(
        &self,
        registry: &CommandRegistry,
        function: &str,
        offset: u32,
    ) -> crate::command_binding::SourceCommandSlotPresence {
        self.bindings
            .diagnostic_math_function_presence_at(registry, function, offset)
    }
    /// Name-reference advice in the consuming command's exact post-argv world.
    pub fn diagnostic_command_slot_presence_at(
        &self,
        name: &str,
        offset: u32,
    ) -> crate::command_binding::SourceCommandSlotPresence {
        self.bindings
            .diagnostic_command_slot_presence_at(name, offset)
    }

    /// Exact evaluated body carrier retained by the shared source interpreter.
    /// The interpreter keys an entered word by its whole written extent; a
    /// segmented argv token keeps the lexer's inner-end extent, so a quoted
    /// or compound word is also tried through the shared lexer widening.
    #[must_use]
    pub fn executed_script_for_word(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<&crate::command_binding::ExecutedScriptSource> {
        self.bindings.executed_script_for_word(span).or_else(|| {
            let image = self.bindings.source_origin()?.source_image();
            let whole = tcl_lexer::word_span_at(image.bytes(), span);
            if whole == span {
                return None;
            }
            self.bindings.executed_script_for_word(whole)
        })
    }

    pub(crate) fn retained_list_element_source_realm(
        &self,
        span: tcl_lexer::Span,
        ordinal: usize,
    ) -> Option<(crate::command_binding::ExecutedScriptSource, Self)> {
        let script = self
            .bindings
            .executed_script_for_list_element(span, ordinal)?
            .clone();
        let bindings = self.bindings.selected_source(&script)?;
        let realm = Self {
            bindings,
            source_input: self.source_input.clone(),
            specs: self.specs.clone(),
            original_tokens: original_tokens::OriginalCommandTokenCache::default(),
            original_bindings: original_bindings::OriginalInvocationBindingCache::default(),
            source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache::default(),
            source_call_advice:
                source_transition_advice::OriginalSourceTransitionAdviceCache::default(),
            query_memo:
                crate::command_binding::original_name_value::OriginalRealmQueryMemo::default(),
            stamped: StampedTokenMemo::default(),
            math_functions: crate::command_binding::OriginalMathFunctionWordMemo::default(),
        };
        Some((script, realm))
    }

    /// Head-only role projection at an actual site in an evaluated body.
    #[must_use]
    pub fn head_words_at_origin<'a>(
        &'a self,
        written: &'a str,
        source: &crate::command_binding::ExecutedScriptSource,
        offset: u32,
    ) -> HeadWords<'a> {
        let binding = self.bindings.invocation_at_origin(&source.origin, offset);
        let resolved = match self.head_fact(&binding) {
            RealmBindingFact::Unchanged => written,
            RealmBindingFact::Command(name) => name,
            RealmBindingFact::Rebound => "",
        };
        HeadWords { written, resolved }
    }

    /// No source entry contract; assistance keeps its written spelling.
    #[must_use]
    pub fn none() -> &'static Self {
        &EMPTY_REALM
    }

    /// Whether the source changes command-table or namespace lookup state.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.bindings.has_transitions()
    }

    /// Resolve a head for callers whose grammar cannot represent alias prefixes.
    #[must_use]
    pub fn resolve<'a>(&'a self, head: &'a str, at: u32) -> RealmBinding<'a> {
        match self.binding_at(head, at) {
            RealmBindingFact::Unchanged => RealmBinding::Command(head),
            RealmBindingFact::Command(name) => RealmBinding::Command(name),
            RealmBindingFact::Rebound => RealmBinding::Rebound,
        }
    }

    /// Preserve explicit uncertainty rather than returning the written name.
    #[must_use]
    pub fn binding_at(&self, head: &str, at: u32) -> RealmBindingFact<'_> {
        if self.is_empty() {
            return RealmBindingFact::Unchanged;
        }
        self.head_fact(&self.bindings.projection_at_source(head, at))
    }

    fn head_fact(&self, binding: &SourceInvocationBinding) -> RealmBindingFact<'_> {
        let Some(target) = binding.proved_target() else {
            return RealmBindingFact::Rebound;
        };
        if !target.registry_backed || !target.prepended.is_empty() {
            return RealmBindingFact::Rebound;
        }
        self.specs
            .get(&target.command)
            .map_or(RealmBindingFact::Rebound, |spec| {
                RealmBindingFact::Command(spec.name())
            })
    }

    /// The existing public knowledge vocabulary is a projection. It cannot
    /// express target-or-absence, which therefore remains explicitly unknown.
    #[must_use]
    pub fn knowledge_at(
        &self,
        context: &ResolvedContext,
        commands: &CommandRegistry,
        head: &str,
        at: u32,
    ) -> BindingKnowledge {
        if self.is_empty() {
            return context.resolve_spec(commands, head).map_or_else(
                || {
                    if context.environment.policy_defaults.closed_world
                        == tcl_dialect::model::WorldPolicy::Closed
                    {
                        BindingKnowledge::Absent
                    } else {
                        BindingKnowledge::Unknown
                    }
                },
                |spec| BindingKnowledge::Must(BindingTarget::Spec(SpecKey::new(spec))),
            );
        }
        let binding = self.bindings.projection_at_source(head, at);
        if binding.unknown || (binding.may_be_absent && !binding.targets.is_empty()) {
            return BindingKnowledge::Unknown;
        }
        if binding.targets.is_empty() {
            return BindingKnowledge::Absent;
        }
        let targets = binding
            .targets
            .iter()
            .map(|target| {
                self.specs
                    .get(&target.command)
                    .filter(|_| target.registry_backed)
                    .map_or_else(
                        || BindingTarget::document(&target.command),
                        |spec| BindingTarget::Spec(*spec),
                    )
            })
            .collect::<Vec<_>>();
        if targets.len() == 1 {
            BindingKnowledge::Must(targets[0].clone())
        } else {
            BindingKnowledge::May(targets.into())
        }
    }

    /// Offset-free assistance abstains unless all represented states agree.
    #[must_use]
    pub fn resolve_unpositioned<'a>(&'a self, head: &'a str) -> RealmBinding<'a> {
        if self.is_empty() {
            return RealmBinding::Command(head);
        }
        match self.head_fact(&self.bindings.invocation_unpositioned(head)) {
            RealmBindingFact::Command(name) => RealmBinding::Command(name),
            RealmBindingFact::Unchanged => RealmBinding::Command(head),
            RealmBindingFact::Rebound => RealmBinding::Rebound,
        }
    }

    /// Retain written and resolved identities as separate projections.
    #[must_use]
    pub fn head_words<'a>(&'a self, head: &'a str, at: u32) -> HeadWords<'a> {
        HeadWords {
            written: head,
            resolved: self.resolve(head, at).spec_name(),
        }
    }

    /// Retain written text when an offset-free query cannot prove identity.
    #[must_use]
    pub fn head_words_unpositioned<'a>(&'a self, head: &'a str) -> HeadWords<'a> {
        HeadWords {
            written: head,
            resolved: self.resolve_unpositioned(head).spec_name(),
        }
    }
}

impl tcl_registry::events::CommandHeadResolver for CommandBindingRealm {
    fn resolve<'a>(&'a self, written: &str, offset: u32) -> Cow<'a, str> {
        match self.binding_at(written, offset) {
            RealmBindingFact::Unchanged => Cow::Owned(written.to_owned()),
            RealmBindingFact::Command(name) => Cow::Borrowed(name),
            RealmBindingFact::Rebound => Cow::Borrowed(""),
        }
    }
}

/// Build the source projection using the shared executable binding kernel.
#[must_use]
pub fn document_realm_bindings(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> CommandBindingRealm {
    // A hosted F5 document names its commands only under its hosted source
    // entry; without it no head is an original input and every one reads
    // as rebound.
    if let Some(realm) = hosted_document_realm_bindings(source, dialect, registry) {
        return realm;
    }
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                registry,
                Some(dialect),
                tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..crate::command_binding::SourceAnalysisOptions::default()
        },
    );
    realm_from_source_bindings(bindings, registry)
}

fn hosted_document_realm_bindings(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Option<CommandBindingRealm> {
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::context_for_profile(dialect)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(
        dialect,
        dialect,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
    );
    if !input.has_hosted_source_name_context() {
        return None;
    }
    document_realm_bindings_for_fresh_hosted_source(source, &input)
}

/// Build the projection under the source's already-resolved lexer policy.
#[must_use]
pub fn document_realm_bindings_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
) -> CommandBindingRealm {
    realm_from_source_bindings(
        SourceCommandBindings::analyse(source, config, registry),
        registry,
    )
}

/// Build a fresh hosted source realm from its complete current input.
///
/// The environment and registry must belong to the selected unit profile.
/// This selects an authored source entry and grants no native execution authority.
#[must_use]
pub fn document_realm_bindings_for_fresh_hosted_source(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<CommandBindingRealm> {
    let context = input.context_registry();
    let registry = context.commands();
    let profile = input.unit_profile();
    if !input.has_hosted_source_name_context()
        || registry
            .profile()
            .is_none_or(|actual| actual.cache_key() != profile.cache_key())
        || context.context().environment.id
            != tcl_registry::model::ingress::context_for_profile(profile)
                .context()
                .environment
                .id
    {
        return None;
    }
    let config = input.lexer_config();
    let entry = crate::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        input,
        config,
        Some(input.unit_profile()),
    );
    let bindings =
        SourceCommandBindings::analyse_with_options(source, config, registry, entry.options());
    Some(realm_from_source_bindings(bindings, registry).with_resolved_analysis_input(input.clone()))
}

/// Interpret source with the driver's retained execution target and full body
/// configuration. The catalogue's convenience default supplies no replacement
/// for this input; actual native entries use their independently sealed path.
#[doc(hidden)]
#[must_use]
pub fn document_realm_bindings_with_resolved_input(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    declared: Option<&crate::command_binding::SourceDeclaredCommandContracts>,
) -> CommandBindingRealm {
    // naming.diagnostic.original-math-function-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
    let context = input.context_registry();
    let config = input.lexer_config();
    let entry = crate::command_binding::SourceAnalysisEntry::for_supplied_source(
        context.commands(),
        input,
        config,
        Some(input.unit_profile()),
    );
    let options = entry.options();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        declared.map_or(options, |contracts| contracts.with_options(options)),
    );
    realm_from_source_bindings(bindings, context.commands())
        .with_resolved_analysis_input(input.clone())
}

/// The document's source identity under `input`, with no command interpreted.
/// A display consumer that reads written words and the document's registry
/// needs only this; it costs one pass over nothing, whatever the file size.
/// Every site-bound query answers as for a site the interpretation never
/// reached, so no proof consumer can read a fact from it.
#[must_use]
pub(crate) fn document_shell_realm_with_resolved_input(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> CommandBindingRealm {
    let context = input.context_registry();
    let config = input.lexer_config();
    let dialect = crate::environment_ingress::authoring_invocation_dialect(
        context.commands(),
        Some(input.unit_profile()),
        config,
    );
    let options = crate::command_binding::SourceAnalysisOptions {
        metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
            Some(input),
        ),
        invocation_dialect: Some(dialect),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        unentered: true,
        ..crate::command_binding::SourceAnalysisOptions::default()
    };
    let bindings =
        SourceCommandBindings::analyse_with_options(source, config, context.commands(), options);
    realm_from_source_bindings(bindings, context.commands())
        .with_resolved_analysis_input(input.clone())
}

/// Interpret explicit Logical authoring inputs without donating the
/// heterogeneous catalogue driver's default Native engine. The document
/// driver's file protocol is retained; this supplies no Native name recipe.
pub(crate) fn document_lexical_realm_with_resolved_input(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    declared: Option<&crate::command_binding::SourceDeclaredCommandContracts>,
) -> CommandBindingRealm {
    // naming.minifier.logical-source-header
    // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
    let context = input.context_registry();
    let options = logical_source_options(input);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        context.commands(),
        declared.map_or(options, |contracts| contracts.with_options(options)),
    );
    realm_from_source_bindings(bindings, context.commands())
        .with_resolved_analysis_input(input.clone())
}

/// The fixed package-index source survey's own realm: the same Logical
/// source options as `document_lexical_realm_with_resolved_input`, with the
/// survey-scoped conditional source control. Only the survey constructs it;
/// a non-Logical or hosted input declines.
pub(crate) fn document_package_index_survey_realm_with_resolved_input(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    declared: &crate::command_binding::SourceDeclaredCommandContracts,
) -> Option<CommandBindingRealm> {
    if input.has_hosted_source_name_context() || !input.has_logical_source_name_context() {
        return None;
    }
    let context = input.context_registry();
    let bindings = SourceCommandBindings::analyse_package_index_survey(
        source,
        input.lexer_config(),
        context.commands(),
        declared.with_options(logical_source_options(input)),
    )?;
    Some(
        realm_from_source_bindings(bindings, context.commands())
            .with_resolved_analysis_input(input.clone()),
    )
}

fn logical_source_options(
    input: &crate::analyser::ResolvedAnalysisInput,
) -> crate::command_binding::SourceAnalysisOptions<'_> {
    let mut dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
    dialect.lexer_grammar = input.lexer_config().grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    crate::command_binding::SourceAnalysisOptions {
        metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
            Some(input),
        ),
        invocation_dialect: Some(dialect),
        logical_source_input: Some(input),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..crate::command_binding::SourceAnalysisOptions::default()
    }
}

/// Retain the exact hosted editing input without borrowing the command
/// catalogue's C-compatible name recipe. Runtime entry and storage remain
/// separate from this source-only domain.
///
/// A document supplied without an execution entry is a freshly loaded rule
/// (user decision 2026-10-21T00:10Z, "realm-aware command world"): when the
/// dialect admits the authored F5 naming simulation, the document realm takes
/// the rule-loader entry every other supplied-source consumer takes
/// ([`crate::command_binding::SourceAnalysisEntry::for_fresh_hosted_source`]),
/// so the loader's own `proc` is an original lookup.
pub(crate) fn document_vendor_realm_with_resolved_input(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    declared: Option<&crate::command_binding::SourceDeclaredCommandContracts>,
) -> CommandBindingRealm {
    // naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    let context = input.context_registry();
    let mut dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
    dialect.lexer_grammar = input.lexer_config().grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    let mut options = crate::command_binding::SourceAnalysisOptions {
        metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
            Some(input),
        ),
        invocation_dialect: Some(dialect),
        vendor_source_input: Some(input),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..crate::command_binding::SourceAnalysisOptions::default()
    };
    if let Some(fresh) = crate::command_binding::SourceAnalysisEntry::for_fresh_hosted_source(input)
        .filter(|entry| !entry.unknown_entry)
    {
        options.unknown_entry = false;
        options.invocation_realm = fresh.invocation_realm;
        options.execution_name_policy = fresh.execution_name_policy;
        options.native_compilation = fresh.native_compilation;
    }
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        context.commands(),
        declared.map_or(options, |contracts| contracts.with_options(options)),
    );
    realm_from_source_bindings(bindings, context.commands())
        .with_resolved_analysis_input(input.clone())
}

/// Project the original positioned Lowerer inventory under the complete
/// retained Module inputs. Missing or changed ownership returns no realm;
/// this query never rebuilds an independent interpretation or refreshes Native
/// compiler, frame, observer or provider currency.
#[must_use]
pub fn document_realm_bindings_for_module(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> Option<std::sync::Arc<CommandBindingRealm>> {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    let retained = module.retained_source_bindings.as_deref()?;
    retained.document_realm_for_module(module, registry, |bindings| {
        let mut realm = realm_from_source_bindings(bindings.clone(), registry);
        realm.source_input.clone_from(&module.source_metadata_input);
        realm
    })
}

/// Build a document realm from explicit driver execution provenance.
#[must_use]
pub fn document_realm_bindings_with_source_entry(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
    entry: &crate::command_binding::SourceAnalysisEntry,
) -> CommandBindingRealm {
    realm_from_source_bindings(
        SourceCommandBindings::analyse_with_options(source, config, registry, entry.options()),
        registry,
    )
}

/// Retain the same completed Logical source owner for readonly token advice.
/// This constructs no interpretation, lookup point or execution permission.
pub(crate) fn original_logical_realm_from_source_bindings(
    bindings: &SourceCommandBindings,
) -> Option<CommandBindingRealm> {
    let input = bindings.original_logical_source_name_advice_input()?;
    let config = bindings.original_source_lexer_config()?;
    let context = input.borrowed_context_registry();
    let image = bindings.source_origin()?.source_image();
    if !input.has_logical_source_name_context()
        || bindings.source_metadata_owner().source_analysis_input() != Some(input)
        || config != input.lexer_config()
        || !bindings.matches_original_source_image(image, config)
    {
        return None;
    }
    crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        input,
    )?;
    Some(
        realm_from_source_bindings(bindings.clone(), context.commands())
            .with_resolved_analysis_input(input.clone()),
    )
}

/// The realm of a release unit's authored simulation, under the unit's own
/// analysis input: the original bindings and registry the Logical realm
/// carries, without a Logical name-advice input.
pub(crate) fn original_authored_realm_from_source_bindings(
    bindings: &SourceCommandBindings,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<CommandBindingRealm> {
    let config = bindings.original_source_lexer_config()?;
    let image = bindings.source_origin()?.source_image();
    if bindings.source_metadata_owner().source_analysis_input() != Some(input)
        || config != input.lexer_config()
        || !bindings.matches_original_source_image(image, config)
    {
        return None;
    }
    let context = input.borrowed_context_registry();
    crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        input,
    )?;
    Some(
        realm_from_source_bindings(bindings.clone(), context.commands())
            .with_resolved_analysis_input(input.clone()),
    )
}

pub(crate) fn conditional_region_realm(
    region: &crate::command_binding::OriginalSourceConditionalRegion,
) -> CommandBindingRealm {
    realm_from_source_bindings(
        region.bindings().clone(),
        region.input().borrowed_context_registry().commands(),
    )
    .with_resolved_analysis_input(region.input().clone())
}

fn realm_from_source_bindings(
    bindings: SourceCommandBindings,
    registry: &CommandRegistry,
) -> CommandBindingRealm {
    let specs = registry
        .snapshot()
        .command_descriptor_coordinates()
        .iter()
        .filter_map(|(coordinate, label)| {
            registry
                .get(label)
                .map(|spec| (coordinate.clone(), SpecKey::new(spec)))
        })
        .collect();
    CommandBindingRealm {
        bindings,
        source_input: None,
        specs,
        original_tokens: original_tokens::OriginalCommandTokenCache::default(),
        original_bindings: original_bindings::OriginalInvocationBindingCache::default(),
        source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache::default(),
        source_call_advice: source_transition_advice::OriginalSourceTransitionAdviceCache::default(
        ),
        query_memo: crate::command_binding::original_name_value::OriginalRealmQueryMemo::default(),
        stamped: StampedTokenMemo::default(),
        math_functions: crate::command_binding::OriginalMathFunctionWordMemo::default(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_resolved_realm_input_tests;

#[cfg(test)]
mod descriptor_index_tests;

// Called only by the existing AUTH-gated report mirror, never ordinary equality.
#[cfg(test)]
impl CommandBindingRealm {
    pub(crate) fn exact_eq_for_report_trace(&self, other: &Self) -> bool {
        let same = self.bindings.exact_eq_for_report_trace(&other.bindings);
        if !same {
            eprintln!(
                "REPORT_CAPTURE stage=compare.owner.command_realm.component field=bindings refuses=true"
            );
            return false;
        }
        let same = self.specs == other.specs;
        if !same {
            eprintln!(
                "REPORT_CAPTURE stage=compare.owner.command_realm.component field=specs refuses=true"
            );
            return false;
        }
        let same = self.source_input == other.source_input;
        if !same {
            eprintln!(
                "REPORT_CAPTURE stage=compare.owner.command_realm.component field=source_input refuses=true"
            );
            return false;
        }
        true
    }
}
