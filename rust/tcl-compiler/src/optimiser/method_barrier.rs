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

//! Per-method dispatch reachability for local-value propagation.
//!
//! Original statement and nested substitution receipts select every possible
//! execution target. Exact class allocations contribute hierarchy components;
//! exact procedure declarations contribute transitive dispatch facts. Registry
//! self/next and callback contracts apply only to selected implementations.
//! Missing carriers, unknown execution alternatives, unavailable bodies and
//! unresolved callback layouts widen reachability to every component.
//!
//! Hierarchy relations conservatively connect every retained candidate class.
//! A component is invalidating when one of its methods reaches a caller frame
//! or its implementation inventory is incomplete. A method may propagate its
//! private locals only when its reachable components exclude those effects.

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::ir::{ExecutionNamespace, Module as IrModule, Script, Statement};

/// The computed barrier: which methods may propagate their private locals.
pub(crate) struct MethodDispatchBarrier {
    bar_all: bool,
    barred: HashSet<String>,
}

impl MethodDispatchBarrier {
    /// Whether `method_qname`'s provably-local variables may propagate.
    pub(crate) fn allows_locals(&self, method_qname: &str) -> bool {
        !self.bar_all && !self.barred.contains(method_qname)
    }

    fn allow_all() -> Self {
        Self {
            bar_all: false,
            barred: HashSet::new(),
        }
    }

    fn bar_all() -> Self {
        Self {
            bar_all: true,
            barred: HashSet::new(),
        }
    }
}

/// Where a body's dispatches can land: a set of hierarchy components,
/// plus `anywhere` when some dispatch target cannot be bounded.
#[derive(Default, Clone)]
struct DispatchFacts {
    anywhere: bool,
    comps: HashSet<usize>,
}

impl DispatchFacts {
    fn absorb(&mut self, other: &DispatchFacts) -> bool {
        let mut changed = false;
        if other.anywhere && !self.anywhere {
            self.anywhere = true;
            changed = true;
        }
        for c in &other.comps {
            changed |= self.comps.insert(*c);
        }
        changed
    }
}

/// Compute the per-method barrier for `cu` — see the module doc.
pub(crate) fn compute(ir: &IrModule, registry: &CommandRegistry) -> MethodDispatchBarrier {
    if ir.oo_evidence.dynamic_target
        || ir.oo_evidence.dynamic_class_relations
        || dispatch_metadata(registry, ir).is_none()
    {
        return MethodDispatchBarrier::bar_all();
    }

    // Bad classes: a caller-frame-reaching body (primary or retained
    // replacement), or a class with an unreadable member.
    let mut bad_classes: HashSet<&str> = ir
        .oo_unanalysed_classes
        .iter()
        .map(String::as_str)
        .collect();
    for m in ir
        .methods
        .values()
        .chain(ir.redefined_methods.values().flatten())
    {
        if crate::cfg_builder::upvar_info::reaches_caller_frame(&m.body, &m.params) {
            bad_classes.insert(m.class_name.as_str());
        }
    }
    if bad_classes.is_empty() {
        return MethodDispatchBarrier::allow_all();
    }

    // Class universe and hierarchy components.
    let classes: Vec<&str> = {
        let mut set: HashSet<&str> = ir.methods.values().map(|m| m.class_name.as_str()).collect();
        set.extend(ir.class_relations.iter().map(|(c, _)| c.as_str()));
        set.extend(ir.oo_unanalysed_classes.iter().map(String::as_str));
        let mut v: Vec<&str> = set.into_iter().collect();
        v.sort_unstable();
        v
    };
    let comp_of = hierarchy_components(&classes, &ir.class_relations);

    let tainted: HashSet<usize> = bad_classes
        .iter()
        .filter_map(|c| comp_of.get(*c).copied())
        .collect();

    // Per-proc dispatch facts, closed over the proc call graph.
    let proc_facts = proc_dispatch_facts(ir, registry, &comp_of);
    let method_facts = method_dispatch_facts(ir, registry, &comp_of, &proc_facts);
    let comp_out = component_dispatch_closure(ir, &comp_of, &method_facts);

    // Bar each method whose reachable dispatch surface meets a bad class.
    let mut barred: HashSet<String> = HashSet::new();
    for (qname, facts) in &method_facts {
        let mut anywhere = facts.anywhere;
        let mut reach: HashSet<usize> = facts.comps.clone();
        for &c in &facts.comps {
            if let Some(out) = comp_out.get(c) {
                anywhere |= out.anywhere;
                reach.extend(out.comps.iter().copied());
            }
        }
        if anywhere || reach.iter().any(|c| tainted.contains(c)) {
            barred.insert((*qname).to_owned());
        }
    }
    MethodDispatchBarrier {
        bar_all: false,
        barred,
    }
}

