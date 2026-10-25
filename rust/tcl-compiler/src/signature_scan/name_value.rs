// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly native naming values with original fragment/list producer lineage.

use super::original_name::SignatureSourceNameKey;
use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_syntax::{
    naming::{NamePolicyProtocol, NativeNameProtocol},
    word_rules::WordValueRules,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceNameValueOrigin {
    Evaluated(Box<crate::command_binding::original_name_value::OriginalProducedNameValue>),
    JimStockPublication(Box<crate::command_binding::OriginalJimStockName>),
    ObjectCommandNameResult {
        receipt: Box<crate::command_binding::named_manufacture::OriginalObjectCommandNameResult>,
        contents_epoch: u64,
    },
    StaticWord {
        word: NativeWord,
        rules: WordValueRules,
    },
    ScriptWord {
        parent: Box<SignatureSourceNameValue>,
        body: tcl_lexer::Span,
        word: NativeWord,
    },
    TextFragment {
        word: NativeWord,
        span: tcl_lexer::Span,
    },
    VariableSubstitutionIndex {
        root: Box<super::variable_name::SignatureSourceVariableRoot>,
    },
    ExecutableTextFragment {
        arena: tcl_lexer::ExecutablePartArena,
        span: tcl_lexer::Span,
    },
    VariableIndex {
        word: NativeWord,
        root: NameBytes,
    },
    VariableOperandRoot {
        word: NativeWord,
        name_span: Span,
    },
    SourceLine {
        image: SourceImage,
        line: usize,
        config: LexerConfig,
    },
    ListElement {
        parent: Box<SignatureSourceNameValue>,
        ordinal: usize,
    },
    DictionaryVariableElement {
        parent: Box<SignatureSourceNameValue>,
        ordinal: usize,
        root: Box<crate::var_resolve::OriginalNameValueRead>,
        receiver: crate::place::Place,
    },
    StoredVariableTracePrefix {
        parents: Vec<SignatureSourceNameInput>,
        dialect: tcl_registry::InvocationDialect,
    },
}

/// An immutable source-produced byte value. Fragment/list-child geometry is
/// distinct from a complete original word and grants no edit, normal execution,
/// captured variable epoch, command allocation or native argument authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceNameValue {
    origin: SourceNameValueOrigin,
    bytes: NameBytes,
    policy: NamePolicyProtocol,
}

/// Readonly correspondence to a complete static list container. Child ordinals
/// describe native parsing; they supply neither child source spans nor edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureSourceStaticListContainer<'a> {
    word: &'a NativeWord,
    ordinals: Vec<usize>,
}

impl<'a> SignatureSourceStaticListContainer<'a> {
    /// Complete original static parent, with its own image and configuration.
    #[must_use]
    pub fn parent_word(&self) -> &'a NativeWord {
        self.word
    }

    /// Native list ordinals from that parent to the selected readonly child.
    #[must_use]
    pub fn ordinals(&self) -> &[usize] {
        &self.ordinals
    }
}

impl SignatureSourceNameValue {
    /// Borrow the sealed source producer, independently of current dispatch.
    pub(crate) fn jim_stock_name(&self) -> Option<&crate::command_binding::OriginalJimStockName> {
        match &self.origin {
            SourceNameValueOrigin::JimStockPublication(name) => Some(name),
            _ => None,
        }
    }

    /// Counted helper-name data from a sealed stock source publication recipe.
    /// This issues no complete word, editable geometry, Native argument or lookup.
    pub(crate) fn from_jim_stock_name(name: &crate::command_binding::OriginalJimStockName) -> Self {
        Self {
            origin: SourceNameValueOrigin::JimStockPublication(Box::new(name.clone())),
            bytes: name.bytes().into(),
            policy: name.policy(),
        }
    }

