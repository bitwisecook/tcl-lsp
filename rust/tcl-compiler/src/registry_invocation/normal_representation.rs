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

//! The normal representation of an invocation: results, index accesses, patterns and taint.

use super::effective_words::compose_original_effective_words;
use super::handler_invocation::resolved_handler_invocation_with_metadata_context;
use super::metadata_context::InvocationMetadataContext;
use super::word_resolution::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    resolve_registry_words_in_realm_with_metadata_context,
};
use super::{
    EffectiveCommandWords, InvocationWordOrigin, NormalRepresentationInvocation,
    ResolvedStatementInvocation, resolved_tokens_invocation_with_metadata_context,
};
use crate::ir::CommandTokens;
use tcl_registry::CommandRegistry;
use tcl_registry::model::semantic::SemanticContext;
use tcl_source::words::WordExpr;

/// Value retained by a successful selected variable assignment.
/// This representation projection supplies neither an opcode nor a write proof.
#[derive(Debug)]
pub struct NormalValueAssignment {
    /// Evaluated literal destination name.
    pub name: String,
    /// Original value word, retaining its positioned substitutions.
    pub value: WordExpr,
}

/// Successful native index access, without compiler or completion permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalIndexAccessKind {
    /// Read one list element.
    ListRead,
    /// Write one list element, subject to the selected append-slot policy.
    ListWrite,
    /// Read one native string character.
    StringRead,
}

/// Original index-operation operands under a conditional native-handler
/// interpretation. This supplies no normal completion, current object class,
/// physical read version or executable operation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConditionalIndexAccessAdvice {
    pub(crate) kind: NormalIndexAccessKind,
    pub(crate) container: String,
    pub(crate) container_literal: Option<String>,
    pub(crate) index: String,
    pub(crate) container_word: WordExpr,
    pub(crate) index_word: WordExpr,
    pub(crate) list_set_bounds: Option<tcl_dialect::ListSetBounds>,
}

pub(crate) fn conditional_index_access_advice_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ConditionalIndexAccessAdvice> {
    use NormalIndexAccessKind as Kind;
    use tcl_registry::{IntrinsicId, SemanticOperationId};
    let advice = tokens
        .source_binding
        .as_ref()?
        .declaration_operand_layout_advice(tokens)?;
    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let offset = facts.argument_offset;
        let operands = effective.words.get(offset + 1..)?;
        let kind = match facts.operation {
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex) if operands.len() == 2 => {
                Kind::ListRead
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ListSet) if operands.len() == 3 => {
                Kind::ListWrite
            }
            SemanticOperationId::Intrinsic(IntrinsicId::StringIndex) if operands.len() == 2 => {
                Kind::StringRead
            }
            _ => return None,
        };
        let literal = match values.get(offset + 1)? {
            EffectiveInvocationWord::Literal(value) => Some(value.clone()),
            _ => None,
        };
        let selected = ConditionalIndexAccessAdvice {
            kind,
            container: if kind == Kind::ListWrite {
                literal.clone()?
            } else {
                operands[0].legacy_text().clone()
            },
            container_literal: (kind != Kind::ListWrite).then_some(literal).flatten(),
            index: operands[1].sole_variable_substitution().map_or_else(
                || operands[1].legacy_text(),
                |(spelling, _)| spelling.to_owned(),
            ),
            container_word: operands[0].clone(),
            index_word: operands[1].clone(),
            list_set_bounds: dialect.list_set_bounds(),
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
}

/// Frozen operands of a successful selected index handler.
#[derive(Debug)]
pub struct NormalIndexAccess {
    /// Authored container operation.
    pub kind: NormalIndexAccessKind,
    /// Written container or variable-name presentation.
    pub container: String,
    /// Container value frozen before later argument effects, if proved.
    pub container_literal: Option<String>,
    /// Written index presentation.
    pub index: String,
    /// Index value frozen before later argument effects, if proved.
    pub index_literal: Option<String>,
    /// Selected native grammar for interpreting index values.
    pub index_syntax: Option<tcl_dialect::IndexSyntax>,
    /// Original effective container word, retaining its exact read site.
    pub container_word: WordExpr,
    /// Original effective index word, retaining its exact read site.
    pub index_word: WordExpr,
    /// Native list write boundary, independent of the editing catalogue.
    pub list_set_bounds: Option<tcl_dialect::ListSetBounds>,
}

