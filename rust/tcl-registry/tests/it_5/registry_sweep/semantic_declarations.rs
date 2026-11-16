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

//! Semantic declarations: referentially transparent specs, CSE candidates, closed dispatch and declared event emissions.

use super::*;

/// One spec level's semantic-optimisation declarations — a command spec or one
/// of its subcommands — flattened so the sweeps below treat both uniformly.
/// The registry expresses the same four fields at either level, and a
/// consumer's resolution chain walks command → subcommand → form, so a sweep
/// that only looked at command specs would miss half the surface.
struct SemanticDeclarations {
    /// `command` or `command subcommand`, for the failure message.
    pub(super) path: String,
    /// The traits this level declares — both command specs and subcommands
    /// carry their own set.
    pub(super) traits: Traits,
    result_stability: Option<ResultStability>,
    pub(super) world_effects: Option<WorldEffectDescriptor>,
    pub(super) state_transitions: Option<StateTransitionDescriptor>,
    dispatch_dependencies: Option<DispatchDependencyDescriptor>,
}

/// Flatten one command spec and its subcommands into their declared
/// semantic-optimisation facts.
fn semantic_declarations(spec: &tcl_registry::CommandSpec) -> Vec<SemanticDeclarations> {
    let mut levels = vec![SemanticDeclarations {
        path: spec.name.to_owned(),
        traits: spec.traits,
        result_stability: spec.result_stability,
        world_effects: spec.world_effects,
        state_transitions: spec.state_transitions,
        dispatch_dependencies: spec.dispatch_dependencies,
    }];
    for sub in spec.subcommands {
        levels.push(SemanticDeclarations {
            path: format!("{} {}", spec.name, sub.name),
            traits: sub.traits,
            result_stability: sub.result_stability,
            world_effects: sub.world_effects,
            state_transitions: sub.state_transitions,
            dispatch_dependencies: sub.dispatch_dependencies,
        });
    }
    levels
}

/// Whether `descriptor` is the closed [`WorldEffectDescriptor::EMPTY`] — no
/// static access, no callback, no resolver, and nothing added to a parent.
/// The descriptor types are deliberately not `PartialEq` (they carry function
/// pointers), so the fields are compared explicitly.
fn is_empty_world_effect(descriptor: WorldEffectDescriptor) -> bool {
    descriptor.composition == WorldEffectComposition::Extend
        && descriptor.static_footprint.accesses.is_empty()
        && descriptor.static_footprint.callback == CallbackEffect::NONE
        && descriptor.resolver.is_none()
}

/// Whether `descriptor` is the closed [`StateTransitionDescriptor::EMPTY`] —
/// no resolver, no widening rule, no effect coverage, and nothing added to a
/// parent.
fn is_empty_state_transition(descriptor: StateTransitionDescriptor) -> bool {
    descriptor.composition == StateTransitionComposition::Extend
        && descriptor.resolver.is_none()
        && descriptor.argument_shape == StateTransitionArgumentShape::Independent
        && descriptor.dynamic_widening.is_empty()
        && descriptor.effect_coverage.is_empty()
        && descriptor.commit == StateTransitionCommit::OnOkOnly
}

/// A referentially transparent *result* claim is only safe for a
/// semantic optimisation when the same level also closes the world-effect and
/// state-transition questions.
///
/// `ResultStability::ReferentiallyTransparent` says the successful result
/// depends only on the evaluated arguments. On its own that says nothing about
/// what the invocation *does*: `world_effects: None` and
/// `state_transitions: None` both resolve conservatively, so a stable-call CSE
/// candidate carrying only the result claim would ride an open world contract —
/// the call the optimiser deletes could have been the one carrying the effect.
/// The declarations therefore travel together, which is what
/// `CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT` (and its `SubCommand`
/// counterpart) exists to make unavoidable.
///
/// registry-metadata: the semantic-optimisation contract is registry data.
#[test]
fn sweep_referentially_transparent_specs_declare_closed_dispatch_surface() {
    let mut open: BTreeSet<String> = BTreeSet::new();
    let mut checked = 0usize;

    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            for level in semantic_declarations(spec) {
                if level.result_stability != Some(ResultStability::ReferentiallyTransparent) {
                    continue;
                }
                checked += 1;
                if level.world_effects.is_none() {
                    open.insert(format!("{}: world_effects is None", level.path));
                }
                if level.state_transitions.is_none() {
                    open.insert(format!("{}: state_transitions is None", level.path));
                }
            }
        }
    }

    assert!(
        open.is_empty(),
        "referentially transparent specs left the world contract open — a CSE \
         candidate would inherit a conservative effect/transition resolution \
         from an unstamped field: {open:?}"
    );
    assert!(
        checked > 0,
        "the sweep must actually reach a ReferentiallyTransparent declaration"
    );
}

