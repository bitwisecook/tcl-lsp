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

//! Per-environment registry assembly on the new model (centralisation
//! §1.1's per-context registries).
//!
//! A [`ContextRegistry`] is one **registry generation**: the spec multimap
//! assembled for one resolved context by **provider filtering** — every
//! spec whose [`SurfaceDeclaration`]s admit the environment under its
//! world policy is included — rather than by the old per-profile bit
//! loading. Release-window filtering (the axis primary against each
//! row's applicability) stays a query-time concern, exactly as the old
//! mask query was.
//!
//! Generations are `Arc`-owned and cached by
//! `(environment id, registry generation, overlay hash, keyed-versions
//! hash, pack-overlay key)` — the same identity the environment layer
//! resolves, plus the pack-overlay content key the old
//! `registry_for_profile_if_built` door threads. The spec *sources* are
//! the very `&'static` groups `build_default`/`load_dialect` draw from
//! (statics stay `&'static`, shared with the old registries, no second
//! leak): each generation's [`ContextRegistry::commands`] store **is** the
//! old cache's `(profile, overlay)` `Arc`, shared by handle through the
//! [`command_store`] interop seam so the two models cannot drift while
//! both exist. **P2 seam, documented**: dynamic pack ingestion joins by
//! adding pack-owned declaration sources to the store inputs and bumping
//! the environment generation in the cache key; nothing dynamic may ever
//! be handed out as `&'static` (review B8).
//!
//! The two equivalence sweeps in this module's tests are the acceptance
//! gate of P1-E: for every compiled spec and every old catalogue profile,
//! old-model visibility equals new-model visibility, and each profile's
//! visible command-name set and per-name resolution answers are
//! reproduced exactly (deliberate-divergence allowlist: **empty**).

use std::cmp::Reverse;
use std::sync::{Arc, Mutex, OnceLock};

use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use tcl_dialect::model::{EnvironmentDefinition, EnvironmentIdentity};
use tcl_dialect::{DialectProfile, DialectSet};

use crate::model::context::{ContextQueries, KeyedVersions, ResolvedContext, specificity_breadth};
use crate::model::surface::{SurfaceDeclaration, declarations_for_spec};
use crate::registry::CommandRegistry;
use crate::registry::ResolvedCall;
use crate::resolved_invocation::ResolvedInvocation;
use crate::spec::CommandSpec;

/// The compiled spec universe: every group `build_default` and
/// `load_dialect` can reach, loaded once behind the same shared `&'static`
/// spec slices the old per-profile registries use.
///
/// The `TMSH` layer is deliberately not loaded: `tmsh_command_specs` is a
/// re-collection of specs `iapps_command_specs` already registers (the
/// `IAPPS|TMSH`-tagged shared surface), so loading both would double-enter
/// content-identical specs.
#[cfg(test)]
pub(crate) fn universe() -> &'static CommandRegistry {
    static CELL: OnceLock<CommandRegistry> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut registry = CommandRegistry::build_default();
        for pack in [
            DialectSet::BPF,
            DialectSet::IRULES,
            DialectSet::IAPPS,
            DialectSet::EXPECT,
            DialectSet::SPECTCL,
        ] {
            registry.load_dialect(pack);
        }
        registry
    })
}

/// One admitted spec with its translated declarations and precomputed
/// specificity breadth. Position in the per-name list is the registration
/// index — the last-registered-wins tiebreak, as in the old multimap.
struct SpecEntry {
    spec: &'static CommandSpec,
    declarations: SmallVec<[SurfaceDeclaration; 2]>,
    breadth: u32,
}

/// One assembled, `Arc`-owned registry generation for a resolved context
/// (see the module docs): the per-name spec multimap admitted by provider
/// filtering, the context its queries answer under, and the generation's
/// **command store** — the same per-`(environment, pack overlay)` spec
/// store the old per-profile cache owns, shared by handle so the two
/// models can never drift while both exist (the P1-G deletion re-homes
/// ownership here).
pub struct ContextRegistry {
    context: ResolvedContext,
    commands: Arc<CommandRegistry>,
    entries: FxHashMap<&'static str, Vec<SpecEntry>>,
}