#[derive(Debug)]
pub(super) enum NormalRepresentationProof {
    Handler(Box<ResolvedStatementInvocation>),
    // Normal-edge operand layout only. Unknown positional arity cannot donate
    // the representation, type, security or effect contracts of Handler.
    PatternLayout(Box<ResolvedStatementInvocation>),
    Iteration {
        expected: Option<tcl_registry::TclType>,
        words: EffectiveCommandWords,
        label: String,
    },
}

impl NormalRepresentationInvocation {
    /// Frozen name and nominal class of a selected normal naming factory.
    /// This candidate grants no command allocation, physical object receipt,
    /// method implementation, completion or executable transformation.
    #[must_use]
    pub fn naming_factory_candidate(&self) -> Option<(String, &'static str)> {
        let call = self.handler()?;
        let factory = call.facts.named_object_factory?;
        let argument = call.facts.argument_offset.checked_add(factory.argument())?;
        Some((call.argument_literal(argument)?, factory.class_name()))
    }

    /// Nominal class of a successful selected callable-result factory.
    /// No written name, created command token or method implementation is proved.
    #[must_use]
    pub fn callable_result_class_candidate(&self) -> Option<&'static str> {
        let call = self.handler()?;
        (call.facts.return_type == Some(tcl_registry::TclType::Object))
            .then_some(call.facts.named_object_factory?.class_name())
    }

    /// Literal list result and its first original written operand. The selected
    /// normal handler's result contract owns the construction; public spelling,
    /// captured alias prefixes and substituted operands cannot create an anchor.
    #[must_use]
    pub fn plain_literal_list_result(&self) -> Option<(String, tcl_lexer::Span)> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let tcl_registry::native_result::NativeResultSelection::ListArguments { from, len } = call
            .with_argument_words(|words| {
                contract.select(words.arguments(), call.facts.argument_offset)
            })
        else {
            return None;
        };
        if len == 0 {
            return None;
        }
        let mut values = Vec::with_capacity(len);
        let mut first = None;
        for index in from..from.checked_add(len)? {
            let word_index = index.checked_add(1)?;
            if !matches!(
                call.effective.origins.get(word_index)?,
                InvocationWordOrigin::Written(_)
            ) {
                return None;
            }
            let WordExpr::Literal { text, source } = call.effective.words.get(word_index)? else {
                return None;
            };
            if text.is_empty()
                || text.contains(['{', '}', '"', '\\', '$', '[', ']'])
                || text.chars().any(char::is_whitespace)
            {
                return None;
            }
            if call.argument_literal(index).as_deref() != Some(text) {
                return None;
            }
            let end = source
                .span
                .start()
                .checked_add(u32::try_from(text.len()).ok()?)?;
            first.get_or_insert(tcl_lexer::Span::new(source.span.start(), end));
            values.push(text.as_str());
        }
        Some((values.join(" "), first?))
    }

    /// Dictionary read operands under the selected normal handler. These retain
    /// original value reads and frozen keys without selecting executable code.
    #[must_use]
    pub fn dictionary_lookup(&self) -> Option<(WordExpr, Vec<Option<String>>)> {
        let call = self.handler()?;
        if call.facts.operation
            != tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::DictGet)
        {
            return None;
        }
        let input = call.facts.argument_offset;
        let keys = (input + 1..call.arguments.len())
            .map(|index| call.argument_literal(index))
            .collect();
        Some((call.effective.words.get(input + 1)?.clone(), keys))
    }

    /// Original operands constructing a successful dictionary value, with
    /// duplicate-key order preserved and no inferred representation licence.
    #[must_use]
    pub fn dictionary_constructor_words(&self) -> Option<Vec<WordExpr>> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let selection = call.with_argument_words(|words| {
            contract.select(words.arguments(), call.facts.argument_offset)
        });
        let tcl_registry::native_result::NativeResultSelection::DictionaryArguments { from, len } =
            selection
        else {
            return None;
        };
        Some(call.effective.words.get(from + 1..from + len + 1)?.to_vec())
    }

    /// Ordered original operands of a selected successful List constructor.
    /// Their source spans stay separate; this grants no object representation proof.
    #[must_use]
    pub fn list_constructor_words(&self) -> Option<Vec<WordExpr>> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let selection = call.with_argument_words(|words| {
            contract.select(words.arguments(), call.facts.argument_offset)
        });
        let tcl_registry::native_result::NativeResultSelection::ListArguments { from, len } =
            selection
        else {
            return None;
        };
        let end = from.checked_add(len)?.checked_add(1)?;
        if !(from + 1..end).all(|index| {
            matches!(
                call.effective.origins.get(index),
                Some(InvocationWordOrigin::Written(_))
            )
        }) {
            return None;
        }
        Some(call.effective.words.get(from + 1..end)?.to_vec())
    }

    pub(super) fn handler(&self) -> Option<&ResolvedStatementInvocation> {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation) => Some(invocation),
            NormalRepresentationProof::PatternLayout(_)
            | NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    fn pattern_handler(&self) -> Option<&ResolvedStatementInvocation> {
        match &self.proof {
            NormalRepresentationProof::Handler(call)
            | NormalRepresentationProof::PatternLayout(call) => Some(call),
            NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    /// Rendered-result hints of the selected normal handler only. Conditional
    /// layouts and synthetic iteration contracts do not prove these properties.
    pub(crate) fn rendered_value_traits(&self) -> Option<tcl_registry::Traits> {
        Some(self.handler()?.facts.traits.intersection(
            tcl_registry::Traits::IS_UNESCAPE
                | tcl_registry::Traits::RETURNS_PATH
                | tcl_registry::Traits::UNNORMALISED_HTTP_GETTER,
        ))
    }

    /// Successful assignment value for tracking representations through generic
    /// calls. Physical definitions still come from the shared SSA place owner.
    #[must_use]
    pub fn value_assignment(&self) -> Option<NormalValueAssignment> {
        let invocation = self.handler()?;
        if invocation.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            || invocation.arguments.len() != 2
        {
            return None;
        }
        Some(NormalValueAssignment {
            name: invocation.argument_literal(0)?,
            value: invocation.effective.words.get(2)?.clone(),
        })
    }

    /// The actual selected handler key, without borrowing diagnostic spelling.
    pub(crate) fn descriptor_key(&self) -> Option<tcl_registry::CanonicalCommandDescriptorKey<'_>> {
        Some(self.handler()?.facts.descriptor_key())
    }

    /// Whether the selected normal handler uses no caller frame access.
    /// This escape-analysis property does not select a compiler operation,
    /// donate an ABI implementation or prove an invocation's completion.
    #[must_use]
    pub fn is_frameless_runtime(&self) -> bool {
        self.handler().is_some_and(|invocation| {
            invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::FRAMELESS_RUNTIME)
        })
    }

    /// Normal index operands for diagnostic bounds analysis.
    /// This does not prove that the access succeeds or licence removing it.
    #[must_use]
    pub fn index_access(&self) -> Option<NormalIndexAccess> {
        use NormalIndexAccessKind as Kind;
        use tcl_registry::{IntrinsicId, SemanticOperationId};
        let invocation = self.handler()?;
        let offset = invocation.facts.argument_offset;
        let arguments = invocation.arguments.get(offset..)?;
        let kind = match invocation.facts.operation {
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex) if arguments.len() == 2 => {
                Kind::ListRead
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ListSet) if arguments.len() == 3 => {
                Kind::ListWrite
            }
            SemanticOperationId::Intrinsic(IntrinsicId::StringIndex) if arguments.len() == 2 => {
                Kind::StringRead
            }
            _ => return None,
        };
        Some(NormalIndexAccess {
            kind,
            container: if kind == Kind::ListWrite {
                invocation.argument_literal(offset)?
            } else {
                arguments[0].clone()?
            },
            container_literal: (kind != Kind::ListWrite)
                .then(|| invocation.argument_literal(offset))
                .flatten(),
            index: arguments[1].clone()?,
            index_literal: invocation.argument_literal(offset + 1),
            index_syntax: invocation
                .dialect
                .and_then(tcl_registry::InvocationDialect::index_syntax),
            container_word: invocation.effective.words.get(offset + 1)?.clone(),
            index_word: invocation.effective.words.get(offset + 2)?.clone(),
            list_set_bounds: invocation
                .dialect
                .and_then(tcl_registry::InvocationDialect::list_set_bounds),
        })
    }

    /// Evaluated argument count, including retained alias prefixes.
    #[must_use]
    pub fn argument_count(&self) -> usize {
        self.effective_words().words.len().saturating_sub(1)
    }

    /// Selected subcommand operand offset.
    #[must_use]
    pub fn argument_offset(&self) -> usize {
        self.handler().map_or(0, |call| call.facts.argument_offset)
    }

    /// Original words and retained alias origins, for positioned value reads.
    #[must_use]
    pub fn effective_words(&self) -> &EffectiveCommandWords {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation)
            | NormalRepresentationProof::PatternLayout(invocation) => &invocation.effective,
            NormalRepresentationProof::Iteration { words, .. } => words,
        }
    }

    /// Canonical diagnostic identity; this is not an execution licence.
    #[must_use]
    pub fn diagnostic_command(&self) -> &str {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation)
            | NormalRepresentationProof::PatternLayout(invocation) => {
                &invocation.facts.canonical_command
            }
            NormalRepresentationProof::Iteration { label, .. } => label,
        }
    }

    /// Diagnostic label including the selected canonical subcommand.
    #[must_use]
    pub fn diagnostic_label(&self) -> String {
        let mut label = self.diagnostic_command().to_owned();
        if let Some(invocation) = self.handler()
            && let tcl_registry::OwnedSubcommandResolution::Exact { canonical_name, .. }
            | tcl_registry::OwnedSubcommandResolution::UniquePrefix { canonical_name, .. } =
                &invocation.facts.subcommand
        {
            label.push(' ');
            label.push_str(canonical_name);
        }
        label
    }

    /// Original written pattern operand selected by the normal handler grammar.
    /// Captured alias-prefix patterns have no source argument position.
    #[must_use]
    pub fn pattern_source_argument_index(&self, registry: &CommandRegistry) -> Option<usize> {
        let call = self.pattern_handler()?;
        let query = call
            .dialect
            .and_then(tcl_registry::InvocationDialect::authoring_query);
        if registry
            .get_for_surface(&call.facts.canonical_command, query)?
            .pattern_type
            != Some(tcl_registry::patterns::PatternType::Regex)
        {
            return None;
        }
        if let Some(index) = self.native_pattern_source_argument {
            return Some(index);
        }
        let alternatives = self
            .pattern_operand_alternatives
            .iter()
            .map(|(argument, values)| tcl_registry::RoleOperandAlternatives {
                argument: *argument,
                values: tcl_registry::RoleOperandValues::Closed(values),
            })
            .collect::<Vec<_>>();
        let index = call.with_argument_words(|words| {
            let roles = registry.arg_role_assignments_consensus(
                &call.facts.canonical_command,
                words.arguments(),
                &alternatives,
                &[tcl_registry::ArgRole::Pattern],
            )?;
            match roles.as_slice() {
                [(index, tcl_registry::ArgRole::Pattern)] => Some(*index),
                _ => None,
            }
        })?;
        match call.effective.origins.get(index.checked_add(1)?)? {
            InvocationWordOrigin::Written(index) => index.checked_sub(1),
            _ => None,
        }
    }

    /// Written operands that may carry a regex on an admitted runtime layout.
    /// This hazard-only projection gives no guaranteed role, result type,
    /// variable definition, native operation or execution proof.
    #[must_use]
    pub fn possible_pattern_source_argument_indices(
        &self,
        registry: &CommandRegistry,
    ) -> Option<Vec<usize>> {
        if let Some(index) = self.pattern_source_argument_index(registry) {
            return Some(vec![index]);
        }
        let call = self.pattern_handler()?;
        if let Some(index) = self.native_pattern_source_argument {
            return Some(vec![index]);
        }
        let indices = call.with_argument_words(|words| {
            registry.possible_pattern_argument_indices_words(
                &call.facts.canonical_command,
                words.arguments(),
            )
        })?;
        Some(
            indices
                .into_iter()
                .filter_map(
                    |index| match call.effective.origins.get(index.checked_add(1)?)? {
                        InvocationWordOrigin::Written(index) => index.checked_sub(1),
                        _ => None,
                    },
                )
                .collect(),
        )
    }

    /// Authored normal representation mutation/coercion contract.
    #[must_use]
    pub fn representation_effect(&self) -> tcl_registry::RepresentationEffect {
        self.handler()
            .map_or(tcl_registry::RepresentationEffect::None, |call| {
                call.facts.representation_effect
            })
    }

    /// Normal conversion policy for independently proved ordinary containers.
    /// A semantic type, unknown object or abstract list cannot supply that proof.
    /// The policy applies to operand objects, never the command's return value.
    #[must_use]
    pub fn successful_ordinary_container_coercion(
        &self,
    ) -> Option<tcl_registry::representation::OrdinaryContainerCoercion> {
        let call = self.handler()?;
        if call.facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        call.with_argument_words(|words| {
            call.facts
                .representation_effect
                .successful_ordinary_container_coercion(
                    words.arguments(),
                    call.facts.argument_offset,
                )
        })
    }

    /// Successful ordinary list-input conversions under native foreach layout.
    /// Before success, possible conversions cannot commit a Must representation.
    #[must_use]
    pub fn successful_ordinary_container_coercions(
        &self,
    ) -> Option<Vec<tcl_registry::representation::OrdinaryContainerCoercion>> {
        let call = self.handler()?;
        if call.facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        call.with_argument_words(|words| {
            call.facts
                .representation_effect
                .successful_ordinary_container_coercions(
                    words.arguments(),
                    call.facts.argument_offset,
                )
        })
    }

    /// Authored normal byte-array operand conversion contract.
    #[must_use]
    pub fn byte_array_effect(&self) -> tcl_registry::ByteArrayEffect {
        self.handler()
            .map_or(tcl_registry::ByteArrayEffect::None, |call| {
                call.facts.byte_array_effect
            })
    }

    /// Encoder data object selected by the normal handler's operand protocol.
    /// Encoding names and option values cannot supply data provenance.
    #[must_use]
    pub fn encoded_value_word(&self) -> Option<WordExpr> {
        let call = self.handler()?;
        let offset = call.facts.argument_offset;
        let count = call.arguments.len().checked_sub(offset)?;
        let data = call.facts.byte_array_effect.encoded_value_argument(count)?;
        call.effective
            .words
            .get(offset.checked_add(data)?.checked_add(1)?)
            .cloned()
    }

    /// Declared representation of the successful selected result, when known.
    /// This supplies no result bytes, native opcode or non-normal guarantee.
    #[must_use]
    pub fn result_representation_type(&self) -> Option<tcl_registry::TclType> {
        self.handler()?.facts.return_type
    }

    /// Authored element representation of the successful selected result.
    /// Input element types still require positioned reads of the retained words.
    #[must_use]
    pub fn result_elements(&self) -> Option<tcl_registry::ReturnElements> {
        self.handler()?.facts.return_elements
    }

    /// Whether a successful selected form produces a byte-array object.
    #[must_use]
    pub fn returns_byte_array(&self) -> bool {
        self.handler()
            .is_some_and(|call| call.facts.return_type == Some(tcl_registry::TclType::ByteArray))
    }

    /// Selected byte payload layout, with unknown forms excluded.
    #[must_use]
    pub fn byte_array_payload(&self) -> Option<tcl_registry::BytePayloadSpec> {
        self.handler()?.facts.byte_array_payload
    }

    /// Whether normal mutation reads the old value before writing it.
    #[must_use]
    pub fn reads_before_write(&self) -> bool {
        self.handler().is_some_and(|call| {
            call.facts
                .traits
                .contains(tcl_registry::Traits::READS_BEFORE_WRITE)
        })
    }

    /// Argument roles only for distinguishing inert words from value reads.
    /// These do not establish script entry or execution topology.
    #[must_use]
    pub fn operand_roles(&self) -> &[(u8, tcl_registry::ArgRole)] {
        self.handler()
            .map_or(&[], |call| call.facts.arg_roles.as_slice())
    }

    /// Original operands selected as deferred scripts by the actual normal
    /// handler and its frozen layout. This proves metadata only: no script
    /// entry, callback dispatch or editable source is supplied by these indices.
    /// Unknown timing coverage returns `None`. Even `Some([])` cannot prove
    /// callback absence: operands captured in an alias prefix have no writable
    /// source index in this invocation and are deliberately omitted.
    #[must_use]
    pub fn deferred_script_source_argument_indices(&self) -> Option<Vec<usize>> {
        let call = self.handler()?;
        Some(
            call.facts
                .deferred_script_argument_indices()?
                .iter()
                .filter_map(|&argument| call.effective.written_argument(argument))
                .collect(),
        )
    }

    /// Frozen evaluated argument knowledge, without inventing unknown bytes.
    #[must_use]
    pub fn argument_word(&self, index: usize) -> EffectiveInvocationWord {
        self.handler()
            .map_or(EffectiveInvocationWord::Dynamic, |call| {
                call.argument_word(index)
            })
    }

    /// Known evaluated value, including an alias prefix's captured value.
    #[must_use]
    pub fn argument_literal(&self, index: usize) -> Option<String> {
        self.handler()?.argument_literal(index)
    }

    /// Registry-owned positional representation hint selected with frozen argv.
    /// Genuine synthetic iterators carry their producer's input representation.
    #[must_use]
    pub fn argument_type_hint(
        &self,
        index: usize,
    ) -> Option<&'static tcl_registry::hooks::ArgTypeHint> {
        use tcl_registry::{TclType, hooks::ArgTypeHint};
        const LIST: ArgTypeHint = ArgTypeHint {
            expected: Some(TclType::List),
            shimmers: true,
            transparent_from: &[],
        };
        const DICT: ArgTypeHint = ArgTypeHint {
            expected: Some(TclType::Dict),
            shimmers: true,
            transparent_from: &[],
        };
        match &self.proof {
            NormalRepresentationProof::Handler(call) => call.facts.argument_type_hint(index),
            NormalRepresentationProof::Iteration { expected, .. }
                if index < self.argument_count() =>
            {
                match expected {
                    Some(TclType::List) => Some(&LIST),
                    Some(TclType::Dict) => Some(&DICT),
                    _ => None,
                }
            }
            NormalRepresentationProof::PatternLayout(_)
            | NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    /// Conditional normal cache disposition for the selected stock Length
    /// operand. The original current stock class must be authenticated by the
    /// caller; semantic types and result types cannot supply that evidence.
    /// This is conversion-cost metadata, not success, value or erasure proof.
    #[must_use]
    pub fn stock_length_operand_cache(
        &self,
        index: usize,
        class: tcl_registry::native_stock_list::NativeStockListInputClass,
        bytes: Option<&[u8]>,
    ) -> Option<tcl_registry::native_stock_list::NativeStockListCacheDisposition> {
        let call = self.handler()?;
        call.with_argument_words(|words| {
            let protocol = call.facts.stock_list_length_protocol(words.arguments())?;
            if protocol.argument() != index {
                return None;
            }
            protocol.normal_cache_disposition(class, bytes)
        })
    }

    /// Normal expression read projection under the caller's retained full grammar.
    pub(crate) fn source_expression_with_syntax_context(
        &self,
        parser: tcl_syntax::expr::parser::ExprParseContext,
        span: tcl_lexer::Span,
        parent: Option<crate::command_binding::CommandAllocationSite>,
    ) -> Option<crate::word_subst::LiftedSourceExpression> {
        crate::word_subst::source_expression_from_invocation_with_syntax_context(
            self.handler()?,
            parser,
            span,
            parent,
        )
    }
}