    /// Trace registration copies counted data. Its original producer receipts
    /// remain historical; subsequent writes to the producer cell do not alter
    /// the registered bytes or supply registration liveness.
    pub(crate) fn copied_variable_trace_prefix(
        input: &SignatureSourceNameInput,
        context: &crate::var_resolve::ResolveContext,
    ) -> Option<Self> {
        input.is_current(context).then_some(())?;
        let dialect = context.invocation_dialect?;
        let protocol = dialect.native_variable_trace_protocol()?;
        (input.bytes().len() <= crate::command_binding::original_name_value::MAX_BYTES)
            .then_some(())?;
        let value = Self {
            origin: SourceNameValueOrigin::StoredVariableTracePrefix {
                parents: vec![input.clone()],
                dialect,
            },
            bytes: protocol.variable_prefix_storage(input.bytes()).into(),
            policy: input.policy(),
        };
        value
            .lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS)
            .then_some(value)
    }

    pub(crate) fn joined_variable_trace_prefix(&self, other: &Self) -> Option<Self> {
        let SourceNameValueOrigin::StoredVariableTracePrefix { parents, dialect } = &self.origin
        else {
            return None;
        };
        let SourceNameValueOrigin::StoredVariableTracePrefix {
            parents: other_parents,
            dialect: other_dialect,
        } = &other.origin
        else {
            return None;
        };
        if self.bytes != other.bytes || self.policy != other.policy || dialect != other_dialect {
            return None;
        }
        let mut parents = parents.clone();
        for parent in other_parents {
            if !parents.contains(parent) {
                parents.push(parent.clone());
            }
        }
        let joined = Self {
            origin: SourceNameValueOrigin::StoredVariableTracePrefix {
                parents,
                dialect: *dialect,
            },
            bytes: self.bytes.clone(),
            policy: self.policy,
        };
        joined
            .lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS)
            .then_some(joined)
    }

    /// Static native list ancestry only. Evaluated values and fragments lack
    /// an original complete container and cannot receive its source geometry.
    #[must_use]
    pub fn original_static_list_container(&self) -> Option<SignatureSourceStaticListContainer<'_>> {
        self.lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS)
            .then_some(())?;
        match &self.origin {
            SourceNameValueOrigin::StaticWord { word, .. } => {
                Some(SignatureSourceStaticListContainer {
                    word,
                    ordinals: Vec::new(),
                })
            }
            SourceNameValueOrigin::ListElement { parent, ordinal } => {
                let mut container = parent.original_static_list_container()?;
                container.ordinals.push(*ordinal);
                Some(container)
            }
            _ => None,
        }
    }

    pub(crate) fn lineage_within(&self, budget: usize) -> bool {
        let mut remaining = budget;
        self.lineage_nodes_in(&mut remaining)
    }

    /// Readonly evaluated bytes retain their complete immutable producer graph;
    /// no complete-word/edit/native authority is inherited by the consumer.
    pub(crate) fn from_original_produced_value(
        value: &crate::command_binding::original_name_value::OriginalProducedNameValue,
    ) -> Self {
        Self {
            origin: SourceNameValueOrigin::Evaluated(Box::new(value.clone())),
            bytes: value.bytes().into(),
            policy: value.policy(),
        }
    }

    /// The full-name result of an independently completed object constructor.
    /// The retained allocation is historical producer identity; later known
    /// table changes do not change these held bytes or grant current liveness.
    pub(crate) fn from_object_command_name_result(
        receipt: &crate::command_binding::named_manufacture::OriginalObjectCommandNameResult,
        context: &crate::var_resolve::ResolveContext,
    ) -> Option<Self> {
        // Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
        if !receipt.requested_name_input().is_current(context)
            || context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(receipt.policy())
        {
            return None;
        }
        let bytes = tcl_syntax::naming::native_command_full_name_bytes(receipt.slot());
        (bytes.len() <= crate::command_binding::original_name_value::MAX_BYTES).then_some(())?;
        let value = Self {
            origin: SourceNameValueOrigin::ObjectCommandNameResult {
                receipt: Box::new(receipt.clone()),
                contents_epoch: context.original_contents_epoch()?,
            },
            bytes: bytes.into(),
            policy: receipt.policy(),
        };
        value
            .lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS)
            .then_some(value)
    }

    /// Static pre-expansion value, independently of expansion cardinality.
    pub(crate) fn from_original_static_word(
        word: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if rules != WordValueRules::from_config(&word.config())
            || word.config().escapes != policy.string_protocol().escape_syntax()
        {
            return None;
        }
        let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            std::slice::from_ref(word),
            policy.string_protocol(),
        )
        .ok()?;
        Some(Self {
            origin: SourceNameValueOrigin::StaticWord {
                word: word.clone(),
                rules,
            },
            bytes: words.literal(0)?.into(),
            policy,
        })
    }

    /// Static word parsed from an exact byte slice of this original static
    /// value. The retained parent owns the slice; its parsed offsets are
    /// readonly value geometry and never original editable source positions.
    pub(crate) fn script_word(&self, body: tcl_lexer::Span, word: &NativeWord) -> Option<Self> {
        self.lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS - 1)
            .then_some(())?;
        let SourceNameValueOrigin::StaticWord { word: original, .. } = &self.origin else {
            return None;
        };
        if word.config() != original.config()
            || word.image().channel() != tcl_lexer::SourceChannel::NativeValue
            || self.bytes.as_bytes().get(body.as_range()) != Some(word.image().bytes())
        {
            return None;
        }
        let captured = Self::from_original_static_word(
            word,
            WordValueRules::from_config(&word.config()),
            self.policy,
        )?;
        Some(Self {
            origin: SourceNameValueOrigin::ScriptWord {
                parent: Box::new(self.clone()),
                body,
                word: word.clone(),
            },
            bytes: captured.bytes,
            policy: self.policy,
        })
    }

    /// Exact contiguous Text components from the authentic original arena.
    /// Partial components and crossings over substitutions supply no fragment.
    pub(crate) fn from_original_text_fragment(
        word: &NativeWord,
        span: tcl_lexer::Span,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if word.config().escapes != policy.string_protocol().escape_syntax() {
            return None;
        }
        let bytes = native_text_fragment(word.executable_parts(), span, policy)?;
        Some(Self {
            origin: SourceNameValueOrigin::TextFragment {
                word: word.clone(),
                span,
            },
            bytes: bytes.into(),
            policy,
        })
    }

    /// An exact Text-only region of an independently retained executable arena.
    /// Its source/configuration correspondence grants no complete word authority.
    pub(crate) fn from_original_executable_text_fragment(
        arena: &tcl_lexer::ExecutablePartArena,
        image: &SourceImage,
        config: LexerConfig,
        span: tcl_lexer::Span,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if arena.image() != image
            || arena.config() != config
            || config.escapes != policy.string_protocol().escape_syntax()
        {
            return None;
        }
        let bytes = native_text_fragment(arena, span, policy)?;
        Some(Self {
            origin: SourceNameValueOrigin::ExecutableTextFragment {
                arena: arena.clone(),
                span,
            },
            bytes: bytes.into(),
            policy,
        })
    }

    pub(crate) fn produced_value(
        &self,
    ) -> Option<&crate::command_binding::original_name_value::OriginalProducedNameValue> {
        match &self.origin {
            SourceNameValueOrigin::Evaluated(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn lineage_nodes_in(&self, remaining: &mut usize) -> bool {
        let Some(next) = remaining.checked_sub(1) else {
            return false;
        };
        *remaining = next;
        match &self.origin {
            SourceNameValueOrigin::Evaluated(value) => value.lineage_nodes_in(remaining),
            SourceNameValueOrigin::JimStockPublication(name) => name.lineage_nodes_in(remaining),
            SourceNameValueOrigin::ObjectCommandNameResult { receipt, .. } => {
                receipt.requested_name_input().lineage_nodes_in(remaining)
            }

            SourceNameValueOrigin::ListElement { parent, .. }
            | SourceNameValueOrigin::ScriptWord { parent, .. } => {
                parent.lineage_nodes_in(remaining)
            }
            SourceNameValueOrigin::DictionaryVariableElement { parent, root, .. } => {
                parent.lineage_nodes_in(remaining) && root.value().lineage_nodes_in(remaining)
            }
            SourceNameValueOrigin::StoredVariableTracePrefix { parents, .. } => parents
                .iter()
                .all(|parent| parent.lineage_nodes_in(remaining)),
            SourceNameValueOrigin::StaticWord { .. }
            | SourceNameValueOrigin::TextFragment { .. }
            | SourceNameValueOrigin::ExecutableTextFragment { .. }
            | SourceNameValueOrigin::VariableSubstitutionIndex { .. }
            | SourceNameValueOrigin::VariableIndex { .. }
            | SourceNameValueOrigin::VariableOperandRoot { .. }
            | SourceNameValueOrigin::SourceLine { .. } => true,
        }
    }

    pub(crate) fn is_current(&self, context: &crate::var_resolve::ResolveContext) -> bool {
        match &self.origin {
            SourceNameValueOrigin::StoredVariableTracePrefix { dialect, .. } => {
                context.invocation_dialect == Some(*dialect)
                    && context
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        == Some(self.policy)
            }
            SourceNameValueOrigin::Evaluated(value) => value.is_current(context),
            SourceNameValueOrigin::JimStockPublication(name) => name.is_current(context),
            SourceNameValueOrigin::ObjectCommandNameResult { contents_epoch, .. } => {
                context.original_contents_epoch() == Some(*contents_epoch)
                    && context
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        == Some(self.policy)
            }

            SourceNameValueOrigin::ListElement { parent, .. }
            | SourceNameValueOrigin::ScriptWord { parent, .. }
            | SourceNameValueOrigin::DictionaryVariableElement { parent, .. } => {
                parent.is_current(context)
            }
            _ => {
                context
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    == Some(self.policy)
            }
        }
    }

    pub(super) fn from_original_substitution_index(
        root: &super::variable_name::SignatureSourceVariableRoot,
    ) -> Option<Self> {
        let mut bytes: Option<Vec<u8>> = None;
        for component in root.index_parts()? {
            let text = tcl_syntax::backslash::native_arena_text(
                root.arena(),
                component,
                root.lexer_config().escapes,
                root.policy().string_protocol(),
            )
            .ok()?;
            bytes = Some(match bytes {
                Some(previous) => tcl_syntax::native_object_append::SourceStringConcatenationProtocol::for_string_protocol(root.policy().string_protocol()).concatenate(&previous, &text)?,
                None => text.into_owned(),
            });
        }
        Some(Self {
            origin: SourceNameValueOrigin::VariableSubstitutionIndex {
                root: Box::new(root.clone()),
            },
            bytes: bytes.unwrap_or_default().into(),
            policy: root.policy(),
        })
    }

    /// Readonly root from the original C variable-word layout. A compound
    /// index retains its genuine source arena but is never evaluated here.
    /// This is not the whole argv value, a fetched element, entered alias,
    /// editable word or native compiler preparation.
    pub(crate) fn from_original_variable_operand_root(
        word: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        use tcl_syntax::native_variable_words::{NativeVariableWordOperand, native_variable_word};
        let NativeNameProtocol::C(version) = policy.recipe() else {
            return None;
        };
        if rules != WordValueRules::from_config(&word.config()) {
            return None;
        }
        let (name, name_span) = if policy.authority()
            == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        {
            let root = tcl_syntax::native_variable_words::original_source_compound_variable_root(
                word,
                version,
                policy.string_protocol(),
            )
            .ok()??;
            (root.name().to_vec(), root.name_span())
        } else {
            let NativeVariableWordOperand::CompoundArray {
                name, name_span, ..
            } = native_variable_word(word, version, policy.string_protocol()).ok()?
            else {
                return None;
            };
            (name, name_span)
        };
        Some(Self {
            origin: SourceNameValueOrigin::VariableOperandRoot {
                word: word.clone(),
                name_span,
            },
            bytes: name.into(),
            policy,
        })
    }

    /// The static index value selected by the original C variable-word owner.
    /// Literal compiler operands, static compound indices and dynamic-word
    /// runtime parsing use their own purposes; substituted indices decline.
    #[must_use]
    pub fn from_original_variable_index(
        word: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        use tcl_syntax::native_variable_words::{NativeVariableWordOperand, native_variable_word};
        let NativeNameProtocol::C(version) = policy.recipe() else {
            return None;
        };
        if rules != WordValueRules::from_config(&word.config()) {
            return None;
        }
        let (root, bytes) = match native_variable_word(word, version, policy.string_protocol())
            .ok()?
        {
            NativeVariableWordOperand::Literal {
                name,
                index: Some(index),
                ..
            } => (name, index),
            NativeVariableWordOperand::CompoundArray { name, index, .. } => {
                let mut bytes: Option<Vec<u8>> = None;
                for component in index.list(index.root()) {
                    let text = tcl_syntax::backslash::native_arena_text(
                        &index,
                        component,
                        word.config().escapes,
                        policy.string_protocol(),
                    )
                    .ok()?;
                    bytes = Some(match bytes {
                        Some(previous) => tcl_syntax::native_object_append::SourceStringConcatenationProtocol::for_string_protocol(policy.string_protocol()).concatenate(&previous, &text)?,
                        None => text.into_owned(),
                    });
                }
                (name, bytes.unwrap_or_default())
            }
            NativeVariableWordOperand::DynamicWord => {
                let key = SignatureSourceNameKey::from_original_native_word(word, rules, policy)?;
                let parsed = policy.recipe().combined_variable_input(key.bytes());
                (
                    parsed.root().selected().to_vec(),
                    parsed.element()?.selected().to_vec(),
                )
            }
            NativeVariableWordOperand::Literal { index: None, .. } => return None,
        };
        Some(Self {
            origin: SourceNameValueOrigin::VariableIndex {
                word: word.clone(),
                root: root.into(),
            },
            bytes: bytes.into(),
            policy,
        })
    }

    /// Source lines after the independently selected input-channel translation.
    /// Document CR and CRLF boundaries are selected in the translated value,
    /// before list parsing. No original substring or edit geometry is asserted.
    #[must_use]
    pub fn original_source_lines(
        image: &SourceImage,
        config: LexerConfig,
        policy: NamePolicyProtocol,
    ) -> Option<Vec<Self>> {
        if config.escapes != policy.string_protocol().escape_syntax() {
            return None;
        }
        let value = tcl_syntax::backslash::native_source_literal_bytes(
            image.bytes(),
            image.channel(),
            policy.string_protocol(),
        )
        .ok()?;
        Some(
            value
                .split(|byte| *byte == b'\n')
                .enumerate()
                .map(|(line, bytes)| Self {
                    origin: SourceNameValueOrigin::SourceLine {
                        image: image.clone(),
                        line,
                        config,
                    },
                    bytes: bytes.into(),
                    policy,
                })
                .collect(),
        )
    }

    /// One line from the same shared translated-line owner. The ordinal never
    /// supplies a written word, normal read, original substring or edit grant.
    #[must_use]
    pub fn from_original_source_line(
        image: &SourceImage,
        line: usize,
        config: LexerConfig,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        Self::original_source_lines(image, config, policy)?
            .get(line)
            .cloned()
    }

    /// Native list element of this immutable source-produced value. This uses
    /// the selected list/escape grammar and keeps the complete parent lineage.
    #[must_use]
    pub fn list_element(&self, ordinal: usize) -> Option<Self> {
        self.lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS - 1)
            .then_some(())?;
        let elements =
            tcl_syntax::list::split_native_list_bytes(self.bytes(), self.policy.string_protocol())
                .ok()?;
        let bytes = elements.get(ordinal)?.clone().into_owned();
        Some(self.list_child(ordinal, bytes))
    }

    /// Parse once and retain each exact native list ordinal under the complete
    /// immutable parent. Parsing grants neither argv completion nor geometry.
    #[must_use]
    pub fn list_elements(&self) -> Option<Vec<Self>> {
        Some(
            self.list_element_records()?
                .into_iter()
                .map(|(child, _)| child)
                .collect(),
        )
    }

    fn list_element_records(&self) -> Option<Vec<(Self, tcl_syntax::list::Element)>> {
        self.lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS - 1)
            .then_some(())?;
        let elements = tcl_syntax::list::split_native_list_elements(
            self.bytes(),
            self.policy.string_protocol(),
        )
        .ok()?;
        Some(
            elements
                .into_iter()
                .enumerate()
                .map(|(ordinal, element)| {
                    (
                        self.list_child(ordinal, element.value.into_owned()),
                        element.source,
                    )
                })
                .collect(),
        )
    }

    fn list_child(&self, ordinal: usize, bytes: Vec<u8>) -> Self {
        Self {
            origin: SourceNameValueOrigin::ListElement {
                parent: Box::new(self.clone()),
                ordinal,
            },
            bytes: bytes.into(),
            policy: self.policy,
        }
    }

    /// A copied dictionary child retains the exact indexed read and root
    /// producer. Later known root writes retire the live read receipt while
    /// these historical bytes keep their independent contents currency.
    pub(crate) fn from_dictionary_variable_element(
        root: &crate::var_resolve::OriginalNameValueRead,
        receiver: &crate::place::Place,
        ordinal: usize,
    ) -> Option<Self> {
        let parent = Self::from_original_produced_value(root.value());
        let child = parent.list_element(ordinal)?;
        let result = Self {
            bytes: child.bytes,
            policy: child.policy,
            origin: SourceNameValueOrigin::DictionaryVariableElement {
                parent: Box::new(parent),
                ordinal,
                root: Box::new(root.clone()),
                receiver: receiver.clone(),
            },
        };
        result
            .lineage_within(crate::command_binding::original_name_value::MAX_ORIGINS)
            .then_some(result)
    }

    /// Count under the selected native list grammar; malformed lists decline.
    #[must_use]
    pub fn list_length(&self) -> Option<usize> {
        Some(
            tcl_syntax::list::split_native_list_bytes(self.bytes(), self.policy.string_protocol())
                .ok()?
                .len(),
        )
    }

    /// Exact source-produced units, independent of optional Unicode advice.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }

    /// Independent name/string provider retained by this immutable value.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Original selected variable root for an index producer, without cell lookup.
    #[must_use]
    pub fn variable_root(&self) -> Option<&[u8]> {
        match &self.origin {
            SourceNameValueOrigin::VariableIndex { root, .. } => Some(root.as_bytes()),
            _ => None,
        }
    }

    /// Optional exact UTF-8 rendering; it is never a naming input.
    #[must_use]
    pub fn display(&self) -> Option<&str> {
        self.bytes.try_utf8().ok()
    }
}

