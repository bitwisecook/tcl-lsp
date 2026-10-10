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

//! Closed native bodies may share a template while physical worlds remain dependencies.

use super::{
    Arc, BTreeMap, CfgFunction, CommandRegistry, InvocationFacts, ModuleCommandBindings, Script,
    SourceInvocationBinding, SourceOriginId, SourceOriginKind, SourceVariableEvaluationOwner,
    Statement, resolve_source_invocation_facts,
};

/// Actual lexical scope whose command allocations are relocated into a body template.
#[derive(Debug, Clone, Copy)]
pub struct BodyProofScope<'a> {
    /// Complete authored source owning the retained carriers.
    pub source: &'a str,
    /// Exact executable body bytes.
    pub body_source: &'a str,
    /// Original source offset of the body.
    pub original_body_offset: u32,
    /// Actual first byte of executable body source, separately from the IR rebase origin.
    pub executable_body_offset: u32,
}

/// A closed native body and the injective physical renaming used to construct it.
#[derive(Debug, Clone)]
pub struct NativeBodyTemplate {
    /// Offset-zero IR with body-scoped temporal ownership.
    pub body: Script,
    /// Native-only CFG dispatch context; actual point worlds remain in the IR.
    pub command_bindings: ModuleCommandBindings,
    /// Exact physical alpha-renaming, including untouched external dependencies.
    pub variable_relocation: crate::var_resolve::VariableProofRelocation,
}

/// Actual offset-zero command carriers used to instantiate a native template.
#[derive(Debug, Clone, Default)]
pub struct BodySourceProofs {
    /// Original evaluation-order proofs, keyed by exact invocation start.
    pub tokens: BTreeMap<u32, crate::ir::CommandTokens>,
    sites: BTreeMap<u32, crate::ir::CommandBindingSite>,
    root_source: Option<Arc<super::ExecutedScriptSource>>,
    root_admission: Option<Arc<crate::native_compilation_admission::NativeCompilationAdmission>>,
    statement_sources: BTreeMap<(u32, u32), Option<Arc<super::ExecutedScriptSource>>>,
    script_sources: BTreeMap<Vec<(u32, u32)>, Option<Arc<super::ExecutedScriptSource>>>,
    script_math: BTreeMap<Vec<(u32, u32)>, Option<Vec<super::SourceMathInvocation>>>,
    math_invocations: Vec<super::SourceMathInvocation>,
    script_preparations: BTreeMap<Vec<(u32, u32)>, Option<Vec<super::SourceExpressionPreparation>>>,
    expression_preparations: Vec<super::SourceExpressionPreparation>,
}

impl BodySourceProofs {
    /// Retain exact original carriers, without re-resolving their command heads.
    #[must_use]
    pub fn from_body(body: &Script) -> Self {
        let mut body = body.clone();
        let mut proofs = Self {
            root_source: body.executed_source.clone(),
            root_admission: body.native_compilation_admission.clone(),
            ..Self::default()
        };
        crate::ir::for_each_script_mut(&mut body, &mut |script| {
            proofs
                .math_invocations
                .extend(script.implicit_math_invocations.iter().cloned());
            proofs
                .expression_preparations
                .extend(script.expression_preparations.iter().cloned());
            let key = script_source_key(script);
            let preparations = &script.expression_preparations;
            proofs
                .script_preparations
                .entry(key.clone())
                .and_modify(|existing| {
                    if existing.as_ref() != Some(preparations) {
                        *existing = None;
                    }
                })
                .or_insert_with(|| Some(preparations.clone()));
            let math = &script.implicit_math_invocations;
            proofs
                .script_math
                .entry(key)
                .and_modify(|existing| {
                    if existing.as_ref() != Some(math) {
                        *existing = None;
                    }
                })
                .or_insert_with(|| Some(math.clone()));
            retain_source_mapping(
                &mut proofs.script_sources,
                script_source_key(script),
                script.executed_source.as_ref(),
            );
            for site in script.command_binding_sites.iter() {
                proofs.sites.insert(site.span.start(), site.clone());
                if let Some(tokens) = &site.source_tokens {
                    proofs
                        .tokens
                        .insert(site.span.start(), tokens.as_ref().clone());
                }
            }
            for statement in &script.statements {
                retain_source_mapping(
                    &mut proofs.statement_sources,
                    span_key(statement.span()),
                    script.executed_source.as_ref(),
                );
                if let Some(tokens) = statement.tokens() {
                    proofs
                        .tokens
                        .insert(statement.span().start(), tokens.clone());
                }
            }
        });
        proofs
    }

