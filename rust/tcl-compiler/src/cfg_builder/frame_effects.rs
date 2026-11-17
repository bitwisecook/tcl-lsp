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

//! The one owner of a procedure's frame effects: which frames its `upvar`,
//! `uplevel`, `global` and evaluated bodies reach ([`FrameContext`],
//! [`FrameTarget`]), the procedure call edges, and the two summaries built
//! from them in one pass: the caller-frame summary ([`UpvarInfo`], projected
//! by [`super::upvar_info`]) and the global-frame summary
//! ([`GlobalWriteInfo`], projected by [`super::global_write_info`]).

use std::collections::{BTreeSet, HashMap};

use tcl_registry::CommandRegistry;
use tcl_registry::frame_effect::FrameLevel;

use super::OriginalProcedureSummaryOwners;
use super::global_write_info::{self, GlobalWriteInfo};
use super::upvar_info::{self, UpvarInfo};
use crate::command_binding::{ModuleCommandBindings, ResolvedFrameBodySelection};
use crate::ir::{Module, ProcedureId};

/// A frame past a procedure's direct caller that its `upvar` / `uplevel`
/// effects reach.
///
/// A **level-1** effect (`upvar 1`, `uplevel 1`) writes the direct caller's
/// frame and stops there: it is emphatically not transitive through an
/// ordinary call, because the callee's caller *is* the wrapper
/// (`detect_upvar_procs_does_not_propagate_through_a_plain_call_wrapper`
/// pins the tclsh transcript). An effect `n` levels out lands `n - 1` levels
/// out of the direct caller, and [`super::detect_upvar_procs`] carries it one
/// ordinary call further per hop until it reaches its frame.
///
/// On tclsh 9.0.4 and 8.6.16, identically: with
/// `proc setUp2 {var} {uplevel 2 [list set $var 99]}`,
/// `proc middle {} {setUp2 answer}` and
/// `proc outer {} {middle; return $answer}`, `outer` returns `99` — `answer`
/// is genuinely set in `outer`'s frame by a proc `outer` never calls
/// directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FrameTarget {
    /// Exactly this many levels out of the procedure's own frame; always
    /// past the direct caller, so at least 2.
    Caller(std::num::NonZeroU32),
    /// A frame no static distance places: `uplevel #3`, a level word nothing
    /// static can read, or a distance past [`MAX_FRAME_DISTANCE`].
    Unknown,
}

/// The furthest exact distance a summary keeps; anything further is
/// [`FrameTarget::Unknown`], so call-edge closure always terminates.
pub const MAX_FRAME_DISTANCE: u32 = 32;

impl FrameTarget {
    /// The target `distance` levels out of a procedure's own frame, when it
    /// lies past the direct caller.
    #[must_use]
    pub fn past_the_caller(distance: u32) -> Option<Self> {
        match distance {
            0 | 1 => None,
            d if d > MAX_FRAME_DISTANCE => Some(Self::Unknown),
            d => std::num::NonZeroU32::new(d).map(Self::Caller),
        }
    }

    /// The same frame seen from the direct caller: one level nearer.
    /// `None` when it is then the direct caller's own caller-frame effect.
    #[must_use]
    pub fn one_call_nearer(self) -> Option<Self> {
        match self {
            Self::Caller(distance) => Self::past_the_caller(distance.get() - 1),
            Self::Unknown => Some(Self::Unknown),
        }
    }
}

/// The variable frame an evaluated body runs in, relative to the procedure
/// whose frame effects are being summarised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameContext {
    Own,
    DirectCaller,
    Global,
    Beyond(FrameTarget),
}

impl FrameContext {
    /// The frame `levels` relative levels out of this one. Out of the global
    /// frame there is no frame to select, so nothing static places it.
    pub(super) fn relative(self, levels: u32) -> Self {
        let distance = match self {
            Self::Own => 0,
            Self::DirectCaller => 1,
            Self::Global if levels == 0 => return Self::Global,
            Self::Global | Self::Beyond(FrameTarget::Unknown) => {
                return Self::Beyond(FrameTarget::Unknown);
            }
            Self::Beyond(FrameTarget::Caller(distance)) => distance.get(),
        };
        match distance.saturating_add(levels) {
            0 => Self::Own,
            1 => Self::DirectCaller,
            further => {
                Self::Beyond(FrameTarget::past_the_caller(further).unwrap_or(FrameTarget::Unknown))
            }
        }
    }
}

impl FrameContext {
    /// The frame `selection` names from this one.
    pub(super) fn select(self, selection: ResolvedFrameBodySelection) -> Self {
        match selection {
            ResolvedFrameBodySelection::Current => self,
            ResolvedFrameBodySelection::Selected(level) => self.level(level),
        }
    }

    /// The frame a level word names from this one.
    pub(super) fn level(self, level: FrameLevel) -> Self {
        match level {
            FrameLevel::Relative(levels) => self.relative(levels),
            FrameLevel::Absolute(0) => Self::Global,
            FrameLevel::Absolute(_) | FrameLevel::Dynamic => Self::Beyond(FrameTarget::Unknown),
        }
    }

    /// Whether this may be the global frame: the global frame itself, or one
    /// nothing static places. A caller frame that is the global frame at a
    /// top-level call is that call site's own caller-frame effect.
    pub(super) const fn may_be_global(self) -> bool {
        matches!(self, Self::Global | Self::Beyond(FrameTarget::Unknown))
    }
}

