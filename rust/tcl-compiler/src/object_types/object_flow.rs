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

//! The object-flow fixpoint: assignment, call-argument and return edges that carry classes between variables.

use super::constructor_harvest::{
    constructor_class_candidate, sole_substitution_tokens, source_assignment_candidates,
};
use super::procedure_results::returning_methods;
use super::{
    FactMode, FactSink, ObjectFlowEdge, ObjectHandleFacts, OwnerIndex, ScanContext, frame_for_unit,
};
use crate::compilation_unit::CompilationUnit;
use crate::ir::MethodCoordinate;
use crate::ir::MethodKind;
use crate::ir::ProcedureId;
use crate::ir::Statement;
use std::collections::{HashMap, HashSet};
use tcl_core_types::BodyFrameId;
use tcl_registry::CommandRegistry;
use tcl_source::words::WordExpr;

/// VTA-lite object-flow fixpoint.  Propagates object classes from the seeded
/// handles along four kinds of type-propagation edge — assignment (aliasing),
/// proc return, proc parameter, and constructor parameter — until no new class
/// reaches any node.  Nodes are name-keyed (a variable / parameter is one node
/// regardless of scope, VTA's field-based object-insensitive economy) and the
/// join is set union.  Highlight-only and union-imprecise, matching the rest of
/// this module.
pub(super) fn propagate_object_flow(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    returns: &HashMap<ProcedureId, String>,
    sink: &mut FactSink,
) {
    let proc_params: HashMap<ProcedureId, &[String]> = cu
        .ir_module
        .procedures
        .iter()
        .map(|(id, procedure)| (*id, procedure.params.as_slice()))
        .collect();
    let ctor_params: HashMap<&str, Vec<(ProcedureId, &crate::ir::MethodDef)>> = {
        let mut constructors: HashMap<&str, Vec<(ProcedureId, &crate::ir::MethodDef)>> =
            HashMap::new();
        for (id, method) in &cu.ir_module.methods {
            if method.kind == MethodKind::Constructor {
                constructors
                    .entry(method.class_name.as_str())
                    .or_default()
                    .push((*id, method));
            }
        }
        constructors
    };
    let mut method_coordinates: HashMap<MethodCoordinate, Vec<ProcedureId>> = HashMap::new();
    for (id, method) in &cu.ir_module.methods {
        method_coordinates
            .entry(method.coordinate())
            .or_default()
            .push(*id);
    }
    let method_returns = returning_methods(cu);
    if returns.is_empty()
        && proc_params.is_empty()
        && ctor_params.is_empty()
        && method_returns.is_empty()
        && sink.facts.any_scope.is_empty()
    {
        return;
    }
    let index = FlowIndex {
        returns: returns.clone(),
        method_returns,
        method_coordinates,
        module: &cu.ir_module,
        proc_params,
        ctor_params,
    };

    // This is a finite, monotone domain: existing binding sites receive only
    // class identities already present in the seeds or return summaries.
    // Cycles converge when no set grows; a private round limit would lose
    // evidence along longer chains. Both scoped and union facts participate
    // in convergence, while reads retain their own scope attribution.
    let scoped = (sink.mode == FactMode::Full).then_some(sink.owners);
    loop {
        let bindings = scan_flow_edges(
            cu,
            registry,
            &sink.facts,
            scoped,
            &index,
            sink.source_analysis,
        );
        sink.stats.rounds += 1;
        let mut changed = false;
        for binding in bindings {
            match binding.kind {
                ObjectFlowEdge::Alias => sink.stats.alias_bindings += 1,
                ObjectFlowEdge::ProcReturn => sink.stats.return_bindings += 1,
                ObjectFlowEdge::MethodReturn => sink.stats.method_return_bindings += 1,
                ObjectFlowEdge::ProcParam => sink.stats.param_bindings += 1,
                ObjectFlowEdge::CtorParam => sink.stats.ctor_param_bindings += 1,
            }
            let entry = sink
                .facts
                .any_scope
                .entry(binding.name.clone())
                .or_default();
            for c in &binding.union_classes {
                changed |= entry.insert(c.clone());
            }
            if sink.mode == FactMode::Full && !binding.scoped_classes.is_empty() {
                let owner = sink.owners.owner_for(binding.owner, &binding.name);
                let entry = sink
                    .facts
                    .by_scope
                    .entry((owner, binding.name))
                    .or_default();
                // The scope-keyed fact can still be growing after the union has
                // settled (it lags by the round its source needed), so it drives
                // the loop too.  Extra rounds cannot change `any_scope`: unioning
                // a converged fixpoint with itself is a no-op.
                for c in binding.scoped_classes {
                    changed |= entry.insert(c);
                }
            }
        }
        if !changed {
            break;
        }
    }
}

