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

//! Reached variable accesses: who evaluated them, where they resolve and the queries over them.

use super::command_identity::CommandAllocationSite;
use super::invocation_reads::SourceVariableReadResidual;
use super::namespace_context::SourceNamespaceKey;
use super::object_instance::SourceObjectInstanceProof;
use super::{
    ModuleCommandBindings, SelectedSourceVariablePlace, SourceCommandBindings,
    SourceVariableAccess, variable_access_owners,
};
use std::hash::Hash;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// Temporal owner of a reached read, independently of its lexical source span.
/// A nested invocation retains the evaluation in which it was entered; native
/// body evaluation is distinct from evaluation of that invocation's argv.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceVariableEvaluationOwner {
    /// Original command-word evaluation, before dispatch.
    InvocationArguments {
        /// Exact invocation in its authored or materialised source instance.
        invocation: CommandAllocationSite,
        /// Enclosing evaluation, when this invocation is itself a substitution.
        parent: Option<Arc<Self>>,
    },
    /// Expression evaluation performed by a selected native invocation.
    NativeExpression {
        /// Invocation selecting this native expression evaluation.
        invocation: CommandAllocationSite,
        /// Enclosing evaluation, when native evaluation occurs in a substitution.
        parent: Option<Arc<Self>>,
    },
    /// Post-argv execution of a reached invocation and its entered bodies.
    InvocationBody {
        /// Actual caller site, independent of the callee's lexical body source.
        invocation: CommandAllocationSite,
        /// Active enclosing evaluation when this call was itself entered there.
        parent: Option<Arc<Self>>,
    },
    /// The same lexical read was reached from several temporal evaluations.
    Alternatives(Vec<Self>),
    /// No exact temporal evaluation owner was retained.
    Unspecified,
}

impl SourceVariableEvaluationOwner {
    pub(super) fn is_argument_evaluation_of(&self, site: &CommandAllocationSite) -> bool {
        match self {
            Self::InvocationArguments { invocation, parent } => {
                invocation == site
                    || parent
                        .as_deref()
                        .is_some_and(|parent| parent.is_argument_evaluation_of(site))
            }
            Self::NativeExpression { parent, .. } | Self::InvocationBody { parent, .. } => parent
                .as_deref()
                .is_some_and(|parent| parent.is_argument_evaluation_of(site)),
            Self::Alternatives(owners) => owners
                .iter()
                .any(|owner| owner.is_argument_evaluation_of(site)),
            Self::Unspecified => false,
        }
    }

    pub(super) fn is_direct_arguments_of(&self, site: &CommandAllocationSite) -> bool {
        match self {
            Self::InvocationArguments { invocation, .. } => invocation == site,
            Self::Alternatives(owners) => owners
                .iter()
                .any(|owner| owner.is_direct_arguments_of(site)),
            Self::NativeExpression { .. } | Self::InvocationBody { .. } | Self::Unspecified => {
                false
            }
        }
    }

    pub(crate) fn is_expression_evaluation_of(&self, site: &CommandAllocationSite) -> bool {
        match self {
            Self::NativeExpression { invocation, parent } => {
                invocation == site
                    || parent
                        .as_deref()
                        .is_some_and(|parent| parent.is_expression_evaluation_of(site))
            }
            Self::InvocationArguments { parent, .. } | Self::InvocationBody { parent, .. } => {
                parent
                    .as_deref()
                    .is_some_and(|parent| parent.is_expression_evaluation_of(site))
            }
            Self::Alternatives(owners) => owners
                .iter()
                .any(|owner| owner.is_expression_evaluation_of(site)),
            Self::Unspecified => false,
        }
    }

    pub(super) fn join(&mut self, other: Self) {
        if *self == other {
            return;
        }
        if let Self::Alternatives(owners) = self {
            if !owners.contains(&other) {
                owners.push(other);
            }
        } else {
            *self = Self::Alternatives(vec![self.clone(), other]);
        }
    }
}