    /// Restore command and read ownership on a returned CFG after physical relocation.
    /// Missing exact authored sites decline the cached artifact.
    pub fn restore_cfg(&self, cfg: &mut CfgFunction) -> bool {
        if !self.restore_cfg_sources(cfg) {
            return false;
        }
        let mut valid = true;
        for site in cfg
            .command_binding_sites
            .iter_mut()
            .chain(cfg.command_boundary_sites.values_mut())
            .chain(cfg.condition_binding_sites.values_mut())
        {
            if let Some(original) = self.sites.get(&site.span.start()) {
                site.variable_frame.clone_from(&original.variable_frame);
                site.variable_context.clone_from(&original.variable_context);
                site.known_namespaces.clone_from(&original.known_namespaces);
                site.existing_namespace_cells
                    .clone_from(&original.existing_namespace_cells);
                site.source_tokens.clone_from(&original.source_tokens);
            } else if let Some(original) = self.tokens.get(&site.span.start()) {
                site.source_tokens = Some(Box::new(original.clone()));
                if let Some(proof) = &original.source_binding {
                    site.variable_frame = Some(proof.variable_frame.clone());
                    site.variable_context = Some(Arc::clone(&proof.variable_context));
                    site.known_namespaces = Some(proof.known_namespaces.clone());
                    site.existing_namespace_cells = Some(proof.existing_namespace_cells.clone());
                } else {
                    valid = false;
                }
            } else {
                valid = false;
            }
        }
        for block in cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                valid &= self.restore_statement(statement);
            }
            if let Some(crate::cfg::Terminator::Return {
                tokens: Some(tokens),
                span: Some(span),
                ..
            }) = &mut block.terminator
            {
                valid &= self.restore_tokens(tokens, span.start());
            }
        }
        for node in cfg.loop_nodes.values_mut() {
            valid &= self.restore_statement(&mut node.statement);
        }
        valid
    }

    fn restore_cfg_sources(&self, cfg: &mut CfgFunction) -> bool {
        let Some(root) = &self.root_source else {
            return false;
        };
        if !cfg.required_math_invocations.is_empty()
            || !cfg.required_expression_preparations.is_empty()
        {
            // Normalized templates do not admit implicit math calls. Consumed
            // fold obligations cannot be alpha-renamed by passive restoration.
            return false;
        }
        let old_base = cfg.executed_source.as_ref().map(|source| source.base());
        for source in cfg.terminator_sources.values_mut() {
            if !self.restore_scoped_source(source, old_base, root.base()) {
                return false;
            }
        }
        for node in cfg.loop_nodes.values_mut() {
            if !self.restore_scoped_source(&mut node.executed_source, old_base, root.base()) {
                return false;
            }
        }
        cfg.executed_source = Some(Arc::clone(root));
        cfg.native_compilation_admission
            .clone_from(&self.root_admission);
        let mut sources = std::collections::HashMap::new();
        for (block_id, block) in &cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                let Some(Some(source)) = self.statement_sources.get(&span_key(statement.span()))
                else {
                    return false;
                };
                sources.insert((*block_id, index), Some(Arc::clone(source)));
            }
        }
        cfg.statement_sources = sources;
        cfg.implicit_math_invocations
            .clone_from(&self.math_invocations);
        cfg.expression_preparations
            .clone_from(&self.expression_preparations);
        true
    }

    fn restore_scoped_source(
        &self,
        source: &mut Option<Arc<super::ExecutedScriptSource>>,
        old_root_base: Option<u32>,
        actual_root_base: u32,
    ) -> bool {
        let Some(old) = source else {
            return true;
        };
        let Some(old_root_base) = old_root_base else {
            return false;
        };
        let base = i64::from(old.base()) + i64::from(actual_root_base) - i64::from(old_root_base);
        let Ok(base) = u32::try_from(base) else {
            return false;
        };
        let mut matching = self
            .script_sources
            .values()
            .flatten()
            .filter(|candidate| candidate.base() == base && candidate.text == old.text);
        let Some(actual) = matching.next() else {
            return false;
        };
        if matching.any(|other| other != actual) {
            return false;
        }
        *source = Some(Arc::clone(actual));
        true
    }

    fn restore_script_source(&self, script: &mut Script) -> bool {
        let Some(Some(source)) = self.script_sources.get(&script_source_key(script)) else {
            return false;
        };
        let Some(Some(math)) = self.script_math.get(&script_source_key(script)) else {
            return false;
        };
        let Some(Some(preparations)) = self.script_preparations.get(&script_source_key(script))
        else {
            return false;
        };
        script.expression_preparations.clone_from(preparations);
        script.implicit_math_invocations.clone_from(math);
        script.executed_source = Some(Arc::clone(source));
        if let Some(admission) = &mut script.native_compilation_admission {
            Arc::make_mut(admission).source = Some(source.as_ref().clone());
        }
        true
    }

    fn restore_tokens(&self, tokens: &mut crate::ir::CommandTokens, offset: u32) -> bool {
        let Some(original) = self.tokens.get(&offset) else {
            return false;
        };
        tokens.restore_source_proofs(original);
        true
    }

    /// Restore retained source proofs on one statement without changing its lowered semantics.
    pub fn restore_statement(&self, statement: &mut Statement) -> bool {
        let offset = statement.span().start();
        let mut valid = true;
        if let Some(tokens) = statement.tokens_mut()
            && tokens.synthetic.is_none()
        {
            valid &= self.restore_tokens(tokens, offset);
        }
        for body in statement.child_scripts_mut() {
            valid &= self.restore_script_source(body);
            for statement in &mut body.statements {
                valid &= self.restore_statement(statement);
            }
        }
        valid
    }
}