fn native_text_fragment(
    arena: &tcl_lexer::ExecutablePartArena,
    span: tcl_lexer::Span,
    policy: NamePolicyProtocol,
) -> Option<Vec<u8>> {
    let lists = std::iter::once(arena.root()).chain(arena.all_parts().filter_map(|part| {
        if let tcl_lexer::ExecutablePart::Variable { index, .. } = part.part {
            index
        } else {
            None
        }
    }));
    for list in lists {
        let parts = arena.list(list);
        let Some(start) = parts
            .iter()
            .position(|part| part.span.start() == span.start())
        else {
            continue;
        };
        let mut next = span.start();
        let mut bytes: Option<Vec<u8>> = None;
        for part in &parts[start..] {
            if part.span.start() != next || part.span.end() > span.end() {
                break;
            }
            let text = tcl_syntax::backslash::native_arena_text(
                arena,
                part,
                arena.config().escapes,
                policy.string_protocol(),
            )
            .ok()?;
            bytes = Some(match bytes {
                Some(previous) => tcl_syntax::native_object_append::SourceStringConcatenationProtocol::for_string_protocol(policy.string_protocol()).concatenate(&previous, &text)?,
                None => text.into_owned(),
            });
            next = part.span.end();
            if next == span.end() {
                return bytes;
            }
        }
    }
    None
}