/// Select a representation contract for a reached normal handler.
/// Compiler uncertainty never becomes an opcode or body-execution proof here.
///
/// ```no_run
/// use tcl_compiler::{ir::CommandTokens, registry_invocation::normal_representation_invocation};
/// use tcl_registry::CommandRegistry;
/// fn successful_result(registry: &CommandRegistry, tokens: &CommandTokens) {
///     if let Some(normal) = normal_representation_invocation(registry, None, tokens) {
///         let result_representation = normal.result_representation_type();
///         // This hint supplies neither result bytes nor an opcode.
///         let _ = result_representation;
///     }
/// }
/// ```
#[must_use]
pub fn normal_representation_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Successful representation metadata under the actual availability generation.
/// This retains all normal-handler and operand requirements of the typed owner;
/// availability does not create a successful handler or a native invocation.
#[must_use]
pub fn normal_representation_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Successful representation metadata with the caller's complete supplied
/// availability and source-policy context. Selection remains independent of
/// successful completion, original value identity and Native execution entry.
#[must_use]
pub fn normal_representation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    if let Some(crate::ir::SyntheticMarker::IterationBindings(expected)) = tokens.synthetic {
        // Minted only by the typed iterator lowering. This selects input value
        // conversion, never an invocation of the synthetic diagnostic label.
        return Some(NormalRepresentationInvocation {
            proof: NormalRepresentationProof::Iteration {
                expected,
                words: EffectiveCommandWords {
                    words: tokens.words().to_vec(),
                    origins: (0..tokens.words().len())
                        .map(InvocationWordOrigin::Written)
                        .collect(),
                    binding_prefix: Vec::new(),
                },
                label: tokens.argv_texts.first()?.clone(),
            },
            native_pattern_source_argument: None,
            pattern_operand_alternatives: Vec::new(),
        });
    }
    let invocation = resolved_tokens_invocation_with_metadata_context(registry, context, tokens)
        .or_else(|| resolved_handler_invocation_with_metadata_context(registry, context, tokens))?;
    // The public ensemble's identity does not select an unavailable or
    // indeterminate member. Parent facts retain that uncertainty for other
    // purposes; they cannot supply a successful value-handler contract here.
    if matches!(
        invocation.facts.subcommand,
        tcl_registry::OwnedSubcommandResolution::Unknown { .. }
            | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
            | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
    ) {
        return None;
    }
    // Representation contracts require accepted arity. A conditional Pattern
    // projection may retain unknown positional arity under a separate carrier.
    // Expansion still cannot fabricate ordinary operand positions.
    if invocation
        .effective
        .words
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. }))
    {
        return None;
    }
    if invocation.facts.argument_offset > invocation.arguments.len() {
        return None;
    }
    let accepted = invocation.facts.arity_accepts_frozen_arguments();
    if accepted == Some(false)
        || (accepted.is_none() && !has_regex_pattern_contract(&invocation, registry))
    {
        return None;
    }
    let native_pattern_source_argument = compiled_pattern_source_argument(tokens);
    let pattern_operand_alternatives = pattern_operand_alternatives(tokens, &invocation, registry);
    let invocation = Box::new(invocation);
    Some(NormalRepresentationInvocation {
        native_pattern_source_argument,
        pattern_operand_alternatives,
        proof: if accepted == Some(true) {
            NormalRepresentationProof::Handler(invocation)
        } else {
            NormalRepresentationProof::PatternLayout(invocation)
        },
    })
}

