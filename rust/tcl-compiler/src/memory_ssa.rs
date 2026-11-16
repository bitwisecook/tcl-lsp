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

//! Memory-SSA: versioned memory operations with alias analysis.
//!
//! Memory-SSA extends scalar SSA to track *memory locations* — stores
//! and loads that go through aliased bindings (`upvar`, `global`,
//! `variable`, `namespace upvar`).
//!
//! Key concepts:
//!
//! - [`MemoryLocation`] identifies *what* is being stored/loaded
//!   (local, upvar alias, global, namespace variable, array element).
//! - [`AliasSet`] groups [`MemoryLocation`]s that may refer to the
//!   same underlying storage.
//! - [`MemoryOp`] annotates statements with versioned memory
//!   operations so that downstream passes (GVN, DSE, copy
//!   propagation) can reason about aliased state precisely.
//!
//! This module operates *after* scalar SSA construction. It consumes
//! registry-owned state-transition facts resolved from structured command
//! words, so compiler code never infers variable-cell aliases from command
//! spellings or flattened argument layouts.
//!
//! Organised in three layers:
//! - location types and alias-set queries.
//! - memory-op types + `MemorySsaFunction` + detection helpers.
//! - `compute_aliases` + `build_memory_ssa` driver.

use std::collections::{BTreeSet, HashMap, HashSet};
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{
    CallerFrameSelection, CommandRegistry, FrameLevel, StateTransition, StateTransitionKnowledge,
    Traits, VariableAliasTarget,
};

use crate::cfg::BlockId;
use crate::ir::Statement;
use crate::registry_invocation::{
    InvocationMetadataContext, RegistryInvocationResolution,
    resolve_command_tokens_with_metadata_context,
};
use crate::ssa::{SsaFunction, Version};

/// Classification of a memory location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemoryLocationKind {
    /// Procedure-local variable (no aliasing concerns).
    Local,
    /// Aliased from a caller frame via `upvar`.
    Upvar,
    /// Global variable (`global` command or `::` prefix).
    Global,
    /// Namespace-scoped variable (`variable` or `namespace upvar`).
    NamespaceVar,
    /// Element of a Tcl array (`arrayName(index)`).
    ArrayElement,
    /// OO instance variable (`variable` in method body).
    InstanceVar,
    /// Cannot be determined statically.
    Unknown,
}

/// A specific memory location in the program.
///
/// The `qualifier` field carries location-specific context:
/// namespace (for [`MemoryLocationKind::NamespaceVar`]), caller-side
/// variable name (for [`MemoryLocationKind::Upvar`]), or array index
/// text (for [`MemoryLocationKind::ArrayElement`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Classification of the location.
    pub kind: MemoryLocationKind,
    /// Variable name.
    pub name: String,
    /// Exact canonical storage identity when supplied by a proved cell.
    /// Presentation names cannot reconstruct this key.
    pub storage_key: Option<crate::var_resolve::VariableCellKey>,
    /// Location-specific context.
    pub qualifier: String,
}

impl MemoryLocation {
    /// Build a location with `kind`, `name`, and an empty qualifier.
    #[must_use]
    pub fn new(kind: MemoryLocationKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            storage_key: None,
            qualifier: String::new(),
        }
    }

    /// Build a location with an explicit qualifier.
    #[must_use]
    pub fn with_qualifier(
        kind: MemoryLocationKind,
        name: impl Into<String>,
        qualifier: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            storage_key: None,
            qualifier: qualifier.into(),
        }
    }

    /// Render the location in its canonical textual format for
    /// diagnostics.
    #[must_use]
    pub fn display(&self) -> String {
        match self.kind {
            MemoryLocationKind::Upvar => {
                format!("upvar({} -> {})", self.qualifier, self.name)
            }
            MemoryLocationKind::Global => format!("global({})", self.name),
            MemoryLocationKind::NamespaceVar => {
                format!(
                    "ns({})",
                    crate::naming::rooted_member_key(&self.qualifier, &self.name)
                )
            }
            MemoryLocationKind::InstanceVar => {
                format!(
                    "ivar({})",
                    crate::naming::rooted_member_key(&self.qualifier, &self.name)
                )
            }
            MemoryLocationKind::ArrayElement => {
                format!("{}({})", self.name, self.qualifier)
            }
            MemoryLocationKind::Unknown => format!("?({})", self.name),
            MemoryLocationKind::Local => self.name.clone(),
        }
    }
}

/// A group of memory locations that may alias each other.
///
/// For example, if `upvar 1 caller_x local_x` is in scope then
/// `caller_x` and `local_x` form an alias set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasSet {
    /// Exact locations merged into this set; use `ordered_locations` for presentation.
    pub locations: HashSet<MemoryLocation>,
    /// Reason describing why the set was formed — e.g. `"global-cell"`,
    /// `"caller-frame-cell"`, `"namespace-cell"`, or a combination
    /// (comma-separated, sorted) when several detection paths merged into
    /// the same set; `"alias"` when no path recorded one.
    pub reason: String,
}

