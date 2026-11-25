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

//! Immutable original native bytes, independently of display and cache state.

use crate::signature_scan::scope::SignatureSourceNameInput;
use crate::var_resolve::ResolveContext;
use std::sync::Arc;
use tcl_core_types::NameBytes;
use tcl_syntax::naming::NamePolicyProtocol;

mod recorded_names;
mod span_queries;

pub(super) use recorded_names::{original_command_head_input, original_static_command_head_input};
pub(crate) use span_queries::OriginalRealmQueryMemo;

/// Distinct original-vector queries. Argument geometry never selects a handler.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalWordSourcePurpose {
    Invocation,
    ArgumentEvaluation,
}

#[derive(Clone, Copy)]
enum OriginalHeadInputDecline {
    Tokens,
    RecordedWords,
    WordCorrespondence,
    HeadWords,
    FrozenLength,
    Expanded,
    WrittenInput,
    ListInput,
    ListLength,
    ListFirst,
    NoHead,
}

impl OriginalHeadInputDecline {
    fn stage(self) -> &'static str {
        match self {
            Self::Tokens => "cut.input.tokens",
            Self::RecordedWords => "cut.input.recorded-words",
            Self::WordCorrespondence => "cut.input.word-correspondence",
            Self::HeadWords => "cut.input.head-words",
            Self::FrozenLength => "cut.input.frozen-length",
            Self::Expanded => "cut.input.expanded",
            Self::WrittenInput => "cut.input.written-input",
            Self::ListInput => "cut.input.list-input",
            Self::ListLength => "cut.input.list-length",
            Self::ListFirst => "cut.input.list-first",
            Self::NoHead => "cut.input.no-head",
        }
    }
}

enum OriginalRecordedNamePurpose {
    Native,
    Logical(crate::signature_scan::original_name::OriginalSourceNamePurpose),
}

impl OriginalWordSourcePurpose {
    pub(crate) fn accepts(self, tokens: &crate::ir::CommandTokens) -> bool {
        match self {
            Self::Invocation => tokens.synthetic.is_none(),
            Self::ArgumentEvaluation => matches!(
                tokens.synthetic,
                None | Some(crate::ir::SyntheticMarker::EvaluatedArguments)
            ),
        }
    }
}

#[cfg(test)]
fn trace_original_word_config(stage: &str, tokens: &crate::ir::CommandTokens, passed: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        let offset = tokens
            .words()
            .first()
            .map(|word| word.source().span.start());
        eprintln!("ORIGINAL_WORD_CONFIG stage={stage} offset={offset:?} passed={passed}");
    }
}

pub(crate) const MAX_ORIGINS: usize = 32;
pub(crate) const MAX_BYTES: usize = 65_536;

/// A closed source value graph. Issuers retain complete original producers;
/// bytes alone supply no source word, editable geometry, native object or OK.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalProducedNameValue {
    bytes: NameBytes,
    policy: NamePolicyProtocol,
    origins: Vec<SignatureSourceNameInput>,
    contents_epoch: u64,
    evaluated_index: Option<Box<OriginalEvaluatedVariableIndex>>,
}

