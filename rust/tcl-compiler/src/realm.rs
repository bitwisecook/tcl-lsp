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
}

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
    /// Seal the independently supplied editing inputs with this new source
    /// interpretation. Changing a public result projection cannot replace them.
    pub(crate) fn with_resolved_analysis_input(
        mut self,
        input: crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        self.source_input = Some(input);
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

    /// Borrow the retained inventory for syntax navigation in the same source
    /// root. Reconstructed/entered sources must select their own origin first.
    pub(crate) fn source_bindings_ref(&self) -> &SourceCommandBindings {
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
        self.bindings.invocation_at_source(head, offset)
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
            .original_written_name_input_at_span(span, config)
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
            .original_c_array_operand_root_at_span_in_source(image, span, config, policy)
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
            .original_namespace_operand_at_span_in_source(image, span, config)
    }

    /// Attach retained original-source receipts to unchanged command tokens.
    /// Each consumer must select its own purpose projection afterwards;
    /// attachment itself grants neither dispatch nor body-entry authority.
    pub fn stamp_original_tokens(&self, tokens: &mut crate::ir::CommandTokens) {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        self.bindings
            .stamp_original_tokens_with_bindings(tokens, |offset| {
                self.retained_original_invocation(offset).as_ref().clone()
            });
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
    #[must_use]
    pub fn diagnostic_slot_presence_at(
        &self,
        offset: u32,
    ) -> crate::command_binding::SourceCommandSlotPresence {
        self.bindings.diagnostic_slot_presence_at(offset)
    }

    /// Exact native command-table math lookup after the original operands.
    #[must_use]
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
    #[must_use]
    pub fn diagnostic_command_slot_presence_at(
        &self,
        name: &str,
        offset: u32,
    ) -> crate::command_binding::SourceCommandSlotPresence {
        self.bindings
            .diagnostic_command_slot_presence_at(name, offset)
    }

    /// Exact evaluated body carrier retained by the shared source interpreter.
    #[must_use]
    pub fn executed_script_for_word(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<&crate::command_binding::ExecutedScriptSource> {
        self.bindings.executed_script_for_word(span)
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

/// Interpret source with the driver's retained execution target and full body
/// configuration. The catalogue's convenience default supplies no replacement
/// for this input; actual native entries use their independently sealed path.
pub(crate) fn document_realm_bindings_with_resolved_input(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    declared: Option<&crate::command_binding::SourceDeclaredCommandContracts>,
) -> CommandBindingRealm {
    // naming.diagnostic.original-math-function-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
    let context = input.context_registry();
    let config = input.lexer_config();
    let dialect = crate::environment_ingress::authoring_invocation_dialect(
        context.commands(),
        Some(input.unit_profile()),
        config,
    );
    let options = crate::command_binding::SourceAnalysisOptions {
        invocation_dialect: Some(dialect),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..crate::command_binding::SourceAnalysisOptions::default()
    };
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        declared.map_or(options, |contracts| contracts.with_options(options)),
    );
    realm_from_source_bindings(bindings, context.commands())
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
    let mut dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
    dialect.lexer_grammar = input.lexer_config().grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    let options = crate::command_binding::SourceAnalysisOptions {
        invocation_dialect: Some(dialect),
        logical_source_input: Some(input),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..crate::command_binding::SourceAnalysisOptions::default()
    };
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        context.commands(),
        declared.map_or(options, |contracts| contracts.with_options(options)),
    );
    realm_from_source_bindings(bindings, context.commands())
}

/// Retain the exact hosted editing input without borrowing the command
/// catalogue's C-compatible name recipe. Runtime entry and storage remain
/// separate from this source-only domain.
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
    let options = crate::command_binding::SourceAnalysisOptions {
        invocation_dialect: Some(dialect),
        vendor_source_input: Some(input),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..crate::command_binding::SourceAnalysisOptions::default()
    };
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        context.commands(),
        declared.map_or(options, |contracts| contracts.with_options(options)),
    );
    realm_from_source_bindings(bindings, context.commands())
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

fn realm_from_source_bindings(
    bindings: SourceCommandBindings,
    registry: &CommandRegistry,
) -> CommandBindingRealm {
    let specs = registry
        .command_names()
        .filter_map(|name| {
            registry.get(name).map(|spec| {
                (
                    tcl_syntax::naming::normalise_qualified_name(name),
                    SpecKey::new(spec),
                )
            })
        })
        .collect();
    CommandBindingRealm {
        bindings,
        source_input: None,
        specs,
        original_tokens: original_tokens::OriginalCommandTokenCache::default(),
        original_bindings: original_bindings::OriginalInvocationBindingCache::default(),
        source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_input_preserves_math_policy_without_catalogue_default() {
        // naming.diagnostic.original-math-function-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
        use tcl_registry::mathfunc::NativeMathFunctionDispatch;
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let environment = crate::environment_ingress::resolve_environment(name);
            let profile = environment.analyser_profile();
            let context =
                environment.context_registry(&tcl_registry::model::KeyedVersions::default(), 0);
            let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                environment.unit_profile(),
                context,
                config,
            );
            let source = "expr {Pi()}\n";
            let realm = document_realm_bindings_with_resolved_input(source, &input, None);
            let registry = input.context_registry();
            let image = tcl_lexer::SourceImage::document(source);
            let occurrences = realm
                .source_bindings_ref()
                .original_math_functions_in_source(
                    registry.commands(),
                    &image,
                    config,
                    tcl_lexer::Span::new(6, 8),
                );
            let [occurrence] = occurrences.as_slice() else {
                panic!("{name}: retained complete source expression: {occurrences:?}");
            };
            assert_eq!(occurrence.bytes(), b"Pi", "{name}");
            assert_eq!(occurrence.span(), tcl_lexer::Span::new(6, 8));
            assert_eq!(
                occurrence.dispatch(),
                if matches!(name, "tcl8.4" | "jim") {
                    NativeMathFunctionDispatch::FixedTable
                } else {
                    NativeMathFunctionDispatch::CommandTable
                }
            );
            assert_eq!(
                occurrence.name_policy().recipe(),
                tcl_registry::InvocationDialect::of_profile(input.unit_profile())
                    .native_name_protocol()
                    .unwrap()
            );
            assert!(occurrence.matches_source(&image, config, registry.commands()));
            assert!(!occurrence.matches_source(
                &tcl_lexer::SourceImage::native(source.as_bytes()),
                config,
                registry.commands(),
            ));
            let mut foreign = config;
            foreign.strict_quoting ^= true;
            assert!(!occurrence.matches_source(&image, foreign, registry.commands()));
            assert!(occurrence.command_reference().is_none());
        }
    }

    #[test]
    fn explicit_profile_retains_native_policy_independently_of_lexical_rules() {
        let registry = CommandRegistry::build_default();
        for name in ["tcl8.4", "tcl8.6", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).expect("native core");
            let realm = document_realm_bindings("set x value", profile, &registry);
            assert_eq!(
                realm
                    .invocation_at_source("set", 0)
                    .variable_context
                    .invocation_dialect,
                Some(tcl_registry::InvocationDialect::of_profile(profile))
            );
        }
        let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let realm = document_realm_bindings("set x value", profile, &registry);
        assert_eq!(
            realm
                .invocation_at_source("set", 0)
                .variable_context
                .invocation_dialect
                .and_then(|dialect| dialect.tcl_version),
            Some(tcl_dialect::TclVersion::V9_0)
        );
    }

    fn map_for(src: &str) -> CommandBindingRealm {
        let registry = tcl_registry::model::ingress::static_context_for("tcl").commands();
        document_realm_bindings(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            registry,
        )
    }

    fn irules_map_for(src: &str) -> CommandBindingRealm {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        document_realm_bindings(src, tcl_dialect::DialectProfile::irules(), registry)
    }

    /// Offset just past the first line of `src`, i.e. "after statement 1".
    fn after_first_line(src: &str) -> u32 {
        u32::try_from(src.find('\n').map_or(src.len(), |i| i + 1)).unwrap_or(0)
    }

    /// The [`BindingKnowledge`] view for head-identity consumers: document
    /// facts answer `Must(Spec)` / `Must(Document)` / `Absent`; with no fact
    /// the environment answers, and world policy decides `Absent` vs
    /// `Unknown` for an unprovided name.
    #[test]
    fn knowledge_composes_facts_over_the_environment() {
        let generation = tcl_registry::model::ingress::static_context_for("tcl9.0");
        let context = generation.context();
        let registry = generation.commands();
        let src = "rename format origfmt\nproc lindex {args} { return MINE }\nrename lsort {}\n";
        let map = document_realm_bindings(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            registry,
        );
        let end = u32::try_from(src.len()).unwrap_or(0);
        // A proven rename chain is a spec-keyed Must — the I4 licence.
        let origfmt = map.knowledge_at(context, registry, "origfmt", end);
        assert_eq!(
            origfmt.proved_spec().map(SpecKey::name),
            Some("format"),
            "a proven chain carries the registry spec"
        );
        // A user-proc takeover exists but licenses no hook.
        let lindex = map.knowledge_at(context, registry, "lindex", end);
        assert!(lindex.is_proved());
        assert_eq!(lindex.proved_spec(), None, "no catalogue semantics (I4)");
        // A deletion is honest absence.
        assert_eq!(
            map.knowledge_at(context, registry, "lsort", end),
            BindingKnowledge::Absent
        );
        // No fact: the environment resolves…
        let set = map.knowledge_at(context, registry, "set", end);
        assert_eq!(set.proved_spec().map(SpecKey::name), Some("set"));
        // …and an unprovided name under tcl9.0's open world stays Unknown
        // (a `package require` can introduce it at load time).
        assert_eq!(
            map.knowledge_at(context, registry, "no-such-cmd", end),
            BindingKnowledge::Unknown
        );
    }

    /// Under a closed world (`f5-irules`) an unprovided name is proved
    /// `Absent` — the static decidability iRules derives rather than assumes.
    #[test]
    fn a_closed_world_proves_absence() {
        let generation = tcl_registry::model::ingress::static_context_for("f5-irules");
        let context = generation.context();
        let registry = generation.commands();
        let map = CommandBindingRealm::default();
        assert_eq!(
            map.knowledge_at(context, registry, "no-such-cmd", 0),
            BindingKnowledge::Absent
        );
        // The disabled mutators are likewise absent from the surface…
        assert_eq!(
            map.knowledge_at(context, registry, "interp", 0),
            BindingKnowledge::Absent
        );
        // …while a provided command proves Must.
        assert!(
            map.knowledge_at(context, registry, "when", 0)
                .proved_spec()
                .is_some()
        );
    }

    #[test]
    fn static_rename_moves_the_identity_and_clears_the_old_name() {
        let src = "rename format origfmt\norigfmt %d 7\nformat x\n";
        let map = map_for(src);
        let at = after_first_line(src);
        assert_eq!(map.resolve("origfmt", at), RealmBinding::Command("format"));
        // The `::`-qualified spelling of the new name resolves alike.
        assert_eq!(
            map.resolve("::origfmt", at),
            RealmBinding::Command("format")
        );
        // The old name is gone from the rename onwards …
        assert_eq!(map.resolve("format", at), RealmBinding::Rebound);
        // … but calls *before* it still see the built-in.
        let src = "format {%08x} 42\nrename format origfmt\n";
        let map = map_for(src);
        assert_eq!(map.resolve("format", 0), RealmBinding::Command("format"));
        assert_eq!(
            map.resolve("format", after_first_line(src)),
            RealmBinding::Command("format"),
            "the rename entry still sees the old command before its mutation"
        );
        assert_eq!(
            map.resolve("format", u32::try_from(src.len()).unwrap()),
            RealmBinding::Rebound,
            "the successful rename continuation has retired the old slot"
        );
    }

    #[test]
    fn interp_alias_aliases_only_the_argument_preserving_form() {
        let src = "interp alias {} myfmt {} format\n";
        let map = map_for(src);
        let at = after_first_line(src);
        assert_eq!(map.resolve("myfmt", at), RealmBinding::Command("format"));
        assert_eq!(map.resolve("::myfmt", at), RealmBinding::Command("format"));

        // Pre-bound arguments shift every index — the layout cannot be reused.
        let src = "interp alias {} pad {} format %08x\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("pad", after_first_line(src)),
            RealmBinding::Rebound
        );
    }

    #[test]
    fn dynamic_bindings_abstain() {
        for src in [
            "rename $old myfmt\n",
            "interp alias {} $n {} format\n",
            "interp alias {} myfmt {} $t\n",
        ] {
            let map = map_for(src);
            let at = after_first_line(src);
            assert_eq!(
                map.resolve("myfmt", at),
                RealmBinding::Rebound,
                "an unresolved target must not acquire a registry identity: {src}"
            );
            // Missing argv values prevent the transition from being reached;
            // this cannot donate a new target identity for the alias.
            assert!(
                map.invocation_at_source("myfmt", at)
                    .proved_target()
                    .is_none()
            );
        }
    }

    #[test]
    fn a_child_interpreter_alias_changes_nothing_here() {
        let src = "interp alias slave myfmt {} format\n";
        let map = map_for(src);
        assert!(map.is_empty(), "a foreign srcPath states no fact here");
    }

    /// Child visibility operations do not rebind the parent's stock commands.
    /// Creating a named child also installs its own parent command.
    #[test]
    fn a_safe_interpreters_hidden_command_states_nothing_here() {
        // naming.interpreter.original-created-parent-command
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command.md
        // Native CLI observes named child creation/deletion and handle/path
        // evaluation. These assertions exercise source command-table advice;
        // they do not derive native tokens or effects from the CLI result.
        let creation = "interp create -safe s\n";
        let map = map_for(creation);
        assert!(!map.is_empty(), "creation installs a parent command");
        let offset = u32::try_from(creation.len()).unwrap();
        assert_eq!(map.binding_at("s", offset), RealmBindingFact::Rebound);
        assert_eq!(
            map.resolve("format", offset).spec_name(),
            map_for("").resolve("format", 0).spec_name(),
            "creating a child preserves the parent's stock format binding",
        );
        for src in [
            "interp hide s format\n",
            "interp expose s format\n",
            "interp invokehidden s format %d 7\n",
        ] {
            let map = map_for(src);
            assert!(
                map.is_empty(),
                "child visibility is a separate table: {src}"
            );
            assert_eq!(
                map.resolve("format", u32::try_from(src.len()).unwrap())
                    .spec_name(),
                map_for("").resolve("format", 0).spec_name(),
            );
        }
    }

    /// TN — a malformed call states nothing rather than half a fact.
    #[test]
    fn malformed_binding_calls_abstain() {
        for src in [
            // Arity too short for a rename / an alias creation.
            "rename\n",
            "rename format\n",
            "interp alias\n",
            "interp alias {}\n",
            // Not the alias subcommand at all (`aliases` merely queries).
            "interp aliases {}\n",
            // `proc` with no name.
            "proc\n",
        ] {
            let map = map_for(src);
            assert_eq!(
                map.resolve("format", u32::MAX),
                RealmBinding::Command("format"),
                "a malformed binding must not disturb `format`: {src}"
            );
        }
    }

    #[test]
    fn a_foreign_target_path_rebinds_without_aliasing() {
        let src = "interp alias {} myfmt slave format\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("myfmt", after_first_line(src)),
            RealmBinding::Rebound
        );
    }

    #[test]
    fn a_top_level_proc_shadows_the_builtin_it_names() {
        let src = "proc format {args} { return USER }\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("format", after_first_line(src)),
            RealmBinding::Rebound
        );
        // A fresh procedure changes the table even without shadowing a builtin.
        let src = "proc mything {args} { return 1 }\n";
        let map = map_for(src);
        assert!(!map.is_empty());
        assert_eq!(map.resolve("mything", u32::MAX), RealmBinding::Rebound);
        assert_eq!(
            map.resolve("format", u32::MAX),
            RealmBinding::Command("format")
        );
        // A qualified proc defines a different command entirely.
        let src = "namespace eval ns {}; proc ::ns::format {args} { return 1 }\n";
        let map = map_for(src);
        assert_eq!(map.resolve("::ns::format", u32::MAX), RealmBinding::Rebound);
        assert_eq!(
            map.resolve("format", u32::MAX),
            RealmBinding::Command("format")
        );
    }

    #[test]
    fn a_nested_binding_is_not_a_document_wide_fact() {
        // A conditional or deferred binding is not an unconditional statement
        // about the document's command table.
        for src in [
            "if {0} { rename format origfmt }\n",
            "proc p {} { rename format origfmt }\n",
        ] {
            assert_eq!(
                map_for(src).resolve("format", u32::MAX),
                RealmBinding::Command("format"),
                "deferred binding leaked: {src}"
            );
        }
        // Immediate eval and uplevel actually execute their literal bodies.
        for src in [
            "eval { rename format origfmt }\n",
            "uplevel #0 { rename format origfmt }\n",
        ] {
            let map = map_for(src);
            assert_eq!(
                map.resolve("format", u32::MAX),
                RealmBinding::Rebound,
                "executed rename omitted: {src}"
            );
            assert_eq!(
                map.resolve("origfmt", u32::MAX),
                RealmBinding::Command("format")
            );
        }
        // A `namespace eval` body *is* unconditional, but it binds in its own
        // namespace: `proc format` there defines `::n::format` and leaves the
        // global command table alone (#2065), so the fact is stated — scoped
        // to that namespace — and never reaches a head outside it.
        let src = "namespace eval ::n { proc format {a} { return 1 } }\nformat %b 5\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("format", u32::try_from(src.len()).unwrap_or(0)),
            RealmBinding::Command("format"),
            "a namespace-local shadow must not leak to the document"
        );
    }

    /// A `proc` inside a `namespace eval` body shadows the built-in for the
    /// bare heads in that namespace and nowhere else (#2065).
    ///
    /// tclsh 8.6.18 / 9.0.4, byte-identical:
    ///
    /// ```tcl
    /// namespace eval n {proc format {args} {return NS}; puts [format %b 5]}
    /// puts [format %d 5]      ;# NS, then 5
    /// namespace eval a {proc format {args} {return A}
    ///   namespace eval b {puts [format %d 7]}}   ;# 7 — `b` is not `a`
    /// namespace eval a {puts [format %d 7]}      ;# A — the block re-opens
    /// ```
    #[test]
    fn a_namespace_local_proc_shadows_the_builtin_inside_that_namespace() {
        let src = "namespace eval n {\n    proc format {args} { return NS }\n    format %b 5\n}\nformat %d 5\n";
        let map = map_for(src);
        let inside = u32::try_from(src.find("format %b").unwrap()).unwrap();
        let outside = u32::try_from(src.rfind("format %d").unwrap()).unwrap();
        assert_eq!(
            map.resolve("format", inside),
            RealmBinding::Rebound,
            "the namespace-local proc owns the bare head inside its body"
        );
        assert_eq!(
            map.resolve("format", outside),
            RealmBinding::Command("format"),
            "and says nothing about the global command table"
        );
        // The explicitly global spelling is the built-in even inside the body.
        assert_eq!(
            map.resolve("::format", inside),
            RealmBinding::Command("::format")
        );
    }

    /// The shadow is keyed by namespace, not by body: a later block re-opening
    /// the same namespace sees it, a nested one does not.
    #[test]
    fn a_namespace_local_shadow_follows_the_namespace_not_the_block() {
        let src = "namespace eval a {\n    proc format {args} { return A }\n    namespace eval b { format %b 1 }\n}\nnamespace eval a { format %b 2 }\n";
        let map = map_for(src);
        let nested = u32::try_from(src.find("format %b 1").unwrap()).unwrap();
        let reopened = u32::try_from(src.find("format %b 2").unwrap()).unwrap();
        assert_eq!(
            map.resolve("format", nested),
            RealmBinding::Command("format"),
            "`::a::b` resolves its own table then the global one, never `::a`"
        );
        assert_eq!(
            map.resolve("format", reopened),
            RealmBinding::Rebound,
            "re-opening `::a` sees the proc the first block declared"
        );
    }

    /// An unreadable body retains uncertainty rather than a registry identity.
    #[test]
    fn an_unreadable_namespace_body_states_no_local_fact() {
        for src in [
            // A dynamic namespace word names a namespace we cannot identify.
            "namespace eval $ns { proc format {args} { return 1 }\nformat %b 5 }\n",
            // A non-braced body is not the script Tcl finally runs.
            "namespace eval n $body\n",
        ] {
            let map = map_for(src);
            assert_eq!(
                map.binding_at("format", u32::try_from(src.len()).unwrap()),
                RealmBindingFact::Rebound,
                "unreadable namespace body must not license registry semantics: {src}"
            );
        }
    }

    #[test]
    fn a_qualified_mutator_head_states_the_same_fact() {
        // C Tcl resolves `::rename` to `::rename` — the mutator's own spelling
        // must not be a false negative either.
        let src = "::rename format origfmt\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("origfmt", after_first_line(src)),
            RealmBinding::Command("format")
        );
    }

    #[test]
    fn the_latest_applicable_fact_wins() {
        let src = "rename format origfmt\norigfmt %d 7\nproc origfmt {args} { return 1 }\n";
        let map = map_for(src);
        let after_all = u32::try_from(src.len()).unwrap_or(0);
        // Between the `rename` and the `proc`, `origfmt` *is* the built-in …
        assert_eq!(
            map.resolve("origfmt", after_first_line(src)),
            RealmBinding::Command("format")
        );
        // … and after the `proc` takes the name back it is a user command,
        // even though `origfmt` is not itself a registry name.
        assert_eq!(map.resolve("origfmt", after_all), RealmBinding::Rebound);
    }

    /// Chained bindings compose.
    ///
    /// On tclsh 8.6.16 and 9.0.4 (byte-identical):
    ///
    /// ```tcl
    /// interp alias {} a {} format ; rename a b ; b %08x 42   ;# 0000002a
    /// info commands a                                        ;# (empty)
    /// rename lindex li1 ; rename li1 li2 ; li2 {x y z} 2      ;# z
    /// rename lsort mysort ; interp alias {} sorter {} mysort
    /// sorter {c a b}                                          ;# a b c
    /// ```
    #[test]
    fn chained_bindings_compose() {
        // alias → rename
        let src = "interp alias {} a {} format\nrename a b\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("b", end), RealmBinding::Command("format"));
        assert_eq!(map.resolve("::b", end), RealmBinding::Command("format"));
        // The intermediate name is gone once the rename moves it.
        assert_eq!(map.resolve("a", end), RealmBinding::Rebound);

        // rename → rename
        let src = "rename lindex li1\nrename li1 li2\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("li2", end), RealmBinding::Command("lindex"));

        // rename → alias
        let src = "rename lsort mysort\ninterp alias {} sorter {} mysort\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("sorter", end), RealmBinding::Command("lsort"));
    }

    /// A chain must not inherit a *broken* binding: the rename moves the user
    /// proc, not the built-in it shadowed, so the new name gets no grammar.
    #[test]
    fn a_chain_through_a_shadowed_builtin_stays_rebound() {
        let src = "proc format {args} { return USER }\nrename format myfmt\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("myfmt", end), RealmBinding::Rebound);
        assert_eq!(map.resolve("::myfmt", end), RealmBinding::Rebound);
    }

    /// An alias whose target is an ordinary user proc still *takes over* the
    /// name it binds — C Tcl lets an alias shadow an existing command
    /// (tclsh 8.6.16 / 9.0.4: `proc myproc …; interp alias {} lindex {} myproc`
    /// makes `lindex {a b c} 1` answer `MINE`).
    #[test]
    fn an_alias_over_a_builtin_rebinds_it() {
        let src = "proc myproc {args} { return MINE }\ninterp alias {} lindex {} myproc\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("lindex", end), RealmBinding::Rebound);
    }

    #[test]
    fn an_unpositioned_read_abstains_when_the_facts_disagree() {
        // The renamed slot was absent before creation. An offset-free query
        // must retain that disagreement rather than choose the final state.
        let src = "rename format origfmt\n";
        let map = map_for(src);
        assert_eq!(map.resolve_unpositioned("origfmt"), RealmBinding::Rebound);
        assert_eq!(map.resolve_unpositioned("format"), RealmBinding::Rebound);
        // A head nothing binds keeps its own spelling.
        assert_eq!(
            map.resolve_unpositioned("lindex"),
            RealmBinding::Command("lindex")
        );

        // Two facts that disagree — without a position, neither can be chosen.
        let src = "rename format origfmt\nproc origfmt {args} { return 1 }\n";
        let map = map_for(src);
        assert_eq!(map.resolve_unpositioned("origfmt"), RealmBinding::Rebound);
    }

    #[test]
    fn the_shared_empty_map_binds_nothing() {
        let map = CommandBindingRealm::none();
        assert!(map.is_empty());
        assert_eq!(map.resolve("format", 0), RealmBinding::Command("format"));
        assert_eq!(
            map.resolve_unpositioned("format"),
            RealmBinding::Command("format")
        );
    }

    #[test]
    fn a_rebound_head_resolves_to_no_registry_spec() {
        // The `Rebound` sentinel must be unresolvable, since that is what makes
        // every registry query answer "unknown" without a variant check.
        let registry = tcl_registry::model::ingress::static_context_for("tcl").commands();
        assert!(registry.get(RealmBinding::Rebound.spec_name()).is_none());
    }

    #[test]
    fn irules_proc_identity_requires_a_closed_declaration_body() {
        // In iRules, unlike generic Tcl, only a real file-level declaration
        // takes over a command identity.  These malformed spellings must not
        // hide the following event handler.
        for src in [
            "proc when {} bare\nwhen HTTP_REQUEST {}\n",
            "proc when {} \"quoted\"\nwhen HTTP_REQUEST {}\n",
            "proc when {} {closed} trailing\nwhen HTTP_REQUEST {}\n",
            // The unterminated body consumes the tail as Tcl recovery does;
            // it still must not leave a persistent `when` rebinding behind.
            "proc when {} {unterminated",
        ] {
            let map = irules_map_for(src);
            assert_eq!(
                map.resolve("when", u32::MAX),
                RealmBinding::Command("when"),
                "malformed iRules proc poisoned `when`: {src:?}"
            );
        }

        let valid = irules_map_for("proc when {args} {}\nwhen HTTP_REQUEST {}\n");
        assert_eq!(valid.resolve("when", u32::MAX), RealmBinding::Rebound);

        // Generic Tcl deliberately preserves its existing `proc` binding
        // behaviour; the iRules declaration gate is profile-specific.
        let generic = map_for("proc format {} bare\nformat %d 1\n");
        assert_eq!(generic.resolve("format", u32::MAX), RealmBinding::Rebound);
    }

    #[test]
    fn unavailable_irules_mutators_do_not_change_event_identity() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let profile = registry.profile().expect("dialect registry has a profile");
        for command in ["interp", "rename", "namespace"] {
            assert!(
                registry
                    .get_for_surface(command, Some(profile.surface_query()))
                    .is_none(),
                "F5 K36322151 disables {command} in iRules"
            );
        }
        let events = tcl_registry::events::EventRegistry::build();
        let profiles = tcl_registry::profiles::ProfileRegistry::build();
        // F5 K36322151: iRules disables `interp`, `rename`, and `namespace`.
        // Their Tcl shapes are data/error commands here, never identity facts.
        let source = "interp alias {} event {} when\nrename when event\nnamespace import ::x::*\nwhen HTTP_REQUEST {}\n";
        let identities =
            document_realm_bindings(source, tcl_dialect::DialectProfile::irules(), registry);
        assert_eq!(
            identities.resolve("when", 0),
            RealmBinding::Command("when"),
            "an unavailable mutator must not change the preceding event identity"
        );
        let inferred =
            tcl_registry::profiles::compute_file_profiles_with_registry_and_head_resolver(
                source,
                &events,
                &profiles,
                registry,
                &identities,
            );
        assert!(inferred.contains(&"HTTP".to_owned()));

        // `proc` is available, so a genuine, executable rebinding still
        // removes the registry event-handler grammar.
        let rebound = "proc when {args} {}\nwhen HTTP_REQUEST {}\n";
        let identities =
            document_realm_bindings(rebound, tcl_dialect::DialectProfile::irules(), registry);
        let inferred =
            tcl_registry::profiles::compute_file_profiles_with_registry_and_head_resolver(
                rebound,
                &events,
                &profiles,
                registry,
                &identities,
            );
        assert!(!inferred.contains(&"HTTP".to_owned()));
    }
}