impl SourceVariableAccess {
    /// Physical address frozen after index evaluation and before the read's
    /// own observers. Disagreeing reached addresses retain the original
    /// unresolved lookup rather than selecting one evaluated index.
    #[must_use]
    pub(crate) fn place_in_context(
        &self,
        context: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> crate::place::Place {
        self.selected_places
            .iter()
            .find(|selected| selected.context.as_ref() == context)
            .and_then(|selected| selected.place.as_deref())
            .or_else(|| {
                if context != self.variable_context.as_ref() {
                    return None;
                }
                let place = self.selected_places.first()?.place.as_deref()?;
                self.selected_places
                    .iter()
                    .all(|selected| selected.place.as_deref() == Some(place))
                    .then_some(place)
            })
            .map_or_else(
                || {
                    crate::var_resolve::resolve_substitution_access(
                        &self.original_spelling,
                        context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    )
                },
                Clone::clone,
            )
    }

    pub(super) fn retain_selected_place(
        &mut self,
        context: &Arc<crate::var_resolve::ResolveContext>,
        place: &crate::place::Place,
    ) {
        if let Some(selected) = self
            .selected_places
            .iter_mut()
            .find(|selected| selected.context == *context)
        {
            if selected.place.as_deref() != Some(place) {
                selected.place = None;
            }
        } else {
            self.selected_places.push(SelectedSourceVariablePlace {
                context: Arc::clone(context),
                place: Some(Arc::new(place.clone())),
            });
        }
    }

    /// Actual instance allocation and current dispatch at this exact read.
    /// Nominal type labels and an unchanged variable version cannot supply it.
    #[must_use]
    pub fn proved_object_instance(&self) -> Option<&SourceObjectInstanceProof> {
        self.object_instance.as_deref()
    }

    /// Actual physical read contexts before their compatibility lattice join.
    /// Distinct activations remain distinct; this supplies no logical-slot alias.
    #[must_use]
    pub fn context_alternatives(&self) -> &[Arc<crate::var_resolve::ResolveContext>] {
        &self.context_alternatives
    }

    /// Whether retained contexts leave additional address uncertainty.
    #[must_use]
    pub fn context_residual(&self) -> SourceVariableReadResidual {
        if self
            .context_alternatives
            .iter()
            .any(|context| context.dynamic_bindings)
        {
            SourceVariableReadResidual::Unknown
        } else {
            SourceVariableReadResidual::Closed
        }
    }

    pub(super) fn selected_namespace_context(
        &self,
        namespace: &SourceNamespaceKey,
    ) -> Option<Self> {
        let matches = |context: &crate::var_resolve::ResolveContext| {
            context
                .namespace_identity
                .as_ref()
                .is_some_and(|identity| identity == namespace)
                || (context.namespace_identity.is_none()
                    && matches!(namespace, SourceNamespaceKey::Authored(text) if text == &context.namespace))
        };
        let contexts = self
            .context_alternatives
            .iter()
            .filter(|context| matches(context))
            .cloned()
            .collect::<Vec<_>>();
        let mut context = contexts.first()?.as_ref().clone();
        for other in contexts.iter().skip(1) {
            context.join(other);
        }
        let mut selected = self.clone();
        selected.variable_context = Arc::new(context);
        selected.context_fingerprint = Self::fingerprint(&selected.variable_context);
        selected.context_alternatives = contexts;
        selected
            .selected_places
            .retain(|place| matches(&place.context));
        // Joined source ownership and receiver uncertainty remain withdrawn;
        // selecting a namespace cannot recreate a discarded read/result proof.
        Some(selected)
    }

    /// Relocate physical variable proof while retaining lexical/temporal source ownership.
    #[must_use]
    pub fn relocated_variables(
        &self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) -> Self {
        let mut access = self.clone();
        access.object_instance = None;
        access.selected_places = self
            .selected_places
            .iter()
            .map(|selected| SelectedSourceVariablePlace {
                context: Arc::new(selected.context.relocated(relocation)),
                place: selected
                    .place
                    .as_ref()
                    .map(|place| Arc::new(relocation.place(place))),
            })
            .collect();
        access.variable_context = Arc::new(self.variable_context.relocated(relocation));
        access.context_alternatives = self
            .context_alternatives
            .iter()
            .map(|context| Arc::new(context.relocated(relocation)))
            .collect();
        access.context_fingerprint = Self::fingerprint(&access.variable_context);
        access
    }

    /// Select one exact lexical read from a retained access inventory.
    /// An enclosing invocation or another read of the same name cannot supply
    /// its context. Duplicate records at the exact site remain unresolved,
    /// including duplicates with a different spelling.
    #[must_use]
    pub fn find_at_source<'a>(
        accesses: &'a [Self],
        source: &tcl_source::words::SourceSite,
        original_spelling: &str,
    ) -> Option<&'a Self> {
        Self::find_at_site(accesses, source)
            .filter(|access| access.original_spelling == original_spelling)
    }

    /// Retrieve the original spelling of one unique exact source reference.
    /// Conflicting retained records at that site remain unresolved; a matching
    /// name elsewhere in the containing word is never consulted.
    #[must_use]
    pub fn find_at_site<'a>(
        accesses: &'a [Self],
        source: &tcl_source::words::SourceSite,
    ) -> Option<&'a Self> {
        let mut candidates = accesses.iter().filter(|access| access.source == *source);
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    }

    /// Containment in an authored command range uses the original token's
    /// byte extent. Native read sites retain their grammar-specific inner end.
    #[must_use]
    pub fn is_contained_in(&self, span: tcl_lexer::Span) -> bool {
        u32::try_from(self.original_spelling.len())
            .ok()
            .and_then(|length| self.source.span.start().checked_add(length))
            .is_some_and(|end| {
                self.source.span.start() >= span.start() && end <= span.end().saturating_add(1)
            })
    }

    pub(super) fn fingerprint(context: &crate::var_resolve::ResolveContext) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        context.hash(&mut hasher);
        std::hash::Hasher::finish(&hasher)
    }
}