impl OriginalProducedNameValue {
    /// Capture independently authenticated source units at the current content
    /// stamp. Evaluated inputs retain their complete existing producer graph.
    pub(crate) fn from_source_input(
        input: &SignatureSourceNameInput,
        context: &ResolveContext,
    ) -> Option<Self> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        if !input.is_current(context) {
            return None;
        }
        if let Some(value) = input.produced_value() {
            value.lineage_within(MAX_ORIGINS).then_some(())?;
            return Some(value.clone());
        }
        input.lineage_within(MAX_ORIGINS - 1).then_some(())?;
        (input.bytes().len() <= MAX_BYTES).then_some(())?;
        Some(Self {
            bytes: input.bytes().into(),
            policy: input.policy(),
            origins: vec![input.clone()],
            contents_epoch: context.original_contents_epoch()?,
            evaluated_index: None,
        })
    }

    /// Historical result data from the independently completed named object
    /// constructor. Current object liveness remains owned by its caller.
    pub(crate) fn from_object_command_name_result(
        receipt: &super::named_manufacture::OriginalObjectCommandNameResult,
        context: &ResolveContext,
    ) -> Option<Self> {
        // Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
        let value = crate::signature_scan::scope::SignatureSourceNameValue::from_object_command_name_result(
            receipt, context,
        )?;
        Self::from_source_input(&SignatureSourceNameInput::OriginalValue(value), context)
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }
    pub(crate) const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }
    #[cfg(test)]
    pub(crate) fn original_origins(&self) -> &[SignatureSourceNameInput] {
        &self.origins
    }

    fn lineage_within(&self, budget: usize) -> bool {
        let mut remaining = budget;
        self.lineage_nodes_in(&mut remaining)
    }

    pub(crate) fn lineage_nodes_in(&self, remaining: &mut usize) -> bool {
        let Some(next) = remaining.checked_sub(1) else {
            return false;
        };
        *remaining = next;
        self.origins
            .iter()
            .all(|origin| origin.lineage_nodes_in(remaining))
            && self
                .evaluated_index
                .as_ref()
                .is_none_or(|index| index.value.lineage_nodes_in(remaining))
    }

    /// Current data correspondence is separate from a variable read receipt,
    /// command execution, representation conversion and physical object roots.
    pub(crate) fn is_current(&self, context: &ResolveContext) -> bool {
        self.lineage_within(MAX_ORIGINS)
            && context.original_contents_epoch() == Some(self.contents_epoch)
            && context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                == Some(self.policy)
            && self.origins.iter().all(|origin| origin.is_current(context))
            && self
                .evaluated_index
                .as_ref()
                .is_none_or(|index| index.value.is_current(context))
    }

    /// Equal bytes require every admitted producer, not the first branch's
    /// origin. Unequal policies/currencies and unbounded lineage abstain.
    pub(crate) fn joined(&self, other: &Self) -> Option<Self> {
        if self.bytes != other.bytes
            || self.policy != other.policy
            || self.contents_epoch != other.contents_epoch
        {
            return None;
        }
        let mut result = self.clone();
        merge_origins(&mut result.origins, &other.origins)?;
        result.evaluated_index = match (&self.evaluated_index, &other.evaluated_index) {
            (Some(left), Some(right)) if left.word == right.word && left.arena == right.arena => {
                Some(Box::new(OriginalEvaluatedVariableIndex {
                    word: left.word.clone(),
                    arena: left.arena.clone(),
                    value: left.value.joined(&right.value)?,
                }))
            }
            _ => None,
        };
        result.lineage_within(MAX_ORIGINS).then_some(result)
    }

    /// Attach the separately accumulated index only at the actual whole-word
    /// evaluator. The complete word bytes are never sliced to recover it.
    pub(super) fn with_evaluated_variable_index(
        mut self,
        word: tcl_lexer::NativeWord,
        arena: tcl_lexer::ExecutablePartArena,
        mut value: Self,
        context: &ResolveContext,
    ) -> Option<Self> {
        if !self.is_current(context)
            || !value.is_current(context)
            || self.policy != value.policy
            || word.image() != arena.image()
            || word.config() != arena.config()
        {
            return None;
        }
        value.evaluated_index = None;
        self.evaluated_index = Some(Box::new(OriginalEvaluatedVariableIndex {
            word,
            arena,
            value,
        }));
        self.lineage_within(MAX_ORIGINS).then_some(self)
    }

    fn evaluated_variable_index(
        &self,
        word: &tcl_lexer::NativeWord,
        arena: &tcl_lexer::ExecutablePartArena,
        context: &ResolveContext,
    ) -> Option<SignatureSourceNameInput> {
        let index = self.evaluated_index.as_ref()?;
        (self.is_current(context) && &index.word == word && &index.arena == arena
            && index.value.policy == self.policy).then(|| SignatureSourceNameInput::OriginalValue(
                crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(&index.value),
            ))
    }

    /// Missing member of an actually selected lassign instruction. This is
    /// empty value data under the held list producer, without a fictitious
    /// list-child ordinal, object representation or successful store receipt.
    pub(super) fn list_assignment_empty(operation: &Self) -> Option<Self> {
        let mut result = operation.clone();
        result.bytes = NameBytes::from(b"".as_slice());
        result.evaluated_index = None;
        result.lineage_within(MAX_ORIGINS).then_some(result)
    }

    /// Native List result construction retains the complete elements and the
    /// actual selected serializer; parseable/equal Unicode cannot replace it.
    pub(super) fn list_result(elements: &[Self], operation: &Self) -> Option<Self> {
        let first = elements.first().unwrap_or(operation);
        let mut result = operation.clone();
        result.evaluated_index = None;
        if first.policy != operation.policy || first.contents_epoch != operation.contents_epoch {
            return None;
        }
        for element in elements {
            if first.policy != element.policy || first.contents_epoch != element.contents_epoch {
                return None;
            }
            merge_origins(&mut result.origins, &element.origins)?;
        }
        let values = elements.iter().map(Self::bytes).collect::<Vec<_>>();
        result.with_list_bytes(&values)
    }

    /// Retain a list wrapping this script with generated header bytes.
    pub(super) fn namespace_prefix_result(
        &self,
        operation: &Self,
        [head, selector, namespace]: [&[u8]; 3],
    ) -> Option<Self> {
        if self.policy != operation.policy || self.contents_epoch != operation.contents_epoch {
            return None;
        }
        let mut result = operation.clone();
        result.evaluated_index = None;
        merge_origins(&mut result.origins, &self.origins)?;
        result.with_list_bytes(&[head, selector, namespace, self.bytes()])
    }

    fn with_list_bytes(mut self, elements: &[&[u8]]) -> Option<Self> {
        self.lineage_within(MAX_ORIGINS).then_some(())?;
        let bytes = tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
            self.policy.string_protocol(),
        )
        .render(elements);
        (bytes.len() <= MAX_BYTES).then_some(())?;
        self.bytes = bytes.into();
        Some(self)
    }

    /// Concatenate independently frozen components in evaluation order. No
    /// logical String is encoded into native bytes at this boundary.
    pub(crate) fn concatenated(&self, other: &Self) -> Option<Self> {
        if self.policy != other.policy || self.contents_epoch != other.contents_epoch {
            return None;
        }
        let length = self.bytes().len().checked_add(other.bytes().len())?;
        (length <= MAX_BYTES).then_some(())?;
        let mut result = self.clone();
        result.evaluated_index = None;
        merge_origins(&mut result.origins, &other.origins)?;
        result.lineage_within(MAX_ORIGINS).then_some(())?;
        let bytes = tcl_syntax::native_object_append::SourceStringConcatenationProtocol::for_string_protocol(self.policy.string_protocol()).concatenate(self.bytes(), other.bytes())?;
        result.bytes = bytes.into();
        Some(result)
    }
}

