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

//! Actual post-argv variable reads, independently of lexical body containment.

use super::{
    Arc, CommandAllocationSite, Hash, InvocationFacts, ModuleCommandBindings,
    SourceCommandBindings, SourceExecutionContext, SourceInvocationBinding, SourceOriginId,
    SourceOutcomes, SourceVariableAccess, SourceVariableEvaluationOwner,
};

/// Unenumerated read effects in an actual invocation's execution subtree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SourceVariableReadResidual {
    /// Every reached variable read has a retained physical access.
    Closed,
    /// A callback, opaque body or unresolved access may read additional cells.
    #[default]
    Unknown,
}

/// A native named-variable access at its actual physical read phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInvocationVariableRead {
    /// Selected physical address, including any bounded unknown-address envelope.
    pub place: crate::place::Place,
    /// Variable world at that read, before its own observer callback.
    pub variable_context: Arc<crate::var_resolve::ResolveContext>,
    context_fingerprint: u64,
}

impl Hash for SourceInvocationVariableRead {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.place.hash(state);
        self.context_fingerprint.hash(state);
    }
}

/// May-read dependencies made after original argv evaluation.
/// Recorded accesses do not imply that every body statement executes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SourceInvocationVariableReads {
    /// Reached source substitutions in called bodies and nested evaluations.
    pub substitutions: Vec<SourceVariableAccess>,
    /// Reads through selected native variable-name operands.
    pub native_reads: Vec<SourceInvocationVariableRead>,
    /// Additional effects which the recorded accesses cannot enumerate.
    pub residual: SourceVariableReadResidual,
}

impl SourceInvocationVariableReads {
    fn join(&mut self, other: &Self) {
        for access in &other.substitutions {
            if !self.substitutions.contains(access) {
                self.substitutions.push(access.clone());
            }
        }
        for access in &other.native_reads {
            if !self.native_reads.contains(access) {
                self.native_reads.push(access.clone());
            }
        }
        if other.residual == SourceVariableReadResidual::Unknown {
            self.residual = SourceVariableReadResidual::Unknown;
        }
    }

    fn relocated(&self, relocation: &crate::var_resolve::VariableProofRelocation) -> Self {
        Self {
            substitutions: self
                .substitutions
                .iter()
                .map(|read| read.relocated_variables(relocation))
                .collect(),
            native_reads: self
                .native_reads
                .iter()
                .map(|read| {
                    let context = Arc::new(read.variable_context.relocated(relocation));
                    SourceInvocationVariableRead {
                        place: relocation.place(&read.place),
                        context_fingerprint: SourceVariableAccess::fingerprint(&context),
                        variable_context: context,
                    }
                })
                .collect(),
            residual: self.residual,
        }
    }
}

impl SourceVariableEvaluationOwner {
    fn for_each_body(&self, visit: &mut impl FnMut(&CommandAllocationSite)) {
        match self {
            Self::InvocationBody { invocation, parent } => {
                visit(invocation);
                if let Some(parent) = parent {
                    parent.for_each_body(visit);
                }
            }
            Self::InvocationArguments { parent, .. } | Self::NativeExpression { parent, .. } => {
                if let Some(parent) = parent {
                    parent.for_each_body(visit);
                }
            }
            Self::Alternatives(owners) => {
                for owner in owners {
                    owner.for_each_body(visit);
                }
            }
            Self::Unspecified => {}
        }
    }
}

impl SourceInvocationBinding {
    pub(super) fn join_invocation_reads(&mut self, other: &Self) {
        match (
            &mut self.invocation_variable_reads,
            &other.invocation_variable_reads,
        ) {
            (Some(left), Some(right)) => Arc::make_mut(left).join(right),
            (Some(left), None) => {
                Arc::make_mut(left).residual = SourceVariableReadResidual::Unknown;
            }
            (left @ None, Some(right)) => {
                let mut reads = right.as_ref().clone();
                reads.residual = SourceVariableReadResidual::Unknown;
                *left = Some(Arc::new(reads));
            }
            (None, None) => {}
        }
    }

    pub(super) fn relocate_invocation_reads(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        if let Some(reads) = &mut self.invocation_variable_reads {
            *reads = Arc::new(reads.relocated(relocation));
        }
    }
}