/// One Unicode source-name list field retains the complete original roster.
/// This separate Logical purpose supplies no Native value, name key or cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogicalSourceListNameInput {
    fields: std::sync::Arc<[tcl_syntax::word_rules::OriginalSourceListElement]>,
    ordinal: usize,
    policy: NamePolicyProtocol,
}

impl LogicalSourceListNameInput {
    pub(super) fn from_original_word(
        original: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
        _purpose: super::original_name::LogicalSourceNamePurpose,
    ) -> Option<Vec<Self>> {
        if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            || rules != WordValueRules::from_config(&original.config())
        {
            return None;
        }
        let fields: std::sync::Arc<[_]> =
            tcl_syntax::word_rules::original_static_word_list_elements(original)?.into();
        Some(
            (0..fields.len())
                .map(|ordinal| Self {
                    fields: std::sync::Arc::clone(&fields),
                    ordinal,
                    policy,
                })
                .collect(),
        )
    }

    /// Unicode source naming units under the retained field grammar. These
    /// bytes never stand in for a Native object's encoding or fetched value.
    pub(crate) fn source_units(&self) -> &[u8] {
        self.fields[self.ordinal].value().as_bytes()
    }

    pub(crate) fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    pub(crate) fn original_field(&self) -> &tcl_syntax::word_rules::OriginalSourceListElement {
        &self.fields[self.ordinal]
    }

    pub(crate) fn complete_unchanged_roster(inputs: &[Self]) -> Option<()> {
        let first = inputs.first()?;
        if first.fields.len() != inputs.len() {
            return None;
        }
        for (ordinal, input) in inputs.iter().enumerate() {
            let field = input.original_field();
            if input.ordinal != ordinal
                || input.policy != first.policy
                || !std::sync::Arc::ptr_eq(&input.fields, &first.fields)
                || field.original_word() != first.original_field().original_word()
                || field
                    .original_word()
                    .image()
                    .bytes()
                    .get(field.source_span()?.as_range())?
                    != input.source_units()
            {
                return None;
            }
        }
        Some(())
    }
}

/// One unchanged Logical lambda formal name, retaining the whole selected
/// call and list path. Unicode source units supply no Native value, word key,
/// namespace/command role, cell or edit permission.
#[derive(Debug, Clone)]
pub struct OriginalLogicalListField {
    lambda: std::sync::Arc<crate::command_binding::OriginalLogicalLambdaBody>,
    source: std::sync::Arc<crate::command_binding::SourceOriginId>,
    path: [usize; 3],
    field: tcl_syntax::word_rules::OriginalSourceListElement,
    span: Span,
}

impl PartialEq for OriginalLogicalListField {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.source, &other.source)
            && self.lambda == other.lambda
            && self.path == other.path
            && self.field == other.field
            && self.span == other.span
    }
}

impl Eq for OriginalLogicalListField {}

impl std::hash::Hash for OriginalLogicalListField {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::sync::Arc::as_ptr(&self.source).hash(state);
        self.path.hash(state);
        self.span.hash(state);
        self.source_units().hash(state);
        self.policy().hash(state);
    }
}

