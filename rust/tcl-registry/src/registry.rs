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

//! Command registry — lookup facade.
//!
//! Built once at startup from command spec modules, then queried by
//! every consumer. Supports dialect filtering and trait-membership
//! queries.

use std::cmp::Ordering;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use rustc_hash::FxHashMap;

#[cfg(test)]
use crate::abbrev::PrefixMatching;
#[cfg(test)]
use crate::arg_role::AppendedArity;
use crate::arg_role::ArgRole;
#[cfg(test)]
use crate::arity::Arity;
use crate::definer::DefinitionBodyGrammar;
#[cfg(test)]
use crate::forms::CommandForm;
#[cfg(test)]
use crate::hooks::LoweringHookId;
#[cfg(test)]
use crate::invocation_words::{InvocationWord, VariableWriteProjection};
#[cfg(test)]
use crate::lifecycle::Lifecycle;
use crate::resolved_invocation::ResolvedInvocation;
#[cfg(test)]
use crate::resolved_invocation::SubcommandResolution;
use crate::spec::CommandSpec;
#[cfg(test)]
use crate::spec::SubCommand;
use crate::traits::Traits;
#[cfg(test)]
use crate::{InvocationArguments, InvocationWords};
use tcl_dialect::model::Family;
use tcl_dialect::model::PackageFloor;
use tcl_dialect::model::SurfaceQuery;

#[path = "registry_regex_hazards.rs"]
mod regex_hazards;
#[path = "registry_role_consensus.rs"]
mod role_consensus;
use tcl_dialect::model::SurfaceLayer;
use tcl_dialect::model::surface_breadth;
use tcl_dialect::model::surface_nearness;

/// Where the words of one procedure definition sit — see
/// [`CommandRegistry::procedure_definition_words`]. Indices count words after
/// the command head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcedureWords {
    /// The procedure's name.
    pub name: usize,
    /// Its parameter list.
    pub params: usize,
    /// Its static-variable list, for a definer that takes one and a call that
    /// supplies it.
    pub statics: Option<usize>,
    /// Its body.
    pub body: usize,
}

impl ProcedureWords {
    /// Tcl's `proc name args body`, for a consumer that has no registry to ask.
    pub const TCL_PROC: Self = Self {
        name: 0,
        params: 1,
        statics: None,
        body: 2,
    };
}