impl AliasSet {
    /// Construct an alias set from an owned location set + reason.
    #[must_use]
    pub fn new(locations: HashSet<MemoryLocation>, reason: impl Into<String>) -> Self {
        Self {
            locations,
            reason: reason.into(),
        }
    }

    /// Diagnostic presentation order, independent of canonical storage equality.
    #[must_use]
    pub fn ordered_locations(&self) -> Vec<&MemoryLocation> {
        let mut locations: Vec<_> = self.locations.iter().collect();
        locations.sort_by(|left, right| {
            (left.kind, &left.name, &left.qualifier).cmp(&(
                right.kind,
                &right.name,
                &right.qualifier,
            ))
        });
        locations
    }

    /// True when `loc` is in this alias set.
    #[must_use]
    pub fn may_alias(&self, loc: &MemoryLocation) -> bool {
        self.locations.contains(loc)
    }

    /// All variable names in this set.
    #[must_use]
    pub fn names(&self) -> BTreeSet<String> {
        self.locations.iter().map(|l| l.name.clone()).collect()
    }

    /// True when `name` appears as any location's variable name.
    #[must_use]
    pub fn contains_name(&self, name: &str) -> bool {
        self.locations.iter().any(|location| {
            location.name == name
                && location
                    .storage_key
                    .as_ref()
                    .is_none_or(|key| key.authored_spelling().is_some())
        })
    }
}

/// Kind of memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryOpKind {
    /// Memory write (store).
    Def,
    /// Memory read (load).
    Use,
    /// Memory phi at a merge point.
    Phi,
    /// Barrier / call that may modify any aliased memory.
    Clobber,
}

/// A versioned memory operation attached to a statement.
///
/// Uses are annotated with `reaching_version` indicating the version
/// of the memory state visible at the read; defs/phis/clobbers bump
/// `version` monotonically across a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryOp {
    /// Operation kind.
    pub kind: MemoryOpKind,
    /// Location being read or written.
    pub location: MemoryLocation,
    /// New version number assigned by this op (for Def/Phi/Clobber).
    /// For Use, matches the reaching version.
    pub version: Version,
    /// Version of memory state reaching this read (Use only).
    pub reaching_version: Version,
    /// Block containing the op.
    pub block: String,
    /// Statement index within the block (`-1` for phi).
    pub statement_index: i32,
}

impl MemoryOp {
    /// Build a def/clobber attached to a block+index.
    #[must_use]
    pub fn new_def(
        location: MemoryLocation,
        version: Version,
        block: impl Into<String>,
        statement_index: i32,
    ) -> Self {
        Self {
            kind: MemoryOpKind::Def,
            location,
            version,
            reaching_version: 0,
            block: block.into(),
            statement_index,
        }
    }

    /// Build a use annotated with its reaching version.
    #[must_use]
    pub fn new_use(
        location: MemoryLocation,
        reaching_version: Version,
        block: impl Into<String>,
        statement_index: i32,
    ) -> Self {
        Self {
            kind: MemoryOpKind::Use,
            location,
            version: reaching_version,
            reaching_version,
            block: block.into(),
            statement_index,
        }
    }

    /// Build a memory phi at the start of a block.
    #[must_use]
    pub fn new_phi(location: MemoryLocation, version: Version, block: impl Into<String>) -> Self {
        Self {
            kind: MemoryOpKind::Phi,
            location,
            version,
            reaching_version: 0,
            block: block.into(),
            statement_index: -1,
        }
    }

    /// Build a clobber (e.g. for `eval`/`uplevel` barriers).
    #[must_use]
    pub fn new_clobber(version: Version, block: impl Into<String>, statement_index: i32) -> Self {
        Self {
            kind: MemoryOpKind::Clobber,
            location: MemoryLocation::new(MemoryLocationKind::Unknown, "*"),
            version,
            reaching_version: 0,
            block: block.into(),
            statement_index,
        }
    }
}

