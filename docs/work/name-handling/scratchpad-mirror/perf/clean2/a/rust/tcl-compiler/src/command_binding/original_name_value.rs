// SPDX-License-Identifier: AGPL-3.0-or-later
//! Immutable original native bytes, independently of display and cache state.

use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
use crate::var_resolve::ResolveContext;
use std::sync::Arc;
use tcl_core_types::NameBytes;
use tcl_syntax::naming::NamePolicyProtocol;

/// Distinct original-vector queries. Argument geometry never selects a handler.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalWordSourcePurpose {
    Invocation,
    ArgumentEvaluation,
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
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
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
        result.lineage_within(MAX_ORIGINS).then_some(())?;
        let values = elements.iter().map(Self::bytes).collect::<Vec<_>>();
        let bytes = tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
            first.policy.string_protocol(),
        )
        .render(&values);
        (bytes.len() <= MAX_BYTES).then_some(())?;
        result.bytes = bytes.into();
        Some(result)
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
mod tests {
    use super::*;
    use crate::signature_scan::original_name::SignatureSourceNameKey;
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::{naming::ExecutionNamePolicy, word_rules::WordValueRules};

    fn value(source: &[u8], context: &ResolveContext) -> OriginalProducedNameValue {
        let policy = context
            .execution_name_policy
            .unwrap()
            .native_recipe()
            .unwrap();
        let config = LexerConfig::from_grammar(context.invocation_dialect.unwrap().lexer_grammar);
        let parsed = tcl_lexer::native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let key = SignatureSourceNameKey::from_original_native_word(
            &parsed.commands[0].words[0],
            WordValueRules::from_config(&config),
            policy,
        )
        .unwrap();
        OriginalProducedNameValue::from_source_input(
            &SignatureSourceNameInput::OriginalWord(key),
            context,
        )
        .unwrap()
    }

    #[test]
    fn original_contents_currency_preserves_cache_changes_and_retires_unknown_effects() {
        // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
        for policy in tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(NamePolicyProtocol::authored_tcl)
            .chain([NamePolicyProtocol::authored_jim084()])
        {
            let mut context = ResolveContext::default();
            context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
            context.invocation_dialect = Some(match policy.recipe() {
                tcl_syntax::naming::NativeNameProtocol::C(version) => {
                    tcl_registry::InvocationDialect::for_version(version)
                }
                tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                    tcl_registry::InvocationDialect::of_point(
                        tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
                    )
                }
            });
            let frozen = value(b"p\xff\0tail", &context);
            assert!(frozen.is_current(&context));
            context.invalidate_shared_representations();
            assert!(frozen.is_current(&context));
            context.invalidate_original_contents();
            assert!(!frozen.is_current(&context));
            let fresh = value(b"p\xff\0tail", &context);
            assert!(fresh.is_current(&context));
            assert_ne!(fresh, frozen);
        }
    }

    #[test]
    fn original_content_joins_keep_every_origin_and_do_not_reuse_unknown_currency() {
        // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let mut context = ResolveContext::default();
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        ));
        let plain = value(b"same", &context);
        let grouped = value(b"{same}", &context);
        let joined = plain.joined(&grouped).unwrap();
        assert_eq!(joined.bytes(), b"same");
        assert_eq!(joined.original_origins().len(), 2);
        assert!(plain.joined(&value(b"different", &context)).is_none());
        assert!(plain.concatenated(&value(b"\xff", &context)).is_none());
        assert!(plain.concatenated(&value(b"\0tail", &context)).is_none());
        let held = plain.concatenated(&value(b"{ suffix}", &context)).unwrap();
        assert_eq!(held.bytes(), b"same suffix");
        let mut changed = context.clone();
        changed.invalidate_original_contents();
        context.join(&changed);
        assert_eq!(context.original_contents_epoch(), None);
        context.invalidate_original_contents();
        assert_eq!(context.original_contents_epoch(), None);
        assert!(!held.is_current(&context));
    }

    #[test]
    fn original_evaluated_index_keeps_its_exact_word_and_arena() {
        // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let registry = tcl_registry::CommandRegistry::build_default();
            for (source, expected) in [
                ("set index k; set a(${index}) value", b"k".as_slice()),
                (
                    "set index k; set a(pre${index}post) value",
                    b"prekpost".as_slice(),
                ),
            ] {
                let analysis = super::super::SourceCommandBindings::analyse_with_options(
                    source, config, &registry, super::super::SourceAnalysisOptions {
                        invocation_dialect: Some(dialect),
                        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                            catch_depth: Some(0), loop_depth: 0,
                        },
                        ..Default::default()
                    },
                );
                let offset = u32::try_from(source.find("set a(").unwrap()).unwrap();
                let binding = analysis.invocation_at_source("set", offset);
                let tokens = binding.original_recorded_command_tokens().unwrap();
                let words = crate::registry_invocation::original_native_compiler_words(
                    &tcl_lexer::SourceImage::document(source),
                    tokens.words(),
                    offset,
                    config,
                )
                .unwrap();
                let word = &words[1];
                let tcl_syntax::native_variable_words::NativeVariableWordOperand::CompoundArray {
                    index,
                    ..
                } = tcl_syntax::native_variable_words::native_variable_word(
                    word,
                    version,
                    NamePolicyProtocol::authored_tcl(version).string_protocol(),
                )
                .unwrap()
                else {
                    panic!("expected original compound index");
                };
                let held = binding.frozen_written_names.as_ref().unwrap()[1]
                    .as_deref()
                    .unwrap();
                let context = &binding.variable_context;
                let child = held
                    .evaluated_variable_index(word, &index, context)
                    .unwrap();
                assert_eq!(child.bytes(), expected, "{version:?}/{source}");
                assert!(child.original_word_key().is_none());
                assert!(
                    held.evaluated_variable_index(&words[2], &index, context)
                        .is_none()
                );
                assert!(
                    held.concatenated(&value(b"suffix", context))
                        .unwrap()
                        .evaluated_variable_index(word, &index, context)
                        .is_none()
                );
                assert!(
                    OriginalProducedNameValue::list_result(std::slice::from_ref(held), held)
                        .unwrap()
                        .evaluated_variable_index(word, &index, context)
                        .is_none()
                );
                assert_eq!(
                    held.joined(held)
                        .unwrap()
                        .evaluated_variable_index(word, &index, context)
                        .unwrap(),
                    child
                );
                let mut changed = (**context).clone();
                changed.invalidate_original_contents();
                assert!(
                    held.evaluated_variable_index(word, &index, &changed)
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn original_child_lineage_is_bounded_across_repeated_capture() {
        // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
        let mut context = ResolveContext::default();
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
            NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
        ));
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        ));
        let mut held = value(b"7", &context);
        let mut captures = 0;
        loop {
            assert!(held.is_current(&context));
            assert_eq!(held.bytes(), b"7");
            let parent = crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(&held);
            let Some(child) = parent.list_element(0) else {
                break;
            };
            let input = SignatureSourceNameInput::OriginalValue(child);
            let Some(next) = OriginalProducedNameValue::from_source_input(&input, &context) else {
                break;
            };
            held = next;
            captures += 1;
            assert!(captures < MAX_ORIGINS);
        }
        assert!(captures > 0);
        assert!(held.lineage_within(MAX_ORIGINS));
        assert!(!held.lineage_within(1));
    }
}

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

/// Static pre-argv head with its complete original source-vector owner. This
/// value supplies lookup bytes, independently of selected compiler recipes.
pub(super) fn original_static_command_head_input(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &super::ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<SignatureSourceNameInput> {
    let original = state
        .current_source_origin
        .as_ref()?
        .original_lexical_command(words, offset, config)?;
    let native = original.native_words();
    let policy = state
        .source_variables
        .execution_name_policy?
        .native_recipe()?;
    crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
        native.first()?,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
        policy,
    )
    .map(SignatureSourceNameInput::OriginalWord)
}