/// Direct dispatch facts per method, including every retained replacement
/// body (the live body is one of them, so the union is the sound answer).
fn method_dispatch_facts<'a>(
    ir: &'a IrModule,
    registry: &CommandRegistry,
    comp_of: &HashMap<String, usize>,
    proc_facts: &HashMap<&str, DispatchFacts>,
) -> HashMap<&'a str, DispatchFacts> {
    let metadata = dispatch_metadata(registry, ir);

    let mut method_facts: HashMap<&str, DispatchFacts> = HashMap::new();
    for (qname, m) in &ir.methods {
        let own_comp = comp_of.get(m.class_name.as_str()).copied();
        let mut facts = DispatchFacts::default();
        for body_def in std::iter::once(m).chain(
            ir.redefined_methods
                .get(qname)
                .map_or(&[][..], Vec::as_slice),
        ) {
            collect_dispatches(
                &body_def.body,
                &body_def.execution_namespace,
                &ScanEnv {
                    metadata,
                    registry,
                    comp_of,
                    proc_facts,
                    ir,
                    own_comp,
                },
                &mut facts,
                &mut HashSet::new(),
                0,
            );
        }
        method_facts.insert(qname.as_str(), facts);
    }
    method_facts
}

/// Component-level dispatch closure: dispatching into a component also
/// reaches everything its methods dispatch to.
fn component_dispatch_closure(
    ir: &IrModule,
    comp_of: &HashMap<String, usize>,
    method_facts: &HashMap<&str, DispatchFacts>,
) -> Vec<DispatchFacts> {
    let comp_count = comp_of.values().copied().max().map_or(0, |m| m + 1);
    let mut comp_out: Vec<DispatchFacts> = vec![DispatchFacts::default(); comp_count];
    for (qname, m) in &ir.methods {
        if let (Some(&comp), Some(facts)) = (
            comp_of.get(m.class_name.as_str()),
            method_facts.get(qname.as_str()),
        ) {
            comp_out[comp].absorb(facts);
        }
    }
    loop {
        let mut changed = false;
        for i in 0..comp_count {
            let reached: Vec<usize> = comp_out[i].comps.iter().copied().collect();
            for j in reached {
                if i != j {
                    let other = comp_out[j].clone();
                    changed |= comp_out[i].absorb(&other);
                }
            }
        }
        if !changed {
            break;
        }
    }
    comp_out
}

/// Union the classes into connected components under the captured
/// `superclass` / `mixin` relations. A written relation word connects the
/// declaring class to every module class it could resolve to — exact
/// qualified spelling, the `::`-rooted spelling, or (conservatively) any
/// class sharing its tail; Tcl's own resolution can only ever pick a
/// command whose tail matches the written word's tail, so tail matching
/// over-approximates it (extra edges only ever bar more, never less).
/// A word matching no module class names an external class, which is
/// outside this closed-world analysis exactly as external methods are.
fn hierarchy_components(
    classes: &[&str],
    relations: &[(String, String)],
) -> HashMap<String, usize> {
    let mut parent: Vec<usize> = (0..classes.len()).collect();
    let index: HashMap<&str, usize> = classes.iter().enumerate().map(|(i, c)| (*c, i)).collect();
    for (class, written) in relations {
        let Some(&ci) = index.get(class.as_str()) else {
            continue;
        };
        let written_tail = written.rsplit("::").next().unwrap_or(written);
        for (other, &oi) in &index {
            let matches = *other == written.as_str()
                || other.strip_prefix("::") == Some(written.as_str())
                || other.rsplit("::").next() == Some(written_tail);
            if matches {
                let (a, b) = (uf_find(&mut parent, ci), uf_find(&mut parent, oi));
                if a != b {
                    parent[a] = b;
                }
            }
        }
    }
    let mut roots: HashMap<usize, usize> = HashMap::new();
    let mut out = HashMap::new();
    for (i, class) in classes.iter().enumerate() {
        let root = uf_find(&mut parent, i);
        let next_id = roots.len();
        let id = *roots.entry(root).or_insert(next_id);
        out.insert((*class).to_owned(), id);
    }
    out
}