fn has_regex_pattern_contract(
    invocation: &ResolvedStatementInvocation,
    registry: &CommandRegistry,
) -> bool {
    let query = invocation
        .dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query);
    registry
        .get_for_surface(&invocation.facts.canonical_command, query)
        .and_then(|spec| spec.pattern_type)
        == Some(tcl_registry::patterns::PatternType::Regex)
}

// Values come from each original read boundary, before later argv callbacks.
// They are used only to compare operand layouts, never to freeze an argv value.
pub(super) fn pattern_operand_alternatives(
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
    registry: &CommandRegistry,
) -> Vec<(usize, Vec<String>)> {
    if !has_regex_pattern_contract(invocation, registry) {
        return Vec::new();
    }
    invocation
        .effective
        .words
        .iter()
        .skip(1)
        .enumerate()
        .filter(|(index, _)| invocation.argument_literal(*index).is_none())
        .filter_map(|(index, word)| {
            let (_, source) = word.sole_variable_substitution()?;
            let access = crate::command_binding::SourceVariableAccess::find_at_site(
                &tokens.variable_accesses,
                source,
            )?;
            closed_read_literals(access, registry).map(|values| (index, values))
        })
        .collect()
}

fn closed_read_literals(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> Option<Vec<String>> {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed
        || access.context_alternatives().is_empty()
    {
        return None;
    }
    let mut values = std::collections::BTreeSet::new();
    for context in access.context_alternatives() {
        let crate::literal_contents::LiteralContentsAlternatives::Closed(contents) =
            context.substitution_literal_alternatives(&access.original_spelling, registry)
        else {
            return None;
        };
        values.extend(contents.values().iter().cloned());
        if values.len() > 8 || values.iter().map(String::len).sum::<usize>() > 8192 {
            return None;
        }
    }
    Some(values.into_iter().collect())
}

fn compiled_pattern_source_argument(tokens: &CommandTokens) -> Option<usize> {
    let binding = tokens.source_binding.as_ref()?;
    let proof = binding.native_operand_layout()?;
    // Layout and normal handler identities have independent authority. An
    // argv mutation or conflicting registration cannot borrow this layout.
    if binding.proved_handler_target()? != proof.target()
        || proof.compilation_site().offset != tokens.words().first()?.source().span.start()
    {
        return None;
    }
    let tcl_registry::native_compilation::NativeCompilationOperandLayout::Pattern {
        argument_index,
        ..
    } = proof.layout();
    let effective = compose_original_effective_words(
        tokens,
        &proof.target().command,
        &proof.target().prepended,
    )?;
    match effective.origins.get(argument_index.checked_add(1)?)? {
        InvocationWordOrigin::Written(index) => index.checked_sub(1),
        _ => None,
    }
}

/// Successful result security properties of the exact selected normal handler.
/// No compiler operation, completion or catalogue candidate is exposed.
#[derive(Debug)]
pub struct NormalTaintInvocation {
    pub(super) invocation: Box<ResolvedStatementInvocation>,
}

impl NormalTaintInvocation {
    /// Authored source colour after actual frozen argc selection. Safety bits
    /// require this selected implementation and never come from its spelling.
    #[must_use]
    pub fn source_colour(&self) -> Option<tcl_registry::taint::TaintColour> {
        self.invocation.facts.taint_source
    }
    /// Declared bounded successful numeric result, without an execution licence.
    #[must_use]
    pub fn is_sanitiser(&self) -> bool {
        matches!(
            self.invocation.facts.return_type,
            Some(tcl_registry::TclType::Int | tcl_registry::TclType::Boolean)
        )
    }

    /// Authored successful-result transform proven from frozen evaluated argv.
    #[must_use]
    pub fn transform_colour(&self) -> Option<tcl_registry::taint::TaintColour> {
        self.invocation.facts.taint_transform
    }
}

/// Resolve security metadata through the same accepted normal handler proof as
/// representation queries, retaining private worker and vendor prerequisites.
#[must_use]
pub fn normal_taint_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalTaintInvocation> {
    let normal = normal_representation_invocation(registry, context, tokens)?;
    let NormalRepresentationProof::Handler(invocation) = normal.proof else {
        return None;
    };
    Some(NormalTaintInvocation { invocation })
}

/// Select normal result metadata through the supplied generation and original
/// handler proof. Missing or foreign availability never reconstructs a label.
pub(crate) fn normal_taint_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalTaintInvocation> {
    let context = context.filter(|context| context.matches_registry(registry))?;
    let normal =
        normal_representation_invocation_with_metadata_context(registry, Some(context), tokens)?;
    let NormalRepresentationProof::Handler(invocation) = normal.proof else {
        return None;
    };
    Some(NormalTaintInvocation { invocation })
}

/// Statement convenience projection retaining the original token proof.
#[must_use]
pub fn normal_statement_representation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    statement: &crate::ir::Statement,
) -> Option<NormalRepresentationInvocation> {
    normal_statement_representation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        statement,
    )
}

pub(crate) fn normal_statement_representation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    statement: &crate::ir::Statement,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(registry, context, statement.tokens()?)
}