/// Actual effective head after the complete written argv has been frozen.
/// Expanded heads retain their native list child; a captured byte value never
/// borrows a complete static word or a pre-argument compiler coordinate.
pub(super) fn original_command_head_input(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &super::ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> Option<SignatureSourceNameInput> {
    let written = context.written_arguments?;
    (written.len() == words.len()).then_some(())?;
    let policy = state
        .source_variables
        .execution_name_policy?
        .native_recipe()?;
    let origin = state.current_source_origin.as_ref()?;
    let image = origin.source_image();
    let owned;
    let native = if let Some(original) = context.original_written_projection {
        original.words_for(image, words, context.config)?
    } else {
        owned = origin.original_lexical_command(words, offset, context.config)?;
        owned.native_words()
    };
    (native.first()?.group().span.start() == offset).then_some(())?;
    for (ordinal, word) in written.iter().enumerate() {
        use crate::registry_invocation::EffectiveInvocationWord;
        if word.expansion_len() == Some(0) {
            continue;
        }
        if matches!(word, EffectiveInvocationWord::Expanded) {
            return None;
        }
        let original = native.get(ordinal)?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&context.config);
        let input = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(original, rules, policy)
            .map(SignatureSourceNameInput::OriginalWord)
            .or_else(|| crate::signature_scan::scope::SignatureSourceNameValue::from_original_static_word(original, rules, policy)
                .map(SignatureSourceNameInput::OriginalValue))
            .or_else(|| {
                let frozen = context.written_name_values?;
                (frozen.len() == words.len()).then_some(())?;
                let value = frozen.get(ordinal)?.as_deref()?;
                value.is_current(&state.source_variables).then(|| SignatureSourceNameInput::OriginalValue(
                    crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(value),
                ))
            })?;
        return match word.expansion_len() {
            Some(elements) => {
                let children = input.original_list_elements()?;
                (children.len() == elements).then_some(())?;
                children.into_iter().next()
            }
            None => Some(input),
        };
    }
    None
}

impl super::SourceInvocationBinding {
    /// Reborrow the complete original command through its retained vector.
    /// This projects tokens only and supplies no dispatch or completion grant.
    #[cfg(test)]
    pub(crate) fn original_recorded_command_tokens(&self) -> Option<crate::ir::CommandTokens> {
        self.original_recorded_command().map(|(_, tokens)| tokens)
    }