/// Lookup facade over command specs.
///
/// The registry is built once from the command spec modules and then
/// queried read-only. All command-specific knowledge lives in the
/// specs — consumers never match on command name strings.
pub struct CommandRegistry {
    by_name: FxHashMap<&'static str, Vec<&'static CommandSpec>>,
    /// Specs installed through the public insertion seam, in registration
    /// order. These are authored overlays (including `SpecTcl` commands), not
    /// rows from the compiled-in command universe.
    ///
    /// Keeping that provenance beside the index lets an explicit-profile
    /// compiler projection filter the shipped surface exactly while retaining
    /// an authored `surface: None` override as the effective command binding.
    overlay_specs: Vec<&'static CommandSpec>,
    loaded_layers: Vec<SurfaceLayer>,
    /// The dialect profile this registry was built for, when it came from
    /// `registry_for_profile` / `registry_for_dialect`. `None` for
    /// hand-assembled registries (tests, ad-hoc tools), which fall back to
    /// the `loaded_layers`-derived behaviour answers.
    profile: Option<&'static tcl_dialect::DialectProfile>,
    /// Packages a `SpecTcl` pack declared **ambient** in this registry's
    /// dialect, with the version the runtime provides — `(package, version)`,
    /// in installation order.
    ///
    /// Empty for every compiled-in registry. A pack fills it through
    /// [`Self::insert_ambient_package`], which is how a package that comes
    /// *with* a dialect rather than being `package require`d gets a version
    /// floor at all.
    ///
    /// This is the pack-authored twin of [`tcl_dialect::LibraryPin`] with
    /// `ambient: true`. The profile axis is compiled in and describes the
    /// dialects this repository models; this one is authored outside it, which
    /// is the axis that has to exist before `tk` / `tcllib` / `iapps` can move
    /// to packs — a package's own version floor must not depend
    /// on whether this crate happens to know the package's name.
    ambient_packages: Vec<(&'static str, &'static str)>,
    /// The packages this registry's own point carries and the floor the
    /// registry guarantees of each — what [`Self::own_surface_query`] lends
    /// as the query's packages.
    ///
    /// Derived from [`Self::profile`] (which packages) and
    /// [`Self::package_floor`] (which release), so every seam that moves
    /// either refreshes it. Empty for a profile-less registry and for a
    /// profile whose point carries no package.
    own_packages: Vec<PackageFloor<'static>>,
    /// Special variables a `SpecTcl` pack declared with `special_var`, in
    /// installation order — the pack-authored rows [`Self::special_vars`]
    /// reads beside the shipped table.
    ///
    /// Empty for every compiled-in registry; a pack fills it through
    /// [`Self::insert_special_var`].
    special_vars: Vec<&'static crate::special_vars::SpecialVarSpec>,
    /// The member grammar of a **document** in this registry's dialect, when
    /// its command surface declares one.
    ///
    /// An authoring dialect whose file is itself a declaration body — a
    /// `.sslictcl` document, a `.tclspec` pack — has a fixed set of words that
    /// are legal at the root and nothing else. That is the same fact a
    /// [`DefinitionBodyGrammar`] states about a *nested* body, so it is stated
    /// the same way, and the generic consumers (completion, the token walker,
    /// folding) answer the root question with the machinery they already use
    /// for every level below it.
    ///
    /// `None` for an ordinary Tcl dialect, where the root is an open command
    /// position and the whole registry is the answer.
    document_grammar: Option<&'static DefinitionBodyGrammar>,
    /// Lazily-built facts for the one effective spec selected for every
    /// command in this exact registry generation.
    ///
    /// Registry construction and `SpecTcl` overlays are mutable, while all
    /// compiler consumers receive the completed registry read-only. Every
    /// mutation seam invalidates this cell, so a profile change or an authored
    /// override can never reuse facts from the preceding generation. Once the
    /// registry is frozen, command-binding and CFG consumers share the same
    /// `Arc` instead of independently walking the full command universe.
    effective_semantics: OnceLock<Arc<EffectiveRegistrySemantics>>,
    snapshot: OnceLock<RegistrySnapshot>,
    native_registrations: Arc<RwLock<NativeRegistrationCache>>,
    /// The workspace pack overlay this registry was built with
    /// ([`crate::registry_for_profile_with_overlay`]), when it was one.
    overlay: Option<u64>,
    /// The spec pack each installed pack command came from, keyed by the
    /// installed spec's address ([`Self::pack_origin`]). Every spec the
    /// registry indexes is `&'static` and never freed, so an address names
    /// one spec for the life of the process.
    pack_origins: FxHashMap<usize, crate::pack_origin::PackOrigin>,
    /// The text of the definition a `TclBody`-backed pack command's
    /// `PackageSource` pointer resolved to at load, keyed by the installed
    /// spec's address as [`Self::pack_origins`] is ([`Self::reference_body`]).
    reference_texts: FxHashMap<usize, Arc<str>>,
    /// This registry's generation: a number no other registry, and no
    /// earlier state of this one, has had. Every mutation draws a new one,
    /// so a memo keyed by it names exactly the command surface it resolved
    /// against.
    generation: u64,
}