/// The `CSE_CANDIDATE` specs that still declare no result contract at any
/// level, recorded so the sweep below can be exact rather than weakened.
///
/// None of these takes a plain `CLOSED_REFERENTIALLY_TRANSPARENT` stamp
/// across the board, for three distinct reasons:
///
/// - the iRules data getters read the live connection, so their contract is a
///   `ResultStability::ReadsVersionedWorld` domain set, not referential
///   transparency;
/// - the dispatching Tcl commands (`binary`, `clock`, `dict`, `lsearch`,
///   `string`, `unicode`) need per-subcommand contracts, since their
///   subcommands differ (`clock seconds` is volatile, `clock format` is not);
/// - the Tcl 9.1 list-returning math commands are genuinely closed and can
///   take [`tcl_registry::CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT`]
///   directly.
///
/// The `PURE` trait does not separate these — 32 of the entries carry it — so
/// scoping the sweep by `PURE` would exempt almost the whole list while
/// pretending to a narrower contract. An exact list keeps the strong form of
/// the assertion instead.
const CSE_CANDIDATES_AWAITING_A_RESULT_CONTRACT: &[&str] = &[
    // iRules connection data getters (f5-irules).
    "HTTP::cookie",
    "HTTP::header",
    "HTTP::host",
    "HTTP::method",
    "HTTP::path",
    "HTTP::payload",
    "HTTP::query",
    "HTTP::request",
    "HTTP::request_num",
    "HTTP::status",
    "HTTP::uri",
    "HTTP::version",
    "IP::client_addr",
    "IP::local_addr",
    "IP::protocol",
    "IP::remote_addr",
    "IP::server_addr",
    "IP::tos",
    "SSL::cert",
    "SSL::cipher",
    "SSL::extensions",
    "TCP::bandwidth",
    "TCP::client_port",
    "TCP::local_port",
    "TCP::remote_port",
    "TCP::rtt",
    "TCP::server_port",
    "node",
    "virtual",
    // Dispatching Tcl commands whose subcommands differ.
    "binary",
    "clock",
    "dict",
    "lsearch",
    "string",
    "unicode",
    // Tcl 9.1 list-returning math commands.
    "divmod",
    "frexp",
    "modf",
    "remquo",
];

/// `Traits::CSE_CANDIDATE` marks a call the optimiser may prove
/// redundant against an earlier one. That proof needs the declared result
/// contract: without `result_stability`, resolution falls back to
/// `ResultStability::Unknown`, and a common-subexpression pass must fail closed
/// there. The trait and the declaration therefore have to be stamped together.
///
/// A subcommand-level candidate is satisfied by its parent command's
/// declaration, since the resolution chain inherits it.
///
/// The registry does not hold this contract everywhere yet, so the sweep pins
/// the shortfall exactly against
/// [`CSE_CANDIDATES_AWAITING_A_RESULT_CONTRACT`] and fails in both directions:
/// a newly-added candidate cannot join the list silently, and a spec that
/// gains a contract forces its entry to be deleted. Every candidate already
/// carrying a `world_effects` or `state_transitions` declaration — the whole
/// `CLOSED_REFERENTIALLY_TRANSPARENT` family — is held to the full contract
/// with no exemption.
///
/// registry-metadata: both the trait and the result contract are registry data.
#[test]
fn sweep_cse_candidates_declare_result_stability() {
    let mut undeclared: BTreeSet<String> = BTreeSet::new();
    let mut checked = 0usize;

    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            let inherited = spec.result_stability;
            for level in semantic_declarations(spec) {
                if !level.traits.contains(Traits::CSE_CANDIDATE) {
                    continue;
                }
                checked += 1;
                if level.result_stability.or(inherited).is_none() {
                    undeclared.insert(level.path);
                }
            }
        }
    }

    let known: BTreeSet<String> = CSE_CANDIDATES_AWAITING_A_RESULT_CONTRACT
        .iter()
        .copied()
        .map(str::to_owned)
        .collect();
    let unexpected: Vec<&String> = undeclared.difference(&known).collect();
    assert!(
        unexpected.is_empty(),
        "CSE_CANDIDATE without a declared result_stability — resolution would \
         fall back to ResultStability::Unknown and the pass must then abstain, \
         so the trait claims a redundancy the registry never described: \
         {unexpected:?}"
    );
    let stale: Vec<&String> = known.difference(&undeclared).collect();
    assert!(
        stale.is_empty(),
        "CSE_CANDIDATES_AWAITING_A_RESULT_CONTRACT is stale — these specs now \
         declare a result contract, so delete their entries: {stale:?}"
    );
    assert!(
        checked > 0,
        "the sweep must actually reach a CSE_CANDIDATE spec"
    );
}