impl OriginalLogicalListField {
    pub(crate) fn from_lambda_formal(
        lambda: std::sync::Arc<crate::command_binding::OriginalLogicalLambdaBody>,
        input: &crate::analyser::ResolvedAnalysisInput,
        parameter: usize,
    ) -> Option<Self> {
        let (source, ..) = lambda.frame_source(input)?;
        let field = lambda.original_formal_name_field(input, parameter)?;
        let span = field.source_span()?;
        Some(Self {
            lambda,
            source,
            path: [0, parameter, 0],
            field,
            span,
        })
    }

    pub(crate) fn matches_source(
        &self,
        image: &SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        self.path[0] == 0
            && self.path[2] == 0
            && self.lambda.matches_source(image, input)
            && self
                .lambda
                .frame_source(input)
                .is_some_and(|(source, ..)| std::sync::Arc::ptr_eq(&self.source, &source))
            && self
                .lambda
                .original_formal_name_field(input, self.path[1])
                .as_ref()
                == Some(&self.field)
            && self.field.source_span() == Some(self.span)
    }

    /// The entire current canonical owner must agree; equal field text cannot
    /// substitute a sibling call, different source allocation or purpose facet.
    pub(crate) fn matches_lambda(
        &self,
        lambda: &crate::command_binding::OriginalLogicalLambdaBody,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        lambda
            .frame_source(input)
            .is_some_and(|(source, ..)| std::sync::Arc::ptr_eq(&self.source, &source))
            && self.lambda.as_ref() == lambda
            && self.matches_source(lambda.lambda().image(), input)
    }

    pub(crate) fn source_units(&self) -> &[u8] {
        self.field.value().as_bytes()
    }
    pub(crate) fn source_span(&self) -> Span {
        self.span
    }
    #[cfg(test)]
    pub(crate) fn source_path(&self) -> &[usize; 3] {
        &self.path
    }
    pub(crate) fn policy(&self) -> NamePolicyProtocol {
        self.lambda.name_policy()
    }
    #[cfg(test)]
    pub(crate) fn original_container(&self) -> &NativeWord {
        self.field.original_word()
    }
}

/// Complete-word and readonly produced-value inputs retain distinct authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignatureSourceNameInput {
    /// Original lexical variable root; it supplies no substituted value.
    OriginalVariableRoot(super::variable_name::SignatureSourceVariableRoot),
    /// Complete original static word, with no automatic edit or dispatch grant.
    OriginalWord(SignatureSourceNameKey),
    /// Original fragment/list-child value, with no complete-word edit geometry.
    OriginalValue(SignatureSourceNameValue),
    /// Conditional Logical formal source units, without a Native value or edit.
    OriginalLogicalListField(OriginalLogicalListField),
}

