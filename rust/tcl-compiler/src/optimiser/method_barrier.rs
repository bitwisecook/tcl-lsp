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

//! The **per-method** method-dispatch propagation barrier (issue #1164).
//!
//! `my` / `next` / object dispatch never names its callee, so the CFG
//! builder's per-call-site upvar widening cannot model what a method
//! dispatch does to the dispatching frame's private locals. The barrier
//! answers instead from whole-module evidence — but, unlike its
//! predecessor (`method_dispatch_evidence_is_incomplete`, a single
//! module-wide switch), it bars only the methods whose dispatches can
//! actually **reach** an invalidating fact:
//!
//! * A **bad** class is one defining a method (primary or retained
//!   replacement body — issue #1166) that can reach its caller's frame
//!   ([`crate::cfg_builder::upvar_info::reaches_caller_frame`]), or one the
//!   lowering flagged unanalysable
//!   ([`crate::ir::Module::oo_unanalysed_classes`]).
//! * Classes are grouped into **hierarchy components** — the connected
//!   components of the `superclass` / `mixin` relations the lowering
//!   captured ([`crate::ir::Module::class_relations`]). Within a
//!   component, `my` / `next` dispatch can (via the MRO of whatever the
//!   receiver's concrete class is, including subclasses overriding a
//!   name) land on any member class's method; across unrelated
//!   components it cannot — an object cannot carry two unrelated
//!   classes' methods unless some class relates them, and a module-local
//!   relating class is visible in `class_relations` (the same
//!   closed-world convention every other whole-module OO fact here
//!   already uses).
//! * A method's dispatches are classified from its statements: a
//!   registry head carrying the `TclOO` self-dispatch / next-chain traits
//!   targets the method's **own component**; a head naming a module
//!   class (`D new`, `D create …`) targets **that class's component**; a
//!   dynamic head (`$obj m`), an unresolvable literal head (an object
//!   command created at runtime, a `link`ed bareword), or a registry
//!   call that hands a command prefix to something else (`lsort
//!   -command …`) may dispatch **anywhere**. Plain calls to module procs
//!   inherit the callee proc's dispatch facts transitively — a proc that
//!   dispatches `$obj m` on the method's behalf reaches wherever the
//!   method itself could.
//! * Reachability closes over components: dispatching into a component
//!   also reaches everything that component's methods dispatch to.
//!
//! A method is **barred** iff some bad class is reachable that way (or a
//! dispatch may go anywhere while any bad class exists). A method that
//! never dispatches is never barred — nothing it calls can alias its
//! frame beyond what the per-call-site proc modelling already covers.
//!
//! Widening stays the governing rule where the evidence itself is
//! incomplete: a dynamic OO definition target
//! ([`crate::ir::OoDefinitionEvidence::dynamic_target`]) or a dynamic
//! `superclass` word ([`crate::ir::OoDefinitionEvidence::dynamic_class_relations`])
//! bars every method, exactly like the old module-wide switch.
//!
//! Known evidence limits (pre-existing, shared with the old gate): the
//! lowering models `oo::class` / `oo::define` block bodies only — an
//! `oo::objdefine` per-object method, or the single-member
//! `oo::define C method m {…} {…}` spelling, contributes no body here,
//! so a caller-frame reach hidden in one is invisible to both the old
//! and the new gate.

use std::collections::{HashMap, HashSet};

use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};

/// The computed barrier: which methods may propagate their private locals.
pub(super) struct MethodDispatchBarrier {
    bar_all: bool,
    barred: HashSet<String>,
}

impl MethodDispatchBarrier {
    /// Whether `method_qname`'s provably-local variables may propagate.
    pub(super) fn allows_locals(&self, method_qname: &str) -> bool {
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
pub(super) fn compute(cu: &CompilationUnit, registry: &CommandRegistry) -> MethodDispatchBarrier {
    let ir = &cu.ir_module;
    if ir.oo_evidence.dynamic_target || ir.oo_evidence.dynamic_class_relations {
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
    let proc_facts = proc_dispatch_facts(cu, registry, &comp_of);
    let method_facts = method_dispatch_facts(cu, registry, &comp_of, &proc_facts);
    let comp_out = component_dispatch_closure(cu, &comp_of, &method_facts);

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
    cu: &'a CompilationUnit,
    registry: &CommandRegistry,
    comp_of: &HashMap<String, usize>,
    proc_facts: &HashMap<&str, DispatchFacts>,
) -> HashMap<&'a str, DispatchFacts> {
    let ir = &cu.ir_module;
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
                &ScanEnv {
                    registry,
                    comp_of,
                    proc_facts,
                    procedures: &ir.procedures,
                    // TclOO method bodies resolve bare commands globally
                    // (object-namespace semantics).
                    namespace: "::",
                    own_comp,
                },
                &mut facts,
            );
        }
        method_facts.insert(qname.as_str(), facts);
    }
    method_facts
}