/// The callee-side maps one fixpoint round resolves call edges against.
struct FlowIndex<'a> {
    /// Proc qualified name → returned object class.
    pub(super) returns: HashMap<ProcedureId, String>,
    /// Method [`FunctionUnit`] key (`::Class::method`) → returned object
    /// class, for the `set b [$a make]` method-return edge.
    method_returns: HashMap<ProcedureId, &'a str>,
    method_coordinates: HashMap<MethodCoordinate, Vec<ProcedureId>>,
    pub(super) module: &'a crate::ir::Module,
    /// Proc qualified name → parameter names.
    proc_params: HashMap<ProcedureId, &'a [String]>,
    /// Class qualified name → (constructor unit key, constructor parameters).
    ctor_params: HashMap<&'a str, Vec<(ProcedureId, &'a crate::ir::MethodDef)>>,
}

/// One type-propagation edge's product: the classes `name` gains, and the unit
/// the edge binds that name **in** (the assigning unit for an alias / return,
/// the callee for a parameter, the constructor for a constructor parameter).
///
/// The two class sets are the two facts the module maintains, and they are
/// **not** the same set:
///
/// * `union_classes` resolves the edge's source against the scope-blind
///   [`ObjectHandleFacts::any_scope`] map, and feeds it back — the deliberate,
///   documented highlight-grade imprecision this module has always had.
/// * `scoped_classes` resolves the edge's source *in the scope that owns it*
///   and feeds [`ObjectHandleFacts::by_scope`].  Empty means "no evidence in
///   the owning scope", which is the whole point: a `set y $x` in one proc
///   must not read a same-named `x` bound in another: a false singleton in
///   the narrow map is a wrong rename, not a missed one.
pub(super) struct Binding {
    pub(super) owner: BodyFrameId,
    pub(super) name: String,
    union_classes: HashSet<String>,
    scoped_classes: HashSet<String>,
    pub(super) kind: ObjectFlowEdge,
}

/// The two class sets one edge yields — see [`Binding`].
pub(super) struct EdgeClasses {
    union: HashSet<String>,
    scoped: HashSet<String>,
}

impl EdgeClasses {
    /// An edge whose source is **unit-independent** (a callee's return type, a
    /// literal `[Class new]` argument): both facts agree, because there is no
    /// variable read to scope.
    fn unscoped(class: &str) -> Self {
        let one: HashSet<String> = std::iter::once(class.to_owned()).collect();
        Self {
            union: one.clone(),
            scoped: one,
        }
    }
}