/// Path-compressing union-find lookup over the flat `parent` table
/// ([`hierarchy_components`]).
fn uf_find(parent: &mut [usize], i: usize) -> usize {
    let mut root = i;
    while parent[root] != root {
        root = parent[root];
    }
    let mut cur = i;
    while parent[cur] != root {
        let next = parent[cur];
        parent[cur] = root;
        cur = next;
    }
    root
}

/// Dispatch facts for every module proc, closed over the proc call graph
/// (a proc that calls a proc that dispatches `$obj m` dispatches it too,
/// on behalf of whoever called it).
fn proc_dispatch_facts<'a>(
    ir: &'a IrModule,
    registry: &CommandRegistry,
    comp_of: &HashMap<String, usize>,
) -> HashMap<&'a str, DispatchFacts> {
    let metadata = dispatch_metadata(registry, ir);

    // Direct facts + direct proc callees per proc.
    let mut facts: HashMap<&str, DispatchFacts> = HashMap::new();
    let mut callees: HashMap<&str, HashSet<String>> = HashMap::new();
    for (qname, proc) in &ir.procedures {
        let namespace = super::helpers::naming::namespace_from_qualified(qname);
        let mut f = DispatchFacts::default();
        let mut c: HashSet<String> = HashSet::new();
        collect_dispatches(
            &proc.body,
            &ExecutionNamespace::exact(namespace),
            &ScanEnv {
                metadata,
                registry,
                comp_of,
                proc_facts: &HashMap::new(),
                ir,
                own_comp: None,
            },
            &mut f,
            &mut c,
            0,
        );
        facts.insert(qname.as_str(), f);
        callees.insert(qname.as_str(), c);
    }
    // Fixpoint over the (small) proc call graph.
    loop {
        let mut changed = false;
        let names: Vec<&str> = facts.keys().copied().collect();
        for name in names {
            let callee_facts: Vec<DispatchFacts> = callees
                .get(name)
                .into_iter()
                .flatten()
                .filter_map(|callee| facts.get(callee.as_str()).cloned())
                .collect();
            if let Some(f) = facts.get_mut(name) {
                for cf in &callee_facts {
                    changed |= f.absorb(cf);
                }
            }
        }
        if !changed {
            break;
        }
    }
    facts
}

/// Everything [`collect_dispatches`] needs to classify one call head.
struct ScanEnv<'a> {
    // The outer option retains terminal supplied refusal. Some(None) is only
    // the deliberately unprofiled standalone compatibility route.
    metadata: Option<Option<crate::registry_invocation::InvocationMetadataContext<'a>>>,
    registry: &'a CommandRegistry,
    comp_of: &'a HashMap<String, usize>,
    proc_facts: &'a HashMap<&'a str, DispatchFacts>,
    ir: &'a IrModule,
    own_comp: Option<usize>,
}

fn dispatch_metadata<'a>(
    registry: &CommandRegistry,
    module: &'a IrModule,
) -> Option<Option<crate::registry_invocation::InvocationMetadataContext<'a>>> {
    if module.source_metadata_input.is_none()
        && module.source_entry.metadata_context.is_standalone()
    {
        module
            .source_entry
            .metadata_context
            .metadata_context(registry)
    } else {
        if !module
            .retained_source_bindings
            .as_deref()?
            .matches_module(module, registry)
        {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
            .map(Some)
    }
}

