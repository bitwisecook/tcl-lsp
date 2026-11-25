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

//! Evaluating the graph: strict and declared states, their transfer and the observations recorded.

use super::{
    Builder, DeclarationFlowPurpose, DeclarationFlowReport, DeclaredOverwrite, Node, State,
    representation,
};
#[cfg(test)]
use crate::command_binding::declaration_flow::ConditionalRepresentationBodyContext;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

impl State {
    /// Whether `name` is defined, or is an array an element store made
    /// exist: an authored read of `$c(x)` names the array `c`, and
    /// `set c(x) 1` defines `c(x)`.
    fn defines_array_or_name(&self, name: &str) -> bool {
        self.defined.contains(name)
            || self.defined.iter().any(|defined| {
                tcl_syntax::naming::split_element_ref(defined)
                    .is_some_and(|(array, _)| array == name)
            })
    }

    pub(super) fn join(&mut self, other: &Self) -> bool {
        let old = self.clone();
        self.defined.retain(|name| other.defined.contains(name));
        self.uncertain_writes
            .extend(other.uncertain_writes.iter().cloned());
        self.incoming.retain(|name| other.incoming.contains(name));
        self.aliases.extend(other.aliases.iter().cloned());
        self.caller_frame_aliases
            .retain(|name| other.caller_frame_aliases.contains(name));
        self.unknown_writers |= other.unknown_writers;
        self.literals
            .retain(|name, value| other.literals.get(name) == Some(value));
        *self != old
    }
}