    pub(crate) fn original_recorded_command(
        &self,
    ) -> Option<(crate::segmenter::SegmentedCommand, crate::ir::CommandTokens)> {
        let site = self.invocation_site()?;
        let (words, config) = self.original_recorded_name_words()?;
        let image = site.source.source_image();
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let command = original.command();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            config,
            command,
        );
        (tokens.words() == words).then(|| (command.clone(), tokens))
    }

    /// Complete syntax configuration from this original word-vector owner.
    /// It grants syntax only, independently of any Native-purpose admission.
    pub(crate) fn original_lexer_config_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::LexerConfig> {
        if !OriginalWordSourcePurpose::Invocation.accepts(tokens) {
            return None;
        }
        self.original_word_vector_config(tokens)
    }

    /// Original word geometry for argument evaluation alone. An argv boundary
    /// retains its producer's vector, but never selects the original handler.
    pub(crate) fn original_argument_lexer_config(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::LexerConfig> {
        if !OriginalWordSourcePurpose::ArgumentEvaluation.accepts(tokens) {
            return None;
        }
        self.original_word_vector_config(tokens)
    }

    /// The same checked original argument vector's retained bytes and channel.
    /// Compiler-only syntax keeps its own image without issuing an invocation.
    pub(crate) fn original_argument_source_image(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_lexer::SourceImage> {
        self.original_argument_lexer_config(tokens)?;
        let source = if let Some(original) = &self.original_written_words {
            &original.site.source
        } else if let Some(original) = &self.original_compiler_words {
            &original.site.source
        } else {
            &self.invocation_site()?.source
        };
        Some(source.source_image())
    }

    fn original_word_vector_config(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::LexerConfig> {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            eprintln!(
                "ORIGINAL_WORD_CONFIG stage=request offset={:?} words={} synthetic={:?} written={} compiler={} declaration={}",
                tokens
                    .words()
                    .first()
                    .map(|word| word.source().span.start()),
                tokens.words().len(),
                tokens.synthetic,
                self.original_written_words.is_some(),
                self.original_compiler_words.is_some(),
                self.declaration_layout_observations.is_some(),
            );
        }
        if !tokens.original_words_match_argv_text() {
            #[cfg(test)]
            trace_original_word_config("compatibility_vector", tokens, false);
            return None;
        }
        #[cfg(test)]
        trace_original_word_config("compatibility_vector", tokens, true);
        if let Some(original) = &self.original_written_words {
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
                trace_original_word_config(
                    "written_site",
                    tokens,
                    self.invocation_site()
                        .is_some_and(|site| original.site == *site),
                );
                trace_original_word_config(
                    "written_vector",
                    tokens,
                    original.words.as_ref() == tokens.words(),
                );
            }
            return (original.site == *self.invocation_site()?
                && original.words.as_ref() == tokens.words())
            .then_some(original.config);
        }
        if let Some(original) = &self.original_compiler_words {
            // A compiler visit can own this vector after an abrupt runtime
            // prefix. Its exact snapshot supplies syntax, never invocation.
            #[cfg(test)]
            trace_original_word_config(
                "compiler_snapshot",
                tokens,
                self.compiler_lookup_state.is_some(),
            );
            let snapshot = self.compiler_lookup_state.as_ref()?;
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
                trace_original_word_config(
                    "compiler_origin",
                    tokens,
                    snapshot.state.current_source_origin.as_ref() == Some(&original.site.source),
                );
                trace_original_word_config(
                    "compiler_site",
                    tokens,
                    self.invocation_site()
                        .is_none_or(|site| site == &original.site),
                );
                trace_original_word_config(
                    "compiler_head",
                    tokens,
                    tokens
                        .words()
                        .first()
                        .is_some_and(|word| word.source().span.start() == original.site.offset),
                );
                trace_original_word_config(
                    "compiler_vector",
                    tokens,
                    original.words.as_ref() == tokens.words(),
                );
            }
            return (snapshot.state.current_source_origin.as_ref() == Some(&original.site.source)
                && self
                    .invocation_site()
                    .is_none_or(|site| site == &original.site)
                && tokens.words().first()?.source().span.start() == original.site.offset
                && original.words.as_ref() == tokens.words())
            .then_some(original.config);
        }
        #[cfg(test)]
        trace_original_word_config("declaration_site", tokens, self.invocation_site().is_some());
        let site = self.invocation_site()?;
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_word_config("declaration_rows", tokens, rows.clone().next().is_some());
        }
        let first = rows.clone().next()?;
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_word_config(
                "declaration_vector",
                tokens,
                first.words.as_ref() == tokens.words(),
            );
            trace_original_word_config(
                "declaration_origin",
                tokens,
                first.snapshot.state.current_source_origin.as_ref() == Some(&site.source),
            );
            trace_original_word_config(
                "declaration_consensus",
                tokens,
                rows.clone()
                    .all(|row| row.words == first.words && row.config == first.config),
            );
        }
        (first.words.as_ref() == tokens.words()
            && first.snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            && rows
                .clone()
                .all(|row| row.words == first.words && row.config == first.config))
        .then_some(first.config)
    }
    /// Same-vector source grammar from its original dispatch, compiler or
    /// declaration owner. This does not select a physical compiler engine.
    pub(crate) fn original_source_word_dialect_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_registry::InvocationDialect> {
        self.original_lexer_config_for_tokens(tokens)?;
        if let Some(original) = &self.original_written_words {
            return original.dialect;
        }
        if self.original_compiler_words.is_some() {
            return self.compiler_word_dialect();
        }
        let site = self.invocation_site()?;
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let dialect = rows.clone().next()?.snapshot.state.baseline.dialect?;
        rows.clone()
            .all(|row| {
                row.snapshot.state.baseline.dialect == Some(dialect)
                    && row.snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            })
            .then_some(dialect)
    }
    /// Exact original written operand, independently of effective alias or
    /// expansion ordinals. Readonly evaluated values retain their frozen graph
    /// and cannot become editable words, compiler operands or native argv.
    #[must_use]
    pub fn original_written_name_input(
        &self,
        tokens: &crate::ir::CommandTokens,
        written: usize,
    ) -> Option<SignatureSourceNameInput> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let (words, config) = self.original_recorded_name_words()?;
        (words == tokens.words()).then_some(())?;
        self.original_recorded_written_name_input(written, words, config)
    }

    /// Explicit retained Native/authored variable-name policy for the exact
    /// original vector. Logical source syntax supplies no Native name recipe.
    pub(crate) fn original_variable_reference_policy(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<NamePolicyProtocol> {
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let (words, original_config) = self.original_recorded_name_words()?;
        (words == tokens.words() && original_config == config).then_some(())?;
        matches!(
            self.original_recorded_name_purpose(words, config)?,
            OriginalRecordedNamePurpose::Native
        )
        .then_some(())?;
        self.variable_context.execution_name_policy?.native_recipe()
    }

    pub(crate) fn original_compound_array_argument(
        &self,
        tokens: &crate::ir::CommandTokens,
        written: usize,
        escapes: tcl_dialect::EscapeSyntax,
        rules: tcl_syntax::word_rules::WordValueRules,
    ) -> Option<crate::registry_invocation::EffectiveInvocationWord> {
        let policy = self.original_variable_reference_policy(tokens)?;
        let config = self.original_argument_lexer_config(tokens)?;
        if config.escapes != escapes
            || rules != tcl_syntax::word_rules::WordValueRules::from_config(&config)
        {
            return None;
        }
        let site = self.invocation_site()?;
        let original = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        crate::registry_invocation::original_array_element_word(
            original.native_words().get(written)?,
            policy,
        )
    }

    /// Readonly input at one exact extent in this invocation's original full
    /// vector. The source channel, complete config and current retained value
    /// are checked before the ordinary original-input factory is reused.
    pub(crate) fn original_written_name_input_at_invocation_span(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        span: tcl_lexer::Span,
    ) -> Option<SignatureSourceNameInput> {
        // Implementation contract: naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let site = self.invocation_site()?;
        (site.source.source_image() == image).then_some(())?;
        let (words, retained_config) = self.original_recorded_name_words()?;
        (retained_config == config).then_some(())?;
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let native = original.native_words();
        let mut ordinals =
            words
                .iter()
                .zip(native)
                .enumerate()
                .filter_map(|(index, (word, native))| {
                    (word.source().span == span
                        || native
                            .tokens()
                            .first()
                            .is_some_and(|token| token.span == span))
                    .then_some(index)
                });
        let written = ordinals.next()?;
        if ordinals.next().is_some() {
            return None;
        }
        self.original_recorded_written_name_input(written, words, config)
    }

    /// Original written operand from this binding's retained complete vector.
    /// No consumer token reconstruction or reporting spelling is accepted.
    #[must_use]
    pub fn original_retained_written_name_input(
        &self,
        written: usize,
    ) -> Option<SignatureSourceNameInput> {
        let (words, config) = self.original_recorded_name_words()?;
        self.original_recorded_written_name_input(written, words, config)
    }

    fn original_recorded_name_words(
        &self,
    ) -> Option<(&[crate::ir::WordExpr], tcl_lexer::LexerConfig)> {
        let site = self.invocation_site()?;
        if let Some(original) = &self.original_written_words {
            return (original.site == *site).then_some((original.words.as_ref(), original.config));
        }
        if let Some(original) = &self.original_compiler_words {
            (original.site == *site).then_some(())?;
            Some((&original.words, original.config))
        } else {
            let rows = super::declaration_layout::original_declaration_layouts(
                self.declaration_layout_observations.as_deref()?,
            )?;
            let first = rows.clone().next()?;
            (first.snapshot.state.current_source_origin.as_ref() == Some(&site.source)
                && rows
                    .clone()
                    .all(|row| row.words == first.words && row.config == first.config))
            .then_some(())?;
            Some((&first.words, first.config))
        }
    }

    /// Conditional receiver rows retain source geometry without original invocation authority.
    fn original_logical_source_geometry_words(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<(&[crate::ir::WordExpr], tcl_lexer::LexerConfig)> {
        input.has_logical_source_name_context().then_some(())?;
        if self.original_written_words.is_some() || self.original_compiler_words.is_some() {
            return self.original_recorded_name_words();
        }
        let site = self.invocation_site()?;
        let rows = super::declaration_layout::original_source_catalogue_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let first = rows.clone().next()?;
        let registry = input.borrowed_context_registry().commands();
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset)
                || row.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
                || row.config != input.lexer_config()
                || row.words != first.words
            {
                return None;
            }
            let owner = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .source_analysis_input()?;
            if owner != input && owner != &input.for_nested_source() {
                return None;
            }
            let metadata = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .metadata_context_for_source(registry, row.config, Some(owner.unit_profile()))?;
            if row
                .snapshot
                .state
                .retained_original_source_purpose(metadata, registry)?
                != super::OriginalInvocationSourcePurpose::Logical
            {
                return None;
            }
        }
        Some((&first.words, first.config))
    }

    /// Static naming units from a retained Logical source vector, independently
    /// of original invocation or Native value/currentness authority.
    fn original_logical_source_written_name_input(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        written: usize,
    ) -> Option<SignatureSourceNameInput> {
        if self.original_written_words.is_some() || self.original_compiler_words.is_some() {
            return None;
        }
        let (words, config) = self.original_logical_source_geometry_words(input)?;
        let site = self.invocation_site()?;
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let policy = tcl_registry::InvocationDialect::of_profile(input.analyser_profile())
            .authored_name_policy()?;
        if tcl_registry::InvocationDialect::of_profile(input.unit_profile()).authored_name_policy()
            != Some(policy)
        {
            return None;
        }
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        if !matches!(
            purpose,
            crate::signature_scan::original_name::OriginalSourceNamePurpose::Logical(_, _)
        ) {
            return None;
        }
        purpose
            .key(
                original.native_words().get(written)?,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            )
            .map(SignatureSourceNameInput::OriginalWord)
    }

    fn original_recorded_written_name_input(
        &self,
        written: usize,
        words: &[crate::ir::WordExpr],
        config: tcl_lexer::LexerConfig,
    ) -> Option<SignatureSourceNameInput> {
        use crate::signature_scan::scope::{SignatureSourceNameKey, SignatureSourceNameValue};
        let site = self.invocation_site()?;
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let word = original.native_words().get(written)?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        match self.original_recorded_name_purpose(words, config)? {
            OriginalRecordedNamePurpose::Logical(purpose) => {
                return purpose
                    .key(word, rules)
                    .map(SignatureSourceNameInput::OriginalWord);
            }
            OriginalRecordedNamePurpose::Native => {}
        }
        let policy = self
            .variable_context
            .execution_name_policy?
            .native_recipe()?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        if let Some(key) = SignatureSourceNameKey::from_original_native_word(word, rules, policy) {
            return Some(SignatureSourceNameInput::OriginalWord(key));
        }
        if let Some(value) =
            SignatureSourceNameValue::from_original_static_word(word, rules, policy)
        {
            return Some(SignatureSourceNameInput::OriginalValue(value));
        }
        let values = self.frozen_written_names.as_deref()?;
        let frozen = self.frozen_written_words.as_deref()?;
        (values.len() == words.len() && frozen.len() == values.len()).then_some(())?;
        let value = values.get(written)?.as_deref()?;
        value.is_current(&self.variable_context).then(|| {
            SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_produced_value(value),
            )
        })
    }

    fn original_recorded_name_purpose(
        &self,
        words: &[crate::ir::WordExpr],
        config: tcl_lexer::LexerConfig,
    ) -> Option<OriginalRecordedNamePurpose> {
        let candidate = if let Some(point) = &self.lookup_state {
            point
                .state
                .baseline
                .metadata_context
                .source_analysis_input()
        } else {
            self.declaration_layout_observations
                .as_deref()
                .and_then(super::declaration_layout::original_declaration_layouts)
                .and_then(|mut rows| rows.next())
                .and_then(|row| {
                    row.snapshot
                        .state
                        .baseline
                        .metadata_context
                        .source_analysis_input()
                })
        };
        let Some(candidate) = candidate.filter(|input| input.has_logical_source_name_context())
        else {
            return Some(OriginalRecordedNamePurpose::Native);
        };
        let (_, mut tokens) = self.original_recorded_command()?;
        tokens.source_binding = Some(self.clone());
        (tokens.words() == words && self.original_lexer_config_for_tokens(&tokens)? == config)
            .then_some(())?;
        let registry = candidate.borrowed_context_registry().commands();
        let metadata = self
            .original_source_metadata_for_advice(&tokens, registry)
            .ok()??;
        let input = metadata.source_analysis_input()?;
        input.has_logical_source_name_context().then_some(())?;
        match self.original_invocation_source_purpose(&tokens, metadata, registry)? {
            super::OriginalInvocationSourcePurpose::Logical => {}
            super::OriginalInvocationSourcePurpose::Execution
            | super::OriginalInvocationSourcePurpose::Standalone
            | super::OriginalInvocationSourcePurpose::AuthoredSimulation => {
                return Some(OriginalRecordedNamePurpose::Native);
            }
        }
        let policy = tcl_registry::InvocationDialect::of_profile(input.analyser_profile())
            .authored_name_policy()?;
        (tcl_registry::InvocationDialect::of_profile(input.unit_profile()).authored_name_policy()
            == Some(policy))
        .then_some(())?;
        Some(OriginalRecordedNamePurpose::Logical(
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            ),
        ))
    }

    /// Original head producer with the same full-vector correspondence as an
    /// operand. Equal command spelling supplies no substitute for this owner.
    #[must_use]
    pub fn original_head_name_input(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<SignatureSourceNameInput> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let (words, _) = self.original_recorded_name_words()?;
        (words == tokens.words()).then_some(())?;
        self.original_recorded_head_name_input()
    }

    /// Actual head producer from this binding's independently retained full
    /// original vector. This facade creates no command tokens or written word
    /// from a reporting name; readonly expanded and frozen values stay readonly.
    #[must_use]
    pub fn original_recorded_head_name_input(&self) -> Option<SignatureSourceNameInput> {
        let (words, config) = self.original_recorded_name_words()?;
        let Some(frozen) = self.frozen_written_words.as_deref() else {
            return self.original_recorded_written_name_input(0, words, config);
        };
        (frozen.len() == words.len()).then_some(())?;
        for (written, word) in frozen.iter().enumerate() {
            use crate::registry_invocation::EffectiveInvocationWord;
            if word.expansion_len() == Some(0) {
                continue;
            }
            if matches!(word, EffectiveInvocationWord::Expanded) {
                return None;
            }
            let input = self.original_recorded_written_name_input(written, words, config)?;
            return match word.expansion_len() {
                Some(elements) => {
                    let children = input.original_list_elements()?;
                    (children.len() == elements).then_some(())?;
                    children.into_iter().next()
                }
                None => Some(input),
            };
        }
        None
    }
}

