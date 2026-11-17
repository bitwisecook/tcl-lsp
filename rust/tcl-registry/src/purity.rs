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

//! Purity, derived from the four facts domains that decide it (issue #1712).
//!
//! An invocation is pure at a release when, there, its side effects are
//! [`Knowledge::Complete`] and empty, its world effects are complete and
//! touch nothing, its completion is complete and only normal or an error,
//! and its result stability is complete and referentially transparent. A
//! domain that is unknown, or only partly known, or complete for other
//! releases only, is not evidence of absence, so the call is not pure.

use crate::completion::{
    CompletionCode, CompletionCodeDomain, CompletionDescriptor, CompletionValueSemantics,
};
use crate::knowledge::Knowledge;
use crate::result_stability::ResultStability;
use crate::side_effects::SideEffect;
use crate::spec::{CommandSpec, SubCommand};
use crate::world_effect::{ResolvedWorldEffects, WorldEffectDescriptor};
use tcl_dialect::model::SurfaceQuery;

/// The four domains purity is derived from, as one invocation shape resolves
/// them.
#[derive(Debug, Clone, Copy)]
pub struct PurityDomains {
    /// The side effects.
    pub side_effects: Knowledge<&'static [SideEffect]>,
    /// The world effects, layer by layer.
    pub world_effects: ResolvedWorldEffects,
    /// The completion contract.
    pub completion: Knowledge<CompletionDescriptor>,
    /// The result stability.
    pub result_stability: Knowledge<ResultStability>,
}

impl PurityDomains {
    /// A command's own domains, before any subcommand or form.
    #[must_use]
    pub fn of_command(spec: &CommandSpec) -> Self {
        Self {
            side_effects: spec.side_effects,
            world_effects: ResolvedWorldEffects {
                command: spec.world_effects,
                ..ResolvedWorldEffects::default()
            },
            completion: spec.completion,
            result_stability: spec.result_stability,
        }
    }

    /// A subcommand's domains: each one it states replaces its command's,
    /// and world effects compose layer by layer.
    #[must_use]
    pub fn of_subcommand(spec: &CommandSpec, sub: &SubCommand) -> Self {
        Self {
            side_effects: sub.side_effects.or(spec.side_effects),
            world_effects: ResolvedWorldEffects {
                command: spec.world_effects,
                subcommand: sub.world_effects,
                ..ResolvedWorldEffects::default()
            },
            completion: sub.completion.or(spec.completion),
            result_stability: sub.result_stability.or(spec.result_stability),
        }
    }

    /// Whether the side effects are known to be none at `query`.
    #[must_use]
    pub fn side_effects_closed_at(&self, query: Option<&SurfaceQuery<'_>>) -> bool {
        self.side_effects.complete_and_empty_at(query)
    }

    /// Whether the world effects are known to touch nothing at `query`.
    #[must_use]
    pub fn world_effects_closed_at(&self, query: Option<&SurfaceQuery<'_>>) -> bool {
        self.world_effects.complete_and_empty_at(query)
    }

    /// Whether the call is known to finish only normally or with an error
    /// at `query`.
    #[must_use]
    pub fn completion_closed_at(&self, query: Option<&SurfaceQuery<'_>>) -> bool {
        self.completion
            .complete_at(query)
            .is_some_and(|descriptor| completes_normally_or_raises(*descriptor))
    }

    /// Whether the result is known to depend only on the arguments at
    /// `query`.
    #[must_use]
    pub fn result_stability_closed_at(&self, query: Option<&SurfaceQuery<'_>>) -> bool {
        self.result_stability.complete_at(query) == Some(&ResultStability::ReferentiallyTransparent)
    }

    /// Whether the invocation is pure at `query`: every one of the four
    /// domains is closed there.
    #[must_use]
    pub fn pure_at(&self, query: Option<&SurfaceQuery<'_>>) -> bool {
        self.side_effects_closed_at(query)
            && self.world_effects_closed_at(query)
            && self.completion_closed_at(query)
            && self.result_stability_closed_at(query)
    }
}

