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

//! The command schema possible at a source origin, and the list-quoted producers observed through it.

use super::super::{Analyser, ResolvedAnalysisInput};
use super::{SourceCommandOrigin, SourceInterpreterVisibilitySnapshot, SourceVisibleSlot};
use tcl_lexer::{NativeWord, SourceImage, Token};
use tcl_names::namespace::SignatureNamespaceScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalSourceAsciiListValue {
    operand: NativeWord,
    pub(super) producer: Vec<NativeWord>,
    pub(super) input: ResolvedAnalysisInput,
    bytes: Vec<u8>,
}

impl OriginalSourceAsciiListValue {
    pub(super) fn matches(
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

pub(super) struct ConditionalSourceCommandSchema {
    pub(super) roles: Vec<(usize, tcl_registry::ArgRole)>,
    scripts: Vec<usize>,
    source_values: Vec<Option<OriginalSourceAsciiListValue>>,
    pub(super) traits: tcl_registry::Traits,
    pub(super) hook: Option<tcl_registry::hooks::AnalyserHookId>,
    pub(super) transitions: tcl_registry::StateTransitions,
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
    pub(super) fn possible_source_schema(
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

    pub(super) fn source_load_schema(
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
                .with_dialect(tcl_native_model::InvocationDialect::of_profile(
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
    pub(in super::super) fn observe_indirect_interp_visibility(
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
            let Some(segment) = tcl_source::segment::segment_commands_with_offset_and_config(
                text,
                body.start(),
                word.config().nested(),
            )
            .into_iter()
            .next() else {
                return;
            };
            if segment.argv.len() != native.len() {
                return;
            }
            let domain = super::super::scope::scope_at(&self.result.global_scope, scope_path)
                .and_then(|scope| scope.original_interpreter_source_domain.clone());
            let original = snapshot.literal_lambda_body(&native[1..], native, namespace, domain);
            self.handle_apply_command_with_source_namespace(
                &segment.texts[2..],
                &segment.argv[2..],
                original,
            );
        }
    }
}