impl PartialEq for SourceVariableAccess {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.owner == other.owner
            && self.original_spelling == other.original_spelling
            && self.context_fingerprint == other.context_fingerprint
            && self.variable_context == other.variable_context
            && self.context_alternatives == other.context_alternatives
            && self.object_instance == other.object_instance
            && self.selected_places == other.selected_places
    }
}

impl Eq for SourceVariableAccess {}

impl Hash for SourceVariableAccess {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.owner.hash(state);
        self.original_spelling.hash(state);
        self.context_fingerprint.hash(state);
        self.context_alternatives.hash(state);
        self.object_instance.hash(state);
        for selected in &self.selected_places {
            Self::fingerprint(&selected.context).hash(state);
            selected.place.hash(state);
        }
    }
}

impl SourceCommandBindings {
    /// Reached variable reads wholly contained in an original source span.
    /// Records use this analysis instance's source root and absolute offsets;
    /// absent or skipped reads remain absent instead of inheriting dispatch facts.
    #[must_use]
    pub fn variable_accesses_in_span(&self, span: tcl_lexer::Span) -> Vec<SourceVariableAccess> {
        self.variable_accesses
            .range(span.start()..=span.end())
            .flat_map(|(_, accesses)| accesses)
            .filter(|access| access.is_contained_in(span))
            .cloned()
            .collect()
    }

    /// Reads directly owned by this invocation's original word evaluation.
    /// Nested commands and native body phases have their own temporal owners;
    /// the lexical inventory is deliberately broader than this projection.
    #[must_use]
    pub fn variable_accesses_for_invocation_args(&self, offset: u32) -> Vec<SourceVariableAccess> {
        self.variable_accesses_owned_by(
            offset,
            variable_access_owners::AccessOwnerProjection::DirectArguments,
        )
    }