/// Whether a completion contract admits only `Ok` and `Error`, with no
/// value-dependent completion.
#[must_use]
pub fn completes_normally_or_raises(descriptor: CompletionDescriptor) -> bool {
    matches!(descriptor.value_semantics, CompletionValueSemantics::None)
        && matches!(descriptor.codes, CompletionCodeDomain::Exact(codes)
            if codes.iter().all(|code| matches!(code, CompletionCode::Ok | CompletionCode::Error)))
}

/// The four domains closed for the windows `surfaces` names: no side
/// effects, no world access, completion only normal or an error, and a
/// referentially transparent result. A row evidenced from its
/// implementation's source states this for exactly those windows.
#[must_use]
pub const fn closed_domains(
    surfaces: &'static [tcl_dialect::model::SpecSurface],
) -> (
    Knowledge<&'static [SideEffect]>,
    Knowledge<WorldEffectDescriptor>,
    Knowledge<CompletionDescriptor>,
    Knowledge<ResultStability>,
) {
    (
        Knowledge::Complete(&[], surfaces),
        Knowledge::Complete(WorldEffectDescriptor::EMPTY, surfaces),
        Knowledge::Complete(
            CompletionDescriptor::exact(&[CompletionCode::Ok, CompletionCode::Error]),
            surfaces,
        ),
        Knowledge::Complete(ResultStability::ReferentiallyTransparent, surfaces),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, SpecSurface};

    const NINE: &[SpecSurface] = &[SpecSurface::core_in(Family::Tcl, &[("9.0", None)])];

    fn closed_command() -> CommandSpec {
        let (side_effects, world_effects, completion, result_stability) = closed_domains(NINE);
        CommandSpec {
            name: "probe",
            side_effects,
            world_effects,
            completion,
            result_stability,
            ..CommandSpec::DEFAULT
        }
    }

    #[test]
    fn purity_needs_all_four_domains_closed_at_the_release() {
        let at = |release| SurfaceQuery::core(Family::Tcl, release);
        let spec = closed_command();
        let domains = PurityDomains::of_command(&spec);
        assert!(domains.pure_at(Some(&at("9.0"))) && domains.pure_at(Some(&at("9.1"))));
        assert!(
            !domains.pure_at(Some(&at("8.6"))),
            "one release never fills another"
        );
        assert!(!domains.pure_at(Some(&SurfaceQuery::any_release(Family::Tcl))));
        assert!(!domains.pure_at(None));

        let mut partial = spec.clone();
        partial.result_stability = Knowledge::Partial(ResultStability::ReferentiallyTransparent);
        assert!(!PurityDomains::of_command(&partial).pure_at(Some(&at("9.0"))));

        let mut breaks = spec.clone();
        breaks.completion = Knowledge::Complete(
            CompletionDescriptor::exact(&[CompletionCode::Ok, CompletionCode::Break]),
            NINE,
        );
        assert!(!PurityDomains::of_command(&breaks).pure_at(Some(&at("9.0"))));

        assert!(!PurityDomains::of_command(&CommandSpec::DEFAULT).pure_at(Some(&at("9.0"))));
    }

    #[test]
    fn a_subcommand_states_its_own_domains_or_inherits_its_commands() {
        let at = SurfaceQuery::core(Family::Tcl, "9.0");
        let spec = closed_command();
        let inherits = SubCommand {
            name: "inherits",
            ..SubCommand::DEFAULT
        };
        assert!(PurityDomains::of_subcommand(&spec, &inherits).pure_at(Some(&at)));
        let writes = SubCommand {
            name: "writes",
            world_effects: Knowledge::Partial(WorldEffectDescriptor::VARIABLE_WRITE),
            ..SubCommand::DEFAULT
        };
        assert!(!PurityDomains::of_subcommand(&spec, &writes).pure_at(Some(&at)));
    }
}