/// One round of the VTA-lite fixpoint: scan every statement, reading current
/// node classes from `out`, and return the `(node, classes)` bindings the
/// type-propagation edges imply this round.  The caller unions them into `out`
/// and iterates until nothing changes.
fn scan_flow_edges(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    facts: &ObjectHandleFacts,
    scoped: Option<&OwnerIndex>,
    index: &FlowIndex,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> Vec<Binding> {
    let metadata_context =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module);
    let ctor_params = &index.ctor_params;
    let resolve_ctor_class =
        |head: &str| -> Option<String> { ctor_params.contains_key(head).then(|| head.to_owned()) };

    let mut bindings: Vec<Binding> = Vec::new();
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    // A `FactMode::AnyScopeOnly` build has no scope-keyed fact to maintain, so
    // it never pays for the scoped lookups; the empty index only satisfies the
    // shared context type.
    let owners: &OwnerIndex = scoped.unwrap_or_else(|| OwnerIndex::shared_empty());
    let by_scope = scoped.map(|_| &facts.by_scope);
    for fu in units {
        let Some(frame) = frame_for_unit(cu, fu) else {
            continue;
        };
        let ctx = ScanContext {
            ctor_params,
            out: &facts.any_scope,
            by_scope,
            owners,
            registry,
            metadata_context,
            source_analysis,
            unit: frame,
            tokens: None,
        };
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let ctx = ScanContext {
                    tokens: stmt.tokens(),
                    ..ctx
                };
                scan_flow_statement(stmt, ctx, index, &resolve_ctor_class, &mut bindings);
            }
        }
    }
    for (unit, script) in cu
        .ir_module
        .original_source_procedure_bodies()
        .filter(|(name, _)| {
            metadata_context.is_some_and(
                crate::registry_invocation::InvocationMetadataContext::permits_logical_source_names,
            ) || !cu.procedures.contains_key(*name)
        })
        .map(|(id, procedure)| (BodyFrameId::Body(*id), &procedure.body))
        .chain(
            cu.ir_module
                .methods
                .iter()
                .filter(|(name, _)| !cu.methods.contains_key(*name))
                .map(|(id, method)| (BodyFrameId::Body(*id), &method.body)),
        )
    {
        let mut pending = vec![script];
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                let ctx = ScanContext {
                    ctor_params,
                    out: &facts.any_scope,
                    by_scope,
                    owners,
                    registry,
                    metadata_context,
                    source_analysis,
                    unit,
                    tokens: script.retained_source_tokens_for_statement(statement),
                };
                scan_flow_statement(statement, ctx, index, &resolve_ctor_class, &mut bindings);
                pending.extend(statement.child_scripts());
            }
        }
    }

    bindings
}

/// One advisory flow step, shared by executable CFG and retained generic IR.
fn scan_flow_statement(
    stmt: &Statement,
    ctx: ScanContext<'_>,
    index: &FlowIndex<'_>,
    resolve_ctor_class: &impl Fn(&str) -> Option<String>,
    bindings: &mut Vec<Binding>,
) {
    // Argument substitutions precede the parent handler. Their retained
    // constructor entries remain possible even when that handler's normal
    // assignment protocol is unknown after the child returns.
    scan_substitution_constructor_edges(resolve_ctor_class, ctx, bindings);
    let FlowIndex {
        returns,
        method_returns,
        proc_params,
        ..
    } = index;
    match stmt {
        Statement::AssignValue { name, value, .. } => {
            let Some(value) = ctx
                .tokens
                .and_then(|tokens| unique_original_value_word(tokens, value))
            else {
                return;
            };
            scan_assign_edges(
                AssignSite {
                    unit: ctx.unit,
                    name,
                    value,
                },
                ReturnEdges {
                    procs: returns,
                    methods: method_returns,
                    coordinates: &index.method_coordinates,
                    module: index.module,
                },
                ctx,
                bindings,
            );
        }
        Statement::Call { .. } => {
            for assignment in ctx.tokens.map_or_else(Vec::new, |tokens| {
                source_assignment_candidates(ctx.registry, ctx.metadata_context, tokens)
            }) {
                scan_assign_edges(
                    AssignSite {
                        unit: ctx.unit,
                        name: &assignment.name,
                        value: &assignment.value,
                    },
                    ReturnEdges {
                        procs: returns,
                        methods: method_returns,
                        coordinates: &index.method_coordinates,
                        module: index.module,
                    },
                    ctx,
                    bindings,
                );
            }
            // Proc-parameter edge: `f $obj` binds f's params — in
            // the *callee*, which is where the name lives.
            if let Some(target) = ctx
                .tokens
                .and_then(|tokens| tokens.source_binding.as_ref())
                .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target)
                .filter(|target| target.kind == tcl_names::command::BindingKind::Proc)
                && let Some((id, _)) = index.module.source_procedure_for_target(target)
                && let Some(params) = proc_params.get(&id)
                && let Some(effective) = ctx
                    .tokens
                    .and_then(crate::registry_invocation::effective_command_words)
            {
                emit_proc_param_bindings(id, params, &effective.words[1..], ctx, bindings);
            }
            // Constructor-parameter edge: `Class create NAME …`.
            emit_ctor_param_bindings(resolve_ctor_class, ctx, bindings);
        }
        _ => {}
    }
}

