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

//! Projection of registry-owned world effects into compiler state intents.
//!
//! This module is deliberately *before* state-SSA renaming.  It turns a
//! resolved [`tcl_registry::EffectFootprint`] into ordered uses, definitions,
//! and clobbers with typed [`crate::state_ssa::adapters::WorldRegion`]
//! locations.  It never fabricates reaching versions from source or CFG block
//! order; a later CFG-aware renamer consumes these intents and creates
//! [`crate::state_ssa::StateOp`] records.

use std::fmt;

use tcl_registry::{EffectAccessMode, EffectFootprint};

use crate::state_ssa::adapters::WorldRegion;
use crate::state_ssa::{CfgStatePosition, CfgStateSite, StateSite};

/// A version-free operation that a future world-state SSA renamer must place
/// on the executable control-flow graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldStateIntentKind {
    /// Read a world-state location before the invoking operation proceeds.
    Use,
    /// Write one world-state location.
    Def,
    /// Invalidate every location that overlaps the scoped location.
    Clobber,
}

/// One ordered, version-free world-state operation.
///
/// `site` orders operations from one semantic action deterministically.  It
/// is not evidence of execution order between CFG blocks and must not be used
/// as a reaching-version rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateIntent {
    /// Read, write, or clobber operation kind.
    pub kind: WorldStateIntentKind,
    /// Typed mutable world-state location.
    pub location: WorldRegion,
    /// Deterministic source or CFG anchor.
    pub site: StateSite,
}

/// A closed interprocedural callback summary.
///
/// Supplying this summary replaces the conservative all-world barrier for a
/// re-entrant callback.  The caller must establish that `footprint` describes
/// the complete transitive callback effect, including callbacks the callback
/// itself may invoke.  This module trusts that proof; it intentionally does
/// not recurse or infer it from call order.
#[derive(Debug, Clone, Copy)]
pub struct CallbackSummary<'a> {
    footprint: &'a EffectFootprint,
}

impl<'a> CallbackSummary<'a> {
    /// Mark a previously computed footprint as a closed callback summary.
    #[must_use]
    pub const fn closed(footprint: &'a EffectFootprint) -> Self {
        Self { footprint }
    }
}

/// Failure while assigning deterministic operation sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectProjectionError {
    /// The requested projection has more operations than its base site can
    /// represent with a `u32` ordinal.
    SiteOrdinalOverflow {
        /// Source or CFG anchor whose ordinal overflowed.
        base: StateSite,
    },
}

impl fmt::Display for EffectProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SiteOrdinalOverflow { .. } => {
                f.write_str("world-effect projection exhausted state-site ordinals")
            }
        }
    }
}

impl std::error::Error for EffectProjectionError {}

/// Project one resolved registry footprint into ordered world-state intents.
///
/// Each registry access retains its descriptor order. A
/// [`EffectAccessMode::ReadWrite`] access becomes a use followed immediately
/// by a definition of the same location. A scoped
/// [`EffectAccessMode::Clobber`] remains scoped; it does not unnecessarily
/// become an all-world clobber.
///
/// When the footprint reports a re-entrant callback, the final intents are an
/// all-world use followed by an all-world clobber unless `callback_summary`
/// proves a closed, narrower transitive footprint. The use preserves writes
/// that the callback can observe; the summary's own accesses are appended in
/// place of that conservative barrier.
///
/// This function emits no [`crate::state_ssa::StateOp`] because only a
/// CFG-aware renamer can supply the correct reaching and produced versions.
pub fn project_effect_footprint(
    footprint: &EffectFootprint,
    base: &StateSite,
    callback_summary: Option<CallbackSummary<'_>>,
) -> Result<Vec<WorldStateIntent>, EffectProjectionError> {
    let mut intents = Vec::new();
    append_footprint_intents(&mut intents, footprint, base)?;

    if footprint.requires_world_barrier() {
        if let Some(summary) = callback_summary {
            append_footprint_intents(&mut intents, summary.footprint, base)?;
        } else {
            push_intent(
                &mut intents,
                WorldStateIntentKind::Use,
                WorldRegion::Any,
                base,
            )?;
            push_intent(
                &mut intents,
                WorldStateIntentKind::Clobber,
                WorldRegion::Any,
                base,
            )?;
        }
    }

    Ok(intents)
}

