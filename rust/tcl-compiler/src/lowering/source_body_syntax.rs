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

//! Conditional source syntax is cached separately from executed body proofs.

use super::{CommandTokens, Lowerer, Script, SegmentedCommand, Statement};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SourcePurpose {
    Native,
    Logical,
    Authored,
}

#[derive(Debug, Clone)]
pub(super) enum SourceBodySyntaxOwner {
    Native(std::sync::Arc<crate::command_binding::OriginalNativeProcedureBody>),
    Logical(std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>),
    Authored(std::sync::Arc<crate::registry_invocation::OriginalAuthoredProcedureBody>),
}

impl SourceBodySyntaxOwner {
    fn geometry(
        &self,
    ) -> Option<(
        SourcePurpose,
        &tcl_lexer::SourceImage,
        tcl_lexer::Span,
        String,
    )> {
        match self {
            Self::Native(card) => Some((
                SourcePurpose::Native,
                card.site().source.source_image(),
                card.body_span()?,
                card.namespace().display()?,
            )),
            Self::Logical(card) => Some((
                SourcePurpose::Logical,
                card.site().source.source_image(),
                card.scope().scope_body().content_span(),
                tcl_syntax::naming::checked_namespace_path_key(card.scope().namespace_recipe())?,
            )),
            Self::Authored(card) => Some((
                SourcePurpose::Authored,
                card.site().source.source_image(),
                card.body_span(),
                card.namespace().advisory_key()?,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CallInput {
    span: tcl_lexer::Span,
    command: String,
    arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SyntaxDependencies {
    purpose: SourcePurpose,
    body: String,
    namespace: String,
    config: tcl_lexer::LexerConfig,
    input: Option<crate::analyser::ResolvedAnalysisInput>,
    entry: crate::command_binding::SourceAnalysisEntry,
    registry: tcl_registry::RegistrySemanticKey,
    calls: Vec<CallInput>,
    grammar: Vec<tcl_registry::InvocationFacts>,
}

impl Hash for SyntaxDependencies {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.purpose.hash(state);
        self.body.hash(state);
        self.namespace.hash(state);
        self.config.hash(state);
        self.input.hash(state);
        self.entry.hash(state);
        self.registry.hash(state);
        self.calls.hash(state);
        // Full grammar equality remains mandatory. Its registry and original
        // literal argv already select this hash bucket.
    }
}

/// Pre-lowering request issued from a current original declaration body.
/// Equality compares independently checked syntax dependencies; current token
/// receipts remain private and are restored only by this request.
#[derive(Debug, Clone)]
pub struct SourceBodySyntaxRequest {
    dependencies: SyntaxDependencies,
    owner: SourceBodySyntaxOwner,
    offset: u32,
    tokens: Vec<CommandTokens>,
}

impl PartialEq for SourceBodySyntaxRequest {
    fn eq(&self, other: &Self) -> bool {
        self.dependencies == other.dependencies
    }
}
impl Eq for SourceBodySyntaxRequest {}
impl Hash for SourceBodySyntaxRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dependencies.hash(state);
    }
}

/// Neutral generic-call syntax containing no command/source/runtime receipts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceBodySyntaxTemplate {
    dependencies: SyntaxDependencies,
    body: Script,
}

/// Memo callback for independently certified source syntax requests.
pub type SourceBodySyntaxCache<'a> =
    dyn Fn(&SourceBodySyntaxRequest) -> SourceBodySyntaxTemplate + 'a;

impl SourceBodySyntaxRequest {
    /// Lower the captured original literal syntax without retaining its old
    /// source owner. The current request restores that owner after a cache hit.
    #[must_use]
    pub fn lower_neutral(&self) -> SourceBodySyntaxTemplate {
        let statements = self
            .dependencies
            .calls
            .iter()
            .map(|call| generic_call(call.span, &call.command, &call.arguments, None))
            .collect();
        SourceBodySyntaxTemplate {
            dependencies: self.dependencies.clone(),
            body: Script::from_statements(statements),
        }
    }

    fn restore(&self, template: &SourceBodySyntaxTemplate) -> Option<Script> {
        let (purpose, image, span, namespace) = self.owner.geometry()?;
        if purpose != self.dependencies.purpose
            || span.start() != self.offset
            || image.try_text().ok()?.get(span.as_range())? != self.dependencies.body
            || namespace != self.dependencies.namespace
            || template.dependencies != self.dependencies
            || template.body.statements.len() != self.tokens.len()
        {
            return None;
        }
        self.validate_shape(template)?;
        let mut body = template.body.clone();
        for ((statement, call), tokens) in body
            .statements
            .iter_mut()
            .zip(&self.dependencies.calls)
            .zip(&self.tokens)
        {
            let Statement::Call {
                span,
                tokens: restored,
                ..
            } = statement
            else {
                return None;
            };
            *span = tcl_lexer::Span::new(
                call.span.start().checked_add(self.offset)?,
                call.span.end().checked_add(self.offset)?,
            );
            *restored = Some(tokens.clone());
        }
        Some(body)
    }

    fn validate_shape(&self, template: &SourceBodySyntaxTemplate) -> Option<()> {
        // Validate the existing neutral artifact; do not lower another body or
        // compute the memoised result as a cache key.
        if template.body.namespace_context.is_some()
            || template.body.executed_source.is_some()
            || !template.body.implicit_math_invocations.is_empty()
            || !template.body.expression_preparations.is_empty()
            || template.body.command_binding_sites.iter().next().is_some()
            || template
                .body
                .procedure_binding_requirements
                .iter()
                .next()
                .is_some()
            || template.body.native_compilation_failure.is_some()
            || template.body.native_compilation_admission.is_some()
        {
            return None;
        }
        for (statement, original) in template
            .body
            .statements
            .iter()
            .zip(&self.dependencies.calls)
        {
            if !matches!(statement, Statement::Call {
                span, command, canonical_command: None, args,
                defs, reads, reads_own_defs: false, safe_on_uninit: false,
                tokens: None, foreach_groups: None,
            } if span == &original.span && command == &original.command
                && args == &original.arguments && defs.is_empty() && reads.is_empty())
            {
                return None;
            }
        }
        Some(())
    }
}

pub(super) fn generic_call(
    span: tcl_lexer::Span,
    command: &str,
    arguments: &[String],
    tokens: Option<CommandTokens>,
) -> Statement {
    Statement::Call {
        span,
        command: command.to_owned(),
        canonical_command: None,
        args: arguments.to_vec(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens,
        foreach_groups: None,
    }
}

fn generic_call_schema_facts(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::InvocationFacts> {
    let facts = schema.facts();
    (schema.semantics.lowering_hook.is_none()
        && schema.semantics.body_execution.is_none()
        && schema.semantics.frame_effect.is_none()
        && schema.semantics.named_object_factory.is_none()
        && schema.state_transitions().facts().is_empty()
        && facts.arg_roles_complete
        && !facts.arg_roles.iter().any(|(_, role)| {
            matches!(
                role,
                tcl_registry::ArgRole::VarWrite
                    | tcl_registry::ArgRole::VarRead
                    | tcl_registry::ArgRole::Body
                    | tcl_registry::ArgRole::ParamList
                    | tcl_registry::ArgRole::LambdaLiteral
                    | tcl_registry::ArgRole::CommandPrefix
            )
        }))
    .then_some(facts)
}

impl Lowerer<'_> {
    fn source_body_text_at(&self, offset: u32, len: usize) -> Option<&str> {
        let range = offset as usize..(offset as usize).checked_add(len)?;
        self.module
            .source
            .try_text()
            .ok()
            .and_then(|source| source.get(range.clone()))
            .or_else(|| self.source.get(range))
    }

    fn source_syntax_tokens(
        &mut self,
        segments: &[SegmentedCommand],
        image: &tcl_lexer::SourceImage,
        purpose: SourcePurpose,
    ) -> Option<(Vec<CommandTokens>, Vec<tcl_registry::InvocationFacts>)> {
        let mut tokens = Vec::new();
        let mut grammar = Vec::new();
        for segment in segments {
            if segment.is_partial || segment.texts.is_empty() {
                return None;
            }
            let original = self.cmd_tokens(segment);
            if original.synthetic.is_some()
                || !original.original_words_match_argv_text()
                || !original.words().iter().all(|word| {
                    matches!(
                        word,
                        crate::ir::WordExpr::Literal { .. }
                            | crate::ir::WordExpr::BracedLiteral { .. }
                    )
                })
                || original
                    .source_binding
                    .as_ref()?
                    .proved_execution_target()
                    .is_some()
            {
                return None;
            }
            let written = crate::registry_invocation::original_native_compiler_words(
                image,
                original.words(),
                original.words().first()?.source().span.start(),
                self.config,
            )?;
            if written.iter().any(|word| word.group().expand) {
                return None;
            }
            let binding = original.source_binding.as_ref()?;
            let facts = match purpose {
                SourcePurpose::Native => {
                    let input = self.metadata_input.as_ref()?;
                    binding
                        .original_native_source_words(&original, input, self.registry)?
                        .with_source_schema(
                            input.borrowed_context_registry(),
                            generic_call_schema_facts,
                        )??
                }
                SourcePurpose::Logical => {
                    let input = self.metadata_input.as_ref()?;
                    binding
                        .original_logical_source_words(&original, input, self.registry)?
                        .with_source_schema(
                            input.borrowed_context_registry(),
                            generic_call_schema_facts,
                        )??
                }
                SourcePurpose::Authored => {
                    let scope = binding
                        .original_authored_scalar_body_scope(&original, self.registry)
                        .or_else(|| {
                            binding.original_authored_source_body_scope_for_name_roles(
                                &original,
                                self.registry,
                            )
                        })?;
                    scope.matches_module(&self.module).then_some(())?;
                    scope.with_source_schema(&original, self.registry, generic_call_schema_facts)?
                }
            };
            tokens.push(original);
            grammar.push(facts);
        }
        Some((tokens, grammar))
    }

    fn source_syntax_request(
        &mut self,
        text: &str,
        offset: u32,
        namespace: &str,
        owner: SourceBodySyntaxOwner,
    ) -> Option<SourceBodySyntaxRequest> {
        let purpose = match (
            self.readonly_native_declaration_body,
            self.readonly_logical_body,
            self.readonly_authored_body,
        ) {
            (true, false, false) if self.native_source_children.is_some() => SourcePurpose::Native,
            (false, true, false) if self.source_bindings.is_some() => SourcePurpose::Logical,
            (false, false, true) if self.authored_source_scope.is_some() => SourcePurpose::Authored,
            _ => return None,
        };
        let (owned_purpose, image, span, owned_namespace) = owner.geometry()?;
        let current_image = if self.module.source.bytes().is_empty() {
            image.bytes() == self.source.as_bytes() && image.channel() == self.source_channel
        } else {
            image == &self.module.source
        };
        if !current_image
            || owned_purpose != purpose
            || span.start() != offset
            || image.try_text().ok()?.get(span.as_range())? != text
            || owned_namespace != namespace
        {
            return None;
        }
        match &owner {
            SourceBodySyntaxOwner::Native(card) => {
                let input = self.metadata_input.as_ref()?;
                (self.module.source_metadata_input.as_ref() == Some(input)).then_some(())?;
                card.matches_source(image, input, self.registry)
                    .then_some(())?;
            }
            SourceBodySyntaxOwner::Logical(card) => {
                (self.module.source_metadata_input.as_ref() == self.metadata_input.as_ref())
                    .then_some(())?;
                card.matches_source(image, self.metadata_input.as_ref()?)
                    .then_some(())?;
            }
            SourceBodySyntaxOwner::Authored(_) => {
                self.authored_source_scope
                    .as_ref()?
                    .matches_module(&self.module)
                    .then_some(())?;
            }
        }
        if self.target != super::CompileTarget::Analysis
            || self.proc_depth != 1
            || super::MAX_LOWER_NEST_DEPTH.exceeded(self.nest_depth + 1)
            || self.source_body_text_at(offset, text.len()) != Some(text)
        {
            return None;
        }
        let segments = self.segment_source(
            text,
            offset,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )?;
        let (tokens, grammar) = self.in_word_space(text, offset, |lowerer| {
            lowerer.source_syntax_tokens(&segments, image, purpose)
        })?;
        let calls = segments
            .iter()
            .map(|segment: &SegmentedCommand| {
                Some(CallInput {
                    span: tcl_lexer::Span::new(
                        segment.span.start().checked_sub(offset)?,
                        segment.span.end().checked_sub(offset)?,
                    ),
                    command: segment.name().to_owned(),
                    arguments: segment.args().to_vec(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(SourceBodySyntaxRequest {
            dependencies: SyntaxDependencies {
                purpose,
                body: text.to_owned(),
                namespace: namespace.to_owned(),
                config: self.config,
                input: self.metadata_input.clone(),
                entry: self.module.source_entry.clone(),
                registry: self.registry.snapshot().semantic_key(),
                calls,
                grammar,
            },
            owner,
            offset,
            tokens,
        })
    }

    pub(super) fn lower_original_source_body(
        &mut self,
        text: &str,
        offset: u32,
        namespace: &str,
        owner: SourceBodySyntaxOwner,
    ) -> Script {
        if let Some(cache) = self.source_body_syntax_cache
            && let Some(request) = self.source_syntax_request(text, offset, namespace, owner)
            && let Some(body) = request.restore(&cache(&request))
        {
            #[cfg(test)]
            if self.verify_source_body_syntax_cache {
                // Compare under the same current owner; independent analyses
                // intentionally retain different attachment receipts.
                let fresh = self.lower_body(text, offset, namespace);
                assert_eq!(
                    body, fresh,
                    "cached syntax must equal genuine current lowering"
                );
            }
            return body;
        }
        self.lower_body(text, offset, namespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::sync::Arc;

    fn native_input() -> crate::analyser::ResolvedAnalysisInput {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        )
    }

    fn build(
        source: &str,
        input: &crate::analyser::ResolvedAnalysisInput,
        cache: &SourceBodySyntaxCache<'_>,
    ) -> crate::ir::Module {
        let mut lowerer = Lowerer::with_config(
            input.borrowed_context_registry().commands(),
            input.lexer_config(),
        )
        .with_resolved_analysis_input(input.clone())
        // The cache serves unentered declarations, which need explicit unknown
        // history; no entry means a fresh interpreter that defines them.
        .with_explicit_unknown_entry()
        .with_source_body_syntax_cache(cache);
        lowerer.verify_source_body_syntax_cache = true;
        crate::lowering::lower_to_ir_with(lowerer, source)
    }

    fn current_receipts(module: &crate::ir::Module) {
        let input = module.source_metadata_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        assert_eq!(module.original_native_declaration_body_units.len(), 3);
        for (id, card) in &module.original_native_declaration_body_units {
            assert!(module.procedure_ids.owns(*id));
            let procedure = module.body_units.get(id).unwrap();
            assert!(card.matches_procedure(module, procedure));
            assert!(procedure.body.executed_source.is_none());
            assert!(procedure.body.native_compilation_admission.is_none());
            let [
                Statement::Call {
                    tokens: Some(tokens),
                    ..
                },
            ] = procedure.body.statements.as_slice()
            else {
                panic!("original literal body must remain one generic call")
            };
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(binding.original_native_source_owner_for_body(tokens, module, *id, registry));
            let words = binding
                .original_native_source_words(tokens, input, registry)
                .unwrap();
            assert_eq!(words.command(), "puts");
            assert_eq!(words.arguments().len(), 1);
            assert!(tokens.words().iter().all(|word| {
                word.source().span.start() >= procedure.body_offset
                    && word.source().span.end() <= card.body_span().unwrap().end()
            }));
        }
    }

    #[test]
    fn source_syntax_cache_reuses_only_unchanged_dependencies_and_restores_current_receipts() {
        let input = native_input();
        let cached =
            RefCell::new(Vec::<(SourceBodySyntaxRequest, SourceBodySyntaxTemplate)>::new());
        let misses = Cell::new(0usize);
        let cache = |request: &SourceBodySyntaxRequest| {
            if let Some((_, template)) = cached.borrow().iter().find(|(old, _)| old == request) {
                return template.clone();
            }
            misses.set(misses.get() + 1);
            let template = request.lower_neutral();
            assert!(
                template
                    .body
                    .statements
                    .iter()
                    .all(|statement| statement.tokens().is_none())
            );
            cached
                .borrow_mut()
                .push((request.clone(), template.clone()));
            template
        };
        let source =
            "proc a {} { puts 11111 }\nproc b {} { puts 22222 }\nproc c {} { puts 33333 }\n";
        let cold = build(source, &input, &cache);
        assert_eq!(misses.replace(0), 3, "each genuine source body lowers once");
        current_receipts(&cold);
        let edited = source.replace("22222", "99999999");
        let changed = build(&edited, &input, &cache);
        assert_eq!(misses.replace(0), 1, "only changed original syntax lowers");
        current_receipts(&changed);
        let shifted = build(&format!("\n{edited}"), &input, &cache);
        assert_eq!(
            misses.replace(0),
            0,
            "source relocation changes receipts, not syntax"
        );
        current_receipts(&shifted);
        let (request, original) = cached.borrow()[0].clone();
        let mut forged = original.clone();
        let Statement::Call { command, .. } = &mut forged.body.statements[0] else {
            unreachable!()
        };
        *command = "foreign".into();
        assert!(request.restore(&forged).is_none());
        let different = cached
            .borrow()
            .iter()
            .find(|(other, _)| other != &request)
            .map(|(_, template)| template.clone())
            .unwrap();
        assert!(request.restore(&different).is_none());
    }

    #[test]
    fn source_syntax_cache_keeps_complete_input_and_registry_generation() {
        let input = native_input();
        let source =
            "proc a {} { puts 11111 }\nproc b {} { puts 22222 }\nproc c {} { puts 33333 }\n";
        let requests = RefCell::new(Vec::<SourceBodySyntaxRequest>::new());
        let cache = |request: &SourceBodySyntaxRequest| {
            requests.borrow_mut().push(request.clone());
            request.lower_neutral()
        };
        current_receipts(&build(source, &input, &cache));
        assert_eq!(requests.borrow().len(), 3);
        let original = requests.borrow().clone();
        requests.borrow_mut().clear();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let changed = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        current_receipts(&build(source, &changed, &cache));
        assert_eq!(requests.borrow().len(), 3);
        for (before, after) in original.iter().zip(requests.borrow().iter()) {
            assert_ne!(before, after);
            assert!(after.restore(&before.lower_neutral()).is_none());
        }
    }

    #[test]
    fn authored_original_expansion_never_enters_the_source_syntax_cache() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let requests = Cell::new(0);
        let cache = |request: &SourceBodySyntaxRequest| {
            requests.set(requests.get() + 1);
            request.lower_neutral()
        };
        for source in [
            "proc f {} {puts {*}{a b}}\n",
            "proc f {words} {puts {*}$words}\n",
        ] {
            let module = crate::lowering::lower_to_ir_with(
                Lowerer::with_config(&registry, tcl_lexer::LexerConfig::default())
                    .with_source_body_syntax_cache(&cache),
                source,
            );
            assert_eq!(module.original_authored_declaration_body_units.len(), 1);
            let (id, card) = module
                .original_authored_declaration_body_units
                .iter()
                .next()
                .unwrap();
            let procedure = module.body_units.get(id).unwrap();
            assert!(card.matches_procedure(&module, procedure));
            let [
                Statement::Call {
                    tokens: Some(tokens),
                    ..
                },
            ] = procedure.body.statements.as_slice()
            else {
                panic!("expanded original source retains its generic call")
            };
            let scope = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_authored_source_body_scope_for_name_roles(tokens, &registry)
                .expect("actual original Authored body scope");
            assert!(scope.matches_module(&module));
            assert!(
                scope
                    .original_words(tokens, &registry)
                    .unwrap()
                    .iter()
                    .any(|word| word.group().expand)
            );
            assert_eq!(
                requests.replace(0),
                0,
                "original expansion remains fresh: {source}"
            );
        }
    }

    #[test]
    fn source_syntax_cache_declines_substitution_expansion_and_structured_lowering() {
        let input = native_input();
        let requests = Cell::new(0);
        let cache = |request: &SourceBodySyntaxRequest| {
            requests.set(requests.get() + 1);
            request.lower_neutral()
        };
        for source in [
            "proc f {x} {puts $x}\n",
            "proc f {} {puts [set x]}\n",
            "proc f {} {puts {*}{a b}}\n",
            "proc f {} {set value TEXT; return $value}\n",
            "proc f {} {namespace export f}\n",
        ] {
            let module = build(source, &input, &cache);
            assert_eq!(
                module.original_native_declaration_body_units.len(),
                1,
                "{source}"
            );
            assert_eq!(
                requests.replace(0),
                0,
                "unsupported syntax must lower fresh: {source}"
            );
        }
    }
}