fn scan_substitution_constructor_edges(
    resolve_ctor_class: &impl Fn(&str) -> Option<String>,
    ctx: ScanContext<'_>,
    bindings: &mut Vec<Binding>,
) {
    let Some(parent) = ctx.tokens else {
        return;
    };
    let mut pending = vec![parent.clone()];
    while let Some(parent) = pending.pop() {
        let config =
            parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(ctx.registry.profile()));
        for word in parent.words() {
            let Some(children) =
                crate::value_shapes::command_substitution_tokens(word, Some(&parent), config)
            else {
                continue;
            };
            for child in children {
                if child.source_binding.as_ref().is_none_or(|binding| {
                    binding.runtime_reachability()
                        == crate::command_binding::SourceRuntimeReachability::NotEntered
                }) {
                    continue;
                }
                emit_ctor_param_bindings(
                    resolve_ctor_class,
                    ScanContext {
                        tokens: Some(&child),
                        ..ctx
                    },
                    bindings,
                );
                pending.push(child);
            }
        }
    }
}

/// One `set NAME VALUE` statement, with the unit it is written in.
#[derive(Clone, Copy)]
struct AssignSite<'a> {
    pub(super) unit: BodyFrameId,
    pub(super) name: &'a str,
    pub(super) value: &'a WordExpr,
}

/// The callee-side return-type maps [`scan_assign_edges`] resolves a
/// bracketed value's head against: [`FlowIndex::returns`] and
/// [`FlowIndex::method_returns`].
#[derive(Clone, Copy)]
struct ReturnEdges<'a, 'b> {
    procs: &'b HashMap<ProcedureId, String>,
    pub(super) methods: &'b HashMap<ProcedureId, &'a str>,
    coordinates: &'b HashMap<MethodCoordinate, Vec<ProcedureId>>,
    pub(super) module: &'a crate::ir::Module,
}

/// The edges an assignment can carry — aliasing (`set A $B`), proc return
/// (`set A [make …]`), and method return (`set A [$obj make …]`).
/// Assignment edges bind in the assigning unit; child constructor edges
/// are scanned independently before this normal assignment projection.
fn scan_assign_edges(
    site: AssignSite,
    returns: ReturnEdges<'_, '_>,
    ctx: ScanContext,
    bindings: &mut Vec<Binding>,
) {
    if let Some(src) = deref_arg_var(site.value, ctx)
        && let Some(classes) = ctx.out.get(src).filter(|s| !s.is_empty())
    {
        bindings.push(Binding {
            owner: site.unit,
            name: site.name.to_owned(),
            union_classes: classes.clone(),
            scoped_classes: ctx.scoped_classes(src),
            kind: ObjectFlowEdge::Alias,
        });
    }
    let Some(nested) = ctx
        .tokens
        .and_then(|parent| sole_substitution_tokens(site.value, parent, ctx.registry))
    else {
        return;
    };
    let target = nested
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target);
    // Return flow follows the retained implementation token, independently
    // of its written spelling or present command slot.
    if let Some(class) = target
        .filter(|target| target.kind == tcl_names::command::BindingKind::Proc)
        .and_then(|target| returns.module.source_procedure_for_target(target))
        .and_then(|(id, _)| returns.procs.get(&id))
    {
        // A legitimately cross-scope flow: the source is the *callee's* return
        // type, not a variable read, so there is no scope to confuse and both
        // facts take it.
        let classes = EdgeClasses::unscoped(class);
        bindings.push(Binding {
            owner: site.unit,
            name: site.name.to_owned(),
            union_classes: classes.union,
            scoped_classes: classes.scoped,
            kind: ObjectFlowEdge::ProcReturn,
        });
    }
    // Method-return edge: `set B [$a make …]` — the receiver's
    // classes are already tracked, and a directly-declared `::Class::make`
    // whose own return type names an object class types the captured handle,
    // exactly like a proc return.  The *receiver* is a variable read in this
    // unit, so the scope-keyed fact resolves it here (the union fact stays
    // blind, as for the aliasing edge). The selected grammar must decode a
    // literal member; computed member words supply no coordinate.
    let words = nested.words();
    if let Some(recv) = words.first().and_then(|word| {
        deref_arg_var(
            word,
            ScanContext {
                tokens: Some(&nested),
                ..ctx
            },
        )
    }) && let Some(method) = words
        .get(1)
        .and_then(|word| literal_source_word(word, &nested, ctx.registry))
    {
        let resolve = |classes: &HashSet<String>| -> HashSet<String> {
            classes
                .iter()
                .flat_map(|class| {
                    returns
                        .coordinates
                        .get(&MethodCoordinate {
                            class_label: class.clone(),
                            member_label: method.clone(),
                            kind: MethodKind::Method,
                        })
                        .into_iter()
                        .flatten()
                })
                .filter_map(|id| returns.methods.get(id))
                .map(|ret| (*ret).to_owned())
                .collect()
        };
        if let Some(recv_classes) = ctx.out.get(recv).filter(|s| !s.is_empty()) {
            let union_classes = resolve(recv_classes);
            if !union_classes.is_empty() {
                bindings.push(Binding {
                    owner: site.unit,
                    name: site.name.to_owned(),
                    union_classes,
                    scoped_classes: resolve(&ctx.scoped_classes(recv)),
                    kind: ObjectFlowEdge::MethodReturn,
                });
            }
        }
    }
}