fn append_footprint_intents(
    intents: &mut Vec<WorldStateIntent>,
    footprint: &EffectFootprint,
    base: &StateSite,
) -> Result<(), EffectProjectionError> {
    for access in footprint.accesses() {
        let location = WorldRegion::from_effect_access(access);
        match access.mode {
            EffectAccessMode::Read => {
                push_intent(intents, WorldStateIntentKind::Use, location, base)?;
            }
            EffectAccessMode::Write => {
                push_intent(intents, WorldStateIntentKind::Def, location, base)?;
            }
            EffectAccessMode::ReadWrite => {
                push_intent(intents, WorldStateIntentKind::Use, location.clone(), base)?;
                push_intent(intents, WorldStateIntentKind::Def, location, base)?;
            }
            EffectAccessMode::Clobber => {
                push_intent(intents, WorldStateIntentKind::Clobber, location, base)?;
            }
        }
    }
    Ok(())
}

fn push_intent(
    intents: &mut Vec<WorldStateIntent>,
    kind: WorldStateIntentKind,
    location: WorldRegion,
    base: &StateSite,
) -> Result<(), EffectProjectionError> {
    let ordinal = u32::try_from(intents.len())
        .map_err(|_| EffectProjectionError::SiteOrdinalOverflow { base: base.clone() })?;
    intents.push(WorldStateIntent {
        kind,
        location,
        site: offset_site(base, ordinal)?,
    });
    Ok(())
}