fn span_key(span: tcl_lexer::Span) -> (u32, u32) {
    (span.start(), span.end())
}

fn script_source_key(script: &Script) -> Vec<(u32, u32)> {
    script
        .statements
        .iter()
        .map(|statement| span_key(statement.span()))
        .collect()
}

fn retain_source_mapping<K: Ord>(
    mappings: &mut BTreeMap<K, Option<Arc<super::ExecutedScriptSource>>>,
    key: K,
    source: Option<&Arc<super::ExecutedScriptSource>>,
) {
    mappings
        .entry(key)
        .and_modify(|old| {
            if old.as_ref() != source {
                *old = None;
            }
        })
        .or_insert_with(|| source.cloned());
}

fn normalize_script_source(
    script: &mut Script,
    scope: BodyProofScope<'_>,
    origin: &Arc<SourceOriginId>,
) -> Option<()> {
    let source = script.executed_source.as_ref()?;
    if !matches!(source.origin.kind(), SourceOriginKind::Authored(bytes) if bytes.bytes() == scope.source.as_bytes())
    {
        return None;
    }
    let super::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
        return None;
    };
    let base = base.checked_sub(scope.original_body_offset)?;
    let normalized = super::ExecutedScriptSource::contiguous_image(
        Arc::clone(origin),
        source.text.clone(),
        base,
    )?;
    if let Some(admission) = &mut script.native_compilation_admission {
        let admission = Arc::make_mut(admission);
        if admission.requires_native_provider() {
            return None;
        }
        admission.source = Some(normalized.clone());
    }
    script.executed_source = Some(Arc::new(normalized));
    Some(())
}