impl SignatureSourceNameInput {
    /// Static container correspondence, independently of a future validated
    /// whole-container replacement. No child word or editable span is issued.
    #[must_use]
    pub fn original_static_list_container(&self) -> Option<SignatureSourceStaticListContainer<'_>> {
        match self {
            Self::OriginalWord(key) => Some(SignatureSourceStaticListContainer {
                word: key.original_word(),
                ordinals: Vec::new(),
            }),
            Self::OriginalValue(value) => value.original_static_list_container(),
            Self::OriginalVariableRoot(_) | Self::OriginalLogicalListField(_) => None,
        }
    }
    fn original_list_value(&self) -> Option<SignatureSourceNameValue> {
        match self {
            Self::OriginalWord(key) => SignatureSourceNameValue::from_original_static_word(
                key.original_word(),
                key.word_value_rules(),
                key.policy(),
            ),
            Self::OriginalValue(value) => Some(value.clone()),
            Self::OriginalVariableRoot(_) | Self::OriginalLogicalListField(_) => None,
        }
    }

    /// Readonly native list child of the retained original value. A lexical
    /// variable root is a name and cannot stand in for its fetched value.
    #[must_use]
    pub fn original_list_element(&self, ordinal: usize) -> Option<Self> {
        self.original_list_value()?
            .list_element(ordinal)
            .map(Self::OriginalValue)
    }

    /// All native list children from one parse of the original parent.
    #[must_use]
    pub fn original_list_elements(&self) -> Option<Vec<Self>> {
        Some(
            self.original_list_value()?
                .list_elements()?
                .into_iter()
                .map(Self::OriginalValue)
                .collect(),
        )
    }
    /// Native list children and optional original presentation extents from
    /// one shared list parse. Extents are available only when the complete
    /// static parent's literal content is the same native list value. Quoted
    /// escape programs and evaluated parents retain children with no invented
    /// element substring. Neither facet issues a key or edit permission.
    #[must_use]
    pub fn original_list_elements_with_source_spans(&self) -> Option<Vec<(Self, Option<Span>)>> {
        let records = self.original_list_value()?.list_element_records()?;
        Some(
            records
                .into_iter()
                .map(|(child, element)| {
                    let extent = self.original_static_value_source_extent(element.value);
                    (Self::OriginalValue(child), extent)
                })
                .collect(),
        )
    }

    /// Map a range in this readonly value through genuine static list ancestry
    /// to original source units. Intermediate children must preserve their
    /// complete literal bytes. Decoded or evaluated ancestry supplies no extent;
    /// the result grants neither a child Word key nor edit permission.
    #[must_use]
    pub fn original_static_value_source_extent(
        &self,
        range: std::ops::Range<usize>,
    ) -> Option<Span> {
        if range.start > range.end || range.end > self.bytes().len() {
            return None;
        }
        let container = self.original_static_list_container()?;
        let word = container.parent_word();
        let root = SignatureSourceNameValue::from_original_static_word(
            word,
            WordValueRules::from_config(&word.config()),
            self.policy(),
        )?;
        let content = word.content_span().ok()?;
        let raw = word.image().bytes().get(content.as_range())?;
        let literal = tcl_syntax::backslash::native_source_literal_bytes(
            raw,
            word.image().channel(),
            self.policy().string_protocol(),
        )
        .ok()?;
        if literal.as_ref() != root.bytes() {
            return None;
        }
        let mut selected = root.bytes().to_vec();
        let mut offset = 0usize;
        for &ordinal in container.ordinals() {
            let elements = tcl_syntax::list::split_native_list_elements(
                &selected,
                self.policy().string_protocol(),
            )
            .ok()?;
            let element = elements.get(ordinal)?;
            if !element.source.literal {
                return None;
            }
            offset = offset.checked_add(element.source.value.start)?;
            let next = element.value.to_vec();
            drop(elements);
            selected = next;
        }
        if selected != self.bytes() {
            return None;
        }
        let native_range = offset.checked_add(range.start)?..offset.checked_add(range.end)?;
        let mapped = tcl_syntax::backslash::native_source_literal_extent(
            raw,
            word.image().channel(),
            self.policy().string_protocol(),
            native_range,
        )?;
        Some(Span::new(
            content
                .start()
                .checked_add(u32::try_from(mapped.start).ok()?)?,
            content
                .start()
                .checked_add(u32::try_from(mapped.end).ok()?)?,
        ))
    }

    pub(crate) fn lineage_within(&self, budget: usize) -> bool {
        let mut remaining = budget;
        self.lineage_nodes_in(&mut remaining)
    }

    pub(crate) fn lineage_nodes_in(&self, remaining: &mut usize) -> bool {
        match self {
            Self::OriginalValue(value) => value.lineage_nodes_in(remaining),
            Self::OriginalLogicalListField(_) => false,
            Self::OriginalWord(_) | Self::OriginalVariableRoot(_) => {
                let Some(next) = remaining.checked_sub(1) else {
                    return false;
                };
                *remaining = next;
                true
            }
        }
    }

    pub(crate) fn produced_value(
        &self,
    ) -> Option<&crate::command_binding::original_name_value::OriginalProducedNameValue> {
        match self {
            Self::OriginalValue(value) => value.produced_value(),
            Self::OriginalWord(_)
            | Self::OriginalVariableRoot(_)
            | Self::OriginalLogicalListField(_) => None,
        }
    }
    pub(crate) fn is_current(&self, context: &crate::var_resolve::ResolveContext) -> bool {
        match self {
            Self::OriginalWord(key) => {
                context
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    == Some(key.policy())
            }
            Self::OriginalVariableRoot(root) => {
                context
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    == Some(root.policy())
            }
            Self::OriginalValue(value) => value.is_current(context),
            Self::OriginalLogicalListField(_) => false,
        }
    }
    /// Original naming units. Logical formal fields expose Unicode source
    /// units; their Native value and execution projections remain unavailable.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::OriginalWord(key) => key.bytes(),
            Self::OriginalVariableRoot(root) => root.bytes(),
            Self::OriginalValue(value) => value.bytes(),
            Self::OriginalLogicalListField(field) => field.source_units(),
        }
    }
    /// Independently selected naming policy.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        match self {
            Self::OriginalWord(key) => key.policy(),
            Self::OriginalVariableRoot(root) => root.policy(),
            Self::OriginalValue(value) => value.policy(),
            Self::OriginalLogicalListField(field) => field.policy(),
        }
    }
    /// Complete word authority is absent for readonly produced values.
    #[must_use]
    pub fn original_word_key(&self) -> Option<&SignatureSourceNameKey> {
        match self {
            Self::OriginalWord(key) => Some(key),
            Self::OriginalValue(_)
            | Self::OriginalVariableRoot(_)
            | Self::OriginalLogicalListField(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;

    #[test]
    fn original_input_list_children_keep_parent_and_reject_lexical_roots() {
        // Implementation contract: naming.source.readonly-original-operand-projections (docs/design/analysis/name-resolution-proofs/readonly-original-operand-projections.md).
        let dialects = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(tcl_registry::InvocationDialect::for_version)
            .chain([tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
            )]);
        for dialect in dialects {
            let config = LexerConfig::from_grammar(dialect.lexer_grammar);
            let policy = dialect.authored_name_policy().unwrap();
            let source = r#"list "p\uD800 {two words}" $array(index)"#;
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[1],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            let parent = SignatureSourceNameInput::OriginalWord(key);
            let children = parent.original_list_elements().unwrap();
            assert_eq!(children.len(), 2);
            assert_eq!(parent.original_list_element(0).as_ref(), children.first());
            assert_eq!(children[1].bytes(), b"two words");
            assert!(
                children
                    .iter()
                    .all(|child| child.original_word_key().is_none())
            );
            let container = children[1].original_static_list_container().unwrap();
            assert_eq!(container.parent_word(), &plan.commands[0].words[1]);
            assert_eq!(container.ordinals(), &[1]);
            let grandchild = children[1].original_list_element(1).unwrap();
            assert_eq!(grandchild.bytes(), b"words");
            assert_eq!(
                grandchild
                    .original_static_list_container()
                    .unwrap()
                    .ordinals(),
                &[1, 1]
            );
            assert!(parent.original_list_element(2).is_none());
            let variable = &plan.commands[0].words[2];
            let part = variable
                .executable_parts()
                .all_parts()
                .find(|part| matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. }))
                .unwrap();
            let root =
                super::super::variable_name::SignatureSourceVariableRoot::from_original_word(
                    variable,
                    part.span,
                    WordValueRules::from_config(&config),
                    policy,
                )
                .unwrap();
            let root = SignatureSourceNameInput::OriginalVariableRoot(root);
            assert!(root.original_list_element(0).is_none());
            assert!(root.original_list_elements().is_none());
            assert!(root.original_static_list_container().is_none());
        }
    }

    #[test]
    fn original_index_fragment_and_list_child_preserve_native_units() {
        for version in tcl_dialect::TclVersion::ALL {
            let config = LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
            );
            let policy = NamePolicyProtocol::authored_tcl(version);
            let source = r"set auto_index(p\u0000tail) value";
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let index = SignatureSourceNameValue::from_original_variable_index(
                &plan.commands[0].words[1],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            assert_eq!(index.variable_root(), Some(b"auto_index".as_slice()));
            assert_eq!(index.bytes(), b"p\xc0\x80tail");
            let source = SourceImage::document("# header\n{$literal} {p\\u0000tail}\n");
            let line =
                SignatureSourceNameValue::from_original_source_line(&source, 1, config, policy)
                    .unwrap();
            assert_eq!(line.list_length(), Some(2));
            assert_eq!(line.list_element(0).unwrap().bytes(), b"$literal");
            let child = SignatureSourceNameInput::OriginalValue(line.list_element(1).unwrap());
            assert!(child.original_word_key().is_none());
        }
    }

    #[test]
    fn original_executable_fragment_requires_full_owner_and_single_text_region() {
        for version in tcl_dialect::TclVersion::ALL {
            let config = LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
            );
            let policy = NamePolicyProtocol::authored_tcl(version);
            let image = SourceImage::native(b"$a(k\\x2d$b tail)".as_slice());
            let arena = tcl_lexer::ExecutablePartArena::decompose(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::word_parts::SubstFlags::default(),
                config,
            )
            .unwrap();
            let part = arena
                .all_parts()
                .find(|part| {
                    matches!(part.part, tcl_lexer::ExecutablePart::Text(_))
                        && arena
                            .bytes(part.span)
                            .is_some_and(|bytes| bytes.starts_with(b"k"))
                })
                .unwrap();
            let fragment = SignatureSourceNameValue::from_original_executable_text_fragment(
                &arena, &image, config, part.span, policy,
            )
            .unwrap();
            assert_eq!(fragment.bytes(), b"k-");
            assert!(
                SignatureSourceNameInput::OriginalValue(fragment)
                    .original_word_key()
                    .is_none()
            );
            assert!(
                SignatureSourceNameValue::from_original_executable_text_fragment(
                    &arena,
                    &image,
                    config,
                    Span::new(part.span.start(), part.span.start() + 1),
                    policy
                )
                .is_none()
            );
            assert!(
                SignatureSourceNameValue::from_original_executable_text_fragment(
                    &arena,
                    &image,
                    config,
                    Span::new(part.span.start(), u32::try_from(image.len() - 1).unwrap()),
                    policy
                )
                .is_none()
            );
            assert!(
                SignatureSourceNameValue::from_original_executable_text_fragment(
                    &arena,
                    &SourceImage::document(image.try_text().unwrap()),
                    config,
                    part.span,
                    policy
                )
                .is_none()
            );
            let other = LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            };
            assert!(
                SignatureSourceNameValue::from_original_executable_text_fragment(
                    &arena, &image, other, part.span, policy
                )
                .is_none()
            );
        }
    }
}