fn offset_site(base: &StateSite, offset: u32) -> Result<StateSite, EffectProjectionError> {
    let overflow = || EffectProjectionError::SiteOrdinalOverflow { base: base.clone() };
    match base {
        StateSite::Node { node, ordinal } => Ok(StateSite::Node {
            node: node.clone(),
            ordinal: ordinal.checked_add(offset).ok_or_else(overflow)?,
        }),
        StateSite::Cfg(CfgStateSite { block, position }) => {
            let position = match position {
                CfgStatePosition::Phi { ordinal } => CfgStatePosition::Phi {
                    ordinal: ordinal.checked_add(offset).ok_or_else(overflow)?,
                },
                CfgStatePosition::Statement { index, ordinal } => CfgStatePosition::Statement {
                    index: *index,
                    ordinal: ordinal.checked_add(offset).ok_or_else(overflow)?,
                },
                CfgStatePosition::Terminator { ordinal } => CfgStatePosition::Terminator {
                    ordinal: ordinal.checked_add(offset).ok_or_else(overflow)?,
                },
            };
            Ok(StateSite::Cfg(CfgStateSite {
                block: *block,
                position,
            }))
        }
        StateSite::Edge(edge) => {
            let origin = match edge.origin {
                CfgStatePosition::Phi { ordinal } => CfgStatePosition::Phi {
                    ordinal: ordinal.checked_add(offset).ok_or_else(overflow)?,
                },
                CfgStatePosition::Statement { index, ordinal } => CfgStatePosition::Statement {
                    index,
                    ordinal: ordinal.checked_add(offset).ok_or_else(overflow)?,
                },
                CfgStatePosition::Terminator { ordinal } => CfgStatePosition::Terminator {
                    ordinal: ordinal.checked_add(offset).ok_or_else(overflow)?,
                },
            };
            Ok(StateSite::Edge(crate::state_ssa::CfgEdgeStateSite {
                predecessor: edge.predecessor,
                successor: edge.successor,
                origin,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use tcl_registry::{
        CallbackEffect, CallbackKinds, EffectAccess, InterpreterScope, NamespaceScope, Reentrancy,
        SubjectScope, WorldStateDomain,
    };

    use super::*;
    use crate::ir::NodeId;
    use crate::state_ssa::adapters::{WorldInterpreterScope, WorldNamespaceScope, WorldRegionKind};

    fn base_site() -> StateSite {
        StateSite::node(NodeId::from_path(vec![17]), 4)
    }

    fn access(domain: WorldStateDomain, mode: EffectAccessMode) -> EffectAccess {
        EffectAccess::new(
            domain,
            mode,
            InterpreterScope::Current,
            NamespaceScope::Current,
            SubjectScope::Wildcard,
        )
    }

    #[test]
    fn read_write_projects_a_use_before_its_definition() {
        let mut footprint = EffectFootprint::default();
        footprint.add_access(access(
            WorldStateDomain::VariableStore,
            EffectAccessMode::ReadWrite,
        ));

        let intents =
            project_effect_footprint(&footprint, &base_site(), None).expect("projection succeeds");

        assert_eq!(
            intents.iter().map(|intent| intent.kind).collect::<Vec<_>>(),
            vec![WorldStateIntentKind::Use, WorldStateIntentKind::Def]
        );
        assert_eq!(
            intents[0].location,
            WorldRegion::domain_wildcard(
                WorldRegionKind::VariableStore,
                WorldInterpreterScope::Current,
                WorldNamespaceScope::Current,
            )
        );
        assert!(matches!(
            &intents[0].site,
            StateSite::Node { ordinal: 4, .. }
        ));
        assert!(matches!(
            &intents[1].site,
            StateSite::Node { ordinal: 5, .. }
        ));
    }

    #[test]
    fn scoped_clobber_stays_scoped_and_external_resources_keep_their_kind() {
        let mut footprint = EffectFootprint::default();
        footprint.add_access(EffectAccess::new(
            WorldStateDomain::NamespaceUnknown,
            EffectAccessMode::Clobber,
            InterpreterScope::named("i1"),
            NamespaceScope::named("::handlers"),
            SubjectScope::named("unknown"),
        ));
        footprint.add_access(EffectAccess::new(
            WorldStateDomain::LegacyExternal(
                tcl_registry::side_effects::SideEffectTarget::HttpHeader,
            ),
            EffectAccessMode::Read,
            InterpreterScope::Current,
            NamespaceScope::Current,
            SubjectScope::Wildcard,
        ));

        let intents =
            project_effect_footprint(&footprint, &base_site(), None).expect("projection succeeds");

        assert_eq!(intents[0].kind, WorldStateIntentKind::Clobber);
        assert_eq!(
            intents[0].location,
            WorldRegion::exact(
                WorldRegionKind::NamespaceUnknown,
                WorldInterpreterScope::named("i1"),
                WorldNamespaceScope::named("::handlers"),
                "unknown",
            )
        );
        assert_eq!(intents[1].kind, WorldStateIntentKind::Use);
        assert_eq!(
            intents[1].location,
            WorldRegion::domain_wildcard(
                WorldRegionKind::ExternalResource(
                    tcl_registry::side_effects::SideEffectTarget::HttpHeader,
                ),
                WorldInterpreterScope::Any,
                WorldNamespaceScope::Any,
            )
        );
        assert!(
            !intents
                .iter()
                .any(|intent| intent.location == WorldRegion::Any)
        );
    }

    #[test]
    fn reentrant_callbacks_add_an_all_world_barrier_without_a_closed_summary() {
        let mut reentrant = EffectFootprint::default();
        reentrant.add_access(access(
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
        ));
        reentrant.add_callback(CallbackEffect {
            kinds: CallbackKinds::TRACE,
            reentrancy: Reentrancy::CurrentInterpreter,
        });

        let conservative =
            project_effect_footprint(&reentrant, &base_site(), None).expect("projection succeeds");
        assert_eq!(conservative.len(), 3);
        assert_eq!(conservative[0].kind, WorldStateIntentKind::Def);
        assert_eq!(conservative[1].kind, WorldStateIntentKind::Use);
        assert_eq!(conservative[1].location, WorldRegion::Any);
        assert_eq!(conservative[2].kind, WorldStateIntentKind::Clobber);
        assert_eq!(conservative[2].location, WorldRegion::Any);

        let mut callback_effects = EffectFootprint::default();
        callback_effects.add_access(access(
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
        ));
        let closed = CallbackSummary::closed(&callback_effects);
        let summarised = project_effect_footprint(&reentrant, &base_site(), Some(closed))
            .expect("projection succeeds");
        assert_eq!(summarised.len(), 2);
        assert_eq!(summarised[0].kind, WorldStateIntentKind::Def);
        assert_eq!(summarised[1].kind, WorldStateIntentKind::Def);
        assert!(
            summarised
                .iter()
                .all(|intent| intent.location != WorldRegion::Any)
        );
    }

    #[test]
    fn unknown_invocation_projects_a_world_use_before_its_clobber() {
        let unknown = EffectFootprint::conservative_unknown_invocation();

        let intents =
            project_effect_footprint(&unknown, &base_site(), None).expect("projection succeeds");

        assert_eq!(intents.len(), 2);
        assert_eq!(intents[0].kind, WorldStateIntentKind::Use);
        assert_eq!(intents[0].location, WorldRegion::Any);
        assert_eq!(intents[1].kind, WorldStateIntentKind::Clobber);
        assert_eq!(intents[1].location, WorldRegion::Any);
    }
}