impl ContextRegistry {
    /// Assemble a generation for `context` over the `commands` store:
    /// admit every store spec with some declaration whose provider is
    /// active (and whose predicate holds) under the context's world
    /// policy, and record the store's pack-declared ambient rows on the
    /// context.
    fn assemble(mut context: ResolvedContext, commands: Arc<CommandRegistry>) -> Self {
        for &(package, version) in commands.ambient_package_rows() {
            context.record_pack_ambient(package, version);
        }
        let mut entries: FxHashMap<&'static str, Vec<SpecEntry>> = FxHashMap::default();
        let names: Vec<&'static str> = commands
            .command_names()
            .filter_map(|name| commands.specs(name).first().map(|spec| spec.name))
            .collect();
        for name in names {
            let mut admitted: Vec<SpecEntry> = Vec::new();
            for spec in commands.specs(name) {
                let declarations = declarations_for_spec(spec);
                let in_world = declarations.iter().any(|declaration| {
                    context.provider_active(&declaration.provider)
                        && context.predicate_passes(&declaration.predicate)
                });
                if in_world {
                    let breadth = specificity_breadth(&declarations);
                    admitted.push(SpecEntry {
                        spec,
                        declarations,
                        breadth,
                    });
                }
            }
            if !admitted.is_empty() {
                // `command_names` yields each registered key once; the
                // specs (and their `'static` names) keep registration
                // order.
                entries.insert(name, admitted);
            }
        }
        Self {
            context,
            commands,
            entries,
        }
    }

    /// The context this generation answers under.
    #[must_use]
    pub fn context(&self) -> &ResolvedContext {
        &self.context
    }

    /// The generation's command store — the spec content this context was
    /// assembled over, including any pack overlay. Consumers that iterate
    /// or read raw spec data (segmentation recovery's known-name set, hook
    /// tables, event data) read it here; availability questions go through
    /// the context queries.
    #[must_use]
    pub fn commands(&self) -> &Arc<CommandRegistry> {
        &self.commands
    }

    /// Every assembled command name (admitted at assembly; a name may
    /// still resolve to nothing at query time when the release window
    /// excludes it).
    pub fn command_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.entries.keys().copied()
    }

    /// Resolve `name` under this context — the assistance-view command
    /// resolution (centralisation R-c's assistance column), layered over
    /// the multimap with the generalised most-specific-wins rule.
    ///
    /// **Selection, then the package conjunct** — mirroring the old
    /// `get_for_dialect → is_available` layering: among the specs with a
    /// declaration admitted for selection, the winner has the **narrowest
    /// total applicability breadth** ([`specificity_breadth`]; the
    /// documented tiebreaks are: a scoped spec beats the universal
    /// `dialects: None` translation because the universal breadth of 22
    /// exceeds every explicit gate's maximum of 13, and among equal
    /// breadths the **last-registered** spec wins so curated overrides
    /// keep beating the data they shadow). The winner then still has to
    /// pass the full [`ContextQueries::is_available`], which adds the
    /// closed-world required-package conjunct — a winner failing it
    /// resolves to nothing rather than falling back to a wider loser,
    /// exactly as the old layering behaved.
    ///
    /// A leading `::` falls back to the bare name, as the old `get` family
    /// did.
    #[must_use]
    pub fn resolve_command(&self, name: &str) -> Option<&'static CommandSpec> {
        let candidates = self.entries.get(name).or_else(|| {
            name.strip_prefix("::")
                .and_then(|bare| self.entries.get(bare))
        })?;
        let winner = candidates
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry
                    .declarations
                    .iter()
                    .any(|declaration| self.context.admits_for_selection(declaration))
            })
            .max_by_key(|&(index, entry)| (Reverse(entry.breadth), index))
            .map(|(_, entry)| entry)?;
        self.context
            .is_available(&winner.declarations)
            .then_some(winner.spec)
    }

    /// The visible command-name set: every assembled name that resolves
    /// under this context, sorted.
    #[must_use]
    pub fn visible_command_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self
            .entries
            .keys()
            .filter(|name| self.resolve_command(name).is_some())
            .copied()
            .collect();
        names.sort_unstable();
        names
    }
}

impl std::fmt::Debug for ContextRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextRegistry")
            .field("environment", &self.context.environment.id)
            .field("names", &self.entries.len())
            .finish_non_exhaustive()
    }
}

