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

//! The trace fact owner: the trace registrations a statement may make, read
//! from the registry's `TraceTransition`, and the views consumers ask — the
//! per-point traced-name set, the module projection
//! (`Module::traced_variables` / `traced_commands`) and its name-key policy.

use std::collections::{BTreeSet, HashSet};
use std::hash::BuildHasher;

use tcl_registry::{CommandRegistry, StateTransition, TraceTarget, TraceTransition, Traits};

use crate::ir::{Module, Script, Statement};
use crate::registry_invocation::InvocationMetadataContext;

/// One trace registration (or removal) a statement may make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceRegistration {
    /// A variable trace on this literal target, as written.
    Variable(String),
    /// A variable trace on a computed or unknown target.
    AnyVariable,
    /// An execution trace on these command keys.
    Execution(Vec<String>),
    /// An execution trace on an unknown command.
    AnyExecution,
}

/// Whether a command with these traits may register a trace.
#[must_use]
pub const fn traits_may_register_trace(traits: Traits) -> bool {
    traits.contains(Traits::ESTABLISHES_VARIABLE_TRACE)
}

/// The traits that mark a trace registration, for consumers that mask
/// several observer hazards at once.
pub const TRACE_REGISTRATION_TRAITS: Traits = Traits::ESTABLISHES_VARIABLE_TRACE;

/// The variable registrations `stmt` may make. Unavailable metadata makes
/// every place and command possibly traced.
pub(crate) fn variable_trace_registrations(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> Vec<TraceRegistration> {
    let Some(possible) = stmt.tokens().and_then(|tokens| {
        crate::registry_invocation::possible_variable_trace_transitions_with_metadata_context(
            registry, context, tokens,
        )
    }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for transition in possible.transitions() {
        let (TraceTransition::Add { target, .. } | TraceTransition::Remove { target, .. }) =
            transition;
        if let TraceTarget::Variable(subject) = target {
            out.push(
                subject
                    .literal()
                    .map_or(TraceRegistration::AnyVariable, |name| {
                        TraceRegistration::Variable(name.to_owned())
                    }),
            );
        }
    }
    if possible.metadata_unavailable() {
        out.extend([
            TraceRegistration::AnyVariable,
            TraceRegistration::AnyExecution,
        ]);
    }
    out
}

/// The execution traces `stmt` may add.
fn execution_trace_registrations(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
) -> Vec<TraceRegistration> {
    let Some(invocation) =
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            registry,
            Some(context),
            stmt,
        )
    else {
        return Vec::new();
    };
    let Some(transitions) = invocation.facts.state_transitions.declared() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for fact in transitions.facts() {
        let StateTransition::Trace(TraceTransition::Add {
            target: TraceTarget::Execution(subject),
            ..
        }) = &fact.transition
        else {
            continue;
        };
        let keys = stmt.tokens().and_then(|tokens| {
            crate::registry_invocation::original_execution_trace_keys(
                registry, context, tokens, subject,
            )
        });
        out.push(keys.map_or(TraceRegistration::AnyExecution, |keys| {
            TraceRegistration::Execution(keys)
        }));
    }
    out
}

/// Every registration a call statement may make. Missing metadata closes
/// neither inventory.
fn statement_trace_registrations(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> Vec<TraceRegistration> {
    if !matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
        return Vec::new();
    }
    let Some(context) = context else {
        return vec![
            TraceRegistration::AnyVariable,
            TraceRegistration::AnyExecution,
        ];
    };
    let mut out = variable_trace_registrations(stmt, registry, Some(context));
    out.extend(execution_trace_registrations(stmt, registry, context));
    out
}

/// The variable names a literal trace may observe at one program point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TracedNames(HashSet<String>);

impl TracedNames {
    /// Add the literal variable registrations `stmt` makes. A computed
    /// target is left to the module projection's dynamic flag.
    pub(crate) fn record_statement(
        &mut self,
        stmt: &Statement,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
    ) {
        for registration in variable_trace_registrations(stmt, registry, context) {
            if let TraceRegistration::Variable(name) = registration {
                let name = crate::naming::normalise_var_name_braced(&name, true);
                self.0.insert(name.to_owned());
            }
        }
    }

    /// Union `other` in; whether anything was added.
    pub(crate) fn join(&mut self, other: &Self) -> bool {
        let before = self.0.len();
        self.0.extend(other.0.iter().cloned());
        self.0.len() != before
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.0
            .contains(crate::naming::normalise_var_name_braced(name, true))
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }
}

/// The module projection of every retained body's registrations.
#[derive(Default)]
struct ModuleTraceRegistrations {
    variables: BTreeSet<String>,
    commands: BTreeSet<String>,
    dynamic_variable: bool,
    dynamic: bool,
}

impl ModuleTraceRegistrations {
    fn record(&mut self, registration: TraceRegistration) {
        match registration {
            TraceRegistration::Variable(name) => {
                self.variables.insert(observation_key(&name).to_owned());
            }
            TraceRegistration::AnyVariable => self.dynamic_variable = true,
            TraceRegistration::Execution(keys) => self.commands.extend(keys),
            TraceRegistration::AnyExecution => self.dynamic = true,
        }
    }

    fn walk(
        &mut self,
        script: &Script,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
    ) {
        crate::ir::for_each_statement(script, &mut |stmt| {
            for registration in statement_trace_registrations(stmt, registry, context) {
                self.record(registration);
            }
        });
    }
}

/// Project every retained body's registrations onto
/// `Module::traced_variables`, `traced_commands`,
/// `has_dynamic_variable_trace` and `has_dynamic_trace`: the top level,
/// procedures, `namespace eval` / `apply` bodies and `TclOO` methods.
pub(crate) fn populate_trace_facts(
    module: &mut Module,
    registry: &CommandRegistry,
    standalone: bool,
) {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context = InvocationMetadataContext::for_module(registry, module).or_else(|| {
        (standalone && module.source_metadata_input.is_none())
            .then(|| {
                registry
                    .profile()
                    .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                    .map(InvocationMetadataContext::from)
            })
            .flatten()
    });
    // Borrow every retained body: a derived `Clone` of cap-deep structured IR
    // overflows a 2 MiB thread.
    let mut facts = ModuleTraceRegistrations::default();
    facts.walk(&module.top_level, registry, context);
    for body in module
        .procedures
        .values()
        .map(|p| &p.body)
        .chain(module.body_lowerings().map(|(_, p)| &p.body))
        .chain(module.methods.values().map(|m| &m.body))
    {
        facts.walk(body, registry, context);
    }
    module.traced_variables.extend(facts.variables);
    module.traced_commands.extend(facts.commands);
    module.has_dynamic_variable_trace |= facts.dynamic_variable;
    module.has_dynamic_trace |= facts.dynamic;
}

/// Whether a module trace may observe the variable `name`.
#[must_use]
pub fn variable_may_be_traced(traced: &BTreeSet<String>, dynamic: bool, name: &str) -> bool {
    name_may_be_observed(traced, name, dynamic, ObservationPolicy::Module)
}

/// Whether a module execution trace may fire on a call written `command`.
#[must_use]
pub fn command_may_be_traced(traced: &BTreeSet<String>, dynamic: bool, command: &str) -> bool {
    if dynamic || traced.contains(command) {
        return true;
    }
    let written = tcl_syntax::naming::canonical_written_command(command);
    traced.contains(tcl_syntax::naming::unroot_rooted_key(&written).unwrap_or(&written))
}

/// Readonly literal name membership, independent of storage ordering. It
/// supplies no interpreted lookup or variable-cell identity.
pub trait ObservedNames {
    /// Whether the inventory holds exactly `name`.
    fn contains_name(&self, name: &str) -> bool;
}

impl ObservedNames for BTreeSet<String> {
    fn contains_name(&self, name: &str) -> bool {
        self.contains(name)
    }
}

impl<S: BuildHasher> ObservedNames for HashSet<String, S> {
    fn contains_name(&self, name: &str) -> bool {
        self.contains(name)
    }
}

/// The text policy of an observation inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationPolicy {
    /// Module trace and deferred-write inventories hold root-free keys.
    Module,
    /// Mixed source guards hold exact keys as well as module keys.
    MixedSource,
    /// A native-lowering ledger holds exact, one-root and rooted aliases.
    NativeLoweringLedger,
}