/// Each retained body's ordinary call targets, and whether it may call a
/// procedure the edges cannot name.
pub(super) type CallEdges = HashMap<ProcedureId, (BTreeSet<ProcedureId>, bool)>;

fn call_edges(
    module: &Module,
    owners: &OriginalProcedureSummaryOwners,
    registry: &CommandRegistry,
) -> CallEdges {
    module
        .summary_procedure_bodies()
        .map(|(id, procedure)| {
            (
                *id,
                global_write_info::direct_call_targets(&procedure.body, owners, registry),
            )
        })
        .collect()
}

/// Both frame-effect summaries of every retained body.
pub struct FrameEffectSummaries {
    /// What each body may do to its caller's frame and frames further out.
    pub caller: HashMap<ProcedureId, UpvarInfo>,
    /// What each body may write in the global frame.
    pub global: HashMap<ProcedureId, GlobalWriteInfo>,
}

/// Both summaries in one pass: one command-binding analysis and one set of
/// call edges, the caller-frame summary closed first because the
/// global-frame projection reads it.
pub fn detect_frame_effect_bodies(
    module: &Module,
    registry: &CommandRegistry,
) -> FrameEffectSummaries {
    let bindings = ModuleCommandBindings::analyse(module, registry);
    let owners = OriginalProcedureSummaryOwners::for_module(module, registry);
    let edges = call_edges(module, &owners, registry);
    let caller = caller_frame_summaries(module, registry, &bindings, &edges);
    let global = global_write_info::global_frame_summaries(
        module, registry, &bindings, &owners, &caller, &edges,
    );
    FrameEffectSummaries { caller, global }
}

/// The caller-frame summary alone.
pub(crate) fn detect_caller_frame_bodies(
    module: &Module,
    registry: &CommandRegistry,
) -> HashMap<ProcedureId, UpvarInfo> {
    let bindings = ModuleCommandBindings::analyse(module, registry);
    let owners = OriginalProcedureSummaryOwners::for_module(module, registry);
    caller_frame_summaries(
        module,
        registry,
        &bindings,
        &call_edges(module, &owners, registry),
    )
}

pub(super) fn opaque_frame_summary() -> UpvarInfo {
    UpvarInfo {
        has_unresolvable_caller_target: true,
        caller_frame_opaque_writes: true,
        caller_frame_opaque_reads: true,
        beyond_caller: BTreeSet::from([FrameTarget::Unknown]),
        ..UpvarInfo::default()
    }
}

fn caller_frame_summaries(
    module: &Module,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    edges: &CallEdges,
) -> HashMap<ProcedureId, UpvarInfo> {
    let owner = module
        .retained_source_bindings
        .as_ref()
        .filter(|owner| owner.matches_module(module, registry));
    let mut summaries: HashMap<_, _> = module
        .summary_procedure_bodies()
        .map(|(id, procedure)| {
            let namespace = procedure.body.namespace_context.as_deref();
            let mut info = if let Some(namespace) = namespace.filter(|_| {
                module.procedure_ids.owns(*id)
                    && owner.is_some_and(|owner| owner.matches_summary_body(*id, procedure))
            }) {
                upvar_info::collect_upvar_targets_with_bindings(
                    &procedure.body,
                    &procedure.params,
                    registry,
                    &bindings.entered_body_namespace(namespace),
                    namespace,
                    module,
                )
            } else {
                opaque_frame_summary()
            };
            if !info.uplevel_forwarded_calls.is_empty() {
                info = opaque_frame_summary();
            }
            info.param_defaults =
                upvar_info::name_param_defaults(&info, &procedure.params_raw, module.word_values());
            info.plain_calls.clear();
            (*id, info)
        })
        .collect();
    close_beyond_caller_reach(edges, &mut summaries);
    summaries
}

/// Carry each effect that lands past a callee's direct caller out along the
/// ordinary calls that reach it, one level per hop, to a fixpoint.
///
/// A callee's `Caller(2)` effect lands in the frame of whoever called its
/// caller, so it becomes the caller's own caller-frame barrier; a
/// `Caller(n)` further out becomes the caller's `Caller(n - 1)`. Exact
/// distances only shrink, and [`FrameTarget::Unknown`] — like a call the
/// edges cannot name — makes the whole caller opaque, so the closure ends.
fn close_beyond_caller_reach(edges: &CallEdges, summaries: &mut HashMap<ProcedureId, UpvarInfo>) {
    loop {
        let prior = summaries.clone();
        for (id, (calls, opaque)) in edges {
            let mut opaque = *opaque;
            let mut caller_frame = false;
            let mut further = BTreeSet::new();
            for callee in calls {
                let Some(summary) = prior.get(callee) else {
                    opaque = true;
                    break;
                };
                for target in &summary.beyond_caller {
                    match target.one_call_nearer() {
                        None => caller_frame = true,
                        Some(FrameTarget::Unknown) => opaque = true,
                        Some(nearer) => {
                            further.insert(nearer);
                        }
                    }
                }
            }
            if opaque {
                summaries.insert(*id, opaque_frame_summary());
                continue;
            }
            let Some(summary) = summaries.get_mut(id) else {
                continue;
            };
            if caller_frame {
                summary.caller_frame_opaque_writes = true;
                summary.caller_frame_opaque_reads = true;
            }
            summary.beyond_caller.extend(further);
        }
        if *summaries == prior {
            return;
        }
    }
}