/// The generation cache key: the environment's resolved identity
/// (id, registry generation, overlay hash), the keyed-versions hash, and
/// the pack-overlay content key.
type GenerationKey = (EnvironmentIdentity, u64, u64);

/// The interned catalogue profile whose command store backs
/// `environment_id`'s generations — the wave-1 interop seam (P1-F): the
/// catalogue environments share their canonical id with their old
/// profile, and the model-only environments (`tcl`, `tk`, third-party
/// ids) fall back to the permissive plain profile, exactly the store
/// every unresolved dialect string read before the port. Deleted with the
/// old cache in P1-G, when the store becomes environment-owned.
fn store_profile(environment_id: &str) -> &'static DialectProfile {
    DialectProfile::find(environment_id).unwrap_or_else(DialectProfile::plain_tcl)
}

/// The command store for one `(environment, pack overlay)` generation —
/// the very `Arc` the old `(profile, overlay)` cache owns
/// ([`crate::cache::registry_for_profile_if_built`]), shared by handle:
/// same statics, no second copy, no drift while both models coexist.
///
/// Mirrors that function's contract exactly: overlay `0` is the
/// always-buildable un-overlaid store; a non-zero overlay is **look-up
/// only** — its contents come from a loader closure only `tcl-spectcl`
/// can write, so a miss returns `None` and the caller falls back to the
/// un-overlaid generation, exactly as the analyser always has.
fn command_store(environment_id: &str, overlay: u64) -> Option<Arc<CommandRegistry>> {
    crate::cache::registry_for_profile_if_built(store_profile(environment_id), overlay)
}

/// The per-context registry for `environment`, assembled on first use and
/// cached by `(identity, keyed-versions hash)` — the un-overlaid (pack
/// overlay `0`) generation, which always builds.
///
/// `identity` is the `(id, generation, overlay)` identity the environment
/// registry resolved `environment` under — pass the value from
/// [`tcl_dialect::model::EnvironmentRegistry::identity_of`] or
/// `apply_overlay`, so an overlaid environment can never alias its base's
/// generation. Cache entries are `Arc`-owned and bounded by the resolved
/// identities a process actually uses (a closed set today: compiled
/// environments × keyed pins); the P2 pack-ingestion seam adds generation
/// bumps and pruning alongside dynamic sources.
#[must_use]
pub fn registry_for_environment(
    environment: &Arc<EnvironmentDefinition>,
    identity: &EnvironmentIdentity,
    keyed: &KeyedVersions,
) -> Arc<ContextRegistry> {
    registry_for_environment_if_built(environment, identity, keyed, 0)
        .expect("the un-overlaid generation always builds")
}

/// [`registry_for_environment`] with the pack-overlay content key
/// threaded — the model mirror of the old
/// `registry_for_profile_if_built(profile, overlay)` door: overlay `0`
/// always builds; a non-zero overlay resolves only when its pack-carrying
/// store has been installed, and a miss returns `None` so the caller
/// falls back to the un-overlaid generation rather than caching a
/// pack-less generation under the pack's key forever.
#[must_use]
pub fn registry_for_environment_if_built(
    environment: &Arc<EnvironmentDefinition>,
    identity: &EnvironmentIdentity,
    keyed: &KeyedVersions,
    overlay: u64,
) -> Option<Arc<ContextRegistry>> {
    static CACHE: OnceLock<Mutex<FxHashMap<GenerationKey, Arc<ContextRegistry>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(FxHashMap::default()));
    let key: GenerationKey = (identity.clone(), keyed.content_hash(), overlay);
    if let Some(generation) = cache
        .lock()
        .expect("context registry cache mutex")
        .get(&key)
    {
        return Some(Arc::clone(generation));
    }
    // Assembled outside the lock; a racing thread's duplicate build is
    // dropped in favour of the first published entry. The store lookup
    // stays outside too: an overlay miss must not park a `None` in the
    // cache — the packs may be installed a moment later.
    let commands = command_store(environment.id.as_str(), overlay)?;
    let assembled = Arc::new(ContextRegistry::assemble(
        ResolvedContext::resolve(Arc::clone(environment), keyed),
        commands,
    ));
    let mut guard = cache.lock().expect("context registry cache mutex");
    prune_overlaid_generations(&mut guard, overlay);
    Some(Arc::clone(guard.entry(key).or_insert(assembled)))
}