#[derive(Default)]
struct NativeRegistrationCache {
    compilers: FxHashMap<
        (tcl_native_model::InvocationDialect, String),
        Option<crate::native_compilation::NativeCompilationSpec>,
    >,
    lookups: FxHashMap<
        (tcl_native_model::InvocationDialect, String),
        Option<crate::native_compilation::NativeCompilerImplementationLookup>,
    >,
    /// Each dialect's implementation-path entries by unrooted slot, in the
    /// registry's own iteration order, so a registration reads its slot's
    /// entries instead of every visible spec.
    prerequisites: FxHashMap<tcl_native_model::InvocationDialect, Arc<PrerequisiteIndex>>,
    /// Each dialect's registration slots by unrooted slot, so selecting one
    /// registration reads its slot instead of every visible spec.
    registration_slots: FxHashMap<tcl_native_model::InvocationDialect, Arc<RegistrationSlots>>,
    /// The few specs each registry-wide query can select, so those queries
    /// read a handful of specs instead of every registered command.
    query_surfaces: Option<Arc<RegistryQuerySurfaces>>,
    dialect_semantics: FxHashMap<
        (
            tcl_native_model::InvocationDialect,
            tcl_dialect::model::InvocationRealm,
        ),
        Arc<EffectiveRegistrySemantics>,
    >,
}

/// Specs a registry-wide query can select, in the registry's iteration order.
#[derive(Default)]
struct RegistryQuerySurfaces {
    /// Each name's last spec carrying [`Traits::OBJECT_COMMAND_SURFACE`].
    object_command_surfaces: Vec<&'static CommandSpec>,
    /// Each name's last spec with manufacturer rows or a definition grammar.
    definers: Vec<&'static CommandSpec>,
    /// Every spec with a data-collection operation.
    data_collection: Vec<&'static CommandSpec>,
}

impl RegistryQuerySurfaces {
    fn of(by_name: &FxHashMap<&'static str, Vec<&'static CommandSpec>>) -> Self {
        let mut surfaces = Self::default();
        for specs in by_name.values() {
            if let Some(&spec) = specs.last() {
                if spec.traits.contains(Traits::OBJECT_COMMAND_SURFACE) {
                    surfaces.object_command_surfaces.push(spec);
                }
                if !spec.manufacturer_methods.is_empty() || spec.definition_body.is_some() {
                    surfaces.definers.push(spec);
                }
            }
            surfaces.data_collection.extend(
                specs
                    .iter()
                    .copied()
                    .filter(|spec| spec.data_collection.is_some()),
            );
        }
        surfaces
    }
}

/// One implementation-path step a visible compilation names.
#[derive(Clone, Copy)]
struct PrerequisiteEntry {
    compilation: crate::native_compilation::NativeCompilationSpec,
    lookup: crate::native_compilation::NativeCompilerImplementationLookup,
    /// The path's last step of a `WithImplementationPath` compilation, which
    /// owns its descriptor.
    terminal: bool,
}

type PrerequisiteIndex = FxHashMap<&'static str, Vec<PrerequisiteEntry>>;

/// A dialect's registration slots by unrooted slot: the one lookup every
/// visible spec agrees on, or `None` when two of them disagree.
type RegistrationSlots =
    FxHashMap<&'static str, Option<crate::native_compilation::NativeCompilerImplementationLookup>>;

/// A registry generation no registry has had.
fn next_registry_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Shipped metadata for a selected fixed expression-function registration.
/// This is independent of the command-wrapper surface filtered by a Registry
/// projection and supplies no callable command or function-table receipt.
pub(super) fn fixed_math_function_metadata(bare: &str) -> Option<&'static CommandSpec> {
    let qualified = crate::mathfunc::qualified_name(bare);
    surfaces::installed()
        .surface("tcl")?
        .specs()
        .iter()
        .copied()
        .find(|spec| spec.name == qualified)
}

mod argument_roles;
mod construction;
mod control_arms;
mod effective_semantics;
mod instance_methods;
mod invocation_completion;
mod invocation_resolution;
mod irules;
mod lookup;
mod native_registration;
mod object_model;
mod script_timing;
mod snapshot;
pub mod surfaces;
mod trait_queries;
mod variable_projection;