/// Memory-SSA annotations for a single function.
///
/// Produced by `build_memory_ssa`. Carries:
/// - `alias_sets`: every detected alias set in the function.
/// - `memory_ops`: one entry per def/use/phi/clobber, in emission
///   order.
/// - `memory_phis`: per-block memory phis, keyed by block name.
/// - pre-computed counts (`count_defs`, `count_uses`, `count_clobbers`)
///   for O(1) summary queries.
/// - `has_wildcard_aliasing`: whether an unresolved or widened transition
///   requires every memory consumer to retain a wildcard clobber obligation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemorySsaFunction {
    /// Reaching cell-content graph. Whole-function alias sets are only a may-exposure projection.
    pub cell_state: Option<crate::state_ssa::StateSsa<crate::place::Place>>,
    /// Alias sets covering this function's aliased variables.
    pub alias_sets: Vec<AliasSet>,
    /// Memory operations in emission order.
    pub memory_ops: Vec<MemoryOp>,
    /// Block-indexed memory phi nodes.
    pub memory_phis: HashMap<String, Vec<MemoryOp>>,
    /// Number of [`MemoryOpKind::Def`] ops.
    pub count_defs: usize,
    /// Number of [`MemoryOpKind::Use`] ops.
    pub count_uses: usize,
    /// Number of [`MemoryOpKind::Clobber`] ops.
    pub count_clobbers: usize,
    /// Whether any invocation could change an unenumerated variable-cell
    /// identity through an unresolved or widened registry transition.
    pub has_wildcard_aliasing: bool,
}

impl MemorySsaFunction {
    /// Relocate physical cells while preserving memory definitions, edges and versions.
    /// Legacy location qualifiers are regenerated from typed cells, never parsed as display text.
    ///
    /// # Errors
    /// Returns an error if a retained state graph fails its structural validation.
    pub fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) -> Result<(), crate::state_ssa::StateSsaError> {
        let mut locations = HashMap::new();
        if let Some(state) = &self.cell_state {
            let operations = state
                .operations()
                .iter()
                .map(|operation| {
                    let old = memory_location_of_place(operation.location());
                    let place = relocation.place(operation.location());
                    locations.insert(old, memory_location_of_place(&place));
                    let mut operation = operation.clone();
                    match &mut operation {
                        crate::state_ssa::StateOp::Use(value) => value.location = place,
                        crate::state_ssa::StateOp::Def(value) => value.location = place,
                        crate::state_ssa::StateOp::Phi(value) => value.location = place,
                        crate::state_ssa::StateOp::Clobber(value) => value.location = place,
                    }
                    operation
                })
                .collect();
            self.cell_state = Some(crate::state_ssa::StateSsa::new(operations)?);
        }
        let relocate = |location: &MemoryLocation| {
            locations.get(location).cloned().unwrap_or_else(|| {
                let mut location = location.clone();
                if let Some(key) = &location.storage_key {
                    let key = relocation.cell_key(key);
                    location.name = key.compatibility_name();
                    location.storage_key = Some(key);
                } else {
                    location.name = relocation.storage_key(&location.name).compatibility_name();
                }
                location
            })
        };
        for alias in &mut self.alias_sets {
            alias.locations = alias.locations.iter().map(&relocate).collect();
        }
        for operation in self
            .memory_ops
            .iter_mut()
            .chain(self.memory_phis.values_mut().flatten())
        {
            operation.location = relocate(&operation.location);
        }
        Ok(())
    }

    /// All variable names involved in aliasing.
    #[must_use]
    pub fn aliased_names(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for aset in &self.alias_sets {
            out.extend(aset.names());
        }
        out
    }
}

/// One precise alias pair materialised from a registry transition fact.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryAliasPair {
    target: MemoryLocation,
    local: MemoryLocation,
    reason: &'static str,
}

fn command_tokens(stmt: &Statement) -> Option<&crate::ir::CommandTokens> {
    match stmt {
        Statement::Call {
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            tokens: Some(tokens),
            ..
        } => Some(tokens),
        _ => None,
    }
}

fn registry_resolution_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> Option<RegistryInvocationResolution> {
    let context = context?;
    if !context.matches_registry(registry) {
        return None;
    }
    let tokens = command_tokens(stmt)?;
    resolve_command_tokens_with_metadata_context(registry, Some(context), tokens).ok()
}

fn literal_subject(subject: &tcl_registry::TransitionSubject) -> Option<&str> {
    subject.literal()
}

#[cfg(test)]
fn transition_alias_pairs(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> Vec<RegistryAliasPair> {
    transition_alias_pairs_with_metadata_context(
        stmt,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
    )
}

fn original_alias_frame(
    word: &str,
    statement: &Statement,
    context: Option<InvocationMetadataContext<'_>>,
) -> Option<FrameLevel> {
    let dialect = command_tokens(statement)
        .and_then(|tokens| tokens.source_binding.as_ref())
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context?
                .context()
                .environment
                .point()
                .map(tcl_registry::InvocationDialect::of_point)
        })?;
    FrameLevel::parse_for_dialect(word, dialect)
}