/// Bound the generation cache the way the old overlay cache bounds
/// itself: past the cap, drop overlaid generations other than the one
/// being built — un-overlaid entries (a closed set) are retained
/// unconditionally, and dropping a table handle frees the generation once
/// its last holder finishes.
fn prune_overlaid_generations(
    map: &mut FxHashMap<GenerationKey, Arc<ContextRegistry>>,
    current: u64,
) {
    const GENERATION_LIMIT: usize = 64;
    if map.len() >= GENERATION_LIMIT {
        map.retain(|&(_, _, overlay), _| overlay == 0 || overlay == current);
    }
}

/// Resolve a concrete invocation to its target-neutral registry semantics
/// **in context** — the single selection primitive of centralisation
/// rows C3/C7 (`R-e`), used by the compiler's lowering-hook and
/// side-effect selection. `commands` is the store the caller reads (its
/// walk generation's, or a caller-supplied unit registry); `context` is
/// the resolved context the invocation executes under, when the caller
/// has resolved one.
///
/// P1a: selection here still precedes binding proof — the head is
/// resolved dialect-blind against the store (which is already
/// per-context at every current call site), exactly the behaviour of the
/// `resolve_invocation(…, DialectSet::empty())` call sites this replaces,
/// and the carried `context` is deliberately unread. Invariant I4 (no
/// taint/effect/lowering/codegen hook is selected before its binding is
/// proved) lands in P1a by replacing this body with
/// realm-`BindingKnowledge` proof over that context; call sites then
/// change behaviour only where a binding is genuinely unproved.
#[must_use]
pub fn resolve_invocation_in_context<'r, 'w>(
    commands: &'r CommandRegistry,
    _context: Option<&ResolvedContext>,
    name: &'w str,
    args: &'w [&'w str],
) -> Option<ResolvedInvocation<'r, 'w>> {
    commands.resolve_invocation(name, args, DialectSet::empty())
}

/// The legacy-selection twin of [`resolve_invocation_in_context`] for the
/// analyser-hook path, which resolves through the registry's
/// `resolve_call` compatibility selection (exact subcommand lookup) and
/// needs the composed hook stamps that projection carries.
///
/// P1a: same seam, same invariant (I4) — see
/// [`resolve_invocation_in_context`].
#[must_use]
pub fn resolve_call_in_context<'r>(
    commands: &'r CommandRegistry,
    _context: Option<&ResolvedContext>,
    name: &str,
    args: &[&str],
) -> Option<ResolvedCall<'r>> {
    commands.resolve_call(name, args, DialectSet::empty())
}