pub use argument_roles::FormatStringArg;
#[cfg(test)]
use argument_roles::push_option_value_roles;
pub(crate) use argument_roles::{
    command_prefixes_in, layout_is_proven_in, pattern_args_in, sort_role_table, sub_options_at,
};
pub use control_arms::{
    ControlArmSemantics, TryClauseKind, TryCompletionSelector, TryControlClause,
    TryControlInvocation, selected_try_control_invocation,
};
pub(crate) use control_arms::{
    control_arm_semantics_for_hook, parse_selected_try_control_invocation,
    selected_try_control_arms,
};
pub use effective_semantics::{EffectiveCommandSemantics, EffectiveRegistrySemantics};
pub use instance_methods::InstanceMethodContextUnavailable;
pub use invocation_completion::{
    ExactInvocationCompletion, InvocationCompletion, InvocationCompletionKnowledge,
    authored_source_return_state_effect, native_return_state_effect,
};
pub(crate) use invocation_completion::{
    invocation_completion_for_selected, native_return_result_selection,
};
#[cfg(test)]
use invocation_resolution::successful_handler_contracts;
pub use invocation_resolution::{
    ResolvedCall, ResolvedTerminator, resolve_source_descriptor_invocation,
};
pub use irules::EventInfo;
pub use lookup::NameProviders;
pub use native_registration::NativeCompilerHooks;
pub use object_model::MethodDispatchKind;
pub use snapshot::{RegistrySemanticKey, RegistrySnapshot};

/// The `commands/<pack>/` module that declares `spec`.
///
/// `None` for a spec that reached the registry from a `.tclspec` pack — the
/// bundled EDA libraries, a workspace pack, the document under a spec
/// author's cursor. Provenance for those is the pack file, which the loader
/// reports; this deliberately does not guess one.
#[must_use]
pub fn spec_pack_of(spec: &CommandSpec) -> Option<&'static str> {
    surfaces::installed()
        .pack_by_identity()
        .get(&(std::ptr::from_ref(spec) as usize))
        .copied()
}

/// Every authoring pack that declares `name`, in browser order.
///
/// Nearly always one, and empty for a pack-loaded name. More than one means
/// a vendor or DSL surface re-declares the name with its own facts, and a
/// browser can say so instead of silently filing it under one of them.
#[must_use]
pub fn spec_packs_of(name: &str) -> &'static [&'static str] {
    surfaces::installed()
        .packs_by_name()
        .get(tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name))
        .map_or(&[][..], |packs| packs.as_slice())
}

/// The one numeral grammar `family` has at `release`, or across its whole
/// ladder when no release is pinned. `None` when the release is unknown or
/// the ladder's releases disagree.
fn point_number_syntax(
    family: Family,
    release: Option<&str>,
) -> Option<tcl_syntax::number::NumberSyntax> {
    let grammar_for = |release| tcl_dialect::model::grammar(family, release).numbers;
    if let Some(spelling) = release {
        family
            .releases()
            .iter()
            .find(|release| release.as_str() == spelling)
            .map(|&release| grammar_for(release))
            .or_else(|| {
                if family == Family::Tcl {
                    tcl_dialect::TclVersion::from_package_version(spelling)
                        .map(tcl_dialect::TclVersion::number_syntax)
                } else {
                    None
                }
            })
    } else {
        let mut releases = family.releases().iter().copied();
        let first = releases.next().map(grammar_for);
        first.filter(|&first| releases.all(|release| grammar_for(release) == first))
    }
}

impl CommandRegistry {
    /// Resolve `head` the way this registry's own availability rules
    /// resolve it: through [`Self::get_for_surface`] against the dialect
    /// profile this registry was built for, or through the dialect-agnostic
    /// [`Self::get`] when it has no profile.
    ///
    /// The single place the "same availability rules as `get`" promise made
    /// by the profile-aware behaviour queries is implemented, so a query
    /// added later cannot quietly answer for a command the profile's
    /// dialect does not have.
    fn spec_for_this_registry(&self, head: &str) -> Option<&CommandSpec> {
        match self.own_surface_query() {
            Some(query) => self.get_for_surface(head, Some(query)),
            None => self.get(head),
        }
    }