/// The aliases a conditional write summary records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteNamePolicy {
    /// Global callback writes keep root-free names and array roots.
    DeferredGlobal,
    /// Outer-frame summaries keep the original and one-root-free spelling.
    OuterScope,
}

/// A module observation key, never an interpreted name or resolved cell.
/// Literal sigils, braces, interior colon runs and a lone colon stay data.
#[must_use]
pub fn observation_key(mut name: &str) -> &str {
    while let Some(unrooted) = tcl_syntax::naming::unroot_rooted_key(name) {
        name = unrooted;
    }
    name
}

/// Whether this inventory may observe `name`. Unknown input observes every
/// name; a false result proves no native absence or callback-free state.
#[must_use]
pub fn name_may_be_observed(
    names: &impl ObservedNames,
    name: &str,
    unknown: bool,
    policy: ObservationPolicy,
) -> bool {
    if unknown {
        return true;
    }
    match policy {
        ObservationPolicy::Module => names.contains_name(observation_key(name)),
        ObservationPolicy::MixedSource => {
            names.contains_name(name) || names.contains_name(observation_key(name))
        }
        ObservationPolicy::NativeLoweringLedger => {
            names.contains_name(name)
                || tcl_syntax::naming::unroot_rooted_key(name)
                    .is_some_and(|unrooted| names.contains_name(unrooted))
                || names.contains_name(&tcl_syntax::naming::root_unrooted_key(name))
        }
    }
}