impl super::SourceCommandBindings {
    pub(crate) fn retain_original_registered_instance_source(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        receipt: super::OriginalSourceRegisteredInstanceWords,
    ) -> Option<()> {
        let owner = self.original_analysis_owner.as_ref()?;
        if !owner.matches(self, input)
            || !std::sync::Arc::ptr_eq(&receipt.site().source, &owner.source)
            || !receipt.matches_source_context(
                owner.source.source_image(),
                owner.config,
                input.borrowed_context_registry(),
            )
        {
            return None;
        }
        let cache = &self.original_registered_instance_sources;
        let memo = owner.derived_memo();
        let linked = memo
            .registered_instance_sources
            .get_or_init(|| std::sync::Arc::downgrade(cache));
        if !std::sync::Weak::ptr_eq(linked, &std::sync::Arc::downgrade(cache)) {
            return None;
        }
        let mut entries = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match entries.entry(receipt.site().offset) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Some(std::sync::Arc::new(receipt)));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if entry.get().as_deref() != Some(&receipt) {
                    entry.insert(None);
                }
            }
        }
        Some(())
    }

    /// Whether the retained root analysis has this exact source image, input
    /// channel and complete lexer configuration. This is correspondence only;
    /// it supplies no completed-source, current value or invocation receipt.
    #[must_use]
    pub fn matches_original_source_image(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.lexer_config == Some(config)
            && self
                .root_origin
                .as_ref()
                .is_some_and(|origin| origin.source_image() == image)
    }

    /// One complete retained Logical command vector, including independent
    /// receiver source catalogues. The existing lexical owner reauthenticates
    /// every original word and the exact source/configuration/site. This grants
    /// readonly source geometry, without declaration, dispatch or Native entry.
    pub(crate) fn original_logical_source_command_at_in_source(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<std::sync::Arc<crate::registry_invocation::OriginalSourceCommandProjection>> {
        if !input.has_logical_source_name_context()
            || self.original_logical_source_name_advice_input() != Some(input)
            || input.lexer_config() != config
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let origin = self.source_origin()?;
        let binding = self.invocation_at_source("", offset);
        if binding.original_written_words.is_some()
            || binding.original_compiler_words.is_some()
            || binding.declaration_layout_observations.is_some()
        {
            let site = binding.invocation_site()?;
            if !std::sync::Arc::ptr_eq(&site.source, origin) || site.offset != offset {
                return None;
            }
            let (words, retained_config) = binding.original_logical_source_geometry_words(input)?;
            (retained_config == config).then_some(())?;
            return origin.original_lexical_command(words, offset, config);
        }
        self.original_source_transition_advice_tape_for(
            input.borrowed_context_registry(),
            super::OriginalSourceCataloguePurpose::CompletePrefix,
        )?
        .logical_original_command_at(origin, input, offset)
    }

    /// Require the complete caller image, including its input channel, before
    /// projecting a retained written value at a source extent.
    #[must_use]
    pub fn original_written_name_input_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SignatureSourceNameInput> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        self.original_written_name_input_at_span(span, config)
    }

    /// Original written input at an exact retained word or first-token extent.
    /// The authentic dispatch point supplies the complete frozen argv and
    /// current value context; a span alone cannot create a producer or word.
    #[must_use]
    pub fn original_written_name_input_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SignatureSourceNameInput> {
        let mut found = None;
        for (input, _, _) in self.original_written_name_observations_at_span(span, config)? {
            if found.as_ref().is_some_and(|previous| previous != &input) {
                return None;
            }
            found = Some(input);
        }
        found
    }

    /// Original operand and the actual caller's namespace geometry at the same
    /// complete retained invocation. This is readonly source correspondence;
    /// it grants no selected argument role, namespace existence or completion.
    #[must_use]
    pub fn original_namespace_operand_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNamespaceOperand> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let logical = self.original_logical_source_name_advice_input();
        let tape = match logical {
            Some(input) => Some(self.original_source_transition_advice_tape_for(
                input.borrowed_context_registry(),
                super::OriginalSourceCataloguePurpose::CompletePrefix,
            )?),
            None => None,
        };
        let mut found = None;
        for (input, binding, site) in
            self.original_written_name_observations_at_span(span, config)?
        {
            let scope = match (logical, tape.as_ref()) {
                (Some(owner), Some(tape)) => tape
                    .logical_variable_scope_at(owner, image, span, &input)?
                    .namespace(),
                (None, None) => binding.original_operand_naming_scope(&input)?,
                _ => return None,
            };
            let operand = crate::signature_scan::original_name::SourceOriginalNamespaceOperand::new(
                input.clone(),
                scope,
                site,
            );
            if found.as_ref().is_some_and(|previous| previous != &operand) {
                return None;
            }
            found = Some(operand);
        }
        found
    }

    fn original_written_name_observations_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<
        Vec<(
            SignatureSourceNameInput,
            super::SourceInvocationBinding,
            super::CommandAllocationSite,
        )>,
    > {
        self.original_written_word_observations_at_span(span, config)?
            .into_iter()
            .map(|(word, written, binding, site)| {
                let input = if let Some((words, retained_config)) =
                    binding.original_recorded_name_words()
                {
                    binding.original_recorded_written_name_input(written, words, retained_config)?
                } else if binding.declaration_layout_observations.is_some() {
                    binding.original_logical_source_written_name_input(
                        self.original_logical_source_name_advice_input()?, written,
                    )?
                } else {
                    let owner = self.original_logical_source_name_advice_input()?;
                    let policy = tcl_registry::InvocationDialect::of_profile(owner.analyser_profile())
                        .authored_name_policy()?;
                    if tcl_registry::InvocationDialect::of_profile(owner.unit_profile())
                        .authored_name_policy() != Some(policy) { return None; }
                    let purpose = crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                        owner, None, policy,
                    );
                    if !matches!(purpose, crate::signature_scan::original_name::OriginalSourceNamePurpose::Logical(_, _)) {
                        return None;
                    }
                    SignatureSourceNameInput::OriginalWord(purpose.key(
                        &word, tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    )?)
                };
                Some((input, binding, site))
            })
            .collect()
    }

    /// Original C array-name root with independently unknown compound index.
    /// Complete image/configuration/argv correspondence is retained; this
    /// projection supplies no whole argv value, element cell or compiler gate.
    #[must_use]
    pub fn original_c_array_operand_root_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<SignatureSourceNameInput> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, _, binding, _) in
            self.original_written_word_observations_at_span(span, config)?
        {
            if binding
                .variable_context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(policy)
            {
                return None;
            }
            let input = SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_variable_operand_root(
                    &word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    policy,
                )?,
            );
            if selected.as_ref().is_some_and(|previous| previous != &input) {
                return None;
            }
            selected = Some(input);
        }
        selected
    }

    /// Unanimous genuine whole source word and its original parent head.
    /// Complete source/channel/configuration correspondence is required. This
    /// geometry supplies no value, selected schema, name, cell or execution.
    #[must_use]
    pub fn original_written_word_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(tcl_lexer::NativeWord, u32)> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, _, _, site) in self.original_written_word_observations_at_span(span, config)? {
            (site.source.source_image() == image).then_some(())?;
            let current = (word, site.offset);
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        selected
    }

    /// Genuine ordinal-zero original whole head at this source occurrence.
    /// Token/content extents do not identify the head's ordinal or site. This
    /// geometry grants no handler, lookup, value, class or edit authority.
    #[must_use]
    pub fn original_written_head_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(tcl_lexer::NativeWord, super::CommandAllocationSite)> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, ordinal, _, site) in
            self.original_written_word_observations_at_span(span, config)?
        {
            if ordinal != 0 {
                continue;
            }
            (site.source.source_image() == image).then_some(())?;
            let current = (word, site);
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        selected
    }

    fn original_written_word_observations_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<
        Vec<(
            tcl_lexer::NativeWord,
            usize,
            super::SourceInvocationBinding,
            super::CommandAllocationSite,
        )>,
    > {
        let origin = self.root_origin.as_ref()?;
        let mut found = Vec::new();
        let mut offsets = self
            .dispatch_points
            .range(..=span.start())
            .map(|(offset, _)| *offset)
            .collect::<std::collections::BTreeSet<_>>();
        if self.original_logical_source_name_advice_input().is_some() {
            offsets.extend(self.declaration_layouts.iter().filter_map(|(site, _)| {
                (site.source == *origin && site.offset <= span.start()).then_some(site.offset)
            }));
        }
        for offset in offsets.iter().rev() {
            let binding = self.attach_invocation_reads(
                Self::query_dispatch_points(
                    self.dispatch_points_at(*offset)
                        .filter(|point| point.state.current_source_origin.as_ref() == Some(origin)),
                ),
                Some(origin),
                *offset,
            );
            let geometry = if let Some(input) = self.original_logical_source_name_advice_input() {
                binding.original_logical_source_geometry_words(input)
            } else {
                binding.original_recorded_name_words()
            };
            let Some((words, retained_config)) = geometry else {
                continue;
            };
            if retained_config != config
                || !words.iter().any(|word| {
                    word.original_written_span(origin.source_image().bytes())
                        .is_some_and(|written| {
                            written.start() <= span.start() && span.end() <= written.end()
                        })
                })
            {
                continue;
            }
            let original = origin.original_lexical_command(words, *offset, config)?;
            let native = original.native_words();
            let mut ordinals =
                words
                    .iter()
                    .zip(native)
                    .enumerate()
                    .filter_map(|(index, (word, native))| {
                        (word.source().span == span
                            || native.span() == span
                            || native.content_span().ok() == Some(span)
                            || native
                                .tokens()
                                .first()
                                .is_some_and(|token| token.span == span))
                        .then_some(index)
                    });
            let Some(written) = ordinals.next() else {
                continue;
            };
            if ordinals.next().is_some() {
                return None;
            }
            let site = binding.invocation_site()?.clone();
            found.push((native[written].clone(), written, binding, site));
        }
        if found.is_empty() {
            if let Some(input) = self.original_logical_source_name_advice_input() {
                if config != input.lexer_config() {
                    return None;
                }
                let tape = self.original_source_transition_advice_tape_for(
                    input.borrowed_context_registry(),
                    super::OriginalSourceCataloguePurpose::CompletePrefix,
                )?;
                for (word, written, site) in
                    tape.logical_original_word_observations_at_span(origin, input, span)?
                {
                    let binding = self.invocation_at_source("", site.offset);
                    // A present original owner stays terminal even when its checks
                    // refuse. Independent body geometry only fills an absent facet.
                    if binding.original_written_words.is_some()
                        || binding.original_compiler_words.is_some()
                        || binding.declaration_layout_observations.is_some()
                    {
                        continue;
                    }
                    found.push((word, written, binding, site));
                }
            }
        }
        (!found.is_empty()).then_some(found)
    }
}