/// The structured side-effect hints for one invocation — the third face
/// of the C7 selection primitive, centralising the walk `side_effects.rs`
/// hand-rolled: newest-registered first over `command`'s specs in
/// `commands`, keeping only specs available in `context` (no context — no
/// filter, the old `dialect: None` behaviour), and returning the first
/// spec's subcommand-level hints (when `subcommand` resolves on it and
/// declares any) else its command-level hints.
///
/// The walk is deliberately **not** collapsed onto the single-winner
/// selection above: measured on the compiled universe, `classvariable`
/// and `next` under `bpf` (and their kin) draw hints from an *older*
/// available spec when the newest — or the most specific — spec carries
/// none, so a single-winner rule loses hints the shipped behaviour has.
///
/// P1a: hint selection precedes binding proof exactly as the hook
/// selection above does — invariant I4 replaces this walk with
/// realm-`BindingKnowledge` proof over the carried context.
#[must_use]
pub fn side_effect_hints_in_context<'r>(
    commands: &'r CommandRegistry,
    context: Option<&ResolvedContext>,
    command: &str,
    subcommand: Option<&str>,
) -> Option<&'r [crate::side_effects::SideEffect]> {
    for spec in commands.specs(command).iter().rev() {
        if let Some(context) = context
            && !context.spec_available(spec)
        {
            continue;
        }
        if let Some(sub_name) = subcommand
            && let Some(sub) = spec.resolve_subcommand(sub_name)
            && !sub.side_effects.is_empty()
        {
            return Some(sub.side_effects);
        }
        if !spec.side_effects.is_empty() {
            return Some(spec.side_effects);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::registry_for_profile;
    use crate::profile_queries::ProfileQueries;
    use std::collections::BTreeSet;
    use tcl_dialect::DialectProfile;
    use tcl_dialect::model::EnvironmentRegistry;

    fn new_registry_for(profile_name: &str) -> Arc<ContextRegistry> {
        let environments = EnvironmentRegistry::compiled();
        let definition = environments.resolve(profile_name).expect(profile_name);
        let identity = environments.identity_of(&definition);
        registry_for_environment(&definition, &identity, &KeyedVersions::default())
    }

    /// The identity facts two same-shaped spec copies share (the tmsh
    /// pack re-collects specs the iapps group also registers, so pointer
    /// identity alone cannot compare across the two models there).
    fn identity_of(
        spec: &'static CommandSpec,
    ) -> (
        &'static str,
        Option<DialectSet>,
        Option<&'static str>,
        Option<&'static str>,
    ) {
        (
            spec.name,
            spec.dialects,
            spec.required_package,
            spec.tcllib_package,
        )
    }

    /// **Acceptance gate 1 (P1-E)**: for EVERY spec in the compiled
    /// universe and EVERY old catalogue profile, old-model visibility
    /// (`ProfileQueries::is_available` — mask ∧ operator exclusion ∧
    /// package gate) equals new-model availability over the translated
    /// declarations. Divergence allowlist: none.
    #[test]
    fn per_spec_visibility_matches_the_old_model_for_every_profile() {
        let universe = universe();
        let environments = EnvironmentRegistry::compiled();
        let keyed = KeyedVersions::default();
        let translated: Vec<(&'static str, &'static CommandSpec, Vec<SurfaceDeclaration>)> =
            universe
                .command_names()
                .flat_map(|name| {
                    universe
                        .specs(name)
                        .iter()
                        .map(move |spec| (name, *spec, declarations_for_spec(spec).into_vec()))
                        .collect::<Vec<_>>()
                })
                .collect();
        let mut checks = 0usize;
        for profile in DialectProfile::all() {
            // F5 reclassification (measurements §4a): the iapps/tmsh
            // rows compare against the documented reclassified twin —
            // see `crate::model::f5_reclassified_oracle`.
            let oracle = crate::model::f5_reclassified_oracle(profile);
            let definition = environments.resolve(profile.name).expect(profile.name);
            let context = ResolvedContext::resolve(definition, &keyed);
            for (name, spec, declarations) in &translated {
                let old = oracle.is_available(spec);
                let new = context.is_available(declarations);
                assert_eq!(
                    old, new,
                    "`{name}` (gate {:?}, requires {:?}) under `{}`: old {old} vs new {new}",
                    spec.dialects, spec.required_package, profile.name
                );
                checks += 1;
            }
        }
        println!(
            "per-spec equivalence sweep: {} specs x {} profiles = {checks} checks, 0 divergences",
            translated.len(),
            DialectProfile::all().len(),
        );
    }

    /// **Acceptance gate 2 (P1-E)**: for each old profile, the
    /// corresponding environment's assembled registry has exactly the old
    /// `registry_for_profile` visible command-name set, and resolves every
    /// visible name to the same spec `best_visible` picked. Divergence
    /// allowlist: none.
    #[test]
    fn per_environment_visibility_reproduces_the_old_registries() {
        let mut names_checked = 0usize;
        for profile in DialectProfile::all() {
            // F5 reclassification (measurements §4a): compare against
            // the reclassified twin for iapps/tmsh — see
            // `crate::model::f5_reclassified_oracle`.
            let oracle = crate::model::f5_reclassified_oracle(profile);
            let old_registry = registry_for_profile(profile);
            let old_visible: BTreeSet<&str> = old_registry
                .command_names()
                .filter(|name| oracle.resolve_command(old_registry, name).is_some())
                .collect();
            let new_registry = new_registry_for(profile.name);
            let new_visible: BTreeSet<&str> =
                new_registry.visible_command_names().into_iter().collect();
            let only_old: Vec<&&str> = old_visible.difference(&new_visible).collect();
            let only_new: Vec<&&str> = new_visible.difference(&old_visible).collect();
            assert!(
                only_old.is_empty() && only_new.is_empty(),
                "`{}`: only-old {only_old:?}; only-new {only_new:?}",
                profile.name
            );
            for name in &old_visible {
                let old = oracle
                    .resolve_command(old_registry, name)
                    .expect("visible name resolves");
                let new = new_registry
                    .resolve_command(name)
                    .expect("visible name resolves in the new model");
                assert!(
                    std::ptr::eq(old, new) || identity_of(old) == identity_of(new),
                    "`{name}` under `{}` resolves differently: old {:?} vs new {:?}",
                    profile.name,
                    identity_of(old),
                    identity_of(new),
                );
                names_checked += 1;
            }
            println!(
                "`{}`: {} visible names match the old registry",
                profile.name,
                old_visible.len()
            );
        }
        println!(
            "per-environment sweep: {} profiles, {names_checked} resolved names, 0 divergences",
            DialectProfile::all().len()
        );
    }

    /// **C7 retirement gate**: the compiler's hand-rolled side-effect
    /// spec selection (newest-first over `specs(name)`, availability
    /// filter, first spec with a subcommand- or command-level hint) picks
    /// the same hints as the model-owned
    /// [`side_effect_hints_in_context`] primitive — for every store name,
    /// every catalogue profile, no subcommand and every declared
    /// subcommand spelling. (A single-winner selection is proven *not*
    /// equivalent here: see the primitive's docs for the measured
    /// counterexamples.)
    #[test]
    fn side_effect_hint_selection_matches_the_hand_rolled_rule() {
        use crate::profile_queries::ProfileQueries;
        use crate::side_effects::SideEffect;
        use tcl_dialect::DialectProfile;

        fn hand_rolled(
            registry: &CommandRegistry,
            profile: &DialectProfile,
            command: &str,
            subcommand: Option<&str>,
        ) -> Option<Vec<SideEffect>> {
            for spec in registry.specs(command).iter().rev() {
                if !profile.is_available(spec) {
                    continue;
                }
                if let Some(sub_name) = subcommand
                    && let Some(sub) = spec.resolve_subcommand(sub_name)
                    && !sub.side_effects.is_empty()
                {
                    return Some(sub.side_effects.to_vec());
                }
                if !spec.side_effects.is_empty() {
                    return Some(spec.side_effects.to_vec());
                }
            }
            None
        }

        fn via_primitive(
            generation: &ContextRegistry,
            _profile: &DialectProfile,
            command: &str,
            subcommand: Option<&str>,
        ) -> Option<Vec<SideEffect>> {
            side_effect_hints_in_context(
                generation.commands(),
                Some(generation.context()),
                command,
                subcommand,
            )
            .map(<[SideEffect]>::to_vec)
        }

        let mut checks = 0usize;
        for profile in DialectProfile::all() {
            // F5 reclassification (measurements §4a): the hand-rolled
            // rule runs on the reclassified twin for iapps/tmsh.
            let oracle = crate::model::f5_reclassified_oracle(profile);
            let profile: &DialectProfile = &oracle;
            let generation = new_registry_for(profile.name);
            let store = generation.commands();
            for name in store.command_names() {
                let mut sub_names: Vec<Option<&str>> = vec![None];
                for spec in store.specs(name) {
                    sub_names.extend(spec.subcommands.iter().map(|sub| Some(sub.name)));
                }
                sub_names.sort_unstable();
                sub_names.dedup();
                for subcommand in sub_names {
                    assert_eq!(
                        hand_rolled(store, profile, name, subcommand),
                        via_primitive(&generation, profile, name, subcommand),
                        "`{name}` {subcommand:?} under `{}`",
                        profile.name
                    );
                    checks += 1;
                }
            }
        }
        println!("side-effect selection sweep: {checks} checks, 0 divergences");
    }

    #[test]
    fn generations_cache_by_identity_and_keyed_hash() {
        let environments = EnvironmentRegistry::compiled();
        let definition = environments.resolve("f5-irules").expect("irules");
        let identity = environments.identity_of(&definition);
        let default_keyed = KeyedVersions::default();
        let first = registry_for_environment(&definition, &identity, &default_keyed);
        let second = registry_for_environment(&definition, &identity, &default_keyed);
        assert!(Arc::ptr_eq(&first, &second), "same key, same generation");
        let pinned = KeyedVersions {
            bigip: Some(tcl_dialect::model::Version::parse("17.1.0").expect("version")),
            ..KeyedVersions::default()
        };
        let third = registry_for_environment(&definition, &identity, &pinned);
        assert!(
            !Arc::ptr_eq(&first, &third),
            "a different keyed pin is a different generation"
        );
    }

    /// The generation's command store is the very `(profile, overlay)`
    /// `Arc` the old cache owns — shared by handle, so spec content (and
    /// everything reading it: recovery name sets, hooks, events) cannot
    /// drift between the two models while both exist.
    #[test]
    fn generations_share_the_old_caches_command_store() {
        let environments = EnvironmentRegistry::compiled();
        for name in ["tcl9.0", "f5-irules", "f5-iapps", "tk", "tcl"] {
            let definition = environments.resolve(name).expect(name);
            let identity = environments.identity_of(&definition);
            let generation =
                registry_for_environment(&definition, &identity, &KeyedVersions::default());
            let profile = DialectProfile::find(name).unwrap_or_else(DialectProfile::plain_tcl);
            let old = crate::cache::registry_handle_for_profile(profile);
            assert!(
                Arc::ptr_eq(generation.commands(), &old),
                "`{name}` must share the old cache's store"
            );
        }
    }

    /// The pack-overlay door mirrors `registry_for_profile_if_built`: an
    /// uninstalled overlay misses (the caller falls back to the
    /// un-overlaid generation), an installed overlay resolves to a
    /// generation over the pack-carrying store, and the store's
    /// `ambient_package` rows surface through the context's pack floors.
    #[test]
    fn pack_overlays_thread_through_the_generation_door() {
        const OVERLAY: u64 = 0x00F1_F00D;
        let environments = EnvironmentRegistry::compiled();
        let definition = environments.resolve("tcl8.6").expect("tcl8.6");
        let identity = environments.identity_of(&definition);
        let keyed = KeyedVersions::default();
        assert!(
            registry_for_environment_if_built(&definition, &identity, &keyed, OVERLAY).is_none(),
            "an uninstalled overlay must miss, never cache a pack-less generation"
        );
        let profile = DialectProfile::find("tcl8.6").expect("catalogue profile");
        let overlaid = crate::cache::registry_for_profile_with_overlay(profile, OVERLAY, |r| {
            r.insert_ambient_package("model-test-pack", "2.5");
        });
        let generation = registry_for_environment_if_built(&definition, &identity, &keyed, OVERLAY)
            .expect("the installed overlay resolves");
        assert!(Arc::ptr_eq(generation.commands(), &overlaid));
        assert_eq!(
            generation.context().pack_ambient_floor("model-test-pack"),
            Some("2.5")
        );
        assert!(generation.context().ambient_package("model-test-pack"));
        // The un-overlaid generation carries no pack rows.
        let plain = registry_for_environment(&definition, &identity, &keyed);
        assert_eq!(plain.context().pack_ambient_floor("model-test-pack"), None);
    }

    #[test]
    fn resolution_applies_the_rooted_name_fallback() {
        let registry = new_registry_for("tcl9.0");
        let bare = registry.resolve_command("foreach").expect("foreach");
        let rooted = registry.resolve_command("::foreach").expect("::foreach");
        assert!(std::ptr::eq(bare, rooted));
        assert!(registry.resolve_command("no-such-command").is_none());
    }

    /// The new-model-only environments behave sensibly even though no old
    /// profile pins them: the lenient `tk` environment resolves the Tk
    /// surface (through its Tcl core rows, with the hosted placement
    /// supplying the Tk-axis floor — review B11), and the closed
    /// `f5-irules` world never does.
    #[test]
    fn the_tk_environment_hosts_tk_without_a_vendor_bit() {
        let tk = new_registry_for("tk");
        assert!(tk.resolve_command("button").is_some());
        assert!(tk.resolve_command("lmap").is_some(), "core rides along");
        let irules = new_registry_for("f5-irules");
        assert!(
            irules.resolve_command("button").is_none(),
            "a closed world without a Tk placement never resolves Tk"
        );
    }
}