/// Record the names a possible write may reach under `policy`. The empty
/// name stays possible; aliases do not identify cells.
pub fn record_possible_write_name(
    names: &mut BTreeSet<String>,
    name: &str,
    policy: WriteNamePolicy,
) {
    let unrooted = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
    match policy {
        WriteNamePolicy::DeferredGlobal => {
            names.insert(unrooted.to_owned());
            if let Some((array, _)) = tcl_syntax::naming::split_element_ref(unrooted) {
                names.insert(array.to_owned());
            }
        }
        WriteNamePolicy::OuterScope => {
            if unrooted != name {
                names.insert(unrooted.to_owned());
            }
            names.insert(name.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn module_protection_keeps_namespace_and_literal_name_data() {
        let protected = names(&["n::x", ":x", "$x", "", "{x}"]);
        for name in ["::n::x", "::::n::x", ":::x", "$x", "", "{x}"] {
            assert!(
                name_may_be_observed(&protected, name, false, ObservationPolicy::Module),
                "{name:?}"
            );
            assert!(variable_may_be_traced(&protected, false, name), "{name:?}");
        }
        for name in ["::other::x", "x", "${x}", "n:::x"] {
            assert!(
                !name_may_be_observed(&protected, name, false, ObservationPolicy::Module),
                "{name:?}"
            );
            assert!(!variable_may_be_traced(&protected, false, name), "{name:?}");
            assert!(variable_may_be_traced(&protected, true, name), "{name:?}");
        }
    }

    #[test]
    fn native_lowering_ledger_does_not_borrow_module_root_pair_policy() {
        let protected = names(&["x", "::n::x"]);
        for name in ["x", "::x", "n::x", "::n::x"] {
            assert!(name_may_be_observed(
                &protected,
                name,
                false,
                ObservationPolicy::NativeLoweringLedger
            ));
        }
        assert!(!name_may_be_observed(
            &protected,
            "::::x",
            false,
            ObservationPolicy::NativeLoweringLedger
        ));
        assert!(name_may_be_observed(
            &protected,
            "::::x",
            false,
            ObservationPolicy::Module
        ));
    }

    #[test]
    fn deferred_array_and_empty_writes_remain_possible_without_reference_decoration() {
        let mut deferred = BTreeSet::new();
        for name in ["::arr(key)", ":::x", "", "$x", "arr(open"] {
            record_possible_write_name(&mut deferred, name, WriteNamePolicy::DeferredGlobal);
        }
        assert_eq!(
            deferred,
            names(&["arr(key)", "arr", ":x", "", "$x", "arr(open"])
        );
        assert!(!deferred.contains("x"));
        let mut outer = BTreeSet::new();
        record_possible_write_name(&mut outer, "::n::arr(key)", WriteNamePolicy::OuterScope);
        assert_eq!(outer, names(&["::n::arr(key)", "n::arr(key)"]));
        assert!(!outer.contains("n::arr"));
        let mut rooted_empty = BTreeSet::new();
        for name in ["::", ":::"] {
            record_possible_write_name(&mut rooted_empty, name, WriteNamePolicy::OuterScope);
        }
        assert_eq!(rooted_empty, names(&["::", "", ":::", ":"]));
        assert!(!rooted_empty.contains("n"));
    }

    #[test]
    fn equal_text_reports_and_unknown_state_supply_no_cell_identity() {
        use crate::command_binding::SourceNamespaceKey;
        use crate::var_resolve::{VariableCellKey, VariableCellSet};
        let protected = names(&["x"]);
        let left = VariableCellKey::Namespace {
            identity: SourceNamespaceKey::authored("::left"),
            simple: tcl_core_types::NameBytes::from("x"),
        };
        let right = VariableCellKey::Namespace {
            identity: SourceNamespaceKey::authored("::right"),
            simple: tcl_core_types::NameBytes::from("x"),
        };
        let mut cells = VariableCellSet::default();
        cells.insert(left.clone());
        assert_ne!(left, right);
        assert!(cells.contains(&left));
        assert!(!cells.contains(&right));
        // The shared source label protects both reports without merging cells.
        assert!(name_may_be_observed(
            &protected,
            "x",
            false,
            ObservationPolicy::Module
        ));
        for name in ["::left::x", "::right::x", "other(key)", "", "$x"] {
            assert!(name_may_be_observed(
                &BTreeSet::new(),
                name,
                true,
                ObservationPolicy::Module
            ));
        }
    }

    #[test]
    fn command_traces_match_rooted_and_written_spellings() {
        let traced = names(&["foo", "ns::bar"]);
        for command in ["foo", "::foo", "::ns::bar", "ns::bar"] {
            assert!(command_may_be_traced(&traced, false, command), "{command}");
        }
        assert!(!command_may_be_traced(&traced, false, "baz"));
        assert!(command_may_be_traced(&BTreeSet::new(), true, "baz"));
    }

    #[test]
    fn trace_registration_traits_come_from_the_registry_trace_subcommands() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6")
            .commands()
            .clone();
        let trace = registry.get("trace").expect("trace is core");
        for sub in ["add", "remove", "variable", "vdelete"] {
            let sub = trace
                .subcommands
                .iter()
                .find(|candidate| candidate.name == sub)
                .expect("trace subcommand");
            assert!(traits_may_register_trace(sub.traits), "{}", sub.name);
        }
        assert!(!traits_may_register_trace(Traits::empty()));
    }
}