/// Bind `callee`'s parameters to the object classes of a call's arguments.
/// Stops at `args` (a variadic tail is not positionally bindable).
fn emit_proc_param_bindings(
    callee: ProcedureId,
    params: &[String],
    args: &[WordExpr],
    ctx: ScanContext,
    bindings: &mut Vec<Binding>,
) {
    for (i, arg) in args.iter().enumerate() {
        let Some(pname) = params.get(i) else { break };
        if pname == "args" {
            break;
        }
        // Also legitimately cross-scope: the argument is resolved in the
        // *caller* (`ctx.unit`) and the parameter is bound in the callee.
        if let Some(classes) = arg_classes(arg, ctx) {
            bindings.push(Binding {
                owner: BodyFrameId::Body(callee),
                name: pname.clone(),
                union_classes: classes.union,
                scoped_classes: classes.scoped,
                kind: ObjectFlowEdge::ProcParam,
            });
        }
    }
}

impl ScanContext<'_> {
    /// The classes `var` holds **in the unit being scanned** — the owning
    /// unit's binding, or its class's when `var` is one of that class's
    /// instance variables (the cross-method bridge).
    ///
    /// Empty when there is no binding in that scope, which is exactly what
    /// stops one unit's `x` from flowing into another's.
    pub(super) fn scoped_classes(&self, var: &str) -> HashSet<String> {
        let Some(by_scope) = self.by_scope else {
            return HashSet::new();
        };
        let owner = self.owners.owner_for(self.unit, var);
        by_scope
            .get(&(owner, var.to_owned()))
            .cloned()
            .unwrap_or_default()
    }
}