#[cfg(test)]
mod original_list_source_extent_tests {
    use super::*;
    use tcl_syntax::word_rules::WordValueRules;

    fn parent(source: &str) -> SignatureSourceNameInput {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let config = LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
                .lexer_grammar,
        );
        let image = SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[1],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap(),
        )
    }
    #[test]
    fn original_list_spans_preserve_literal_origins_and_decline_escape_program_geometry() {
        let literal = r"list {first other\uD800 third}";
        let children = parent(literal)
            .original_list_elements_with_source_spans()
            .unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[1].0.bytes(), b"other\xed\xa0\x80");
        assert_eq!(&literal[children[1].1.unwrap().as_range()], r"other\uD800");
        let quoted = r#"list "first other\uD800 third""#;
        let children = parent(quoted)
            .original_list_elements_with_source_spans()
            .unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[1].0.bytes(), b"other\xed\xa0\x80");
        assert!(children.iter().all(|(_, span)| span.is_none()));
        assert!(
            children
                .iter()
                .all(|(child, _)| child.original_word_key().is_none())
        );
    }
}

#[cfg(test)]
mod variable_operand_root_tests {
    use super::*;

    #[test]
    fn original_c_array_operand_root_keeps_unknown_index_and_complete_word_separate() {
        // Implementation contract: naming.variable.original-array-operand-root
        // docs/design/analysis/name-resolution-proofs/original-array-operand-root.md
        // Original C variable-word geometry source evidence:
        // docs/design/analysis/name-resolution-proofs/literal-set-name-effects-source.md
        // This uses its compound-word root boundary, not its SET handler,
        // local-slot admission, successful index evaluation or Normal claims.
        for version in tcl_dialect::TclVersion::ALL {
            let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
            let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
            let policy = NamePolicyProtocol::authored_tcl(version);
            let source = tcl_lexer::SourceImage::document("upvar ::N::array($index) local");
            let word = tcl_lexer::native_script_words_in(
                source.clone(),
                Span::new(0, u32::try_from(source.bytes().len()).unwrap()),
                config,
            )
            .unwrap()
            .commands
            .remove(0)
            .words
            .remove(1);
            let value = SignatureSourceNameValue::from_original_variable_operand_root(
                &word,
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            assert_eq!(value.bytes(), b"::N::array");
            assert!(value.produced_value().is_none());
            assert!(value.original_static_list_container().is_none());
            assert!(
                SignatureSourceNameInput::OriginalValue(value)
                    .original_word_key()
                    .is_none()
            );
            for text in [
                "upvar ::N::$name($index) local",
                "upvar ::N::array\\($index) local",
            ] {
                let source = tcl_lexer::SourceImage::document(text);
                let word = tcl_lexer::native_script_words_in(
                    source.clone(),
                    Span::new(0, u32::try_from(source.bytes().len()).unwrap()),
                    config,
                )
                .unwrap()
                .commands
                .remove(0)
                .words
                .remove(1);
                assert!(
                    SignatureSourceNameValue::from_original_variable_operand_root(
                        &word,
                        WordValueRules::from_config(&config),
                        policy,
                    )
                    .is_none()
                );
            }
        }
    }
}

#[cfg(test)]
mod logical_list_name_tests {
    use super::*;
    use crate::signature_scan::original_name::OriginalSourceNamePurpose;
    use crate::signature_scan::scope::SignatureNamespaceScope;

    fn inputs(source: &str) -> (Vec<LogicalSourceListNameInput>, NativeWord) {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        assert!(input.has_logical_source_name_context());
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()
            .unwrap();
        let config = input.lexer_config();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = plan.commands[0].words[1].clone();
        let purpose = OriginalSourceNamePurpose::for_analysis_input(input, None, policy);
        let fields = purpose
            .logical_list_inputs(&word, WordValueRules::from_config(&config))
            .unwrap();
        assert!(
            OriginalSourceNamePurpose::Native(policy)
                .logical_list_inputs(&word, WordValueRules::from_config(&config))
                .is_none()
        );
        (fields, word)
    }

    #[test]
    fn logical_lambda_namespace_requires_complete_original_field_roster() {
        let (omitted, original) = inputs("apply {{} {return}}");
        assert!(
            SignatureNamespaceScope::from_logical_lambda_list(&omitted)
                .unwrap()
                .is_root()
        );
        assert!(
            omitted
                .iter()
                .all(|field| field.original_field().original_word() == &original)
        );
        let (explicit, original) = inputs("apply {{} {return} sub}");
        let namespace = SignatureNamespaceScope::from_logical_lambda_list(&explicit).unwrap();
        assert!(!namespace.is_root());
        assert_eq!(
            namespace
                .source_spelling(Some(explicit[0].policy()))
                .as_deref(),
            Some("::sub")
        );
        assert_eq!(explicit[2].source_units(), b"sub");
        assert_eq!(
            explicit[2].original_field().original_word().config(),
            original.config()
        );
        assert!(SignatureNamespaceScope::from_logical_lambda_list(&explicit[..2]).is_none());
        let mut reordered = explicit.clone();
        reordered.swap(0, 1);
        assert!(SignatureNamespaceScope::from_logical_lambda_list(&reordered).is_none());
        let (foreign, _) = inputs("apply {{} {return} sub}");
        let mut mixed = explicit.clone();
        mixed[2] = foreign[2].clone();
        assert!(SignatureNamespaceScope::from_logical_lambda_list(&mixed).is_none());
        for source in [
            "apply {{} {return} sub extra}",
            "apply {{}}",
            r"apply {{} {return} sub\ namespace}",
        ] {
            let (fields, _) = inputs(source);
            assert!(
                SignatureNamespaceScope::from_logical_lambda_list(&fields).is_none(),
                "{source}"
            );
        }
    }
}

#[cfg(test)]
mod logical_formal_field_tests {
    use super::*;
    use std::sync::Arc;