/// A closed referentially transparent command declares exactly
/// the irreducible dispatch dependencies, not the conservative default.
///
/// `ResolvedDispatchDependencies::resolve` starts at
/// `DispatchDependencies::CONSERVATIVE` and unions `BASE` back in at the end,
/// so an unstamped spec keeps `ObjectDispatch` and `UnknownHandling` — two
/// domains such a command cannot depend on. It is not an object method, and,
/// being a resolvable registry command, it never enters missing-command
/// `unknown` fallback. Leaving those set costs a stable-call CSE the very
/// guards it would otherwise not need, so the base declares
/// `replace(DispatchDependencies::NONE)` and resolution restores exactly
/// `BASE`.
///
/// The assertion is made against the declared descriptor rather than through
/// `resolve_invocation`, so it covers every spec regardless of the argument
/// shape a minimal invocation would need.
///
/// registry-metadata: the dispatch-dependency declaration is registry data.
#[test]
fn sweep_closed_referentially_transparent_dispatch_resolves_to_base() {
    let mut wide: BTreeSet<String> = BTreeSet::new();
    let mut commands_checked = 0usize;

    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            let is_closed_command = spec.result_stability
                == Some(ResultStability::ReferentiallyTransparent)
                && spec.world_effects.is_some_and(is_empty_world_effect)
                && spec
                    .state_transitions
                    .is_some_and(is_empty_state_transition);
            if is_closed_command {
                commands_checked += 1;
            }
            for level in semantic_declarations(spec) {
                let closed = level.result_stability
                    == Some(ResultStability::ReferentiallyTransparent)
                    && level.world_effects.is_some_and(is_empty_world_effect)
                    && level
                        .state_transitions
                        .is_some_and(is_empty_state_transition);
                if !closed {
                    continue;
                }
                let resolved = ResolvedDispatchDependencies {
                    command: level.dispatch_dependencies,
                    ..ResolvedDispatchDependencies::default()
                }
                .resolve();
                if resolved != DispatchDependencies::BASE {
                    wide.insert(format!(
                        "{}: resolves to {:?}",
                        level.path,
                        resolved.iter().collect::<Vec<_>>()
                    ));
                }
            }
        }
    }

    assert!(
        wide.is_empty(),
        "closed referentially transparent specs must declare \
         replace(DispatchDependencies::NONE) so dispatch resolves to exactly \
         the irreducible BASE domains: {wide:?}"
    );
    assert!(
        commands_checked > 0,
        "the sweep must actually reach a CLOSED_REFERENTIALLY_TRANSPARENT \
         command spec"
    );
}

/// Every declared event emission names an event the registry
/// knows.
///
/// The descriptor's whole point is that consumers follow registry data instead
/// of growing command-name tables, so a typo in a target name would not fail
/// anywhere: the edge would simply never be built, and the reachability model
/// would quietly under-report. This is the drift gate the issue asks for.
///
/// It also pins the shape of the two seeded commands, because
/// `TCP::notify eom` declaring *no* events is a positive fact — it stops the
/// form inheriting a broader emission — and an empty list is exactly what an
/// accidental deletion looks like.
#[test]
fn declared_event_emissions_name_known_events() {
    // registry-metadata / f5-dialect: iRules events have no tclsh analogue.
    let registry = registry_for_dialect("f5-irules");
    let events = tcl_registry::events::EventRegistry::build();
    let known: BTreeSet<String> = events
        .all_event_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert!(
        known.contains("USER_REQUEST"),
        "the event table must be loaded for this check to mean anything",
    );

    let mut declaring = BTreeSet::new();
    for name in registry
        .command_names()
        .map(str::to_owned)
        .collect::<Vec<_>>()
    {
        for spec in registry.specs(&name) {
            let declared = spec
                .event_emits
                .into_iter()
                .chain(spec.event_emission_forms.iter().map(|form| form.emission));
            for emission in declared {
                declaring.insert(name.clone());
                for event in emission.events {
                    assert!(
                        known.contains(*event),
                        "{name} declares an emission of `{event}`, which is not in the \
                         event registry — a target that cannot be resolved builds no \
                         edge and silently under-reports reachability",
                    );
                }
            }
        }
    }

    assert!(
        declaring.contains("TCP::notify") && declaring.contains("NAME::lookup"),
        "the seeded commands must still declare an emission; got {declaring:?}",
    );
}