/// Bind a constructor's parameters to the object classes of its arguments.
/// The registry's manufacturer descriptor decides how many leading structural
/// words precede the constructor payload; the flow pass never names a
/// manufacturer keyword.
pub(super) fn emit_ctor_param_bindings(
    resolve_ctor_class: &impl Fn(&str) -> Option<String>,
    ctx: ScanContext,
    bindings: &mut Vec<Binding>,
) {
    let Some(tokens) = ctx.tokens else {
        return;
    };
    let Some((target, entry, payload_from)) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.constructor_entry(ctx.registry))
    else {
        return;
    };
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        return;
    };
    let Some(ctor_args) = effective.words.get(1 + payload_from..) else {
        return;
    };
    if ctor_args.is_empty() {
        return;
    }
    let Some(class) = resolve_ctor_class(&target.command) else {
        return;
    };
    let Some(constructors) = ctx.ctor_params.get(class.as_str()) else {
        return;
    };
    let mut matching = constructors.iter().filter(|(_, method)| {
        method
            .span
            .is_some_and(|span| span.start() == entry.declaration().offset)
            && method.body.executed_source.as_deref() == Some(entry.body())
            && method
                .params
                .iter()
                .map(String::as_str)
                .eq(entry.formals().iter().map(|(name, _)| name.as_str()))
    });
    let Some((ctor_unit, method)) = matching.next() else {
        return;
    };
    if matching.next().is_some() {
        return;
    }
    // The retained body entry chooses the unit. A document-final class name
    // cannot substitute a different constructor with equal parameter names.
    if method
        .span
        .is_none_or(|span| span.start() != entry.declaration().offset)
        || method.body.executed_source.as_deref() != Some(entry.body())
        || method
            .params
            .iter()
            .map(String::as_str)
            .ne(entry.formals().iter().map(|(name, _)| name.as_str()))
    {
        return;
    }
    for (i, arg) in ctor_args.iter().enumerate() {
        let Some((pname, _)) = entry.formals().get(i) else {
            break;
        };
        if pname == "args" {
            break;
        }
        if let Some(classes) = arg_classes(arg, ctx) {
            bindings.push(Binding {
                owner: BodyFrameId::Body(*ctor_unit),
                name: pname.clone(),
                union_classes: classes.union,
                scoped_classes: classes.scoped,
                kind: ObjectFlowEdge::CtorParam,
            });
        }
    }
}

/// The object classes an argument denotes: a tracked `$var` handle, or a direct
/// `[Class new]` registry constructor.  `None` when the argument is not a known
/// object.
///
/// A `$var` argument is resolved twice — blind for the union fact, and in
/// `ctx`'s own unit for the scope-keyed fact.  A literal constructor argument
/// is unit-independent, so both agree.  `by_scope` is a subset of `any_scope`
/// per name by construction, so an empty union implies an empty scoped set and
/// the blind lookup can gate both.
pub(super) fn arg_classes(arg: &WordExpr, ctx: ScanContext) -> Option<EdgeClasses> {
    if let Some(var) = deref_arg_var(arg, ctx)
        && let Some(classes) = ctx.out.get(var).filter(|s| !s.is_empty())
    {
        return Some(EdgeClasses {
            union: classes.clone(),
            scoped: ctx.scoped_classes(var),
        });
    }
    let parent = ctx.tokens?;
    let tokens = sole_substitution_tokens(arg, parent, ctx.registry)?;
    constructor_class_candidate(
        &tokens,
        ctx.registry,
        ctx.metadata_context,
        ctx.source_analysis,
    )
    .map(|class| EdgeClasses::unscoped(&class))
}

fn unique_original_value_word<'a>(
    tokens: &'a crate::ir::CommandTokens,
    compatibility: &str,
) -> Option<&'a WordExpr> {
    let mut matching = tokens
        .words()
        .iter()
        .skip(1)
        .filter(|word| word.legacy_text() == compatibility);
    let word = matching.next()?;
    matching.next().is_none().then_some(word)
}

pub(super) fn deref_arg_var<'a>(word: &'a WordExpr, ctx: ScanContext<'_>) -> Option<&'a str> {
    let (spelling, _) = word.sole_variable_substitution()?;
    let parent = ctx.tokens?;
    let config =
        parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(ctx.registry.profile()));
    let reference = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config).ok()??;
    if reference.index.is_some()
        || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
    {
        return None;
    }
    std::str::from_utf8(reference.name).ok()
}

fn literal_source_word(
    word: &WordExpr,
    parent: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
) -> Option<String> {
    let config =
        parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile()));
    let value = crate::registry_invocation::effective_invocation_word(
        word,
        config.escapes,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    );
    std::str::from_utf8(value.literal_bytes()?)
        .ok()
        .map(str::to_owned)
}