#[cfg(test)]
mod graph_tests {
    use super::super::{Arc, SourceAnalysisOptions, SourceCommandBindings};
    use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationMode};

    fn analyse(
        source: &str,
        dialect: tcl_registry::InvocationDialect,
    ) -> (SourceCommandBindings, tcl_lexer::LexerConfig) {
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        (bindings, config)
    }

    fn tokens(
        source: &str,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> crate::ir::CommandTokens {
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let segment = segments
            .iter()
            .find(|segment| segment.span.start() == offset)
            .unwrap();
        crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            segment,
        )
    }

    fn dialects() -> impl Iterator<Item = tcl_registry::InvocationDialect> {
        tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(tcl_registry::InvocationDialect::for_version)
            .chain([tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
            )])
    }

    #[test]
    fn logical_original_word_geometry_includes_genuine_declaration_vectors() {
        let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let realm = analysis.retained_command_realm().unwrap();
        let config = analysis.body_lexer_config.unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let start = u32::try_from(source.find("set x AGAIN").unwrap() + "set ".len()).unwrap();
        let span = tcl_lexer::Span::new(start, start + 1);
        let (word, head) = realm
            .original_written_word_at_span_in_source(&image, span, config)
            .unwrap();
        assert_eq!(head, start - 4);
        assert_eq!(&image.bytes()[word.span().as_range()], b"x");
        assert!(
            realm
                .original_written_word_at_span_in_source(
                    &tcl_lexer::SourceImage::document(&format!("{source} ")),
                    span,
                    config,
                )
                .is_none()
        );
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            realm
                .original_written_word_at_span_in_source(&image, span, changed)
                .is_none()
        );
    }

    #[test]
    fn logical_receiver_word_geometry_and_write_card_keep_exact_source_currency() {
        let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let realm = analysis.retained_command_realm().unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let head = u32::try_from(source.find("set x AGAIN").unwrap()).unwrap();
        let span = tcl_lexer::Span::new(head + 4, head + 5);
        let original = realm
            .original_written_word_at_span_in_source(&image, span, config)
            .unwrap();
        assert_eq!(original.1, head);
        assert_eq!(&image.bytes()[original.0.span().as_range()], b"x");
        let card = analysis
            .original_variable_write_advice
            .iter()
            .find(|card| card.span() == span)
            .unwrap();
        assert!(analysis.logical_original_word_input_matches_source(
            &image,
            config,
            span,
            card.original_name_input()
        ));
        let stale = tcl_lexer::SourceImage::document(&source.replace("AGAIN", "OTHER"));
        assert!(
            realm
                .original_written_word_at_span_in_source(&stale, span, config)
                .is_none()
        );
        assert!(!analysis.logical_original_word_input_matches_source(
            &stale,
            config,
            span,
            card.original_name_input()
        ));
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            realm
                .original_written_word_at_span_in_source(&image, span, changed)
                .is_none()
        );
        assert!(!analysis.logical_original_word_input_matches_source(
            &image,
            changed,
            span,
            card.original_name_input()
        ));
    }

    #[test]
    fn logical_point_names_keep_original_source_words_and_strict_native_purpose() {
        let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let realm = analysis.retained_command_realm().unwrap();
        let bindings = realm.source_bindings_ref();
        let config = analysis.body_lexer_config.unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let offset = u32::try_from(source.find("set x AGAIN").unwrap()).unwrap();
        let span = tcl_lexer::Span::new(offset + 4, offset + 5);
        let input = bindings
            .original_written_name_input_at_span_in_source(&image, span, config)
            .expect("original static Logical point name");
        assert_eq!(input.bytes(), b"x");
        assert_eq!(
            input.policy().authority(),
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        );
        assert_eq!(
            input.original_word_key().unwrap().original_word().span(),
            span
        );
        let observations = bindings
            .original_written_word_observations_at_span(span, config)
            .expect("independently retained point geometry");
        assert!(observations.iter().all(|(_, _, binding, _)| {
            binding
                .variable_context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .is_none()
        }));
        assert!(
            bindings
                .original_written_name_input_at_span_in_source(
                    &tcl_lexer::SourceImage::document(&format!("{source} ")),
                    span,
                    config,
                )
                .is_none()
        );
        assert!(
            bindings
                .original_written_name_input_at_span_in_source(
                    &tcl_lexer::SourceImage::native(source.as_bytes()),
                    span,
                    config,
                )
                .is_none()
        );
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            bindings
                .original_written_name_input_at_span_in_source(&image, span, changed)
                .is_none()
        );
    }

    #[test]
    fn logical_conditional_receiver_point_names_keep_source_only_authority() {
        let source = include_str!(
            "../../../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
        );
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let owner = analysis.resolved_input.as_ref().unwrap();
        assert!(owner.has_logical_source_name_context());
        let realm = analysis.retained_command_realm().unwrap();
        let bindings = realm.source_bindings_ref();
        let image = tcl_lexer::SourceImage::document(source);
        let start = u32::try_from(source.find("after idle $cb").unwrap() + "after ".len()).unwrap();
        let span = tcl_lexer::Span::new(start, start + 4);
        let observations = bindings
            .original_written_word_observations_at_span(span, owner.lexer_config())
            .expect("genuine retained receiver source vector");
        let (word, written, binding, _) = observations.iter().find(|(_, _, binding, _)| {
            binding.declaration_layout_observations.as_deref().is_some_and(|rows| {
                rows.iter().any(|row| row.issuer == super::super::declaration_layout::DeclarationLayoutIssuer::ConditionalReceiverSource)
            })
        }).expect("actual independently retained ConditionalReceiverSource observation");
        assert!(binding.original_recorded_name_words().is_none());
        let input = bindings
            .original_written_name_input_at_span_in_source(&image, span, owner.lexer_config())
            .expect("static name does not require declaration-only invocation words");
        assert_eq!(input.bytes(), b"idle");
        assert_eq!(input.original_word_key().unwrap().original_word(), word);
        assert_eq!(
            input.policy().authority(),
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        );
        assert!(
            binding
                .original_retained_written_name_input(*written)
                .is_none()
        );
        assert!(
            bindings
                .original_namespace_operand_at_span_in_source(&image, span, owner.lexer_config())
                .is_none()
        );
        let mut changed = binding.clone();
        let rows =
            std::sync::Arc::make_mut(changed.declaration_layout_observations.as_mut().unwrap());
        rows[0].config.strict_quoting = !rows[0].config.strict_quoting;
        assert!(
            changed
                .original_logical_source_written_name_input(owner, *written)
                .is_none()
        );
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            owner.analyser_profile(),
            owner.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            owner.lexer_config(),
        );
        assert!(
            binding
                .original_logical_source_written_name_input(&foreign, *written)
                .is_none()
        );
    }

    #[test]
    fn registered_instance_sources_keep_original_identity_and_source_owner_lifetime() {
        let source = "entry .e; .e configure -textvariable value";
        let (bindings, tokens, input) = {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            let input = analysis.resolved_input.as_ref().unwrap().clone();
            let realm = analysis.retained_command_realm().unwrap();
            let offset = u32::try_from(source.find(".e configure").unwrap()).unwrap();
            let command = crate::segmenter::segment_commands_with_offset_and_config(
                &source[offset as usize..],
                offset,
                input.lexer_config(),
            )
            .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                input.lexer_config(),
                &command,
            );
            realm.stamp_original_tokens(&mut tokens);
            let before = tokens.source_binding.clone();
            realm.stamp_original_tokens(&mut tokens);
            assert_eq!(
                tokens.source_binding, before,
                "cache warmth cannot change invocation identity"
            );
            (realm.source_bindings_ref().clone(), tokens, input)
        };
        let registry = input.borrowed_context_registry().commands();
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &input,
        )
        .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let receipt = binding
            .original_registered_instance_source_for_tokens(&tokens, metadata, registry)
            .expect(
                "the retained source bindings keep original instance metadata after realm drop",
            );
        assert_eq!(receipt.instance().factory().command(), "entry");
        assert_eq!(receipt.original_words().len(), 4);
        assert_eq!(
            receipt.argument_word(2).unwrap().try_text().unwrap(),
            "value"
        );
        assert_eq!(
            receipt.with_source_schema(input.borrowed_context_registry(), |schema| schema
                .authored_source_option_variable_scope_at(2)),
            Some(Some(tcl_registry::VariableScope::Global))
        );
        let mut foreign_cache = bindings.clone();
        foreign_cache.original_registered_instance_sources = std::sync::Arc::default();
        assert!(
            foreign_cache
                .retain_original_registered_instance_source(&input, receipt.as_ref().clone())
                .is_none(),
            "an established owner link cannot be overwritten by another optional cache"
        );
        assert!(
            binding
                .original_registered_instance_source_for_tokens(&tokens, metadata, registry)
                .is_some()
        );
        let mut changed = tokens.clone();
        changed.argv_texts[3] = "foreign".into();
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_registered_instance_source_for_tokens(&changed, metadata, registry)
                .is_none()
        );
        let standalone = crate::registry_invocation::InvocationMetadataContext::from(
            input.borrowed_context_registry(),
        );
        assert!(
            binding
                .original_registered_instance_source_for_tokens(&tokens, standalone, registry)
                .is_none()
        );
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let metadata_foreign =
            crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                registry, &foreign,
            )
            .unwrap();
        assert!(
            binding
                .original_registered_instance_source_for_tokens(&tokens, metadata_foreign, registry)
                .is_none()
        );
        let cache = std::sync::Arc::downgrade(&bindings.original_registered_instance_sources);
        drop(foreign_cache);
        drop(bindings);
        assert!(
            cache.upgrade().is_none(),
            "no receipt/analysis-owner strong cycle retains derived source authority"
        );
        assert!(
            binding
                .original_registered_instance_source_for_tokens(&tokens, metadata, registry)
                .is_none(),
            "the surviving token and readonly receipt cannot upgrade a dropped source owner"
        );
    }

    #[test]
    fn logical_source_body_vectors_reborrow_existing_tape_owners_without_native_entry() {
        for (source, command, expected) in [
            (
                "namespace eval N {set x VALUE}",
                "set x VALUE",
                vec!["set", "x", "VALUE"],
            ),
            (
                "proc p {} {$dispatch $value}",
                "$dispatch $value",
                vec!["$dispatch", "$value"],
            ),
            (
                "oo::class create C {method run {} {set local VALUE}}",
                "set local VALUE",
                vec!["set", "local", "VALUE"],
            ),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            let input = analysis.resolved_input.as_ref().unwrap();
            let bindings = analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref();
            let image = tcl_lexer::SourceImage::document(source);
            let offset = u32::try_from(source.find(command).unwrap()).unwrap();
            let original = bindings
                .original_logical_source_command_at_in_source(
                    input,
                    &image,
                    offset,
                    input.lexer_config(),
                )
                .expect("genuine original body vector remains independent of invocation selection");
            assert_eq!(
                original
                    .native_words()
                    .iter()
                    .map(|word| word.try_text().unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(
                original
                    .native_words()
                    .iter()
                    .all(|word| word.image() == &image && word.config() == input.lexer_config())
            );
            assert!(
                bindings
                    .invocation_at_source("", offset)
                    .original_recorded_command()
                    .is_none(),
                "readonly geometry grants no declaration or invocation words"
            );
            for stale in [
                tcl_lexer::SourceImage::native(source.as_bytes()),
                tcl_lexer::SourceImage::document(&format!("{source} ")),
            ] {
                assert!(
                    bindings
                        .original_logical_source_command_at_in_source(
                            input,
                            &stale,
                            offset,
                            input.lexer_config()
                        )
                        .is_none()
                );
            }
            let mut config = input.lexer_config();
            config.strict_quoting = !config.strict_quoting;
            assert!(
                bindings
                    .original_logical_source_command_at_in_source(input, &image, offset, config)
                    .is_none()
            );
        }
    }

    #[test]
    fn logical_source_command_vectors_keep_genuine_conditional_receiver_geometry() {
        let source = include_str!(
            "../../../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
        );
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let image = tcl_lexer::SourceImage::document(source);
        let offset = u32::try_from(source.find("after idle $cb").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("", offset);
        assert!(binding.declaration_layout_observations.as_deref().unwrap().iter().any(|row| {
            row.issuer == super::super::declaration_layout::DeclarationLayoutIssuer::ConditionalReceiverSource
        }), "actual independently retained receiver source issuer");
        assert!(binding.original_recorded_command().is_none());
        let original = bindings
            .original_logical_source_command_at_in_source(
                input,
                &image,
                offset,
                input.lexer_config(),
            )
            .expect("one genuine complete receiver source vector");
        assert_eq!(
            original
                .native_words()
                .iter()
                .map(|word| word.try_text().unwrap())
                .collect::<Vec<_>>(),
            ["after", "idle", "$cb"]
        );
        assert_eq!(original.command().argv.len(), original.native_words().len());
        assert!(
            original
                .native_words()
                .iter()
                .all(|word| word.image() == &image && word.config() == input.lexer_config())
        );
        assert!(
            binding.original_recorded_command().is_none(),
            "geometry cannot promote declaration authority"
        );
        for image in [
            tcl_lexer::SourceImage::document(&format!("{source} ")),
            tcl_lexer::SourceImage::native(source.as_bytes()),
        ] {
            assert!(
                bindings
                    .original_logical_source_command_at_in_source(
                        input,
                        &image,
                        offset,
                        input.lexer_config()
                    )
                    .is_none()
            );
        }
        let mut changed = input.lexer_config();
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            bindings
                .original_logical_source_command_at_in_source(input, &image, offset, changed)
                .is_none()
        );
        assert!(
            bindings
                .original_logical_source_command_at_in_source(
                    input,
                    &image,
                    offset + 6,
                    input.lexer_config()
                )
                .is_none()
        );
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            input.lexer_config(),
        );
        assert!(
            bindings
                .original_logical_source_command_at_in_source(
                    &foreign,
                    &image,
                    offset,
                    input.lexer_config()
                )
                .is_none()
        );
        for change_config in [true, false] {
            let mut changed = bindings.clone();
            let rows = changed
                .declaration_layouts
                .get_mut(binding.invocation_site().unwrap())
                .unwrap();
            let mut conflict = rows.iter().find(|row| row.issuer == super::super::declaration_layout::DeclarationLayoutIssuer::ConditionalReceiverSource).unwrap().clone();
            if change_config {
                conflict.config.strict_quoting = !conflict.config.strict_quoting;
            } else {
                conflict.words = std::sync::Arc::from(&conflict.words[..conflict.words.len() - 1]);
            }
            rows.push(conflict);
            assert!(
                changed
                    .original_logical_source_command_at_in_source(
                        input,
                        &image,
                        offset,
                        input.lexer_config()
                    )
                    .is_none(),
                "conflicting retained config/vector withdraws the projection"
            );
        }
        let native = crate::analyser::Analyser::new().analyse("list VALUE", "tcl8.6");
        let input = native.resolved_input.as_ref().unwrap();
        let bindings = native
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        assert!(
            bindings
                .invocation_at_source("", 0)
                .original_recorded_command()
                .is_some()
        );
        assert!(
            bindings
                .original_logical_source_command_at_in_source(
                    input,
                    &tcl_lexer::SourceImage::document("list VALUE"),
                    0,
                    input.lexer_config()
                )
                .is_none()
        );
    }

    #[test]
    fn logical_namespace_point_operands_use_independent_original_frame_recipes() {
        use crate::signature_scan::scope::SignatureNamespaceScope;
        for (source, home) in [
            ("namespace eval Target {}", None),
            ("namespace eval N {namespace eval Target {}}", Some("N")),
            (
                "namespace eval N {proc p {} {namespace eval Target {}}}",
                Some("N"),
            ),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            let bindings = analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref();
            let config = analysis.body_lexer_config.unwrap();
            let image = tcl_lexer::SourceImage::document(source);
            let start = u32::try_from(source.find("Target").unwrap()).unwrap();
            let span = tcl_lexer::Span::new(start, start + 6);
            let operand = bindings
                .original_namespace_operand_at_span_in_source(&image, span, config)
                .unwrap_or_else(|| panic!("genuine source namespace recipe: {source}"));
            assert_eq!(operand.name_input().bytes(), b"Target");
            let SignatureNamespaceScope::C(namespace) = operand.context() else {
                panic!("Logical C source recipe");
            };
            assert_eq!(
                namespace.as_segments(),
                home.into_iter()
                    .map(tcl_core_types::NameBytes::from)
                    .collect::<Vec<_>>()
            );
            assert!(
                bindings
                    .original_namespace_operand_at_span_in_source(
                        &tcl_lexer::SourceImage::document(&format!("{source} ")),
                        span,
                        config,
                    )
                    .is_none()
            );
            let mut changed = config;
            changed.strict_quoting = !changed.strict_quoting;
            assert!(
                bindings
                    .original_namespace_operand_at_span_in_source(&image, span, changed)
                    .is_none()
            );
        }
        for source in [
            "namespace eval $unknown {namespace eval Target {}}",
            "oo::class create C {method m {} {namespace eval Target {}}}",
            "set inert {namespace eval Target {}}",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            let config = analysis.body_lexer_config.unwrap();
            let start = u32::try_from(source.find("Target").unwrap()).unwrap();
            assert!(
                analysis
                    .retained_command_realm()
                    .unwrap()
                    .source_bindings_ref()
                    .original_namespace_operand_at_span_in_source(
                        &tcl_lexer::SourceImage::document(source),
                        tcl_lexer::Span::new(start, start + 6),
                        config,
                    )
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn logical_recorded_name_operands_use_the_checked_source_purpose() {
        let source = "proc f {value} {return $value}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let realm = analysis.retained_command_realm().unwrap();
        let binding = realm.invocation_at_source("", 0);
        let (_, tokens) = binding.original_recorded_command().unwrap();
        let input = binding.original_written_name_input(&tokens, 1).unwrap();
        assert_eq!(input.bytes(), b"f");
        assert_eq!(
            input.policy().authority(),
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        );
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "other".to_owned();
        assert!(binding.original_written_name_input(&changed, 1).is_none());
    }

    #[test]
    fn original_word_issuers_require_the_complete_current_compatibility_vector() {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        for dialect in dialects() {
            let source = "list ONE";
            let (bindings, config) = analyse(source, dialect);
            let original = tokens(source, config, 0);
            let binding = bindings.invocation_at_source("list", 0);
            assert_eq!(
                binding.original_lexer_config_for_tokens(&original),
                Some(config)
            );
            assert!(binding.original_written_name_input(&original, 1).is_some());
            assert!(binding.original_head_name_input(&original).is_some());
            for axis in 0..4 {
                let mut changed = original.clone();
                match axis {
                    0 => changed.argv_texts[1] = "OTHER".to_owned(),
                    1 => {
                        changed.argv_texts.pop();
                    }
                    2 => {
                        changed.word_exprs.pop();
                    }
                    _ => {
                        let source = changed.word_exprs[1].source().clone();
                        changed.word_exprs[1] = crate::ir::WordExpr::Opaque {
                            text: "OTHER".to_owned(),
                            source,
                            reason: crate::ir::WordOpacity::LossySnapshot,
                        };
                    }
                }
                assert!(
                    binding.original_lexer_config_for_tokens(&changed).is_none(),
                    "axis={axis}"
                );
                assert!(
                    binding.original_written_name_input(&changed, 1).is_none(),
                    "axis={axis}"
                );
                assert!(
                    binding.original_head_name_input(&changed).is_none(),
                    "axis={axis}"
                );
            }
        }
    }

    #[test]
    fn original_frozen_list_result_keeps_native_units_and_full_vector_owner() {
        let source = r"list [list p\u0000tail p\uD800]";
        for dialect in dialects() {
            let (bindings, config) = analyse(source, dialect);
            let original = tokens(source, config, 0);
            let binding = bindings.invocation_at_source("list", 0);
            let input = binding
                .original_written_name_input(&original, 1)
                .expect("exact nested List result");
            assert!(input.original_word_key().is_none());
            assert!(input.bytes().starts_with(
                if dialect.authored_name_policy().unwrap().recipe()
                    == tcl_syntax::naming::NativeNameProtocol::Jim084
                {
                    b"p\0tail".as_slice()
                } else {
                    b"p\xc0\x80tail".as_slice()
                }
            ));
            let shortened = tokens("list", config, 0);
            assert!(binding.original_written_name_input(&shortened, 0).is_none());
        }
    }

    #[test]
    fn original_copied_store_keeps_data_separate_from_read_and_word_authority() {
        let source = r"set name p\uD800; set copy $name; list $copy";
        for dialect in dialects() {
            let (bindings, config) = analyse(source, dialect);
            let offset = u32::try_from(source.rfind("list").unwrap()).unwrap();
            let original = tokens(source, config, offset);
            let binding = bindings.invocation_at_source("list", offset);
            let input = binding
                .original_written_name_input(&original, 1)
                .expect("current exact stored byte read");
            assert!(input.original_word_key().is_none());
            assert!(!input.bytes().is_empty());
            let held = input.produced_value().unwrap();
            assert!(held.is_current(&binding.variable_context));
            let mut changed = binding.clone();
            Arc::make_mut(&mut changed.variable_context).invalidate_original_contents();
            assert!(changed.original_written_name_input(&original, 1).is_none());
        }
    }

    #[test]
    fn original_retained_span_input_requires_exact_image_config_and_word() {
        // Implementation contract: naming.source.readonly-original-operand-projections (docs/design/analysis/name-resolution-proofs/readonly-original-operand-projections.md).
        let source = "set name {::A}; list $name";
        for dialect in dialects() {
            let (bindings, config) = analyse(source, dialect);
            let offset = u32::try_from(source.rfind("list").unwrap()).unwrap();
            let original = tokens(source, config, offset);
            let binding = bindings.invocation_at_source("list", offset);
            let expected = binding.original_written_name_input(&original, 1).unwrap();
            let span = original.words()[1].source().span;
            let image = tcl_lexer::SourceImage::document(source);
            assert!(bindings.matches_original_source_image(&image, config));
            assert!(!bindings.matches_original_source_image(
                &tcl_lexer::SourceImage::native(source.as_bytes()),
                config,
            ));
            assert!(!bindings.matches_original_source_image(
                &image,
                tcl_lexer::LexerConfig {
                    strict_quoting: !config.strict_quoting,
                    ..config
                },
            ));
            let input = bindings
                .original_written_name_input_at_span_in_source(&image, span, config)
                .unwrap();
            assert_eq!(input, expected);
            assert!(input.original_word_key().is_none());
            assert_eq!(input.bytes(), b"::A");
            assert!(
                bindings
                    .original_written_name_input_at_span_in_source(
                        &tcl_lexer::SourceImage::native(source.as_bytes()),
                        span,
                        config,
                    )
                    .is_none()
            );
            assert!(
                bindings
                    .original_written_name_input_at_span_in_source(
                        &tcl_lexer::SourceImage::document(&source.replace("::A", "::B")),
                        span,
                        config,
                    )
                    .is_none()
            );
            assert!(
                bindings
                    .original_written_name_input_at_span(
                        span,
                        tcl_lexer::LexerConfig {
                            strict_quoting: !config.strict_quoting,
                            ..config
                        },
                    )
                    .is_none()
            );
            assert!(
                bindings
                    .original_written_name_input_at_span(
                        tcl_lexer::Span::new(span.start() + 1, span.end()),
                        config,
                    )
                    .is_none()
            );
        }
    }

    #[test]
    fn original_effective_head_keeps_computed_and_expanded_producers() {
        // Implementation contract: naming.source.readonly-original-operand-projections (docs/design/analysis/name-resolution-proofs/readonly-original-operand-projections.md).
        for dialect in dialects() {
            let source = "set head list; $head VALUE";
            let (bindings, config) = analyse(source, dialect);
            let offset = u32::try_from(source.find("$head").unwrap()).unwrap();
            let original = tokens(source, config, offset);
            let binding = bindings.invocation_at_source("list", offset);
            let input = binding.original_head_name_input(&original).unwrap();
            assert_eq!(input.bytes(), b"list");
            assert!(input.original_word_key().is_none());
            let mut changed = binding.clone();
            Arc::make_mut(&mut changed.variable_context).invalidate_original_contents();
            assert!(changed.original_head_name_input(&original).is_none());
            if !config.expand_syntax {
                continue;
            }
            let source = "{*}{} {*}{list} VALUE";
            let (bindings, config) = analyse(source, dialect);
            let original = tokens(source, config, 0);
            let binding = bindings.invocation_at_source("list", 0);
            let input = binding.original_head_name_input(&original).unwrap();
            assert_eq!(input.bytes(), b"list");
            assert!(input.original_word_key().is_none());
            let container = input.original_static_list_container().unwrap();
            assert_eq!(container.ordinals(), &[0]);
            assert_eq!(
                container.parent_word().span(),
                crate::registry_invocation::original_native_compiler_words(
                    &tcl_lexer::SourceImage::document(source),
                    original.words(),
                    0,
                    config,
                )
                .unwrap()[1]
                    .span()
            );
        }
    }
}

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
                        context.compilation,
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
            compilation: context.compilation,
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