impl ScanEnv<'_> {
    fn original_substitutions(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<Vec<crate::word_subst::LiftedCall>> {
        let metadata = self.metadata?;
        if let Some(metadata) = metadata.filter(|metadata| !metadata.is_standalone()) {
            return crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens,
                self.ir.lexer_config.nested(),
                self.registry,
                metadata,
            );
        }
        // Explicit standalone assistance retains its original catalogue route.
        let config = tokens.native_lexer_config(self.ir.lexer_config.nested());
        crate::word_subst::checked_lifted_calls(tokens, config)?;
        let surface = tcl_registry::model::DocumentCommandSurface::new(
            self.registry,
            self.ir.source_entry.declared_commands.as_ref(),
        );
        Some(crate::word_subst::lifted_calls_with_surface(
            Some(tokens),
            config,
            &surface,
        ))
    }

    fn substitutions(
        &self,
        statement: &Statement,
    ) -> crate::ir_helpers::EvaluatedCommandSubstitutions {
        if self.ir.source_metadata_input.is_none()
            && self.ir.source_entry.metadata_context.is_standalone()
        {
            crate::ir_helpers::evaluated_command_substitutions(statement, self.registry)
        } else {
            crate::ir_helpers::evaluated_command_substitutions_with_metadata_context(
                statement,
                self.registry,
                self.metadata.flatten(),
                self.ir.lexer_config.nested(),
            )
        }
    }
}

/// Walk a method body's direct `Call` heads and evaluated command
/// substitutions — see the module doc for the classification. `Barrier` /
/// `UpFrame` statements are deliberately not direct dispatch sites here: SCCP
/// widens every tracked value at them already. Their ordinary evaluated
/// substitutions and nested executable bodies are still scanned below.
fn collect_dispatches(
    script: &Script,
    execution_namespace: &ExecutionNamespace,
    env: &ScanEnv<'_>,
    facts: &mut DispatchFacts,
    callees: &mut HashSet<String>,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        facts.anywhere = true;
        return;
    }
    if !script.statements.is_empty() && env.metadata.is_none() {
        facts.anywhere = true;
        return;
    }
    for statement in &script.statements {
        if let Some(tokens) = script.retained_source_tokens_for_statement(statement) {
            let Some(children) = env.original_substitutions(tokens) else {
                facts.anywhere = true;
                return;
            };
            if matches!(statement, Statement::Call { .. }) {
                classify_tokens(tokens, env, facts, callees);
            }
            for child in children {
                if let Some(tokens) = child.tokens.as_ref() {
                    classify_tokens(tokens, env, facts, callees);
                } else {
                    facts.anywhere = true;
                }
            }
            if env.substitutions(statement).opaque {
                facts.anywhere = true;
            }
        } else {
            let substitutions = env.substitutions(statement);
            if matches!(statement, Statement::Call { .. })
                || substitutions.opaque
                || substitutions.all_commands().next().is_some()
            {
                facts.anywhere = true;
            }
        }
        for (body, namespace) in
            crate::ir_helpers::nested_execution_bodies(statement, execution_namespace)
        {
            collect_dispatches(body, &namespace, env, facts, callees, depth + 1);
        }
        if facts.anywhere {
            return;
        }
    }
}