    fn lambda(
        source: &str,
        ordinal: usize,
    ) -> (
        Arc<crate::command_binding::OriginalLogicalLambdaBody>,
        crate::analyser::ResolvedAnalysisInput,
    ) {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap().clone();
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .remove(ordinal);
        let words = crate::registry_invocation::source_structure::source_registry_words(
            source, &analysis, &command,
        )
        .unwrap();
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = words.source()
        else {
            panic!("actual Logical source call");
        };
        let original = advice.original_words().to_vec();
        let policy = crate::command_binding::source_input_dialect(&input)
            .authored_name_policy()
            .unwrap();
        assert!(
            crate::command_binding::OriginalLogicalLambdaBody::capture(
                &input,
                words.clone(),
                &original,
                crate::signature_scan::original_name::OriginalSourceNamePurpose::Native(policy),
            )
            .is_none()
        );
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                &input, None, policy,
            );
        let owner = crate::command_binding::OriginalLogicalLambdaBody::capture(
            &input, words, &original, purpose,
        )
        .unwrap();
        (Arc::new(owner), input)
    }

    // Evaluate the repeated issuer query at its original insertion point and
    // retain the actual set through the caller's remaining source queries.
    fn assert_formal_field_hash_identity<T: Clone + Eq + std::hash::Hash>(
        field: &T,
        duplicate: impl FnOnce() -> T,
        foreign_allocation: T,
    ) -> impl Sized {
        let mut set = std::collections::HashSet::new();
        assert!(set.insert(field.clone()));
        assert!(!set.insert(duplicate()));
        assert!(set.insert(foreign_allocation));
        set
    }

    #[test]
    fn logical_formal_fields_retain_complete_call_path_and_unicode_source_units() {
        let source = "apply {{first {second DEFAULT} args} {return $first}} ONE";
        let (owner, input) = lambda(source, 0);
        for (ordinal, expected) in ["first", "second", "args"].into_iter().enumerate() {
            let field = owner.formal_name_field(&input, ordinal).unwrap();
            assert_eq!(field.source_path(), &[0, ordinal, 0]);
            assert_eq!(field.source_units(), expected.as_bytes());
            assert_eq!(&source[field.source_span().as_range()], expected);
            assert_eq!(field.original_container(), owner.lambda());
            assert_eq!(field.original_container().config(), input.lexer_config());
            assert_eq!(field.policy(), owner.name_policy());
            assert!(field.matches_source(&SourceImage::document(source), &input));
            assert!(field.matches_lambda(&owner, &input));
            let name = SignatureSourceNameInput::OriginalLogicalListField(field.clone());
            let mut native = crate::var_resolve::ResolveContext::for_namespace("::");
            native.execution_name_policy = Some(
                tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(field.policy()),
            );
            assert!(!name.is_current(&native));
            assert!(!name.lineage_within(usize::MAX));
            assert!(name.produced_value().is_none());
            assert!(name.original_word_key().is_none());
            assert!(name.original_static_list_container().is_none());
            assert!(name.original_list_element(0).is_none());
            assert!(name.original_list_elements().is_none());
            assert!(name.original_list_elements_with_source_spans().is_none());
            assert!(
                name.original_static_value_source_extent(0..name.bytes().len())
                    .is_none()
            );
            assert!(
                crate::signature_scan::scope::SignatureNamespaceScope::root(Some(field.policy()))
                    .child_from_input(&name)
                    .is_none()
            );
            assert!(
                crate::signature_scan::scope::SignatureSourceLookup::from_input(
                    crate::signature_scan::scope::SignatureNamespaceScope::root(Some(
                        field.policy()
                    )),
                    &name,
                )
                .is_none()
            );
            assert!(crate::signature_scan::formal_parameters::SignatureSourceFormalParameterValue::from_original_input(
                &name,
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            ).is_none());
            let mut foreign_allocation = field.clone();
            foreign_allocation.source =
                Arc::new(crate::command_binding::SourceOriginId::authored_image(
                    owner.lambda().image().clone(),
                ));
            assert_eq!(foreign_allocation.source, field.source);
            assert!(!Arc::ptr_eq(&foreign_allocation.source, &field.source));
            assert_ne!(foreign_allocation, field);
            assert!(!foreign_allocation.matches_source(owner.lambda().image(), &input));
            assert!(!foreign_allocation.matches_lambda(&owner, &input));
            let _set = assert_formal_field_hash_identity(
                &field,
                || owner.formal_name_field(&input, ordinal).unwrap(),
                foreign_allocation,
            );
            let mut foreign_config = input.lexer_config();
            foreign_config.strict_quoting = !foreign_config.strict_quoting;
            let foreign = crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                foreign_config,
            );
            assert!(!field.matches_source(&SourceImage::document(source), &foreign));
            assert!(!field.matches_source(
                &SourceImage::document(&source.replace("DEFAULT", "CHANGED")),
                &input,
            ));
            let mut changed = field.clone();
            changed.path[0] = 1;
            assert!(!changed.matches_source(&SourceImage::document(source), &input));
            changed.path = [0, usize::MAX, 0];
            assert!(!changed.matches_source(&SourceImage::document(source), &input));
        }
        let source = "apply {{é} {return}} ONE";
        let (owner, input) = lambda(source, 0);
        let field = owner.formal_name_field(&input, 0).unwrap();
        assert_eq!(field.source_units(), "é".as_bytes());
        assert_eq!(&source[field.source_span().as_range()], "é");
        let (first, input) = lambda("apply {{first} {return}} ONE", 0);
        let (second, _) = lambda("apply {{first} {return}} TWO", 0);
        let field = first.formal_name_field(&input, 0).unwrap();
        assert!(!field.matches_lambda(&second, &input));
    }

    #[test]
    fn logical_formal_fields_refuse_unrepresented_grammar_or_call_binding() {
        for source in [
            "apply {{first} {return}}",
            "apply {{first first} {return}} ONE TWO",
            r"apply {{{na\u006de}} {return}} ONE",
            r"apply {{{first D\u0045FAULT}} {return}}",
            "apply {{{first SECOND THIRD}} {return}} ONE",
            "apply {{{array(key)}} {return}} ONE",
            "apply {{{ns::first}} {return}} ONE",
        ] {
            let (owner, input) = lambda(source, 0);
            assert!(owner.formal_name_field(&input, 0).is_none(), "{source}");
        }
        let (owner, input) = lambda("apply {{first} {return}} ONE", 0);
        assert!(owner.formal_name_field(&input, 1).is_none());
    }
}
