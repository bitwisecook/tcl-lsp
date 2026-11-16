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

//! Resolving statements and command words to invocations, with the variable effects they project.

use super::super::namespace_context::NamespaceKeyQuery;
use super::super::{ModuleCommandBindings, ResolvedCommandTarget, SourceInvocationBinding};
use super::ResolvedBindingInvocation;
use super::evaluated_bodies::readable_script_argument_with_optional_metadata;
use crate::ir::Statement;
use tcl_registry::CommandRegistry;

impl ModuleCommandBindings {
    /// Availability phase retained by the source-entry owner. Reading this
    /// phase grants no entered body, native handler or compiler admission.
    pub(crate) fn invocation_realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.baseline.invocation_realm
    }

    /// Runtime policies supplied by the source execution entry contract.
    #[must_use]
    pub(crate) fn invocation_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.baseline.dialect
    }
}

/// Frame selected for a registry-resolved evaluated script body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFrameBodySelection {
    /// The frame in which the invocation itself executes.
    Current,
    /// A frame selected by the invocation's registry-owned level grammar.
    Selected(tcl_registry::frame_effect::FrameLevel),
}

/// Source-safe projection of one registry-resolved frame-evaluated body.
pub(crate) enum ResolvedFrameBody {
    /// The invocation has no script-in-frame descriptor.
    NotApplicable,
    /// Tcl rejects the invocation before a body can run.
    KnownError,
    /// One exact source script and its selected frame.
    Readable {
        source: String,
        selection: ResolvedFrameBodySelection,
    },
    /// A body may run in the selected frame, but its source is not exact.
    Opaque {
        selection: ResolvedFrameBodySelection,
    },
}

impl ResolvedBindingInvocation {
    pub(crate) fn frame_alias_layout(
        &self,
        spec: tcl_registry::frame_effect::FrameEffectSpec,
        registry: &CommandRegistry,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> tcl_registry::frame_effect::FrameArgumentResolution {
        if self.exact_argument_count != Some(self.arguments.len()) {
            return tcl_registry::frame_effect::FrameArgumentResolution::Unknown;
        }
        let words = self
            .literal_arguments
            .iter()
            .map(|value| {
                value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                )
            })
            .collect::<Vec<_>>();
        let mut arguments =
            tcl_registry::InvocationArguments::Structured(&words).with_profile(registry.profile());
        if let Some(dialect) = dialect {
            arguments = arguments.with_dialect(dialect);
        }
        spec.successful_layout(arguments).layout
    }

    pub(crate) fn literal_argument(&self, index: usize) -> Option<&str> {
        self.literal_arguments.get(index)?.as_deref()
    }