impl ModuleCommandBindings {
    /// Prepare a native template only after every executable invocation has a
    /// retained original-token proof and no unmodelled callback or mutation.
    /// Bodies outside this contract retain their complete cache identity.
    #[must_use]
    pub fn prepare_native_body_template(
        &self,
        body: &Script,
        scope: BodyProofScope<'_>,
        registry: &CommandRegistry,
    ) -> Option<NativeBodyTemplate> {
        let executable_start = usize::try_from(scope.executable_body_offset).ok()?;
        let executable_end = executable_start.checked_add(scope.body_source.len())?;
        if scope.source.get(executable_start..executable_end) != Some(scope.body_source) {
            return None;
        }
        let mut template = body.clone();
        let mut valid = true;
        let mut relocation = crate::var_resolve::VariableProofRelocation::default();
        crate::ir::for_each_script_mut(&mut template, &mut |script| {
            valid &= script.implicit_math_invocations.is_empty();
            // Prepared trees retain exact source instances and possibly an
            // interpreter-owned table; no alpha contract erases those owners.
            valid &= script.expression_preparations.is_empty();
            valid &= script.native_compilation_failure.is_none();
            for statement in &script.statements {
                if let Some(tokens) = statement.tokens() {
                    valid &= native_tokens_closed(tokens, registry);
                } else {
                    valid &= script.command_binding_sites.iter().any(|site| {
                        site.span.start() == statement.span().start()
                            && site
                                .source_tokens
                                .as_ref()
                                .is_some_and(|tokens| native_tokens_closed(tokens, registry))
                    });
                }
                collect_body_offset(&mut relocation, scope, statement.span().start());
            }
            for site in script.command_binding_sites.iter() {
                collect_body_offset(&mut relocation, scope, site.span.start());
                if let Some(context) = &site.variable_context {
                    collect_body_activation(&mut relocation, context);
                }
                if let Some(tokens) = &site.source_tokens {
                    collect_token_activations(&mut relocation, tokens);
                }
            }
            for statement in &script.statements {
                if let Some(tokens) = statement.tokens() {
                    collect_token_activations(&mut relocation, tokens);
                    for (offset, _) in &tokens.nested_bindings {
                        collect_body_offset(&mut relocation, scope, *offset);
                    }
                }
            }
        });
        if !valid || relocation.activations.len() > 1 || relocation.inverse().is_none() {
            return None;
        }
        let prefix = usize::try_from(
            scope
                .executable_body_offset
                .checked_sub(scope.original_body_offset)?,
        )
        .ok()?;
        let mut template_source = " ".repeat(prefix);
        template_source.push_str(scope.body_source);
        let origin = Arc::new(SourceOriginId::authored(&Arc::from(template_source)));
        relocation.source_origins.insert(
            Arc::new(SourceOriginId::authored(&Arc::from(scope.source))),
            Arc::clone(&origin),
        );
        crate::ir::for_each_script_mut(&mut template, &mut |script| {
            valid &= normalize_script_source(script, scope, &origin).is_some();
            for site in script.command_binding_sites.iter_mut() {
                site.variable_frame = site.variable_frame.as_ref().map(|f| relocation.frame(f));
                site.variable_context = site
                    .variable_context
                    .as_ref()
                    .map(|context| Arc::new(context.relocated(&relocation)));
                if let Some(tokens) = &mut site.source_tokens {
                    normalize_native_tokens(tokens, scope, &origin, &relocation);
                }
            }
            for statement in &mut script.statements {
                if let Some(tokens) = statement.tokens_mut() {
                    normalize_native_tokens(tokens, scope, &origin, &relocation);
                }
            }
        });
        if !valid {
            return None;
        }
        let command_bindings = Self {
            baseline: Arc::clone(&self.baseline),
            ..Self::default()
        };
        Some(NativeBodyTemplate {
            body: template,
            command_bindings,
            variable_relocation: relocation,
        })
    }
}