fn transition_alias_pairs_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> Vec<RegistryAliasPair> {
    // Logical source recipes add MAY exposure only. They cannot remove the
    // separate wildcard/clobber residual or establish a completed physical link.
    let facts = registry_resolution_with_metadata_context(stmt, registry, context)
        .and_then(|resolution| match resolution {
            RegistryInvocationResolution::Resolved(facts) => Some(facts),
            RegistryInvocationResolution::Unresolved(_) => None,
        })
        .or_else(|| {
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                registry,
                context?,
                command_tokens(stmt)?,
            )
            .map(|selected| selected.facts)
        });
    let Some(facts) = facts else {
        return Vec::new();
    };
    let StateTransitionKnowledge::Declared(transitions) = &facts.state_transitions else {
        return Vec::new();
    };

    let mut pairs = Vec::new();
    for fact in transitions.facts() {
        let StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        let Some(local) = literal_subject(&alias.local) else {
            continue;
        };
        match &alias.target {
            VariableAliasTarget::Global { variable } => {
                let Some(variable) = literal_subject(variable) else {
                    continue;
                };
                pairs.push(RegistryAliasPair {
                    target: MemoryLocation::new(MemoryLocationKind::Global, variable),
                    local: MemoryLocation::new(MemoryLocationKind::Local, local),
                    reason: "global-cell",
                });
            }
            VariableAliasTarget::CurrentNamespace { variable } => {
                let Some(variable) = literal_subject(variable) else {
                    continue;
                };
                pairs.push(RegistryAliasPair {
                    target: MemoryLocation::new(MemoryLocationKind::NamespaceVar, variable),
                    local: MemoryLocation::new(MemoryLocationKind::Local, local),
                    reason: "current-namespace-cell",
                });
            }
            VariableAliasTarget::Namespace {
                namespace,
                variable,
            } => {
                let (Some(namespace), Some(variable)) =
                    (literal_subject(namespace), literal_subject(variable))
                else {
                    continue;
                };
                pairs.push(RegistryAliasPair {
                    target: MemoryLocation::with_qualifier(
                        MemoryLocationKind::NamespaceVar,
                        variable,
                        namespace,
                    ),
                    local: MemoryLocation::new(MemoryLocationKind::Local, local),
                    reason: "namespace-cell",
                });
            }
            VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
                let Some(variable) = literal_subject(variable) else {
                    continue;
                };
                push_caller_frame_alias_pair(&mut pairs, (frame, variable, local), stmt, context);
            }
        }
    }
    pairs
}

fn push_caller_frame_alias_pair(
    pairs: &mut Vec<RegistryAliasPair>,
    (frame, variable, local): (&CallerFrameSelection, &str, &str),
    stmt: &Statement,
    context: Option<InvocationMetadataContext<'_>>,
) {
    let is_global = matches!(
        frame,
        CallerFrameSelection::Explicit(level)
            if literal_subject(level)
                .and_then(|word| {
                    original_alias_frame(word, stmt, context)
                })
                .is_some_and(FrameLevel::is_global_frame)
    );
    let is_current = matches!(frame, CallerFrameSelection::Explicit(level)
                    if literal_subject(level).and_then(|word| original_alias_frame(word, stmt, context)).is_some_and(FrameLevel::is_current_frame));
    if is_current {
        pairs.push(RegistryAliasPair {
            target: MemoryLocation::new(MemoryLocationKind::Local, variable),
            local: MemoryLocation::new(MemoryLocationKind::Local, local),
            reason: "current-frame-cell",
        });
    } else if is_global {
        pairs.push(RegistryAliasPair {
            target: MemoryLocation::new(MemoryLocationKind::Global, variable),
            local: MemoryLocation::new(MemoryLocationKind::Local, local),
            reason: "global-frame-cell",
        });
    } else {
        pairs.push(RegistryAliasPair {
            target: MemoryLocation::new(MemoryLocationKind::Upvar, variable),
            local: MemoryLocation::with_qualifier(MemoryLocationKind::Upvar, local, variable),
            reason: "caller-frame-cell",
        });
    }
}

fn has_projected_boundary_effects(
    statement: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    argument_input: crate::place_bridge::ArgumentReadInput<'_>,
) -> bool {
    let Some(tokens) = statement.tokens() else {
        return false;
    };
    match tokens.synthetic {
        Some(crate::ir::SyntheticMarker::IterationBindings(_)) => true,
        Some(crate::ir::SyntheticMarker::CapturedCatchOutputs) => {
            let Some(context) = context else {
                return false;
            };
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                registry,
                Some(context),
                tokens,
            )
            .is_some()
        }
        Some(crate::ir::SyntheticMarker::EvaluatedArguments) => {
            let Some(substitutions) =
                crate::place_bridge::argument_calls(tokens, registry, argument_input)
            else {
                return false;
            };
            substitutions.is_empty()
                && tokens.word_exprs.iter().all(|word| {
                    matches!(
                        crate::registry_invocation::invocation_word(word),
                        tcl_registry::InvocationWord::Literal(_)
                    ) || word
                        .sole_variable_substitution()
                        .and_then(|(_, source)| tokens.variable_access_for_site(source))
                        .is_some_and(|access| {
                            let place = crate::var_resolve::resolve_substitution_access(
                                &access.original_spelling,
                                &access.variable_context,
                                registry,
                                tcl_registry::TraceOperation::Read,
                            );
                            !place.observed && place.kind != crate::place::PlaceKind::Unknown
                        })
                })
                && !tokens.variable_accesses.iter().any(|access| {
                    crate::var_resolve::resolve_substitution_access(
                        &access.original_spelling,
                        &access.variable_context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    )
                    .observed
                })
        }
        _ => false,
    }
}