/// Actual index components frozen by one original template evaluation.
/// This remains a readonly value facet, independently of any compiler purpose.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OriginalEvaluatedVariableIndex {
    word: tcl_lexer::NativeWord,
    arena: tcl_lexer::ExecutablePartArena,
    value: OriginalProducedNameValue,
}

fn merge_origins(
    left: &mut Vec<SignatureSourceNameInput>,
    right: &[SignatureSourceNameInput],
) -> Option<()> {
    for origin in right {
        if !left.contains(origin) {
            (left.len() < MAX_ORIGINS).then_some(())?;
            left.push(origin.clone());
        }
    }
    Some(())
}

#[cfg(test)]
mod tests;

/// Match the sole owning IR word against the canonical original image/config
/// producer. A source offset or equal displayed spelling is insufficient.
pub(super) fn original_native_word(
    word: &crate::ir::WordExpr,
    state: &super::ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::NativeWord> {
    let original = state
        .current_source_origin
        .as_ref()?
        .original_lexical_command(
            std::slice::from_ref(word),
            word.source().span.start(),
            config,
        )?;
    let [word] = original.native_words() else {
        return None;
    };
    Some(word.clone())
}

pub(super) fn capture_word(
    word: &crate::ir::WordExpr,
    state: &super::ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalProducedNameValue> {
    use crate::signature_scan::scope::{SignatureSourceNameKey, SignatureSourceNameValue};
    let policy = state
        .source_variables
        .execution_name_policy?
        .native_recipe()?;
    let native = original_native_word(word, state, config)?;
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let input = SignatureSourceNameKey::from_original_native_word(&native, rules, policy)
        .map(SignatureSourceNameInput::OriginalWord)
        .or_else(|| {
            SignatureSourceNameValue::from_original_static_word(&native, rules, policy)
                .map(SignatureSourceNameInput::OriginalValue)
        })?;
    OriginalProducedNameValue::from_source_input(&input, &state.source_variables)
}

fn original_written_variable_inputs(
    native: super::SourceScriptOperands<'_>,
    offset: u32,
    state: &super::ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> Option<Vec<Option<SignatureSourceNameInput>>> {
    use crate::signature_scan::scope::{SignatureSourceNameKey, SignatureSourceNameValue};
    let variables = &state.source_variables;
    state.current_source_origin.as_ref().and_then(|origin| {
        let policy = variables.execution_name_policy?.native_recipe()?;
        let owned;
        let words = if let Some(original) = context.original_written_projection {
            original.words_for(origin.source_image(), native.words, context.config)?
        } else {
            owned = origin.original_lexical_command(native.words, offset, context.config)?;
            owned.native_words()
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&context.config);
        let frozen = context.written_name_values.filter(|values| {
            values.len() == native.words.len()
                && native
                    .written_arguments
                    .is_some_and(|arguments| arguments.len() == values.len())
        });
        Some(
            words
                .iter()
                .enumerate()
                .map(|(ordinal, word)| {
                    SignatureSourceNameKey::from_original_native_word(word, rules, policy)
                        .map(SignatureSourceNameInput::OriginalWord)
                        .or_else(|| {
                            SignatureSourceNameValue::from_original_static_word(word, rules, policy)
                                .map(SignatureSourceNameInput::OriginalValue)
                        })
                        .or_else(|| {
                            let value = frozen?.get(ordinal)?.as_deref()?;
                            value.is_current(variables).then(|| {
                                SignatureSourceNameInput::OriginalValue(
                                    SignatureSourceNameValue::from_original_produced_value(value),
                                )
                            })
                        })
                })
                .collect::<Vec<_>>(),
        )
    })
}

/// Project genuine effective operands once from their original written vector.
/// Compiler local declarations and readonly values retain separate purposes.
pub(super) fn original_variable_invocation(
    native: super::SourceScriptOperands<'_>,
    offset: u32,
    state: &super::ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> crate::variable_bindings::OriginalVariableInvocation {
    use crate::registry_invocation::InvocationWordOrigin;
    let count = native.arguments.exact_argv_len().unwrap_or(0);
    let variables = &state.source_variables;
    let written = original_written_variable_inputs(native, offset, state, context);
    let expanded = written.as_ref().map(|inputs| {
        inputs
            .iter()
            .enumerate()
            .map(|(ordinal, input)| {
                let elements = native.written_arguments?.get(ordinal)?.expansion_len()?;
                let children = input.as_ref()?.original_list_elements()?;
                (children.len() == elements).then_some(children)
            })
            .collect::<Vec<_>>()
    });
    let mut inputs = Vec::with_capacity(count);
    let mut locals = Vec::with_capacity(count);
    let mut compiled_operands = Vec::with_capacity(count);
    for argument in 0..count {
        let origin = native.written_origin(argument);
        inputs.push(match origin {
            Some(InvocationWordOrigin::Written(ordinal)) => written
                .as_ref()
                .and_then(|values| values.get(ordinal))
                .cloned()
                .flatten(),
            Some(InvocationWordOrigin::ExpandedElement { written, element }) => expanded
                .as_ref()
                .and_then(|values| values.get(written))
                .and_then(Option::as_ref)
                .and_then(|values| values.get(element))
                .cloned(),
            Some(InvocationWordOrigin::BindingPrefix(ordinal)) => native
                .target
                .original_prepended_name_inputs()
                .and_then(|inputs| inputs.get(ordinal))
                .filter(|input| input.is_current(variables))
                .cloned(),
            _ => None,
        });
        locals.push(match (context.original_variable_compilation, origin) {
            (Some(compilation), Some(InvocationWordOrigin::Written(ordinal))) => {
                super::original_variable_compilation::original_compiler_namespace_local_name(
                    compilation,
                    ordinal,
                    variables,
                )
            }
            _ => None,
        });
        compiled_operands.push(match (context.original_variable_compilation, origin) {
            (Some(compilation), Some(InvocationWordOrigin::Written(ordinal))) => {
                super::original_variable_compilation::original_compiler_variable_operand(
                    compilation,
                    ordinal,
                    variables,
                )
            }
            _ => None,
        });
    }
    crate::variable_bindings::OriginalVariableInvocation::from_original_inputs(inputs, locals)
        .with_compiled_operands(compiled_operands)
}

/// Unchanged parser vector retained by an actual dispatch. This supplies source
/// correspondence independently of compiler selection and declaration advice.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalWrittenSourceWords {
    site: super::CommandAllocationSite,
    config: tcl_lexer::LexerConfig,
    words: Arc<[crate::ir::WordExpr]>,
    dialect: Option<tcl_registry::InvocationDialect>,
}

impl OriginalWrittenSourceWords {
    pub(super) fn at_dispatch(
        words: &[crate::ir::WordExpr],
        offset: u32,
        state: &super::ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Self> {
        Self::at_source(
            state.current_source_origin.as_ref()?,
            words,
            offset,
            config,
            state.baseline.dialect,
        )
    }

    fn at_source(
        origin: &Arc<super::SourceOriginId>,
        words: &[crate::ir::WordExpr],
        offset: u32,
        config: tcl_lexer::LexerConfig,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<Self> {
        origin.original_lexical_command(words, offset, config)?;
        Some(Self {
            site: super::CommandAllocationSite {
                source: Arc::clone(origin),
                offset,
            },
            config,
            words: Arc::from(words),
            dialect,
        })
    }

    pub(super) fn site(&self) -> &super::CommandAllocationSite {
        &self.site
    }

    pub(super) fn matches_native_words(&self, words: &[tcl_lexer::NativeWord]) -> bool {
        self.projection()
            .is_some_and(|original| original.native_words() == words)
    }

    pub(super) fn projection(
        &self,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceCommandProjection>> {
        self.site
            .source
            .original_lexical_command(&self.words, self.site.offset, self.config)
    }
}

/// Immutable syntax owner from an authentic original body descent. Its private
/// fields retain complete body bytes/channel and input, never lookup or entry.
#[derive(Debug, Clone)]
pub(crate) struct OriginalBodyWrittenWordOwner {
    body: super::ExecutedScriptSource,
    config: tcl_lexer::LexerConfig,
    input: crate::analyser::ResolvedAnalysisInput,
}

/// Derived instance schemas are owned by one retained analysis, independently
/// of binding identity. A conflicting genuine receipt permanently withdraws
/// that site's projection; the analysis owner holds only a weak cache link.
#[derive(Debug, Default)]
pub(super) struct OriginalRegisteredInstanceSourceCache {
    entries: std::sync::Mutex<
        std::collections::BTreeMap<
            u32,
            Option<std::sync::Arc<super::OriginalSourceRegisteredInstanceWords>>,
        >,
    >,
}

impl super::SourceInvocationBinding {
    /// Genuine original instance syntax and selected constructor descriptor.
    /// This readonly schema supplies no live receiver, store or Normal grant.
    pub(crate) fn original_registered_instance_source_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<std::sync::Arc<super::OriginalSourceRegisteredInstanceWords>> {
        if tokens.source_binding.as_ref() != Some(self) || !metadata.matches_registry(registry) {
            return None;
        }
        self.original_source_advice_purpose(tokens, metadata, registry)?;
        let input = metadata.source_analysis_input()?;
        let owner = self.original_analysis_owner.as_ref()?;
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let site = self.invocation_site()?;
        if owner.metadata.source_analysis_input() != Some(input)
            || owner.config != config
            || config != input.lexer_config()
            || !std::sync::Arc::ptr_eq(&owner.source, &site.source)
        {
            return None;
        }
        let cache = owner
            .derived_memo()
            .registered_instance_sources
            .get()?
            .upgrade()?;
        let receipt = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&site.offset)?
            .as_ref()?
            .clone();
        let original = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        let context = input.borrowed_context_registry();
        (receipt.site() == site
            && std::sync::Arc::ptr_eq(&receipt.site().source, &site.source)
            && receipt.matches_source_context(site.source.source_image(), config, context)
            && receipt.original_words() == original.native_words())
        .then_some(receipt)
    }
}

impl super::SourceCommandBindings {
    /// Freeze written geometry before any structural namespace memo is selected.
    /// The caller independently authenticates the original body and namespace.
    pub(crate) fn original_body_written_word_owner(
        &self,
        body: &super::ExecutedScriptSource,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<OriginalBodyWrittenWordOwner> {
        let point = self.source_metadata_owner().source_analysis_input()?;
        if (point != input && point != &input.for_nested_source())
            || self.lexer_config != Some(config)
            || self.root_origin.as_ref() != Some(&body.origin)
            || body.text.channel() != tcl_lexer::SourceChannel::NativeValue
        {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            input,
            config,
            Some(input.unit_profile()),
        )?;
        body.try_text().ok()?;
        let base = usize::try_from(body.base()).ok()?;
        let end = base.checked_add(body.text.len())?;
        (body.origin.source_image().bytes().get(base..end) == Some(body.text.bytes())).then(|| {
            OriginalBodyWrittenWordOwner {
                body: body.clone(),
                config,
                input: input.clone(),
            }
        })
    }
    /// A materialized original body retains the exact written parent allocation
    /// and argument. The parent inventory, full metadata and canonical operand
    /// projection authenticate this geometry without issuing a child entry.
    pub(crate) fn original_body_written_word_owner_at_parent(
        &self,
        body: &super::ExecutedScriptSource,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        parent: &crate::ir::CommandTokens,
    ) -> Option<OriginalBodyWrittenWordOwner> {
        if self.root_origin.as_ref() == Some(&body.origin) {
            return self.original_body_written_word_owner(body, config, registry, input);
        }
        let super::SourceOriginKind::Derived {
            parent: site,
            arguments,
            source,
            ..
        } = body.origin.kind()
        else {
            return None;
        };
        let [argument] = arguments.as_slice() else {
            return None;
        };
        let point = self.source_metadata_owner().source_analysis_input()?;
        let binding = parent.source_binding.as_ref()?;
        if (point != input && point != &input.for_nested_source())
            || self.lexer_config != Some(config)
            || self.root_origin.as_ref() != Some(&site.source)
            || source != &body.text
            || body.text.channel() != tcl_lexer::SourceChannel::NativeValue
            || binding.invocation_site() != Some(site)
            || &self.invocation_at_source("", site.offset) != binding
        {
            return None;
        }
        let metadata = binding.original_invocation_metadata_for_input(parent, input, registry)?;
        if metadata.source_analysis_input() != Some(input) {
            return None;
        }
        let word = parent.words().get(argument.checked_add(1)?)?;
        if binding
            .original_body_source_for_word(parent, word.source().span, config)
            .as_ref()
            != Some(body)
        {
            return None;
        }
        body.try_text().ok()?;
        let base = usize::try_from(body.base()).ok()?;
        let end = base.checked_add(body.text.len())?;
        (body.origin.source_image().bytes().get(base..end) == Some(body.text.bytes())).then(|| {
            OriginalBodyWrittenWordOwner {
                body: body.clone(),
                config,
                input: input.clone(),
            }
        })
    }
}

impl OriginalBodyWrittenWordOwner {
    /// Attach only exact original geometry to an otherwise unknown point.
    /// Existing reached/compiler/declaration owners keep their own refusals.
    pub(crate) fn stamp_original_tokens(
        &self,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &mut crate::ir::CommandTokens,
    ) -> Option<()> {
        if input != &self.input
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            input,
            self.config,
            Some(input.unit_profile()),
        )?;
        let base = usize::try_from(self.body.base()).ok()?;
        let end = base.checked_add(self.body.text.len())?;
        if tokens.words().iter().any(|word| {
            let span = word.source().span;
            usize::try_from(span.start()).map_or(true, |start| start < base)
                || usize::try_from(span.end()).map_or(true, |finish| finish > end)
        }) {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let original = OriginalWrittenSourceWords::at_source(
            &self.body.origin,
            tokens.words(),
            offset,
            self.config,
            None,
        )?;
        let binding = tokens.source_binding.as_mut()?;
        if binding.invocation_site().is_some()
            || binding.lookup_state.is_some()
            || binding.compiler_lookup_state.is_some()
            || binding.declaration_layout_observations.is_some()
            || !binding.unknown
            || !binding.targets.is_empty()
        {
            return None;
        }
        binding.original_written_words = Some(Arc::new(original));
        Some(())
    }
}

#[cfg(test)]
mod graph_tests;

/// Independently selected original variable compiler preparation. This seal
/// carries recipes and original ordinals only, not a physical local table,
/// activation, successful handler or the value of a variable read.
#[derive(Clone, Copy)]
pub(crate) struct OriginalSourceVariableCompilation<'a> {
    selected: &'a tcl_registry::native_compilation::NativeCompilationSelection,
    structured: Option<&'a super::SourceNativeStructuredPreparation>,
    namespace: Option<&'a super::SourceNativeNamespaceBindingPreparation>,
    original: &'a super::compiled_invocation::SourceOriginalCompilerWords,
    evaluated_words: Option<&'a [Option<std::sync::Arc<OriginalProducedNameValue>>]>,
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
    protocol: Option<tcl_syntax::naming::NativeCompiledVariableProtocol>,
    locals: Option<&'a super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory>,
}

impl<'a> OriginalSourceVariableCompilation<'a> {
    pub(super) fn from_selected(
        selected: &'a super::compiled_invocation::CompiledInvocationSelection,
        state: &super::ModuleCommandBindings,
        context: super::SourceExecutionContext<'a>,
    ) -> Option<Self> {
        let admission = selected.admission.as_ref()?;
        if !matches!(
            admission,
            tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. }
        ) {
            return None;
        }
        let original = selected.original_words.as_deref()?;
        (original.config == context.config
            && state.current_source_origin.as_ref() == Some(&original.site.source))
        .then_some(())?;
        let options = super::SourceAnalysisOptions {
            native_entry: state.baseline.native_entry.as_deref(),
            invocation_dialect: state.baseline.dialect,
            compiled_variable_provider: state.baseline.compiled_variable_provider,
            ..Default::default()
        };
        let protocol = options.compiled_variable_protocol();
        let locals = context
            .compilation_snapshot
            .and_then(|snapshot| snapshot.source_locals.as_deref())
            .filter(|locals| {
                protocol.is_some_and(|protocol| {
                    locals.owns(
                        &original.site,
                        original.config,
                        context.frame,
                        context.compilation(),
                        protocol,
                    )
                })
            });
        Some(Self {
            selected: admission,
            structured: selected.structured.as_deref(),
            namespace: selected.namespace_bindings.as_deref(),
            original,
            evaluated_words: None,
            compilation: context.compilation(),
            protocol,
            locals,
        })
    }
    pub(super) fn with_evaluated_words(
        mut self,
        values: &'a [Option<std::sync::Arc<OriginalProducedNameValue>>],
    ) -> Option<Self> {
        (values.len() == self.original.words.len()).then_some(())?;
        self.evaluated_words = Some(values);
        Some(self)
    }
    /// Same-ordinal frozen value attached only by the complete original argv
    /// evaluator. This receipt cannot be manufactured from equal value bytes.
    pub(crate) fn evaluated_original_word(
        &self,
        written: usize,
    ) -> Option<SignatureSourceNameInput> {
        let value = self.evaluated_words?.get(written)?.as_deref()?;
        Some(SignatureSourceNameInput::OriginalValue(
            crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(
                value,
            ),
        ))
    }
    /// The independent index value produced at this exact written ordinal.
    /// A whole-word value, matching bytes or another arena cannot substitute.
    pub(crate) fn evaluated_original_variable_index(
        &self,
        written: usize,
        word: &tcl_lexer::NativeWord,
        index: &tcl_lexer::ExecutablePartArena,
        context: &ResolveContext,
    ) -> Option<SignatureSourceNameInput> {
        let original = self.site().source.original_lexical_command(
            self.original_words(),
            self.site().offset,
            self.config(),
        )?;
        (original.native_words().get(written)? == word).then_some(())?;
        self.evaluated_words?
            .get(written)?
            .as_deref()?
            .evaluated_variable_index(word, index, context)
    }
    pub(crate) const fn selection(
        &self,
    ) -> &tcl_registry::native_compilation::NativeCompilationSelection {
        self.selected
    }
    pub(crate) const fn structured(&self) -> Option<&super::SourceNativeStructuredPreparation> {
        self.structured
    }
    pub(crate) const fn namespace(
        &self,
    ) -> Option<&super::SourceNativeNamespaceBindingPreparation> {
        self.namespace
    }
    pub(crate) fn original_words(&self) -> &[crate::ir::WordExpr] {
        &self.original.words
    }
    pub(crate) const fn site(&self) -> &super::CommandAllocationSite {
        &self.original.site
    }
    pub(crate) const fn config(&self) -> tcl_lexer::LexerConfig {
        self.original.config
    }
    pub(crate) const fn compilation(
        &self,
    ) -> tcl_registry::native_compilation::NativeCompilationContext {
        self.compilation
    }
    pub(crate) const fn protocol(
        &self,
    ) -> Option<tcl_syntax::naming::NativeCompiledVariableProtocol> {
        self.protocol
    }
    pub(crate) const fn local_inventory(
        &self,
    ) -> Option<&super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory> {
        self.locals
    }
}