fn collect_body_offset(
    relocation: &mut crate::var_resolve::VariableProofRelocation,
    scope: BodyProofScope<'_>,
    relative: u32,
) {
    if let Some(actual) = scope.original_body_offset.checked_add(relative) {
        relocation.source_offsets.insert(actual, relative);
    }
}

fn collect_token_activations(
    relocation: &mut crate::var_resolve::VariableProofRelocation,
    tokens: &crate::ir::CommandTokens,
) {
    let collect_access = |relocation: &mut crate::var_resolve::VariableProofRelocation,
                          access: &super::SourceVariableAccess| {
        for context in access.context_alternatives() {
            collect_body_activation(relocation, context);
        }
    };
    for access in &tokens.variable_accesses {
        collect_access(relocation, access);
    }
    for proof in tokens
        .source_binding
        .iter()
        .chain(tokens.nested_bindings.iter().map(|(_, proof)| proof))
    {
        collect_body_activation(relocation, &proof.variable_context);
        if let Some(context) = proof.normal_variable_continuation() {
            collect_body_activation(relocation, context);
        }
        if let Some(reads) = &proof.invocation_variable_reads {
            for access in &reads.substitutions {
                collect_access(relocation, access);
            }
            for read in &reads.native_reads {
                collect_body_activation(relocation, &read.variable_context);
            }
        }
    }
}

fn collect_body_activation(
    relocation: &mut crate::var_resolve::VariableProofRelocation,
    context: &crate::var_resolve::ResolveContext,
) {
    if let Some(activation) = &context.activation {
        relocation
            .activations
            .insert(activation.clone(), "::native-body-template".to_owned());
    }
}

fn original_native(proof: &SourceInvocationBinding) -> bool {
    // Actual receiver allocations contain source/frame incarnations. They
    // remain in the complete cache key until their relocation is supported.
    if proof.original_argument_completion.is_normal()
        || proof.rhs_read_store_observations.is_some()
        || proof.frozen_head_object.is_some()
        || !proof.method_prefix_arguments.is_empty()
        || proof
            .lookup_state
            .as_ref()
            .is_some_and(|snapshot| !snapshot.state.object_instances.receivers.is_empty())
    {
        return false;
    }
    proof.proved_execution_target().is_some_and(|target| {
        target.registry_backed
            && target.prepended.is_empty()
            && target.implementation_allocation.is_none()
            && target
                .identity
                .as_ref()
                .is_none_or(|identity| identity.allocation.is_none())
    })
}

fn native_tokens_closed(tokens: &crate::ir::CommandTokens, registry: &CommandRegistry) -> bool {
    let Some(proof) = &tokens.source_binding else {
        return false;
    };
    if tokens
        .variable_accesses
        .iter()
        .any(|access| access.proved_object_instance().is_some())
        || proof.named_object_instance_at_dispatch().is_some()
        || tokens.evaluated_body.is_some()
        || !original_native(proof)
        || tokens
            .nested_bindings
            .iter()
            .any(|(_, proof)| !nested_native_closed(proof, registry))
    {
        return false;
    }
    let Some(invocation) =
        crate::registry_invocation::resolved_tokens_invocation(registry, None, tokens)
    else {
        return false;
    };
    native_facts_closed(&invocation.facts, proof)
}

fn nested_native_closed(proof: &SourceInvocationBinding, registry: &CommandRegistry) -> bool {
    use tcl_registry::{InvocationWord, InvocationWords};
    let Some(target) = proof
        .proved_execution_target()
        .filter(|_| original_native(proof))
    else {
        return false;
    };
    let words: Vec<_> = proof
        .evaluated_argument_values
        .iter()
        .map(|word| {
            word.as_deref()
                .map_or(InvocationWord::Opaque, InvocationWord::Literal)
        })
        .collect();
    let mut invocation =
        InvocationWords::structured(InvocationWord::Literal(&target.command), &words);
    if let Some(dialect) = proof.variable_context.invocation_dialect {
        invocation = invocation.with_dialect(dialect);
    }
    let Some(realm) = proof.invocation_realm() else {
        return false;
    };
    resolve_source_invocation_facts(registry, None, invocation, realm).is_some_and(|mut facts| {
        super::object_callbacks::refine_facts(&mut facts, invocation.arguments(), Some(proof));
        native_facts_closed(&facts, proof)
    })
}