impl Builder<'_> {
    #[cfg(test)]
    fn conditional_list_sites(&self) -> (BTreeSet<u32>, BTreeSet<u32>) {
        let mut appends = BTreeSet::new();
        let mut indices = BTreeSet::new();
        for event in self
            .nodes
            .iter()
            .flat_map(|node| &node.representation_events)
        {
            match event {
                representation::Event::ListAppend { site, .. } => {
                    appends.insert(*site);
                }
                representation::Event::ListIndex {
                    site,
                    literal: Some(_),
                    ..
                } => {
                    indices.insert(*site);
                }
                _ => {}
            }
        }
        (appends, indices)
    }

    #[cfg(test)]
    pub(super) fn conditional_substitution_entries(&self) -> BTreeSet<u32> {
        self.nodes
            .iter()
            .flat_map(|node| node.representation_events.windows(3))
            .filter_map(|events| match events {
                [
                    representation::Event::RequireBodyContext { context: required },
                    representation::Event::BodyContext { context: entered },
                    representation::Event::ClearResult,
                ] => {
                    let substitution = entered.substitution.as_ref()?;
                    Arc::ptr_eq(required, &substitution.parent)
                        .then_some(substitution.original.command().span.start())
                }
                _ => None,
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn conditional_body_contexts(
        &self,
    ) -> Vec<Arc<ConditionalRepresentationBodyContext>> {
        self.nodes
            .iter()
            .flat_map(|node| &node.representation_events)
            .filter_map(|event| match event {
                representation::Event::BodyContext { context } => Some(Arc::clone(context)),
                _ => None,
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn conditional_loop_exits(&self) -> Vec<(u32, usize, usize, usize, usize)> {
        self.representation_loops
            .iter()
            .map(|region| {
                (
                    region.site,
                    region.normal_exit,
                    region.error_exit,
                    region.returned_exit,
                    region.other_exit,
                )
            })
            .collect()
    }

    pub(super) fn evaluate(self, entry: usize) -> DeclarationFlowReport {
        let source = Arc::clone(self.entry.origin());
        #[cfg(test)]
        let conditional_list_events = if self.purpose == DeclarationFlowPurpose::Representation {
            representation::inspect_list_events(&self, entry)
        } else {
            Vec::new()
        };
        #[cfg(test)]
        let (conditional_list_append_sites, conditional_literal_index_sites) =
            self.conditional_list_sites();
        let representation_occurrences = if self.purpose == DeclarationFlowPurpose::Representation {
            representation::project(&self, entry)
        } else {
            Vec::new()
        };
        let mut report = DeclarationFlowReport {
            source,
            representation_occurrences,
            reads: BTreeMap::new(),
            reached: BTreeSet::new(),
            represented: self.nodes.iter().filter_map(|node| node.site).collect(),
            #[cfg(test)]
            conditional_handler_sites: self
                .nodes
                .iter()
                .filter_map(|node| node.handler_site)
                .collect(),
            #[cfg(test)]
            conditional_loop_sites: self
                .representation_loops
                .iter()
                .map(|region| region.site)
                .collect(),
            #[cfg(test)]
            conditional_list_append_sites,
            #[cfg(test)]
            conditional_literal_index_sites,
            #[cfg(test)]
            conditional_list_events,
            #[cfg(test)]
            conditional_substitution_entries: self.conditional_substitution_entries(),
            #[cfg(test)]
            conditional_body_contexts: self.conditional_body_contexts(),
            #[cfg(test)]
            conditional_loop_exits: self.conditional_loop_exits(),
            unavailable_before: BTreeSet::new(),
            unavailable: false,
            unknown_reads: false,
            all_reads: BTreeSet::new(),
            authored_reads: BTreeMap::new(),
            warning_reads: BTreeMap::new(),
            conditional_overwrites: Vec::new(),
            declared_defined_reads: BTreeMap::new(),
            authored_uses: BTreeMap::new(),
            named_uses: BTreeSet::new(),
            quoted_uses: BTreeSet::new(),
            quoted_use_unavailable: false,
            declared_entries: BTreeSet::new(),
            conditional_handler_literals: BTreeMap::new(),
            conditional_argument_incoming: BTreeMap::new(),
            conditional_caller_alias_prefixes: BTreeSet::new(),
            aliases: BTreeSet::new(),
            stores: BTreeSet::new(),
            frame: self.entry.clone(),
        };
        let states = self.strict_states(entry);
        self.record_strict_observations(&mut report, &states);
        let declared = self.declared_states(entry, false, false);
        let accumulator_paths = self.declared_states(entry, true, false);
        let warning_paths = self.declared_states(entry, false, true);
        let accumulators: BTreeSet<String> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(id, _)| accumulator_paths[*id].is_some())
            .flat_map(|(_, node)| node.accumulator_writes.iter().cloned())
            .collect();
        self.record_declared_observations(
            &mut report,
            &declared,
            &accumulator_paths,
            &accumulators,
        );
        self.record_warning_observations(&mut report, &warning_paths);
        #[cfg(debug_assertions)]
        self.trace_rbs_report(&report, &warning_paths);
        report
    }

    #[cfg(debug_assertions)]
    fn trace_rbs_report(&self, report: &DeclarationFlowReport, warning_paths: &[Option<State>]) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
            eprintln!(
                "RBS_GRAPH stage=report nodes={} warnings={} strict_reads={} unavailable={} unknown_reads={}",
                self.nodes.len(),
                report.warning_reads.len(),
                report.reads.len(),
                report.unavailable,
                report.unknown_reads,
            );
            for (id, node) in self.nodes.iter().enumerate() {
                let reached = warning_paths[id].is_some();
                let (unknown, defined, uncertain, aliases) =
                    warning_paths[id]
                        .as_ref()
                        .map_or((false, 0, 0, 0), |state| {
                            (
                                state.unknown_writers,
                                state.defined.len(),
                                state.uncertain_writes.len(),
                                state.aliases.len(),
                            )
                        });
                eprintln!(
                    "RBS_GRAPH stage=node id={id} site={} site_set={} handler={} handler_set={} unknown={} prefix_unavailable={} missing_read={} reads={} writes={} successors={} reached={reached} state_unknown={unknown} defined={defined} uncertain={uncertain} aliases={aliases}",
                    node.site.unwrap_or(0),
                    node.site.is_some(),
                    node.handler_site.unwrap_or(0),
                    node.handler_site.is_some(),
                    node.unknown,
                    node.availability.unavailable_declared_prefix,
                    node.declared_read_unavailable,
                    node.authored_reads.len(),
                    node.writes.len(),
                    node.successors.len(),
                );
                for successor in &node.successors {
                    eprintln!("RBS_GRAPH stage=edge id={id} next={successor}");
                }
            }
        }
    }

    fn record_warning_observations(
        &self,
        report: &mut DeclarationFlowReport,
        warning_paths: &[Option<State>],
    ) {
        for (id, store) in &self.conditional_overwrites {
            if !report.unknown_reads
                && !report.aliases.contains(&store.name)
                && warning_paths[*id].as_ref().is_some_and(|state| {
                    !state.unknown_writers
                        && !state.uncertain_writes.contains(&store.name)
                        && !state.aliases.contains(&store.name)
                })
            {
                report.conditional_overwrites.push(DeclaredOverwrite {
                    first: store.first.clone(),
                    next: store.next.clone(),
                    name: store.name.clone(),
                    target: store.target,
                });
            }
        }
        for (id, node) in self.nodes.iter().enumerate() {
            let Some(state) = &warning_paths[id] else {
                continue;
            };
            // naming.compiler.original-formal-argument-entry
            // docs/design/analysis/name-resolution-proofs/original-formal-argument-entry.md
            if let Some(site) = node.site
                && !state.unknown_writers
            {
                report
                    .conditional_argument_incoming
                    .insert(site, state.incoming.clone());
                if state.aliases.is_subset(&state.caller_frame_aliases) {
                    report.conditional_caller_alias_prefixes.insert(site);
                }
            }
            if let Some(site) = node.handler_site
                && !state.unknown_writers
            {
                let literals = state
                    .literals
                    .iter()
                    .filter(|(name, _)| !state.aliases.contains(*name))
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect();
                report.conditional_handler_literals.insert(site, literals);
            }
            for (name, span) in &node.authored_reads {
                if !state.defines_array_or_name(name)
                    && !state.uncertain_writes.contains(name)
                    && !state.aliases.contains(name)
                {
                    report
                        .warning_reads
                        .entry(name.clone())
                        .and_modify(|earlier| {
                            if span.start() < earlier.start() {
                                *earlier = *span;
                            }
                        })
                        .or_insert(*span);
                }
            }
        }
    }

    fn initial_state(&self) -> State {
        let names = self.entry.original_formal_topology().map_or_else(
            || {
                self.entry
                    .parameters()
                    .iter()
                    .map(|formal| formal.name.clone())
                    .collect::<BTreeSet<_>>()
            },
            |topology| {
                topology
                    .parameters()
                    .iter()
                    .filter_map(|formal| std::str::from_utf8(&formal.name).ok().map(str::to_owned))
                    .collect::<BTreeSet<_>>()
            },
        );
        State {
            incoming: names.clone(),
            defined: names,
            ..State::default()
        }
    }

    fn strict_states(&self, entry: usize) -> Vec<Option<State>> {
        let mut states = vec![None; self.nodes.len()];
        states[entry] = Some(self.initial_state());
        let mut queue = VecDeque::from([entry]);
        while let Some(id) = queue.pop_front() {
            let node = &self.nodes[id];
            let mut state = states[id]
                .as_ref()
                .expect("queued diagnostic state")
                .clone();
            state.unknown_writers |= node.unknown;
            if node.unknown {
                state.incoming.clear();
            }
            for name in node
                .removes
                .iter()
                .chain(&node.writes)
                .chain(&node.possible_output_writes)
                .chain(&node.aliases)
            {
                state.incoming.remove(name);
            }
            state.aliases.extend(node.aliases.iter().cloned());
            for name in &node.removes {
                state.defined.remove(name);
            }
            state.defined.extend(node.writes.iter().cloned());
            Self::transfer_uncertain_writes(node, &mut state);
            for &successor in &node.successors {
                let changed = if let Some(existing) = &mut states[successor] {
                    existing.join(&state)
                } else {
                    states[successor] = Some(state.clone());
                    true
                };
                if changed {
                    queue.push_back(successor);
                }
            }
        }
        states
    }

    fn record_strict_observations(
        &self,
        report: &mut DeclarationFlowReport,
        states: &[Option<State>],
    ) {
        for (id, node) in self.nodes.iter().enumerate() {
            let Some(state) = &states[id] else {
                continue;
            };
            let source_catalogue_only = node.availability.schema_origin
                == crate::registry_invocation::DeclarationSchemaOrigin::OriginalSourceCatalogue;
            report.unavailable |= node.unknown || source_catalogue_only;
            if let Some(site) = node.site {
                report.reached.insert(site);
                if state.unknown_writers || source_catalogue_only {
                    report.unavailable_before.insert(site);
                }
            }
            for name in &node.named_reads {
                report.authored_uses.entry(name.clone()).or_default();
                report.named_uses.insert(name.clone());
            }
            report.unknown_reads |= node.availability.unknown_reads;
            report.all_reads.extend(node.named_reads.iter().cloned());
            report.all_reads.extend(node.name_queries.iter().cloned());
            report.named_uses.extend(node.name_queries.iter().cloned());
            report
                .all_reads
                .extend(node.reads.iter().map(|(name, _)| name.clone()));
            report.aliases.extend(node.aliases.iter().cloned());
            if let Some(site) = node.store_site {
                report
                    .stores
                    .extend(node.writes.iter().map(|name| (site, name.clone())));
            }
            for (name, span) in &node.reads {
                if !state.unknown_writers
                    && !state.defined.contains(name)
                    && !state.uncertain_writes.contains(name)
                    && !state.aliases.contains(name)
                {
                    report
                        .reads
                        .entry(name.clone())
                        .and_modify(|old| {
                            if span.start() < old.start() {
                                *old = *span;
                            }
                        })
                        .or_insert(*span);
                }
            }
        }
    }

    fn record_declared_observations(
        &self,
        report: &mut DeclarationFlowReport,
        declared: &[Option<State>],
        accumulator_paths: &[Option<State>],
        accumulators: &BTreeSet<String>,
    ) {
        for (id, node) in self.nodes.iter().enumerate() {
            let Some(state) = &declared[id] else {
                continue;
            };
            report
                .quoted_uses
                .extend(node.quoted_mentions.iter().cloned());
            report.quoted_use_unavailable |= node.availability.quoted_use_unavailable;
            // Original argument entry precedes this command's dispatch.
            // An unavailable handler layout cannot erase that candidate entry
            // or grant successful dispatch to any consumer.
            if let Some(site) = node.site {
                report.declared_entries.insert(site);
            }
            for (name, span) in &node.authored_reads {
                report
                    .authored_uses
                    .entry(name.clone())
                    .or_default()
                    .push(*span);
                let accumulator_defined = accumulators.contains(name)
                    && accumulator_paths[id]
                        .as_ref()
                        .is_some_and(|state| state.defined.contains(name));
                if !state.unknown_writers
                    && !state.uncertain_writes.contains(name)
                    && (state.defined.contains(name) || accumulator_defined)
                {
                    report
                        .declared_defined_reads
                        .entry(name.clone())
                        .or_default()
                        .push(*span);
                }
                if !state.defines_array_or_name(name)
                    && !accumulator_defined
                    && !state.aliases.contains(name)
                {
                    report
                        .authored_reads
                        .entry(name.clone())
                        .and_modify(|old| {
                            if span.start() < old.start() {
                                *old = *span;
                            }
                        })
                        .or_insert(*span);
                }
            }
        }
    }

    fn declared_states(
        &self,
        entry: usize,
        assumes_accumulator_iterations: bool,
        requires_available_prefix: bool,
    ) -> Vec<Option<State>> {
        let mut states = vec![None; self.nodes.len()];
        states[entry] = Some(self.initial_state());
        let policy = crate::tcl_expr_eval::FoldPolicy::from_octal(None)
            .with_invocation_dialect(self.dialect);
        let mut queue = VecDeque::from([entry]);
        while let Some(id) = queue.pop_front() {
            let node = &self.nodes[id];
            let mut state = states[id].as_ref().expect("queued declared state").clone();
            Self::transfer_declared_state(node, &mut state, policy);
            if requires_available_prefix
                && (state.unknown_writers || (node.unknown && !node.declared_read_unavailable))
            {
                continue;
            }
            let successors =
                Self::declared_successors(node, &state, policy, assumes_accumulator_iterations);
            for successor in successors {
                let changed = if let Some(existing) = &mut states[successor] {
                    existing.join(&state)
                } else {
                    states[successor] = Some(state.clone());
                    true
                };
                if changed {
                    queue.push_back(successor);
                }
            }
        }
        states
    }

    fn transfer_declared_state(
        node: &Node,
        state: &mut State,
        policy: crate::tcl_expr_eval::FoldPolicy,
    ) {
        let previous = state.literals.clone();
        state.unknown_writers |= node.availability.unavailable_declared_prefix;
        if node.unknown && !node.declared_read_unavailable {
            state.literals.clear();
            state.incoming.clear();
        }
        if state.unknown_writers {
            state.incoming.clear();
        }
        for name in node
            .removes
            .iter()
            .chain(&node.writes)
            .chain(&node.possible_output_writes)
            .chain(&node.aliases)
        {
            state.incoming.remove(name);
        }
        for name in node.removes.iter().chain(&node.aliases) {
            state.caller_frame_aliases.remove(name);
        }
        state
            .caller_frame_aliases
            .extend(node.caller_frame_aliases.iter().cloned());
        state.aliases.extend(node.aliases.iter().cloned());
        // A possible output write (`dict unset d a`, `lappend l x`) leaves a
        // value no earlier literal states: `set d {a 1}; dict unset d a`
        // holds no `a` (#2246).
        for name in node
            .removes
            .iter()
            .chain(&node.writes)
            .chain(&node.possible_output_writes)
            .chain(&node.aliases)
        {
            state.literals.remove(name);
        }
        for name in &node.removes {
            state.defined.remove(name);
        }
        state.defined.extend(node.writes.iter().cloned());
        state
            .defined
            .extend(node.possible_output_writes.iter().cloned());
        for (name, value) in &node.literal_stores {
            if !tcl_syntax::naming::is_qualified(name.as_bytes())
                && !name.contains('(')
                && !state.aliases.contains(name)
            {
                state.literals.insert(name.clone(), value.clone());
            }
        }
        for (name, amount) in &node.increments {
            let value = previous.get(name).and_then(|value| {
                let crate::tcl_expr_eval::TclValue::Int(old) =
                    crate::tcl_expr_eval::parse_integer_operand_with_policy(value, policy)?
                else {
                    return None;
                };
                let crate::tcl_expr_eval::TclValue::Int(amount) =
                    crate::tcl_expr_eval::parse_integer_operand_with_policy(amount, policy)?
                else {
                    return None;
                };
                old.checked_add(amount).map(|value| value.to_string())
            });
            if let Some(value) = value {
                state.literals.insert(name.clone(), value);
            }
        }
        Self::transfer_uncertain_writes(node, state);
    }

    fn transfer_uncertain_writes(node: &Node, state: &mut State) {
        for name in node.writes.iter().chain(&node.removes).chain(&node.aliases) {
            state.uncertain_writes.remove(name);
        }
        for name in &node.uncertain_writes {
            state.uncertain_writes.insert(name.clone());
            state.defined.remove(name);
            state.incoming.remove(name);
            state.literals.remove(name);
        }
    }

    pub(super) fn declared_successors(
        node: &Node,
        state: &State,
        policy: crate::tcl_expr_eval::FoldPolicy,
        assumes_accumulator_iterations: bool,
    ) -> Vec<usize> {
        if let Some(condition) = &node.condition {
            let env = state
                .literals
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        crate::tcl_expr_eval::EnvValue::text(value.as_str()),
                    )
                })
                .collect();
            match crate::tcl_expr_eval::eval_tcl_expr_with_policy(
                &condition.expression,
                &env,
                policy,
            )
            .map(|value| value.is_truthy())
            {
                Some(true) => vec![condition.yes],
                Some(false) => vec![condition.no],
                None if condition.first_iteration => vec![condition.yes],
                None => node.successors.clone(),
            }
        } else if assumes_accumulator_iterations && node.unknown_list_first.is_some() {
            // The accumulator diagnostic states the condition that each
            // enclosing unknown list loop iterates; runtime zero-trip
            // paths remain in the strict graph and ordinary read advice.
            node.unknown_list_first.into_iter().collect()
        } else {
            node.declared_successors
                .as_ref()
                .unwrap_or(&node.successors)
                .clone()
        }
    }
}