    /// Reads executed during original argv evaluation, including entered nested
    /// substitutions. This transitive evaluation projection is distinct from
    /// the directly owned argv projection and never relies on lexical nesting.
    #[must_use]
    pub fn variable_accesses_during_argument_evaluation(
        &self,
        offset: u32,
    ) -> Vec<SourceVariableAccess> {
        self.variable_accesses_owned_by(
            offset,
            variable_access_owners::AccessOwnerProjection::ArgumentEvaluation,
        )
    }

    /// Reached reads while evaluating this invocation's original argv or its
    /// selected native expression, including substitutions entered by either
    /// evaluation. Native script bodies have separate temporal ownership.
    #[must_use]
    pub fn variable_accesses_during_expression_invocation(
        &self,
        offset: u32,
    ) -> Vec<SourceVariableAccess> {
        self.variable_accesses_owned_by(
            offset,
            variable_access_owners::AccessOwnerProjection::ArgumentOrExpressionEvaluation,
        )
    }

    /// Query one exact original reference; an unrecorded read is unresolved.
    #[must_use]
    pub fn variable_access_at(
        &self,
        source: &tcl_source::words::SourceSite,
        original_spelling: &str,
    ) -> Option<&SourceVariableAccess> {
        SourceVariableAccess::find_at_source(
            self.variable_accesses.get(&source.span.start())?,
            source,
            original_spelling,
        )
    }

    pub(super) fn record_variable_access(
        &mut self,
        source: &tcl_source::words::SourceSite,
        original_spelling: &str,
        state: &ModuleCommandBindings,
        owner: Option<&SourceVariableEvaluationOwner>,
        registry: &CommandRegistry,
        selected_place: &crate::place::Place,
    ) {
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let context = &state.source_variables;
        let object_instance = state.object_read_proof(selected_place, registry);
        let owner = owner
            .cloned()
            .unwrap_or(SourceVariableEvaluationOwner::Unspecified);
        let reached_read = SourceVariableAccess {
            source: source.clone(),
            owner: owner.clone(),
            original_spelling: original_spelling.to_owned(),
            variable_context: Arc::clone(context),
            context_alternatives: vec![Arc::clone(context)],
            context_fingerprint: SourceVariableAccess::fingerprint(context),
            object_instance: object_instance.clone(),
            selected_places: vec![SelectedSourceVariablePlace {
                context: Arc::clone(context),
                place: Some(Arc::new(selected_place.clone())),
            }],
        };
        self.capture_current_expression_read(origin, &state.variable_frame, &reached_read);
        self.record_invocation_substitution_read(reached_read);
        let inventory = if self.root_origin.as_ref() == Some(origin) {
            &mut self.variable_accesses
        } else {
            self.origin_variable_accesses
                .entry(Arc::clone(origin))
                .or_default()
        };
        let accesses = inventory.entry(source.span.start()).or_default();
        if let Some(access) = accesses.iter_mut().find(|access| {
            access.source == *source && access.original_spelling == original_spelling
        }) {
            access.owner.join(owner);
            access.retain_selected_place(context, selected_place);
            if access.object_instance != object_instance {
                access.object_instance = None;
            }
            if !access.context_alternatives.contains(context) {
                access.context_alternatives.push(Arc::clone(context));
            }
            if access.variable_context != *context {
                Arc::make_mut(&mut access.variable_context).join(context);
                access.context_fingerprint =
                    SourceVariableAccess::fingerprint(&access.variable_context);
            }
        } else {
            accesses.push(SourceVariableAccess {
                source: source.clone(),
                owner,
                original_spelling: original_spelling.to_owned(),
                variable_context: Arc::clone(context),
                context_alternatives: vec![Arc::clone(context)],
                context_fingerprint: SourceVariableAccess::fingerprint(context),
                object_instance,
                selected_places: vec![SelectedSourceVariablePlace {
                    context: Arc::clone(context),
                    place: Some(Arc::new(selected_place.clone())),
                }],
            });
        }
    }
}