#[cfg(test)]
fn transition_requires_wildcard(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> bool {
    transition_requires_wildcard_with_read_input(
        stmt,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
        crate::place_bridge::ArgumentReadInput::Standalone,
    )
}

fn transition_requires_wildcard_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> bool {
    transition_requires_wildcard_with_read_input(
        stmt,
        registry,
        context,
        crate::place_bridge::ArgumentReadInput::Supplied(context),
    )
}

fn transition_requires_wildcard_with_read_input(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    argument_input: crate::place_bridge::ArgumentReadInput<'_>,
) -> bool {
    if !stmt.is_executable_invocation()
        || has_projected_boundary_effects(stmt, registry, context, argument_input)
    {
        return false;
    }
    match registry_resolution_with_metadata_context(stmt, registry, context) {
        Some(RegistryInvocationResolution::Unresolved(_)) | None => {
            matches!(
                stmt,
                Statement::Call { .. } | Statement::Barrier { .. } | Statement::NativeCall { .. }
            )
        }
        Some(RegistryInvocationResolution::Resolved(facts)) => match &facts.state_transitions {
            StateTransitionKnowledge::UnknownInvocation => true,
            StateTransitionKnowledge::Declared(transitions) => transitions
                .facts()
                .iter()
                .any(|fact| matches!(fact.transition, StateTransition::Widen(_))),
        },
    }
}

/// Whether `stmt` carries an unresolved or widened variable-cell transition.
///
/// This statement-level query lets consumers account for evaluation order.
/// In particular, a command's argument substitutions happen before the outer
/// invocation can perform its registry-declared transition.
#[must_use]
pub fn statement_has_wildcard_aliasing(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> bool {
    transition_requires_wildcard_with_read_input(
        stmt,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
        crate::place_bridge::ArgumentReadInput::Standalone,
    )
}

pub(crate) fn statement_has_wildcard_aliasing_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> bool {
    transition_requires_wildcard_with_metadata_context(stmt, registry, context)
}

/// The spec-declared trait set that classifies a call as a memory clobber.
const CLOBBER_TRAITS: Traits = Traits::EVALUATES_CODE.union(Traits::CREATES_BARRIER);

/// True if `stmt` may clobber arbitrary memory locations.
///
/// Barriers always clobber. A static-body [`Statement::UpFrame`] clobbers
/// because its body runs in a caller frame and can touch arbitrary caller
/// locals or globals. Calls use traits from structured registry facts. A
/// missing token snapshot, unresolved head, or indeterminate subcommand also
/// clobbers: no raw command spelling or argument text is inspected as a
/// fallback.
#[must_use]
pub fn is_clobber(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> bool {
    is_clobber_with_read_input(
        stmt,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
        crate::place_bridge::ArgumentReadInput::Standalone,
    )
}

pub(crate) fn is_clobber_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> bool {
    is_clobber_with_read_input(
        stmt,
        registry,
        context,
        crate::place_bridge::ArgumentReadInput::Supplied(context),
    )
}

fn is_clobber_with_read_input(
    stmt: &Statement,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    argument_input: crate::place_bridge::ArgumentReadInput<'_>,
) -> bool {
    if !stmt.is_executable_invocation()
        || has_projected_boundary_effects(stmt, registry, context, argument_input)
    {
        return false;
    }
    match stmt {
        Statement::Barrier { .. } | Statement::NativeCall { .. } | Statement::UpFrame { .. } => {
            true
        }
        Statement::Call { .. } => {
            match registry_resolution_with_metadata_context(stmt, registry, context) {
                Some(RegistryInvocationResolution::Resolved(facts)) => {
                    let subcommand_is_determinate = matches!(
                        &facts.subcommand,
                        tcl_registry::OwnedSubcommandResolution::NotApplicable
                            | tcl_registry::OwnedSubcommandResolution::Exact { .. }
                            | tcl_registry::OwnedSubcommandResolution::UniquePrefix { .. }
                    );
                    !subcommand_is_determinate || facts.traits.intersects(CLOBBER_TRAITS)
                }
                Some(RegistryInvocationResolution::Unresolved(_)) | None => true,
            }
        }
        _ => false,
    }
}

/// Union-find over [`MemoryLocation`] values with per-root reason
/// aggregation. Used by [`compute_aliases`] to merge aliases
/// discovered by multiple detection paths.
#[derive(Default)]
struct AliasUnionFind {
    parent: HashMap<MemoryLocation, MemoryLocation>,
    reasons: HashMap<MemoryLocation, BTreeSet<String>>,
}

impl AliasUnionFind {
    fn find(&mut self, loc: &MemoryLocation) -> MemoryLocation {
        if !self.parent.contains_key(loc) {
            self.parent.insert(loc.clone(), loc.clone());
            self.reasons.insert(loc.clone(), BTreeSet::new());
            return loc.clone();
        }
        // Path compression via iterative walk.
        let mut node = loc.clone();
        loop {
            let parent = self.parent.get(&node).expect("node registered").clone();
            if parent == node {
                break;
            }
            node = parent;
        }
        // Compress.
        let root = node.clone();
        let mut curr = loc.clone();
        while curr != root {
            let next = self.parent.get(&curr).expect("node registered").clone();
            self.parent.insert(curr, root.clone());
            curr = next;
        }
        root
    }

    fn union(&mut self, a: &MemoryLocation, b: &MemoryLocation, reason: &str) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            self.reasons
                .entry(ra)
                .or_default()
                .insert(reason.to_owned());
            return;
        }
        // Merge rb into ra.
        let rb_reasons = self.reasons.remove(&rb).unwrap_or_default();
        self.parent.insert(rb, ra.clone());
        let entry = self.reasons.entry(ra).or_default();
        for r in rb_reasons {
            entry.insert(r);
        }
        entry.insert(reason.to_owned());
    }
}

/// Scan the SSA function for registry-declared variable-cell aliases and build
/// alias sets.
///
/// Uses union-find to merge transitive/overlapping aliases into
/// connected components — for example, `upvar 1 x a; upvar 1 x b`
/// correctly merges `a` and `b` into the same alias set because
/// they share the caller-side variable `x`.
///
/// The registry receives the explicitly resolved `context`; this pass never
/// falls back to an all-dialects interpretation of command or subcommand
/// shapes.
#[must_use]
pub fn compute_aliases(
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> Vec<AliasSet> {
    compute_aliases_with_metadata_context(
        ssa,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
    )
}

pub(crate) fn compute_aliases_with_metadata_context(
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> Vec<AliasSet> {
    let mut uf = AliasUnionFind::default();

    // Walk blocks in deterministic id order so alias sets are
    // reproducible across runs.
    let mut block_ids: Vec<BlockId> = ssa.blocks.keys().copied().collect();
    block_ids.sort_unstable();
    for &bid in &block_ids {
        let block = &ssa.blocks[&bid];
        for stmt_ssa in &block.statements {
            let stmt = &stmt_ssa.statement;

            for pair in transition_alias_pairs_with_metadata_context(stmt, registry, context) {
                uf.union(&pair.target, &pair.local, pair.reason);
            }
        }
    }

    // Build connected components.
    let mut components: HashMap<MemoryLocation, HashSet<MemoryLocation>> = HashMap::new();
    let all_locs: Vec<MemoryLocation> = uf.parent.keys().cloned().collect();
    for loc in all_locs {
        let root = uf.find(&loc);
        components.entry(root).or_default().insert(loc);
    }

    let mut alias_sets: Vec<AliasSet> = Vec::new();
    // Sort roots by their display form for deterministic output.
    let mut roots: Vec<MemoryLocation> = components.keys().cloned().collect();
    roots.sort_by_key(MemoryLocation::display);
    for root in roots {
        let reasons = uf.reasons.get(&root).cloned().unwrap_or_default();
        let reason = if reasons.is_empty() {
            "alias".to_string()
        } else {
            reasons.into_iter().collect::<Vec<_>>().join(",")
        };
        let locs = components.remove(&root).unwrap_or_default();
        alias_sets.push(AliasSet::new(locs, reason));
    }
    alias_sets
}

/// Whether any invocation in `ssa` has a wildcard transition obligation.
///
/// An unresolved head, missing structured tokens, unstamped generic invoke,
/// or registry transition widening must clobber memory rather than being
/// mistaken for an empty alias declaration.
#[must_use]
pub fn has_wildcard_aliasing(
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> bool {
    has_wildcard_aliasing_with_metadata_context(
        ssa,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
    )
}

pub(crate) fn has_wildcard_aliasing_with_metadata_context(
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> bool {
    ssa.point_contexts
        .as_ref()
        .is_some_and(super::variable_bindings::PointResolveContexts::has_unknown_source_arguments)
        || ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .any(|statement| {
                statement_has_wildcard_aliasing_with_metadata_context(
                    &statement.statement,
                    registry,
                    context,
                )
            })
}

/// Build memory-SSA annotations for an SSA function.
///
/// Produces versioned memory operations (defs, uses, phis,
/// clobbers) and alias sets. Memory versions increment at each
/// store to an aliased location and at clobber points (barriers /
/// eval / uplevel).
///
/// Walks blocks in dominator-tree order (reverse iteration for
/// stack emulation) for consistent versioning.
#[must_use]
pub fn build_memory_ssa(
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> MemorySsaFunction {
    build_memory_ssa_with_metadata_context(
        ssa,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
    )
}

pub(crate) fn build_memory_ssa_with_metadata_context(
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> MemorySsaFunction {
    let alias_sets = compute_aliases_with_metadata_context(ssa, registry, context);
    let has_wildcard_aliasing = has_wildcard_aliasing_with_metadata_context(ssa, registry, context);
    let aliased_names: BTreeSet<String> = alias_sets.iter().flat_map(AliasSet::names).collect();

    let mut memory_ops: Vec<MemoryOp> = Vec::new();
    let mut memory_phis: HashMap<String, Vec<MemoryOp>> = HashMap::new();
    let mut version_counter: Version = 0;

    let mut visited: BTreeSet<BlockId> = BTreeSet::new();
    let mut stack: Vec<BlockId> = vec![ssa.entry];

    while let Some(bid) = stack.pop() {
        if visited.contains(&bid) || !ssa.blocks.contains_key(&bid) {
            continue;
        }
        visited.insert(bid);

        let block = &ssa.blocks[&bid];
        let bn = ssa.block_name(bid);

        // Memory phis at merge points: one phi per aliased variable
        // that has a scalar phi here.
        let mut block_phis: Vec<MemoryOp> = Vec::new();
        for phi in &block.phis {
            let phi_name = ssa.var_name(phi.name);
            if aliased_names.contains(phi_name) {
                version_counter += 1;
                let op = MemoryOp::new_phi(
                    MemoryLocation::new(MemoryLocationKind::Local, phi_name),
                    version_counter,
                    bn,
                );
                block_phis.push(op.clone());
                memory_ops.push(op);
            }
        }
        if !block_phis.is_empty() {
            memory_phis.insert(bn.to_string(), block_phis);
        }

        // Statements: clobbers, then defs, then uses.
        for (idx, stmt_ssa) in block.statements.iter().enumerate() {
            let stmt = &stmt_ssa.statement;
            let idx_i32 = i32::try_from(idx).unwrap_or(i32::MAX);

            if ssa
                .point_contexts
                .as_ref()
                .is_some_and(|points| points.source_arguments_unknown_at(bid, idx, registry))
                || is_clobber_with_metadata_context(stmt, registry, context)
                || transition_requires_wildcard_with_metadata_context(stmt, registry, context)
            {
                version_counter += 1;
                memory_ops.push(MemoryOp::new_clobber(version_counter, bn, idx_i32));
                // Fall through — a barrier that also defines
                // aliased vars still emits its defs.
            }

            // The version reaching this statement's *reads*, snapshotted before
            // the statement's own defs bump the counter.  A self-referential
            // aliased write (`upvar c x; set x [expr {$x + 1}]`) reads the
            // *incoming* version of `x`, not the one it is defining — tagging
            // the use with the post-def `version_counter` would record the
            // write as the read's own reaching def.
            let reaching_version = version_counter;

            for &sym in stmt_ssa.defs.keys() {
                let name = ssa.var_name(sym);
                if aliased_names.contains(name) {
                    version_counter += 1;
                    memory_ops.push(MemoryOp::new_def(
                        MemoryLocation::new(MemoryLocationKind::Local, name),
                        version_counter,
                        bn,
                        idx_i32,
                    ));
                }
            }

            for &sym in stmt_ssa.uses.keys() {
                let name = ssa.var_name(sym);
                if aliased_names.contains(name) {
                    memory_ops.push(MemoryOp::new_use(
                        MemoryLocation::new(MemoryLocationKind::Local, name),
                        reaching_version,
                        bn,
                        idx_i32,
                    ));
                }
            }
        }

        // Push dominator-tree children in reverse so the iterative
        // stack visits them left-to-right, preserving the recursive
        // visitation order.
        if let Some(children) = ssa.dominator_tree.get(&bid) {
            for &child in children.iter().rev() {
                stack.push(child);
            }
        }
    }

    let count_defs = memory_ops
        .iter()
        .filter(|o| o.kind == MemoryOpKind::Def)
        .count();
    let count_uses = memory_ops
        .iter()
        .filter(|o| o.kind == MemoryOpKind::Use)
        .count();
    let count_clobbers = memory_ops
        .iter()
        .filter(|o| o.kind == MemoryOpKind::Clobber)
        .count();

    MemorySsaFunction {
        cell_state: None,
        alias_sets,
        memory_ops,
        memory_phis,
        count_defs,
        count_uses,
        count_clobbers,
        has_wildcard_aliasing,
    }
}

/// Build memory facts from the executable CFG and shared bound-access owner.
/// Legacy alias sets remain conservative exposure summaries; reaching versions
/// are projections of cell-state SSA and do not use whole-function unions.
#[must_use]
pub fn build_memory_ssa_with_cfg(
    cfg: &crate::cfg::Function,
    ssa: &SsaFunction,
    points: &crate::variable_bindings::PointResolveContexts,
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
) -> MemorySsaFunction {
    build_memory_ssa_with_cfg_with_metadata_context(
        cfg,
        ssa,
        points,
        registry,
        context
            .or_else(|| registry.profile().map(SemanticContext::for_profile))
            .map(Into::into),
    )
}

pub(crate) fn build_memory_ssa_with_cfg_with_metadata_context(
    cfg: &crate::cfg::Function,
    ssa: &SsaFunction,
    points: &crate::variable_bindings::PointResolveContexts,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
) -> MemorySsaFunction {
    let mut memory = build_memory_ssa_with_metadata_context(ssa, registry, context);
    let Ok(cells) = crate::cell_state_ssa::build_cell_state_ssa(cfg, points, registry) else {
        memory.has_wildcard_aliasing = true;
        return memory;
    };
    memory.memory_ops.clear();
    memory.memory_phis.clear();
    for operation in cells.operations() {
        use crate::state_ssa::{CfgStatePosition, StateOp, StateSite};
        let (block, index) = match operation.site() {
            StateSite::Cfg(site) => (
                site.block,
                match site.position {
                    CfgStatePosition::Statement { index, .. } => {
                        i32::try_from(index).unwrap_or(i32::MAX)
                    }
                    CfgStatePosition::Phi { .. } => -1,
                    CfgStatePosition::Terminator { .. } => -2,
                },
            ),
            StateSite::Edge(site) => (site.predecessor, -2),
            StateSite::Node { .. } => continue,
        };
        let location = memory_location_of_place(operation.location());
        let block_name = cfg.block_name(block).to_owned();
        let projected = match operation {
            StateOp::Use(operation) => MemoryOp::new_use(
                location,
                operation.reaching_version.raw(),
                block_name,
                index,
            ),
            StateOp::Def(operation) => {
                MemoryOp::new_def(location, operation.version.raw(), block_name, index)
            }
            StateOp::Phi(operation) => {
                MemoryOp::new_phi(location, operation.version.raw(), block_name)
            }
            StateOp::Clobber(operation) => {
                let mut projected =
                    MemoryOp::new_clobber(operation.version.raw(), block_name, index);
                projected.location = location;
                projected
            }
        };
        if projected.kind == MemoryOpKind::Phi {
            memory
                .memory_phis
                .entry(projected.block.clone())
                .or_default()
                .push(projected.clone());
        }
        memory.memory_ops.push(projected);
    }
    memory.count_defs = memory
        .memory_ops
        .iter()
        .filter(|operation| operation.kind == MemoryOpKind::Def)
        .count();
    memory.count_uses = memory
        .memory_ops
        .iter()
        .filter(|operation| operation.kind == MemoryOpKind::Use)
        .count();
    memory.count_clobbers = memory
        .memory_ops
        .iter()
        .filter(|operation| operation.kind == MemoryOpKind::Clobber)
        .count();
    memory.cell_state = Some(cells);
    memory
}

fn memory_location_of_place(place: &crate::place::Place) -> MemoryLocation {
    use crate::place::PlaceKind;
    let kind = match place.kind {
        PlaceKind::Unknown => MemoryLocationKind::Unknown,
        PlaceKind::UpvarAlias => MemoryLocationKind::Upvar,
        PlaceKind::InstanceVar => MemoryLocationKind::InstanceVar,
        PlaceKind::ArrayElem | PlaceKind::ArrayWhole => MemoryLocationKind::ArrayElement,
        PlaceKind::Scalar | PlaceKind::DictPath if place.ns == crate::place::LOCAL_NS => {
            MemoryLocationKind::Local
        }
        PlaceKind::Scalar | PlaceKind::DictPath if place.ns == "::" => MemoryLocationKind::Global,
        PlaceKind::Scalar | PlaceKind::DictPath => MemoryLocationKind::NamespaceVar,
    };
    let qualifier = if let Some(cell) = &place.cell {
        format!(
            "{:?}:{:?}:{:?}",
            cell.owner,
            cell.generation,
            place.index.as_ref().map(|index| &index.value)
        )
    } else {
        place.ns.clone()
    };
    let key = crate::var_resolve::cell_key(place);
    let mut location = MemoryLocation::with_qualifier(kind, key.compatibility_name(), qualifier);
    location.storage_key = crate::var_resolve::canonical_place_key(place);
    location
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_byte_location_tests;