/// Component-level dispatch closure: dispatching into a component also
/// reaches everything its methods dispatch to.
fn component_dispatch_closure(
    cu: &CompilationUnit,
    comp_of: &HashMap<String, usize>,
    method_facts: &HashMap<&str, DispatchFacts>,
) -> Vec<DispatchFacts> {
    let ir = &cu.ir_module;
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
    cu: &'a CompilationUnit,
    registry: &CommandRegistry,
    comp_of: &HashMap<String, usize>,
) -> HashMap<&'a str, DispatchFacts> {
    let ir = &cu.ir_module;
    // Direct facts + direct proc callees per proc.
    let mut facts: HashMap<&str, DispatchFacts> = HashMap::new();
    let mut callees: HashMap<&str, HashSet<String>> = HashMap::new();
    for (qname, proc) in &ir.procedures {
        let namespace = super::helpers::naming::namespace_from_qualified(qname);
        let mut f = DispatchFacts::default();
        let mut c: HashSet<String> = HashSet::new();
        collect_proc_dispatches(
            &proc.body,
            registry,
            comp_of,
            &ir.procedures,
            &namespace,
            &mut f,
            &mut c,
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
    registry: &'a CommandRegistry,
    comp_of: &'a HashMap<String, usize>,
    proc_facts: &'a HashMap<&'a str, DispatchFacts>,
    procedures: &'a HashMap<String, crate::ir::Procedure>,
    namespace: &'a str,
    own_comp: Option<usize>,
}

/// Walk a method body's statements, classifying every `Call` head — see
/// the module doc for the classification. `Barrier` / `UpFrame`
/// statements are deliberately NOT dispatch sites here: SCCP widens every
/// tracked value at them already, so a dispatch hidden inside one cannot
/// invalidate a constant that survives it.
fn collect_dispatches(script: &Script, env: &ScanEnv<'_>, facts: &mut DispatchFacts) {
    walk_statements(script, &mut |stmt| {
        let Statement::Call { command, args, .. } = stmt else {
            return;
        };
        classify_head(command, args, env, facts);
    });
}

/// The proc-body variant of [`collect_dispatches`]: same head
/// classification, but proc-to-proc calls are collected as `callees` for
/// the caller-graph fixpoint instead of resolved inline, and there is no
/// own-component (a proc has no method frame; a `my` inside a proc body
/// is not callable there and classifies as unresolvable → anywhere).
fn collect_proc_dispatches(
    script: &Script,
    registry: &CommandRegistry,
    comp_of: &HashMap<String, usize>,
    procedures: &HashMap<String, crate::ir::Procedure>,
    namespace: &str,
    facts: &mut DispatchFacts,
    callees: &mut HashSet<String>,
) {
    walk_statements(script, &mut |stmt| {
        let Statement::Call { command, args, .. } = stmt else {
            return;
        };
        if facts.anywhere {
            return;
        }
        if crate::naming::is_dynamic_word(command) {
            facts.anywhere = true;
            return;
        }
        if let Some(comp) = class_head_component(command, namespace, comp_of) {
            facts.comps.insert(comp);
            return;
        }
        if let Some(qname) = resolve_proc(command, namespace, procedures) {
            callees.insert(qname);
            return;
        }
        classify_registry_or_unknown(command, args, registry, None, facts);
    });
}

/// Classify one method-body call head into `facts`.
fn classify_head(command: &str, args: &[String], env: &ScanEnv<'_>, facts: &mut DispatchFacts) {
    if facts.anywhere {
        return; // already saturated
    }
    if crate::naming::is_dynamic_word(command) {
        // `$obj m`, `[pick] m`, `${ns}::cmd` — the receiver is unbounded.
        facts.anywhere = true;
        return;
    }
    if let Some(comp) = class_head_component(command, env.namespace, env.comp_of) {
        // `D new` / `D create …` — dispatches into D's hierarchy
        // component (its constructor, and methods on the object made).
        facts.comps.insert(comp);
        return;
    }
    if let Some(qname) = resolve_proc(command, env.namespace, env.procedures) {
        // A module proc: inherit whatever it can dispatch to.
        if let Some(pf) = env.proc_facts.get(qname.as_str()) {
            facts.absorb(pf);
        }
        return;
    }
    classify_registry_or_unknown(command, args, env.registry, env.own_comp, facts);
}

/// The shared tail of head classification: a registry command with the
/// self-dispatch / next-chain traits targets the own component; one that
/// hands a command prefix onward may dispatch anywhere; any other
/// registry command dispatches nothing; an unknown literal head may be an
/// object command created at runtime (or a `link`ed bareword) — anywhere.
fn classify_registry_or_unknown(
    command: &str,
    args: &[String],
    registry: &CommandRegistry,
    own_comp: Option<usize>,
    facts: &mut DispatchFacts,
) {
    let bare = command.strip_prefix("::").unwrap_or(command);
    if let Some(spec) = registry.get(bare) {
        if spec
            .traits
            .intersects(Traits::TCLOO_SELF_DISPATCH | Traits::TCLOO_NEXT_CHAIN)
        {
            match own_comp {
                Some(c) => {
                    facts.comps.insert(c);
                }
                // `my` outside a known method frame — unbounded.
                None => facts.anywhere = true,
            }
            return;
        }
        // A callback the command will invoke later runs with unknown frame
        // relationships (`lsort -command cb …` invokes `cb` while the
        // enclosing frame is live) — the callback word may name any object
        // command, so widen.
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        if !registry
            .arg_indices_for_role(bare, &arg_refs, ArgRole::CommandPrefix)
            .is_empty()
        {
            facts.anywhere = true;
        }
        return;
    }
    // Not the registry's, not a module proc, not a module class: possibly
    // an object command (`D create obj; obj m`) or a `link`ed bareword.
    facts.anywhere = true;
}

/// Resolve a call head against the module's class set: `D` / `::D` /
/// `<ns>::D` written spellings, plus the conservative tail match (see
/// [`hierarchy_components`]). Returns the class's component.
fn class_head_component(
    command: &str,
    namespace: &str,
    comp_of: &HashMap<String, usize>,
) -> Option<usize> {
    if let Some(c) = comp_of.get(command) {
        return Some(*c);
    }
    let rooted = format!("::{}", command.strip_prefix("::").unwrap_or(command));
    if let Some(c) = comp_of.get(&rooted) {
        return Some(*c);
    }
    if !command.starts_with("::") {
        let ns = namespace.trim_end_matches("::");
        let qualified = format!("{ns}::{command}");
        if let Some(c) = comp_of.get(&qualified) {
            return Some(*c);
        }
    }
    let tail = command.rsplit("::").next().unwrap_or(command);
    comp_of
        .iter()
        .find(|(class, _)| class.rsplit("::").next() == Some(tail))
        .map(|(_, &c)| c)
}

/// Resolve a bare / qualified call head to a module proc qname the way
/// Tcl does (current namespace, then global — see
/// [`crate::naming::resolve_command_with`]).
fn resolve_proc(
    command: &str,
    namespace: &str,
    procedures: &HashMap<String, crate::ir::Procedure>,
) -> Option<String> {
    crate::naming::resolve_command_with::<&str, _>(namespace, &[], command, |qname| {
        procedures.contains_key(qname)
    })
}

/// Depth-capped statement walk over a script and every structured body it
/// carries, invoking `visit` on each statement.
fn walk_statements(script: &Script, visit: &mut dyn FnMut(&Statement)) {
    fn walk(script: &Script, visit: &mut dyn FnMut(&Statement), depth: u32) {
        if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            return;
        }
        for stmt in &script.statements {
            visit(stmt);
            match stmt {
                Statement::If {
                    clauses, else_body, ..
                } => {
                    for c in clauses {
                        walk(&c.body, visit, depth + 1);
                    }
                    if let Some(b) = else_body {
                        walk(b, visit, depth + 1);
                    }
                }
                Statement::For {
                    init, next, body, ..
                } => {
                    walk(init, visit, depth + 1);
                    walk(next, visit, depth + 1);
                    walk(body, visit, depth + 1);
                }
                Statement::While { body, .. }
                | Statement::Catch { body, .. }
                | Statement::Foreach { body, .. }
                | Statement::Block { body, .. } => walk(body, visit, depth + 1),
                Statement::Try {
                    body,
                    handlers,
                    finally_body,
                    ..
                } => {
                    walk(body, visit, depth + 1);
                    for h in handlers {
                        walk(&h.body, visit, depth + 1);
                    }
                    if let Some(fb) = finally_body {
                        walk(fb, visit, depth + 1);
                    }
                }
                Statement::Switch {
                    arms, default_body, ..
                } => {
                    for a in arms {
                        if let Some(b) = &a.body {
                            walk(b, visit, depth + 1);
                        }
                    }
                    if let Some(b) = default_body {
                        walk(b, visit, depth + 1);
                    }
                }
                _ => {}
            }
        }
    }
    walk(script, visit, 0);
}