fn native_facts_closed(facts: &InvocationFacts, proof: &SourceInvocationBinding) -> bool {
    use tcl_registry::world_effect::{CallbackKinds, WorldStateDomain};
    let callbacks = facts.effects.callback().kinds;
    let callback_free = callbacks == CallbackKinds::NONE
        || (callbacks == CallbackKinds::TRACE
            && !proof.variable_context.dynamic_traces
            && proof.variable_context.traced.is_empty()
            && proof.variable_context.trace_registrations.is_empty()
            && proof.variable_context.untracked_traces.is_empty());
    std::iter::once(proof.variable_context.as_ref())
        .chain(proof.normal_variable_continuation())
        .flat_map(|context| context.value_representations.values())
        .all(|value| {
            !matches!(
                value,
                crate::native_numeric::StoredNativeRepresentation::Numeric(_)
            )
        })
        && proof.native_operand_layout().is_none()
        && callback_free
        && facts.body_execution.is_none()
        && facts.frame_effect.is_none()
        && facts
            .state_transitions
            .declared()
            .is_none_or(|transitions| transitions.facts().is_empty())
        && facts.effects.accesses().iter().all(|access| {
            matches!(
                access.domain,
                WorldStateDomain::VariableStore
                    | WorldStateDomain::VariableTraces
                    | WorldStateDomain::InterpreterResult
                    | WorldStateDomain::CompletionState
            )
        })
}

fn normalize_native_tokens(
    tokens: &mut crate::ir::CommandTokens,
    scope: BodyProofScope<'_>,
    origin: &Arc<SourceOriginId>,
    relocation: &crate::var_resolve::VariableProofRelocation,
) {
    tokens.relocate_variable_proofs(relocation);
    for proof in tokens
        .source_binding
        .iter_mut()
        .chain(tokens.nested_bindings.iter_mut().map(|(_, proof)| proof))
    {
        // Template analysis has proved that it never asks the command world
        // for another target. Actual returned carriers are restored by the driver.
        proof.lookup_state = None;
        proof.compiler_lookup_state = None;
        proof.compiler_policy = None;
        if let Some(site) = &mut proof.dispatch_site {
            relocate_allocation_site(site, scope, origin);
        }
        if let Some(reads) = &mut proof.invocation_variable_reads {
            for access in &mut Arc::make_mut(reads).substitutions {
                relocate_read_owner(&mut access.owner, scope, origin);
            }
        }
        for compiled in &mut proof.compiled_candidates {
            relocate_allocation_site(&mut compiled.compilation_site, scope, origin);
        }
        for named in &mut proof.compiled_named_candidates {
            relocate_allocation_site(&mut named.compilation_site, scope, origin);
        }
        if let Some(admission) = &mut proof.native_compiler_admission {
            match Arc::make_mut(admission) {
                super::compiled_invocation::SourceNativeCompilerAdmission::Inline(compiled) => {
                    relocate_allocation_site(&mut compiled.compilation_site, scope, origin);
                }
                super::compiled_invocation::SourceNativeCompilerAdmission::Named(named) => {
                    relocate_allocation_site(&mut named.compilation_site, scope, origin);
                }
            }
        }
    }
    for access in &mut tokens.variable_accesses {
        relocate_read_owner(&mut access.owner, scope, origin);
    }
}