    pub(crate) fn scalar_parameter_argument<'a>(
        &'a self,
        index: usize,
        parameters: &[String],
    ) -> Option<&'a str> {
        let name = self.source_scalar_arguments.get(index)?.as_deref()?;
        parameters
            .iter()
            .any(|parameter| parameter == name)
            .then_some(name)
    }

    /// Explicit standalone frame-body shape under the caller's catalogue.
    /// A deliberate unprofiled query keeps its own-surface route; it does not
    /// manufacture an analysis input, native entry or default context.
    pub(crate) fn resolved_frame_body_standalone(
        &self,
        registry: &CommandRegistry,
        bindings: &ModuleCommandBindings,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) -> ResolvedFrameBody {
        let metadata = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(crate::registry_invocation::InvocationMetadataContext::from);
        self.resolved_frame_body_with_optional_metadata(registry, bindings, namespace, metadata)
    }

    /// Frame-body layout under actual supplied availability, without a
    /// Native frame-entry or execution grant. Missing input widens selection.
    pub(crate) fn resolved_frame_body_with_metadata_context(
        &self,
        registry: &CommandRegistry,
        bindings: &ModuleCommandBindings,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> ResolvedFrameBody {
        let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
            return ResolvedFrameBody::Opaque {
                selection: ResolvedFrameBodySelection::Selected(
                    tcl_registry::frame_effect::FrameLevel::Dynamic,
                ),
            };
        };
        self.resolved_frame_body_with_optional_metadata(
            registry,
            bindings,
            namespace,
            Some(metadata),
        )
    }

    fn resolved_frame_body_with_optional_metadata(
        &self,
        registry: &CommandRegistry,
        bindings: &ModuleCommandBindings,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> ResolvedFrameBody {
        use tcl_registry::frame_effect::FrameArgLayout;

        let Some(spec) = self.facts.frame_effect else {
            return ResolvedFrameBody::NotApplicable;
        };
        let refs: Vec<&str> = self.arguments.iter().map(String::as_str).collect();
        let (body_index, body_len, selection) = match spec.layout {
            FrameArgLayout::ScriptInCurrentFrame => {
                (0, refs.len(), ResolvedFrameBodySelection::Current)
            }
            FrameArgLayout::ScriptInSelectedFrame => {
                let words = self
                    .literal_arguments
                    .iter()
                    .map(|word| {
                        word.as_deref().map_or(
                            tcl_registry::InvocationWord::Dynamic,
                            tcl_registry::InvocationWord::Literal,
                        )
                    })
                    .collect::<Vec<_>>();
                let mut invocation = tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&self.command),
                    &words,
                )
                .with_profile(registry.profile());
                if let Some(dialect) = bindings.baseline.dialect {
                    invocation = invocation.with_dialect(dialect);
                }
                let arguments = invocation.arguments();
                let (width, level) = match spec.resolve_arguments(arguments) {
                    tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                        level_word_len,
                        level,
                    } => (level_word_len, level),
                    tcl_registry::frame_effect::FrameArgumentResolution::Invalid => {
                        return ResolvedFrameBody::KnownError;
                    }
                    tcl_registry::frame_effect::FrameArgumentResolution::Unknown => {
                        return ResolvedFrameBody::Opaque {
                            selection: ResolvedFrameBodySelection::Selected(
                                tcl_registry::frame_effect::FrameLevel::Dynamic,
                            ),
                        };
                    }
                };
                let body = &refs[width..];
                (
                    refs.len().saturating_sub(body.len()),
                    body.len(),
                    ResolvedFrameBodySelection::Selected(level),
                )
            }
            FrameArgLayout::AliasPairs | FrameArgLayout::OpaqueCallerVars => {
                return ResolvedFrameBody::NotApplicable;
            }
        };
        if body_len == 0 {
            return ResolvedFrameBody::KnownError;
        }
        if self.exact_argument_count.is_none() || body_len != 1 {
            return ResolvedFrameBody::Opaque { selection };
        }
        readable_script_argument_with_optional_metadata(
            self, body_index, registry, bindings, namespace, metadata,
        )
        .map_or(ResolvedFrameBody::Opaque { selection }, |source| {
            ResolvedFrameBody::Readable { source, selection }
        })
    }
}

impl ModuleCommandBindings {
    /// Resolve every live source-safe invocation selected by the exact active
    /// registry. Alias chains are expanded to their terminal target.
    #[must_use]
    pub(crate) fn resolve_statement(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<ResolvedBindingInvocation> {
        let context = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile);
        let context = context.map(crate::registry_invocation::InvocationMetadataContext::from);
        self.resolve_statement_with_optional_metadata(stmt, registry, context, namespace)
    }

    /// Original may-target metadata under supplied complete availability.
    /// Missing or foreign metadata refuses; targets and composed original
    /// operands do not establish a Native frame, completion or physical effect.
    #[must_use]
    pub(crate) fn resolve_statement_with_metadata_context(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<ResolvedBindingInvocation> {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return Vec::new();
        };
        self.resolve_statement_with_optional_metadata(stmt, registry, Some(context), namespace)
    }