    /// The specs each registry-wide query can select, derived once per
    /// registry state.
    fn query_surfaces(&self) -> Arc<RegistryQuerySurfaces> {
        if let Some(surfaces) = &self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .query_surfaces
        {
            return Arc::clone(surfaces);
        }
        let surfaces = Arc::new(RegistryQuerySurfaces::of(&self.by_name));
        Arc::clone(
            self.native_registrations
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .query_surfaces
                .get_or_insert(surfaces),
        )
    }

    /// The single spec-selection rule (§5.3, D6): among the specs of one
    /// name visible under `dialect`, pick an admitted pack declaration before
    /// a shipped row, then the **most specific** surface — a dialect-scoped
    /// spec beats a catch-all (`surface: None`), a spec offered by a nearer
    /// core point of the query beats one offered only by a farther one (a
    /// document's own family over its ancestry anchor), a narrower surface
    /// beats a wider one, and among equals the last-registered spec wins.
    /// This lets a catch-all `-override` keep the collision policy it declared
    /// without weakening selection among the shipped variants it replaces.
    /// `get_for_surface`, the iRules event cross-product, and (via
    /// `ProfileQueries::resolve_command`) the CLI snapshot all resolve through
    /// this one rule.
    fn best_visible(
        &self,
        specs: &[&'static CommandSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static CommandSpec> {
        let mut best: Option<(usize, &CommandSpec)> = None;
        let mut best_is_pack = false;
        // Breadth only breaks a tie between two scoped candidates. Most names
        // have one visible spec, so calculating it before a tie is known
        // repeatedly walks their authored availability windows for no effect.
        // The best candidate's nearness is held for the same reason: it is
        // compared against every later candidate.
        let mut best_breadth: Option<u32> = None;
        let mut best_nearness: Option<usize> = None;
        // Nearness only distinguishes candidates when the query has more
        // than one core point; with one, every admitted row ranks `0`.
        let nearness_query = dialect.filter(|query| query.core.len() > 1);
        let nearness = |rows| {
            nearness_query.map_or(0, |query| {
                surface_nearness(rows, &query).unwrap_or(usize::MAX)
            })
        };

        for (index, spec) in specs.iter().copied().enumerate() {
            if !self.spec_visible(spec, dialect) {
                continue;
            }
            let candidate_is_pack = self.pack_origin(spec).is_some();
            let Some((best_index, best_spec)) = best else {
                best = Some((index, spec));
                best_is_pack = candidate_is_pack;
                continue;
            };
            if candidate_is_pack != best_is_pack {
                if candidate_is_pack {
                    best = Some((index, spec));
                    best_is_pack = true;
                    best_breadth = None;
                    best_nearness = None;
                }
                continue;
            }
            match (best_spec.surface, spec.surface) {
                (None, Some(_)) => {
                    best = Some((index, spec));
                    best_breadth = None;
                    best_nearness = None;
                }
                (Some(_), None) => {}
                (None, None) => {
                    // Equal catch-all scopes still use last registration.
                    best = Some((index, spec));
                }
                (Some(best_rows), Some(rows)) => {
                    let held = *best_nearness.get_or_insert_with(|| nearness(best_rows));
                    let candidate = nearness(rows);
                    match candidate.cmp(&held) {
                        Ordering::Less => {
                            best = Some((index, spec));
                            best_breadth = None;
                            best_nearness = Some(candidate);
                        }
                        Ordering::Greater => {}
                        Ordering::Equal => {
                            let old_breadth =
                                *best_breadth.get_or_insert_with(|| surface_breadth(best_rows));
                            let breadth = surface_breadth(rows);
                            if breadth < old_breadth
                                || (breadth == old_breadth && index > best_index)
                            {
                                best = Some((index, spec));
                                best_breadth = Some(breadth);
                            }
                        }
                    }
                }
            }
        }
        best.map(|(_, spec)| spec)
    }

    /// Discard derived command facts after a mutation to the command surface.
    fn invalidate_effective_semantics(&mut self) {
        self.effective_semantics = OnceLock::new();
        self.snapshot = OnceLock::new();
        self.native_registrations = Arc::default();
        self.generation = next_registry_generation();
    }

    /// Record the workspace pack overlay this registry carries; `0` is none.
    pub(crate) fn set_overlay(&mut self, overlay: u64) {
        self.overlay = (overlay != 0).then_some(overlay);
        self.invalidate_effective_semantics();
    }

    /// This registry's generation: the command surface an analysis resolved
    /// against, as the value-transfer context keys it. Every mutation draws
    /// a new one, and no two registries share one.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// The workspace pack overlay this registry was built with, as its
    /// content key (`PackSet::key`), or `None` for a registry with none.
    #[must_use]
    pub const fn overlay_generation(&self) -> Option<u64> {
        self.overlay
    }

    /// The version floor this registry itself guarantees for `spec`'s owning
    /// package. A profile supplies its pinned runtime library version; a
    /// `SpecTcl` ambient-package declaration supplies the pack equivalent.
    /// When both make a claim, their strongest floor wins.
    ///
    /// Request-time consumers that have `package require` facts can raise this
    /// floor through `DocumentFloor`; registry-only lookup deliberately keeps
    /// the static, resolved-profile answer. `None` remains permissive for an
    /// unpinned package or a hand-assembled registry with no ambient claim.
    fn package_floor_for_spec(&self, spec: &CommandSpec) -> Option<&'static str> {
        self.package_floor(spec.owning_package()?)
    }