impl SourceCommandBindings {
    /// Retained execution reads for this exact caller source instance and site.
    /// Missing inventory remains unknown; it never means that the call reads nothing.
    #[must_use]
    pub fn invocation_variable_reads_at(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<&Arc<SourceInvocationVariableReads>> {
        self.invocation_reads.get(site)
    }

    pub(super) fn attach_invocation_reads(
        &self,
        mut binding: SourceInvocationBinding,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
    ) -> SourceInvocationBinding {
        binding.invocation_variable_reads = origin.and_then(|origin| {
            self.invocation_reads
                .get(&CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                })
                .cloned()
        });
        self.attach_invocation_normal_result(&mut binding, origin, offset);
        self.attach_rhs_read_store(&mut binding, origin, offset);
        binding = self.attach_normal_variable_continuation(binding, origin, offset);
        binding = self.attach_object_callback_effects(binding, origin, offset);
        binding = self.attach_declaration_operand_layout(binding, origin, offset);
        binding.conditional_expression_evaluations = origin.and_then(|origin| {
            self.conditional_expression_evaluations
                .get(&CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                })
                .map(|observations| Arc::from(observations.as_slice()))
        });
        self.attach_compiler_invocation(binding, origin, offset)
    }

    pub(super) fn begin_invocation_reads(
        &mut self,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
        parent: Option<&SourceVariableEvaluationOwner>,
    ) -> Option<SourceVariableEvaluationOwner> {
        let invocation = CommandAllocationSite {
            source: Arc::clone(origin?),
            offset,
        };
        self.invocation_reads
            .entry(invocation.clone())
            .or_insert_with(|| {
                Arc::new(SourceInvocationVariableReads {
                    residual: SourceVariableReadResidual::Closed,
                    ..Default::default()
                })
            });
        Some(SourceVariableEvaluationOwner::InvocationBody {
            invocation,
            parent: parent.cloned().map(Arc::new),
        })
    }

    fn extend_invocation_reads(
        &mut self,
        owner: Option<&SourceVariableEvaluationOwner>,
        reads: &SourceInvocationVariableReads,
    ) {
        if let Some(owner) = owner {
            owner.for_each_body(&mut |site| {
                let target = self
                    .invocation_reads
                    .entry(site.clone())
                    .or_insert_with(|| {
                        Arc::new(SourceInvocationVariableReads {
                            residual: SourceVariableReadResidual::Closed,
                            ..Default::default()
                        })
                    });
                Arc::make_mut(target).join(reads);
            });
        }
    }

    pub(super) fn record_invocation_substitution_read(&mut self, read: SourceVariableAccess) {
        let owner = read.owner.clone();
        self.extend_invocation_reads(
            Some(&owner),
            &SourceInvocationVariableReads {
                substitutions: vec![read],
                native_reads: Vec::new(),
                residual: SourceVariableReadResidual::Closed,
            },
        );
    }

    pub(super) fn record_native_invocation_reads(
        &mut self,
        facts: &InvocationFacts,
        arguments: tcl_registry::InvocationArguments<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        let native_reads: Vec<_> = crate::variable_bindings::source_variable_read_places(
            facts,
            arguments,
            &state.source_variables,
            context.registry,
        )
        .into_iter()
        .map(|place| SourceInvocationVariableRead {
            place,
            variable_context: Arc::clone(&state.source_variables),
            context_fingerprint: SourceVariableAccess::fingerprint(&state.source_variables),
        })
        .collect();
        let bounded = facts.arg_roles_complete
            && native_reads.iter().all(|read| {
                !read.place.observed && read.place.kind != crate::place::PlaceKind::Unknown
            });
        self.extend_invocation_reads(
            context.variable_read_owner,
            &SourceInvocationVariableReads {
                native_reads,
                substitutions: Vec::new(),
                residual: if bounded {
                    SourceVariableReadResidual::Closed
                } else {
                    SourceVariableReadResidual::Unknown
                },
            },
        );
    }

    pub(super) fn finish_invocation_reads(
        &mut self,
        owner: Option<&SourceVariableEvaluationOwner>,
        outcomes: &SourceOutcomes,
    ) {
        let opaque = outcomes
            .normal
            .iter()
            .map(AsRef::as_ref)
            .chain(outcomes.abrupt.iter().map(|(_, state)| state.as_ref()))
            .any(|state| state.opaque_domain || state.source_variables.dynamic_bindings);
        if opaque {
            self.extend_invocation_reads(owner, &SourceInvocationVariableReads::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str) -> (SourceCommandBindings, tcl_registry::CommandRegistry) {
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings =
            SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
        (bindings, registry)
    }

    #[test]
    fn entered_body_reads_are_distinct_from_original_arguments() {
        let source = "set x VALUE; catch {set x}";
        let (bindings, _) = analyse(source);
        let offset = u32::try_from(source.find("catch").unwrap()).unwrap();
        let invocation = bindings.invocation_at_source("", offset);
        let reads = invocation.invocation_variable_reads.as_ref().unwrap();
        assert_eq!(reads.native_reads.len(), 1);
        assert_eq!(reads.native_reads[0].place.name, "x");
        assert_eq!(reads.residual, SourceVariableReadResidual::Closed);
        assert_eq!(
            bindings.variable_accesses_for_invocation_args(offset),
            [] as [SourceVariableAccess; 0]
        );
    }

    #[test]
    fn callee_reads_keep_each_actual_caller_alias_target() {
        let source = "proc read {name} {upvar 1 $name a; set a}; set x X; set y Y; read x; read y";
        let (bindings, _) = analyse(source);
        for name in ["x", "y"] {
            let offset = u32::try_from(source.rfind(&format!("read {name}")).unwrap()).unwrap();
            let invocation = bindings.invocation_at_source("", offset);
            let reads = invocation.invocation_variable_reads.as_ref().unwrap();
            assert!(
                reads
                    .native_reads
                    .iter()
                    .any(|read| read.place.name == name),
                "{name}: {reads:?}"
            );
            assert!(
                !reads
                    .native_reads
                    .iter()
                    .any(|read| read.place.name == if name == "x" { "y" } else { "x" })
            );
        }
    }

    #[test]
    fn opaque_body_keeps_an_explicit_unknown_read_residual() {
        let source = "proc p {} {missing_dynamic_handler}; p";
        let (bindings, _) = analyse(source);
        let offset = u32::try_from(source.rfind('p').unwrap()).unwrap();
        let invocation = bindings.invocation_at_source("", offset);
        let reads = invocation.invocation_variable_reads.as_ref().unwrap();
        assert_eq!(reads.residual, SourceVariableReadResidual::Unknown);
    }
}