fn classify_tokens(
    tokens: &crate::ir::CommandTokens,
    env: &ScanEnv<'_>,
    facts: &mut DispatchFacts,
    callees: &mut HashSet<String>,
) {
    let Some(binding) = tokens.source_binding.as_ref() else {
        facts.anywhere = true;
        return;
    };
    if binding.execution_is_unknown() || binding.execution_may_be_absent() {
        facts.anywhere = true;
        return;
    }
    let targets: Vec<_> = binding.execution_targets().collect();
    if targets.is_empty() {
        facts.anywhere = true;
        return;
    }
    for target in targets {
        match target.kind {
            crate::command_binding::BindingKind::Class => {
                if let Some(component) = env.comp_of.get(&target.command) {
                    facts.comps.insert(*component);
                } else {
                    facts.anywhere = true;
                }
            }
            crate::command_binding::BindingKind::Proc => {
                record_procedure(target, env, facts, callees);
            }
            _ if target.registry_backed => {
                let metadata = env.metadata.flatten();
                let invocation = if let Some(metadata) =
                    metadata.filter(|metadata| metadata.permits_logical_source_names())
                {
                    crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                        env.registry, metadata, tokens,
                    )
                } else {
                    crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                        env.registry,
                        metadata,
                        tokens,
                    )
                };
                let Some(invocation) = invocation else {
                    facts.anywhere = true;
                    continue;
                };
                if !invocation.facts.arg_roles_complete {
                    facts.anywhere = true;
                    continue;
                }
                if let Some(call) = crate::registry_invocation::normal_user_procedure_invocation_with_metadata_context(
                    env.registry,
                    env.metadata.flatten(),
                    tokens,
                ) {
                    let selected = binding.lookup_command_word(&call.target);
                    if call.unknown_runtime {
                        facts.anywhere = true;
                    } else if let Some(target) = selected.proved_target() {
                        record_procedure(target, env, facts, callees);
                    } else {
                        facts.anywhere = true;
                    }
                } else if matches!(
                    env.registry
                        .method_dispatch_keyword(&invocation.facts.canonical_command),
                    Some(
                        tcl_registry::MethodDispatchKind::SelfDispatch
                            | tcl_registry::MethodDispatchKind::NextChain
                    )
                ) {
                    if let Some(component) = env.own_comp {
                        facts.comps.insert(component);
                    } else {
                        facts.anywhere = true;
                    }
                } else if invocation.facts.effects.requires_world_barrier()
                    || invocation
                        .facts
                        .arg_roles
                        .iter()
                        .any(|(_, role)| *role == tcl_registry::ArgRole::CommandPrefix)
                {
                    facts.anywhere = true;
                }
            }
            _ => facts.anywhere = true,
        }
    }
}

fn record_procedure(
    target: &crate::command_binding::SourceCommandTarget,
    env: &ScanEnv<'_>,
    facts: &mut DispatchFacts,
    callees: &mut HashSet<String>,
) {
    let Some(procedure) = env.ir.procedures.get(&target.command).filter(|procedure| {
        !env.ir.redefined_procedures.contains(&target.command)
            && target.matches_authored_implementation_image(&env.ir.source, procedure.span.start())
    }) else {
        facts.anywhere = true;
        return;
    };
    callees.insert(procedure.qualified_name.clone());
    if let Some(callee_facts) = env.proc_facts.get(procedure.qualified_name.as_str()) {
        facts.absorb(callee_facts);
    }
}

#[cfg(test)]
mod tests {
    use super::compute;