    fn resolve_statement_with_optional_metadata(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<ResolvedBindingInvocation> {
        if !stmt.is_executable_invocation() {
            return Vec::new();
        }
        let source_span = stmt.span();
        let (Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. }) = stmt
        else {
            return Vec::new();
        };
        if tokens.is_none() {
            return Vec::new();
        }

        let realm = tokens
            .as_ref()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.baseline.invocation_realm);
        let mut resolved = Vec::new();
        self.for_each_resolved_invocation(stmt, namespace, |target, words| {
            if !target.registry_backed {
                return;
            }
            let Some(facts) =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry,
                    context.map(crate::registry_invocation::InvocationMetadataContext::context),
                    words,
                    realm,
                )
                .resolved()
                .map(|resolved| Box::new(resolved.facts()))
            else {
                return;
            };
            let Some(mut arguments) = target
                .prepended
                .iter()
                .map(|word| word.as_registry_word().literal().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
            else {
                return;
            };
            arguments.extend(args.iter().cloned());
            let invocation_arguments = words.arguments();
            let exact_argument_count = invocation_arguments.exact_argv_len();
            let literal_arguments = (0..arguments.len())
                .map(|index| match invocation_arguments.argv_at(index) {
                    tcl_registry::InvocationArgument::Word(
                        tcl_registry::InvocationWord::Literal(literal),
                    ) => Some(literal.to_owned()),
                    tcl_registry::InvocationArgument::Word(_)
                    | tcl_registry::InvocationArgument::Indeterminate
                    | tcl_registry::InvocationArgument::Missing => None,
                })
                .collect();
            let source_scalar_arguments = arguments
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    let written = index.checked_sub(target.prepended.len())?;
                    let tokens = tokens.as_ref()?;
                    let word = tokens.words().get(written.checked_add(1)?)?;
                    if word.legacy_text() != *args.get(written)? {
                        return None;
                    }
                    let config = tokens
                        .source_binding
                        .as_ref()?
                        .original_lexer_config_for_tokens(tokens)?;
                    let (spelling, source) = word.sole_variable_substitution()?;
                    if source.provenance != crate::ir::Provenance::Source {
                        return None;
                    }
                    tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
                        .map(str::to_owned)
                })
                .collect();
            resolved.push(ResolvedBindingInvocation {
                command: target.command.clone(),
                source_span,
                facts,
                arguments,
                literal_arguments,
                source_scalar_arguments,
                exact_argument_count,
            });
        });
        resolved
    }

    /// Visit every effective terminal invocation selected for `stmt`.
    ///
    /// This is the single source of truth for joining source words with the
    /// literal arguments prepended by an `interp alias` chain. Both retained
    /// user procedures and registry-backed targets are reported; consumers
    /// select the semantic domain they own from [`ResolvedCommandTarget`].
    /// A source-aware statement keeps substitution and expansion opaque, while
    /// a hand-built statement without tokens falls back to the all-literal
    /// argument view.
    fn contextual_invocation_words<'w>(
        &self,
        command: &'w str,
        arguments: &'w [tcl_registry::InvocationWord<'w>],
    ) -> tcl_registry::InvocationWords<'w> {
        let words = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(command),
            arguments,
        );
        self.baseline
            .dialect
            .map_or(words, |dialect| words.with_dialect(dialect))
    }

    pub(crate) fn for_each_resolved_invocation<F>(
        &self,
        stmt: &Statement,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        mut visit: F,
    ) where
        F: for<'w> FnMut(&'w ResolvedCommandTarget, tcl_registry::InvocationWords<'w>),
    {
        if !stmt.is_executable_invocation() {
            return;
        }
        let (Statement::Call {
            command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            args,
            tokens,
            ..
        }) = stmt
        else {
            return;
        };
        let targets = self.targets(command, namespace);
        if let Some(tokens) = tokens {
            let words = tokens.words();
            if !matches!(
                words
                    .first()
                    .map(crate::registry_invocation::invocation_word),
                Some(tcl_registry::InvocationWord::Literal(_))
            ) {
                return;
            }
            for target in targets {
                let mut arguments: Vec<_> = target
                    .prepended
                    .iter()
                    .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                    .collect();
                arguments.extend(
                    words
                        .get(1..)
                        .unwrap_or_default()
                        .iter()
                        .map(crate::registry_invocation::invocation_word),
                );
                visit(
                    &target,
                    self.contextual_invocation_words(&target.command, &arguments),
                );
            }
            return;
        }

        for target in targets {
            let arguments: Vec<_> = target
                .prepended
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .chain(
                    args.iter()
                        .map(|word| tcl_registry::InvocationWord::Literal(word.as_str())),
                )
                .collect();
            visit(
                &target,
                self.contextual_invocation_words(&target.command, &arguments),
            );
        }
    }

    /// Visit every effective terminal invocation selected for parsed embedded
    /// command words.
    ///
    /// This is the sole binding owner for joining a tokenised embedded command
    /// with the literal argv prefix supplied by an `interp alias` chain. The
    /// caller supplies the exact execution namespace and words tokenised with
    /// the active dialect; consumers never repeat binding or prefix logic.
    pub(crate) fn for_each_resolved_command_words<F>(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        mut visit: F,
    ) where
        F: for<'w> FnMut(&'w ResolvedCommandTarget, tcl_registry::InvocationWords<'w>),
    {
        let Some(command) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return;
        };
        for target in self.targets(command, namespace) {
            let mut arguments: Vec<_> = target
                .prepended
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .collect();
            arguments.extend(
                words
                    .get(1..)
                    .unwrap_or_default()
                    .iter()
                    .map(crate::ir_helpers::CommandWord::invocation_word),
            );
            visit(
                &target,
                self.contextual_invocation_words(&target.command, &arguments),
            );
        }
    }

    /// Resolve parsed embedded command words through the same closed binding,
    /// alias-prefix, registry, and dialect context as direct statements.
    #[must_use]
    pub(crate) fn resolve_command_words(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        let context = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile);
        let context = context.map(crate::registry_invocation::InvocationMetadataContext::from);
        self.resolve_command_words_with_optional_metadata(words, registry, context, namespace)
    }

    /// Original may-target metadata under supplied complete availability.
    /// Missing or foreign metadata refuses; targets and composed original
    /// operands do not establish a Native frame, completion or physical effect.
    #[must_use]
    pub(crate) fn resolve_command_words_with_metadata_context(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return Vec::new();
        };
        self.resolve_command_words_with_optional_metadata(words, registry, Some(context), namespace)
    }

    fn resolve_command_words_with_optional_metadata(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        let mut resolved = Vec::new();
        self.for_each_resolved_command_words(words, namespace, |target, invocation_words| {
            if !target.registry_backed {
                return;
            }
            if let Some(facts) =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry,
                    context.map(crate::registry_invocation::InvocationMetadataContext::context),
                    invocation_words,
                    self.baseline.invocation_realm,
                )
                .resolved()
                .map(|invocation| invocation.facts())
            {
                resolved.push(facts);
            }
        });
        resolved
    }

    /// Project every registry-backed variable value write a direct statement
    /// may perform after command binding and alias-prefix resolution.
    ///
    /// This is the compiler's single bridge from the module command lattice
    /// to the registry-owned write projection.  In particular, an alias such
    /// as `interp alias {} put {} set x` contributes the literal `x` before
    /// the source call's words, while an alias to a retained user procedure
    /// never borrows a same-named registry command's semantics.
    #[must_use]
    pub(crate) fn variable_write_projection(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        self.variable_write_projection_with_metadata_context(
            stmt,
            registry,
            namespace,
            registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(Into::into),
        )
    }

    /// Conditional source footprint using the supplied availability generation.
    /// Missing or foreign metadata widens; original alias prefixes retain their
    /// values without certifying a variable write or Native frame entry.
    pub(crate) fn variable_write_projection_with_metadata_context(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> tcl_registry::VariableWriteProjection {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return tcl_registry::VariableWriteProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableWriteProjection::default()
            };
        };
        let literal_head = match stmt {
            Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } => {
                tokens.as_ref().is_none_or(|tokens| {
                    tokens.synthetic.is_none()
                        && tokens.words().first().is_none_or(|head| {
                            crate::registry_invocation::invocation_word(head)
                                .literal()
                                .is_some()
                        })
                })
            }
            _ => return tcl_registry::VariableWriteProjection::default(),
        };
        if !literal_head {
            // The ordinary computed-command dispatch path owns this barrier;
            // do not manufacture a second variable-frame effect here.
            return tcl_registry::VariableWriteProjection::default();
        }

        let realm = match stmt
            .tokens()
            .and_then(|tokens| tokens.source_binding.as_ref())
        {
            Some(binding) => {
                // A site the walk never reached retains no phase. Only the
                // hosted loader world has more than one to choose from.
                let Some(realm) = binding.invocation_realm().or_else(|| {
                    (!self.baseline.hosted_rule_loader_world())
                        .then_some(self.baseline.invocation_realm)
                }) else {
                    return tcl_registry::VariableWriteProjection {
                        opaque_variable_frame: true,
                        ..tcl_registry::VariableWriteProjection::default()
                    };
                };
                realm
            }
            None => self.baseline.invocation_realm,
        };
        let mut projection = tcl_registry::VariableWriteProjection::default();
        self.for_each_resolved_invocation(stmt, namespace, |target, words| {
            if !target.registry_backed {
                return;
            }
            let candidate = registry.variable_write_projection_in_resolved_context(
                context.context(),
                words,
                realm,
            );
            projection.opaque_variable_frame |= candidate.opaque_variable_frame;
            for name in candidate.literal_names {
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
        });
        projection
    }

    /// Conditional variable writes from one original embedded command vector.
    /// The terminal source target composes retained alias prefixes before the
    /// Registry footprint; unavailable metadata/targets remain opaque.
    pub(crate) fn variable_write_projection_for_command_words_with_metadata_context(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return tcl_registry::VariableWriteProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableWriteProjection::default()
            };
        };
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return tcl_registry::VariableWriteProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableWriteProjection::default()
            };
        };
        let mut projection = tcl_registry::VariableWriteProjection {
            opaque_variable_frame: self.target_resolution_may_be_unknown(head, namespace),
            ..tcl_registry::VariableWriteProjection::default()
        };
        self.for_each_resolved_command_words(words, namespace, |target, invocation| {
            if !target.registry_backed {
                return;
            }
            let candidate = registry.variable_write_projection_in_resolved_context(
                context.context(),
                invocation,
                self.baseline.invocation_realm,
            );
            projection.opaque_variable_frame |= candidate.opaque_variable_frame;
            for name in candidate.literal_names {
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
            for name in candidate.read_before_write_names {
                if !projection.read_before_write_names.contains(&name) {
                    projection.read_before_write_names.push(name);
                }
            }
        });
        projection
    }

    /// Join possible source effects from an alternative of this same retained
    /// world. This grants no execution; the existing baseline invariant applies.
    pub(crate) fn join_possible_source_effects(&mut self, other: &Self) {
        self.join(other);
    }

    /// Selected by-name reads under complete supplied availability and aliases.
    /// Missing metadata or target alternatives retain an opaque residual.
    pub(crate) fn variable_read_projection_for_command_words_with_metadata_context(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableReadProjection {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return tcl_registry::VariableReadProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableReadProjection::default()
            };
        };
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return tcl_registry::VariableReadProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableReadProjection::default()
            };
        };
        let mut projection = tcl_registry::VariableReadProjection {
            opaque_variable_frame: self.target_resolution_may_be_unknown(head, namespace),
            ..tcl_registry::VariableReadProjection::default()
        };
        self.for_each_resolved_command_words(words, namespace, |target, invocation| {
            if !target.registry_backed {
                return;
            }
            let candidate = registry.variable_read_projection_in_resolved_context(
                context.context(),
                invocation,
                self.baseline.invocation_realm,
            );
            projection.opaque_variable_frame |= candidate.opaque_variable_frame;
            for name in candidate.literal_names {
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
        });
        projection
    }
}