    /// The version floor this registry itself guarantees for `package`: the
    /// profile's pinned runtime library version and the highest a loaded
    /// pack declared with `ambient_package`, the stronger of the two when
    /// both are stated.
    ///
    /// The one place the two claims are combined: a spec's owning package
    /// ([`Self::package_floor_for_spec`]) and a package the registry's own
    /// point carries ([`Self::own_surface_query`]) ask it alike.
    fn package_floor(&self, package: &str) -> Option<&'static str> {
        let profile_floor = self
            .profile
            .and_then(|profile| profile.library_floor_default(package));
        let ambient_floor = self.ambient_package_floor(package);
        match (profile_floor, ambient_floor) {
            (Some(profile), Some(ambient)) => (crate::version::compare(profile, ambient).is_lt())
                .then_some(ambient)
                .or(Some(profile)),
            (profile, ambient) => profile.or(ambient),
        }
    }

    /// The stronger of the registry's own package floor and a caller's
    /// document-resolved floor. `package_version` is intentionally supplied
    /// by the caller rather than inferred from a command spelling: an explicit
    /// `package require` is document data, while the profile/ambient floor is
    /// registry data.
    fn package_floor_for_spec_at<'a>(
        &self,
        spec: &CommandSpec,
        package_version: Option<&'a str>,
    ) -> Option<&'a str> {
        match (self.package_floor_for_spec(spec), package_version) {
            (Some(registry), Some(document)) => {
                if crate::version::compare(registry, document).is_lt() {
                    Some(document)
                } else {
                    Some(registry)
                }
            }
            (Some(registry), None) => Some(registry),
            (None, document) => document,
        }
    }

    /// Numeral grammar for a control invocation query.
    ///
    /// An explicit surface point is authoritative even when this registry is
    /// profile-less (the ordinary cross-dialect command universe). The
    /// nearest core point with one exact grammar answers; a query none of
    /// whose points has one abstains through
    /// [`tcl_syntax::number::Numbers::Unknown`]. Only an absent query falls
    /// back to the registry's attached profile; a profile-less catalogue is
    /// not evidence for any interpreter's control grammar.
    fn control_numbers(&self, dialect: Option<SurfaceQuery<'_>>) -> tcl_syntax::number::Numbers {
        use tcl_syntax::number::Numbers;

        let Some(query) = dialect else {
            return self.profile().map_or(Numbers::Unknown, |profile| {
                Numbers::of_profile(Some(profile))
            });
        };
        query
            .core
            .iter()
            .find_map(|(family, release)| point_number_syntax(family, release))
            .map_or(Numbers::Unknown, Numbers::Target)
    }

    /// A control handler's release is independent of its source numeral
    /// grammar. An explicit query with no release cannot borrow the registry
    /// profile's handler width.
    fn control_release(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<tcl_dialect::TclVersion> {
        match dialect {
            Some(query) => query.core.iter().find_map(|(family, release)| {
                if family == Family::Tcl
                    && let Some(release) = release
                {
                    return tcl_dialect::TclVersion::from_package_version(release);
                }
                let version = tcl_dialect::model::DialectPoint::tcl_version_of_release;
                if let Some(spelling) = release {
                    family
                        .releases()
                        .iter()
                        .find(|release| release.as_str() == spelling)
                        .and_then(|&release| version(release))
                } else {
                    let mut versions = family.releases().iter().copied().map(version);
                    let first = versions.next()?;
                    first.filter(|&first| versions.all(|next| next == Some(first)))
                }
            }),
            None => self.runtime_version(),
        }
    }

    fn return_invocation_grammar(
        &self,
        args: crate::invocation_words::InvocationArguments<'_>,
        query: Option<SurfaceQuery<'_>>,
    ) -> crate::completion_route::ReturnInvocationGrammar {
        use crate::completion_route::ReturnInvocationGrammar as Grammar;
        if let Some(dialect) = args.dialect() {
            return Grammar::for_dialect(dialect);
        }
        match query {
            Some(query) => Grammar::for_family_release(
                query.core.nearest().map(|(family, _)| family),
                self.control_release(Some(query)),
            ),
            None => self.profile().map_or(Grammar::Unknown, |profile| {
                Grammar::for_dialect(tcl_native_model::InvocationDialect::of_profile(profile))
            }),
        }
    }

    fn selected_argument_role_assignments(
        wanted: &[ArgRole],
        resolved: &ResolvedInvocation<'_, '_>,
    ) -> Option<Vec<(usize, ArgRole)>> {
        // naming.source.authored-registry-role-projection
        // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
        Some(
            resolved
                .arg_roles()?
                .into_iter()
                .filter(|(_, role)| *role != ArgRole::CommandPrefix && wanted.contains(role))
                .collect(),
        )
    }

    /// Provider admission for effect queries and instance metadata. Catalogue
    /// availability and a package version alone do not establish this entry.
    fn effect_provider_is_admitted(
        &self,
        spec: &CommandSpec,
        query: Option<SurfaceQuery<'_>>,
    ) -> bool {
        spec.owning_package().is_none_or(|package| {
            self.is_ambient_package(package)
                || query
                    .is_some_and(|query| query.packages.iter().any(|floor| floor.name == package))
        })
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field(
                "snapshot",
                &self
                    .snapshot
                    .get()
                    .map(|snapshot| snapshot.0.semantic_key.0.fingerprint),
            )
            .field("commands", &self.by_name.len())
            .field("overlay_specs", &self.overlay_specs.len())
            .field("loaded_layers", &self.loaded_layers)
            .field("profile", &self.profile.map(|p| p.name))
            .field("ambient_packages", &self.ambient_packages)
            .field("own_packages", &self.own_packages)
            .field("special_vars", &self.special_vars)
            .field(
                "document_grammar",
                &self.document_grammar.map(|g| g.members.len()),
            )
            .field(
                "effective_semantics_cached",
                &self.effective_semantics.get().is_some(),
            )
            .field("overlay", &self.overlay)
            .field("pack_origins", &self.pack_origins.len())
            .field("reference_texts", &self.reference_texts.len())
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod registry_snapshot_tests;

#[cfg(test)]
mod original_document_grammar_tests;

#[cfg(test)]
mod native_return_effect_ingress_tests;

#[cfg(test)]
mod original_instance_descriptor_tests;

#[cfg(test)]
mod declaration_key_tests;

#[cfg(test)]
mod source_empty_write_projection_tests;

#[cfg(test)]
mod instance_method_provider_tests;

#[cfg(test)]
mod inherited_instance_context_tests;

#[cfg(test)]
mod selected_try_list_controls;

#[cfg(test)]
mod fallback_projection_compiler_tests;

#[cfg(test)]
mod prerequisite_index_tests;