    /// Tcl 9.0.4: a procedure installed as `${object_namespace}::my` shadows
    /// the object's normal self-dispatch command. A receiver-selected bare
    /// `my` therefore cannot confine dispatch to its lexical class component.
    #[test]
    fn runtime_selected_embedded_my_cannot_prove_own_component_dispatch() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let module = crate::lowering::lower_to_ir(
            "oo::class create Bad {\n\
             \x20 method mutate {name} { upvar 1 $name local; set local 1 }\n\
             }\n\
             oo::class create C {\n\
             \x20 method helper {} { return ok }\n\
             \x20 method caller {} { ::set ignored [my helper]; ::return ok }\n\
             }",
            registry,
        );
        let barrier = compute(&module, registry);
        assert!(
            !barrier.allows_locals("::C::caller"),
            "receiver-namespace shadowing can redirect bare `my` to the bad class"
        );
    }
    #[test]
    fn retained_dispatch_does_not_select_an_unrelated_class_tail() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let module = crate::lowering::lower_to_ir(
            "oo::class create Bad {method mutate {name} {upvar 1 $name local; set local 1}}
\
             oo::class create C {method safe {} {return SAFE}}
\
             oo::class create Caller {method probe {} {::missing::C new; return ok}}",
            registry,
        );
        assert!(!compute(&module, registry).allows_locals("::Caller::probe"));
    }

    #[test]
    fn retained_dispatch_keeps_namespace_local_class_and_procedure_bindings() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        for helper in [
            "namespace eval N {oo::class create C {method mutate {name} {upvar 1 $name local; set local 1}}; proc make {} {C new}}",
            "namespace eval N {proc C {} {::Bad new}; proc make {} {C}}",
        ] {
            let source = format!(
                "oo::class create Bad {{method mutate {{name}} {{upvar 1 $name local; set local 1}}}}\n\
                 oo::class create C {{method safe {{}} {{return SAFE}}}}\n\
                 {helper}\n\
                 oo::class create Caller {{method probe {{}} {{::N::make; return ok}}}}"
            );
            let module = crate::lowering::lower_to_ir(&source, registry);
            assert!(
                !compute(&module, registry).allows_locals("::Caller::probe"),
                "{source}"
            );
        }
    }

    #[test]
    fn retained_dispatch_without_calls_keeps_local_values_available() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let module = crate::lowering::lower_to_ir(
            "oo::class create Bad {method mutate {name} {upvar 1 $name local; set local 1}}
\
             oo::class create C {method safe {} {return SAFE}}
\
             C create c; c safe",
            registry,
        );
        assert!(compute(&module, registry).allows_locals("::C::safe"));
    }
    #[test]
    fn nested_dispatch_uses_retained_availability_and_source_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Source-only dispatch advice; Native frame and implementation entry stay absent.
        let current =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = current.commands();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&current),
            config,
        );
        let mut module = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            "proc helper {} {return [expr {[dict size [dict create key value]]}]}",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
        .ir_module;
        assert!(module.source_entry.native_entry.is_none());
        let components = std::collections::HashMap::new();
        assert!(!super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere);
        let original = module.clone();
        for change in 0..6 {
            module = original.clone();
            match change {
                0 => module.retained_source_bindings = None,
                1 => module.source_entry.unknown_entry = !module.source_entry.unknown_entry,
                2 => module.top_level_namespace = "::other".into(),
                3 => {
                    module.native_namespace =
                        Some(tcl_core_types::ByteNamespacePath::from_segments(["other"]))
                }
                4 => module.top_level_kind = crate::ir::TopLevelKind::ProcedureBody,
                5 => {
                    module.source_entry.compilation_scope =
                        tcl_runtime_api::SourceCompilationScope::EnteredSource
                }
                _ => unreachable!(),
            }
            assert!(
                crate::registry_invocation::InvocationMetadataContext::for_module(
                    registry, &module
                )
                .is_some()
            );
            assert!(
                super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere,
                "change {change}"
            );
        }
        module = original;
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(std::sync::Arc::clone(registry)),
        );
        for context in [
            older,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        ] {
            module.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            ));
            assert!(
                super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere
            );
        }
        module.source_metadata_input = None;
        assert!(super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere);
        module.source_metadata_input = Some(input);
        module.lexer_config.expand_syntax = !module.lexer_config.expand_syntax;
        assert!(super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere);
    }

    #[test]
    fn dispatch_metadata_requires_actual_module_availability() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Actual original registrations are retained independently of metadata
        // availability. The owner stays alive through every refusal control.
        let current =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = current.commands();
        let profile = registry.profile().unwrap();
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            native_entry: Some(std::sync::Arc::new(native)),
            ..crate::command_binding::SourceAnalysisEntry::default()
        };
        let mut module = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            "proc helper {} {dict create key value}",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&current),
        )
        .ir_module;
        let components = std::collections::HashMap::new();
        assert!(!super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere);
        let input = module.source_metadata_input.as_ref().unwrap().clone();
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry));
        for context in [
            std::sync::Arc::new(older),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        ] {
            module.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                context,
                input.lexer_config(),
            ));
            assert!(
                super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere
            );
        }
        module.source_metadata_input = None;
        assert!(super::proc_dispatch_facts(&module, registry, &components)["::helper"].anywhere);
    }
}