fn relocate_read_owner(
    owner: &mut SourceVariableEvaluationOwner,
    scope: BodyProofScope<'_>,
    origin: &Arc<SourceOriginId>,
) {
    match owner {
        SourceVariableEvaluationOwner::InvocationArguments { invocation, parent }
        | SourceVariableEvaluationOwner::NativeExpression { invocation, parent }
        | SourceVariableEvaluationOwner::InvocationBody { invocation, parent } => {
            relocate_allocation_site(invocation, scope, origin);
            if let Some(parent) = parent {
                relocate_read_owner(Arc::make_mut(parent), scope, origin);
            }
        }
        SourceVariableEvaluationOwner::Alternatives(owners) => {
            for owner in owners {
                relocate_read_owner(owner, scope, origin);
            }
        }
        SourceVariableEvaluationOwner::Unspecified => {}
    }
}

fn relocate_allocation_site(
    invocation: &mut super::CommandAllocationSite,
    scope: BodyProofScope<'_>,
    origin: &Arc<SourceOriginId>,
) {
    if matches!(invocation.source.kind(), SourceOriginKind::Authored(source) if source.bytes() == scope.source.as_bytes())
        && let Some(body_relative) = invocation.offset.checked_sub(scope.executable_body_offset)
        && usize::try_from(body_relative).is_ok_and(|relative| relative < scope.body_source.len())
    {
        invocation.source = Arc::clone(origin);
        invocation.offset -= scope.original_body_offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare(source: &str) -> Option<NativeBodyTemplate> {
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let procedure = unit.ir_module.procedures.get("::p").unwrap();
        let mut body = procedure.body.clone();
        crate::lattice_rebase::rebase_script(&mut body, -i64::from(procedure.span.start()));
        ModuleCommandBindings::analyse(&unit.ir_module, &registry).prepare_native_body_template(
            &body,
            BodyProofScope {
                source,
                body_source: procedure.body_source.as_deref().unwrap(),
                original_body_offset: procedure.span.start(),
                executable_body_offset: procedure.body_offset,
            },
            &registry,
        )
    }

    #[test]
    fn unchanged_native_body_ignores_unrelated_authored_prefix() {
        let first = prepare("proc p {} {set x 1; set x}; p").unwrap();
        let shifted = prepare("# unrelated prefix\nproc p {} {set x 1; set x}; p").unwrap();
        assert_eq!(first.body, shifted.body);
        assert_eq!(first.command_bindings, shifted.command_bindings);
        assert!(first.variable_relocation.inverse().is_some());
    }

    #[test]
    fn returned_template_restores_actual_statement_source_instances() {
        let source = "# current document\nproc p {} {set x 1; set x}; p";
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let procedure = unit.ir_module.procedures.get("::p").unwrap();
        let mut actual = procedure.body.clone();
        crate::lattice_rebase::rebase_script(&mut actual, -i64::from(procedure.span.start()));
        let template = prepare(source).unwrap();
        let mut cfg =
            crate::cfg_builder::build_cfg_function("::p", &template.body, false, &registry, false);
        assert_ne!(cfg.executed_source, actual.executed_source);
        let proofs = BodySourceProofs::from_body(&actual);
        assert!(proofs.restore_cfg(&mut cfg));
        assert_eq!(cfg.executed_source, actual.executed_source);
        assert!(
            cfg.statement_sources
                .values()
                .all(|source| source == &actual.executed_source)
        );
        let mut missing = actual;
        missing.executed_source = None;
        assert!(!BodySourceProofs::from_body(&missing).restore_cfg(&mut cfg));
    }

    #[test]
    fn nested_command_mutation_does_not_become_a_native_template() {
        assert!(prepare("proc p {} {list [rename set saved]}; p").is_none());
    }

    #[test]
    fn callback_and_external_world_effects_keep_complete_cache_identity() {
        assert!(prepare("proc hook {args} {rename set saved}; trace add variable observed read hook; proc p {} {set observed}; p").is_none());
        assert!(prepare("proc p {} {puts output}; p").is_none());
    }
}
