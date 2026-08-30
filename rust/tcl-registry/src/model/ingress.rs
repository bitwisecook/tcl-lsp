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

//! **The** dialect-name ingress seam (centralisation contract R-a, ledger
//! rows C2/F2/F3/F9).
//!
//! Every place the toolchain accepts a dialect *name* — a compiler
//! `analyse` call, an LSP document's recorded dialect, an editor language
//! id, a persisted session, a settings string — resolves it here,
//! **once**, to an [`EnvironmentDefinition`] through the one
//! [`EnvironmentRegistry`] resolver, and derives everything downstream
//! from the resolved environment: the per-context registry generation
//! ([`ContextRegistry`], carrying the [`ResolvedContext`] availability
//! view and the generation's command store), the Tk-environment fact, and
//! — as the
//! documented wave-1 interop — the old interned [`DialectProfile`] the
//! rest of the pipeline still threads for grammar, versions, and
//! diagnostic labels. The interop mapping goes through the **resolved
//! canonical id**, never back through the raw string, so the retired name
//! validators (`by_name` at ingress, `resolve_known`,
//! `availability_for_name`, raw `DialectProfile::find`) have no remaining
//! caller outside the projections documented at their sites.
//!
//! # Why this lives in `tcl-registry`
//!
//! The seam has to hand back both halves of a resolved document
//! environment: the definition ([`tcl_dialect`]) *and* its registry
//! generation ([`ContextRegistry`], this crate). `tcl-dialect` cannot host
//! it — it does not, and must not, depend on `tcl-registry` — so
//! `tcl-registry::model` is the only home that can express the whole
//! answer, and it is where wave 1 already put the generation door
//! ([`registry_for_environment_if_built`]). `tcl-compiler`'s
//! `environment_ingress` module now delegates here rather than owning a
//! second copy; the LSP crates (`tcl-lsp-core`, `tcl-lsp-db`,
//! `tcl-lsp-server`) call it directly.
//!
//! # The accepted micro-unifications (wave 1, unchanged here)
//!
//! Three inputs resolve *slightly* differently than they did before the
//! port. Each was measured to be unexercised by any shipped call site, and
//! each replaces a divergence between the old validators with the single
//! resolver's answer:
//!
//! 1. **Editor language ids resolve everywhere.** `EnvironmentRegistry`
//!    accepts canonical ids, aliases *and* contributed editor identities
//!    (`tcl84`, `tcl91`, `tcl-irule`, `tclspec`, `wish`), where
//!    `DialectProfile::find` accepted only the first two. A name the old
//!    ingress sank to the permissive fallback can now resolve to its real
//!    environment. Every shipped caller already mapped editor ids to
//!    canonical names before the ingress (the LSP's
//!    `dialect_from_language_id`), so no shipped path changes answer.
//! 2. **`tk` resolves as an environment, not a synthesised profile.** The
//!    old ingress recognised `tk` by `DialectProfile::find` and built
//!    `TK_PROFILE`; here it is the `tk` environment, which places the `Tk`
//!    package **ambient** (P3), so every Tk fact — availability, the
//!    authoring point, the Tk-checks activation, W120's silence — is one
//!    placement query on the resolved context, and
//!    [`DocumentEnvironment::unit_profile`] still hands back the same
//!    typed additive profile for the identity interop.
//! 3. **A malformed library-version override drops to the axis default**
//!    rather than being carried verbatim ([`KeyedVersions::from_overrides`]
//!    is `Result`, and this seam takes the default on error). The old
//!    string-floor path carried the spelling through; a non-version
//!    spelling never named a real floor, so the comparison it fed always
//!    answered permissively anyway.
//!
//! [`DialectProfile`]: tcl_dialect::DialectProfile

use crate::model::context::AuthoringScope;
use std::sync::{Arc, Mutex, OnceLock};

use rustc_hash::FxHashMap;

use tcl_dialect::model::{EnvironmentDefinition, EnvironmentIdentity, EnvironmentRegistry};
use tcl_dialect::{DialectProfile, LibraryVersionOverrides};

use crate::model::assembly::{ContextRegistry, registry_for_environment_if_built};
use crate::model::context::{KeyedVersions, ResolvedContext};

/// The **live** environment registry: the compiled seed set at
/// generation 0 until a pack registers environments through
/// [`crate::model::registration::register_environments`] (the P2
/// dynamic-environment channel), then each registration's rebuilt
/// registry at the next generation. Every resolve below reads the live
/// value, so a registered environment becomes resolvable — and its
/// generation bump invalidates downstream generation caches — with no
/// second wiring.
#[must_use]
pub fn environments() -> Arc<EnvironmentRegistry> {
    crate::model::registration::live_environments()
}

/// One resolved document environment: the definition plus the identity
/// its registry generations key on.
#[derive(Debug, Clone)]
pub struct DocumentEnvironment {
    /// The resolved definition.
    pub definition: Arc<EnvironmentDefinition>,
    /// The `(id, generation, overlay)` identity for generation caching.
    pub identity: EnvironmentIdentity,
}

/// Resolve a user-written dialect name — canonical id, alias, or editor
/// language id — to its environment. Unknown names (and the empty string)
/// resolve to the lenient `tcl` environment, exactly as every unknown name
/// resolved to the permissive fallback profile before.
#[must_use]
pub fn resolve_environment(name: &str) -> DocumentEnvironment {
    let registry = environments();
    let definition = registry.resolve(name).unwrap_or_else(|| {
        registry
            .resolve("tcl")
            .expect("the compiled catalogue seeds the lenient `tcl` environment")
    });
    let identity = registry.identity_of(&definition);
    DocumentEnvironment {
        definition,
        identity,
    }
}

/// Whether `name` resolves to a real environment — the one name
/// *validator* (ledger rows F9/T1/T5: the settings, `setDialect`, and CLI
/// ingresses that must reject an unknown spelling rather than silently
/// serve the lenient fallback [`resolve_environment`] hands back).
#[must_use]
pub fn is_known_environment_name(name: &str) -> bool {
    environments().resolve(name).is_some()
}

/// Resolve `name` only when it names a real environment — the
/// `Option`-returning form for ingresses that distinguish "unstated" from
/// "plain Tcl" (an empty dialect string, an absent setting).
#[must_use]
pub fn resolve_known_environment(name: &str) -> Option<DocumentEnvironment> {
    is_known_environment_name(name).then(|| resolve_environment(name))
}

impl DocumentEnvironment {
    /// Whether this is the `tk` environment — an **identity** question
    /// (the `wish` ingress), never an availability one.
    ///
    /// P3 moved every availability reader off this predicate and onto the
    /// placement queries on the resolved context: "is Tk in this
    /// document's world?" is `ResolvedContext::package_active("Tk")`, "is
    /// it there without a `package require`?" is
    /// `ResolvedContext::ambient_package("Tk")`, and "could this
    /// environment host it at all?" is
    /// `ResolvedContext::can_host_package("Tk")`. What is left is the two
    /// places that genuinely ask which environment this is: the profile
    /// interop below, and the seam's own pins.
    ///
    /// [`ResolvedContext::package_active`]: crate::model::ResolvedContext::package_active
    fn is_tk(&self) -> bool {
        self.definition.id.as_str() == "tk"
    }

    /// The environment's canonical id — the spelling every downstream
    /// cache key, persisted session, and diagnostic label should carry
    /// instead of the raw user string.
    #[must_use]
    pub fn id(&self) -> &str {
        self.definition.id.as_str()
    }

    /// The interned profile the analyser threads for this environment —
    /// wave-1 interop (retired with ledger C1's re-type; P1-G deleted the
    /// name validators it replaced): the catalogue environments map to
    /// their same-named profile; the model-only ids (`tcl`, `tk`) map to
    /// the permissive fallback, exactly as the old name ingress resolved
    /// them.
    ///
    /// **The `tk` asymmetry against [`Self::unit_profile`] is permanent**
    /// (P3 ruling). It is not an availability split any more — the
    /// context derives the point, so both faces answer the same
    /// availability question — but a *catalogue* one, and the
    /// classification rule (§2) is what fixes it: `tk` is a package plus
    /// an environment, never a dialect, so `DialectProfile::find("tk")`
    /// must keep answering `None` and this face must keep sinking to the
    /// permissive fallback. Two consumers depend on exactly that — the
    /// CLI's KCS help filter and the pack-carrying registry cache key
    /// (centralisation §2.4, wave 3) — and promoting here would silently
    /// widen both. The asymmetry disappears when the interned
    /// `DialectProfile` does, under ledger C1/F1.
    #[must_use]
    pub fn analyser_profile(&self) -> &'static DialectProfile {
        DialectProfile::find(self.definition.id.as_str()).unwrap_or_else(DialectProfile::plain_tcl)
    }

    /// The interned profile a compilation unit — and every LSP provider
    /// threading a `DialectProfile` — is built for: as
    /// [`Self::analyser_profile`], except the `tk` environment keeps its
    /// typed additive ingress profile (the old `resolve_known`
    /// promotion), so the unit carries the `tk` **identity** (its name,
    /// display label, and Tk library pins) rather than the anonymous
    /// fallback's.
    ///
    /// The promotion is not what carries Tk's *availability*: the `tk`
    /// environment's ambient placement puts Tk in its own derived point, so
    /// `unit_profile().surface_query()` and
    /// [`Self::document_authoring_scope`] agree by derivation rather than
    /// by injection.
    #[must_use]
    pub fn unit_profile(&self) -> &'static DialectProfile {
        if self.is_tk() {
            DialectProfile::tk()
        } else {
            self.analyser_profile()
        }
    }

    /// Whether `name` is one of this environment's **contributed
    /// identities** — its canonical id or its editor language id — as
    /// opposed to a legacy alias it also answers to.
    ///
    /// The editor-side ingress (an LSP `languageId`, a contributed file
    /// association) is a claim about a *contributed identity*, review B7's
    /// fixed-identity rule: `irules` resolves to the `f5-irules`
    /// environment everywhere a dialect *name* is accepted, but it is not a
    /// language id any editor contributes, and an ingress that took it as
    /// one would let a client select an environment through a spelling the
    /// contribution manifest never declares.
    #[must_use]
    pub fn is_contributed_identity(&self, name: &str) -> bool {
        self.definition.id.as_str() == name
            || self
                .definition
                .editor_identity
                .is_some_and(|identity| identity.as_str() == name)
    }

    /// The **catalogue** profile this environment has, `None` when it has
    /// none — the lenient `tcl` sink, the `tk` environment (a library
    /// surface, never a catalogue entry), and every unknown name that fell
    /// through to the sink.
    ///
    /// The environment-derived face of `DialectProfile::find`, for the
    /// label and projection readers (status surfaces, editor-id maps) that
    /// must render *nothing* rather than the fallback's placeholder. Unlike
    /// [`Self::unit_profile`] it never invents a profile; unlike
    /// [`Self::stated_profile`] it does not promote `tk`.
    #[must_use]
    pub fn catalogue_profile(&self) -> Option<&'static DialectProfile> {
        DialectProfile::find(self.definition.id.as_str())
    }

    /// Whether this is the **lenient** environment — the sink every
    /// unknown, unstated, or explicitly-plain dialect name resolves to.
    ///
    /// The environment-model face of `DialectProfile::is_fallback`, and of
    /// the old `DialectProfile::resolve_known(name) == None` gate: a caller
    /// that must distinguish "this document states a dialect" from "this
    /// document states nothing" asks here (see [`Self::stated_profile`]).
    #[must_use]
    pub fn is_lenient(&self) -> bool {
        self.definition.id.as_str() == "tcl"
    }

    /// The profile a **stated** dialect names — `None` when the name stated
    /// none (empty, unknown, or the lenient `tcl` environment itself).
    ///
    /// This is exactly the old `DialectProfile::resolve_known(name)` gate,
    /// derived from the resolved environment: the catalogue environments
    /// answer with their own profile, `tk` with the typed additive one, and
    /// the lenient sink with `None`. Consumers that pass an
    /// `Option<&DialectProfile>` meaning "the dialect this build selected,
    /// if any" read it here rather than re-validating the string.
    #[must_use]
    pub fn stated_profile(&self) -> Option<&'static DialectProfile> {
        (!self.is_lenient()).then(|| self.unit_profile())
    }

    /// The **document** authoring mask: the mask of the un-overlaid
    /// generation's [`ResolvedContext`].
    ///
    /// A plain read of the derived point for **every** environment, `tk`
    /// included: the ambient Tk placement puts Tk in `tk`'s own derivation,
    /// so there is no injected copy and
    /// `the_document_point_matches_the_threaded_profile` pins the two equal.
    ///
    /// [`ResolvedContext::authoring_query`]: crate::model::ResolvedContext::authoring_query
    #[must_use]
    pub fn document_authoring_scope(&self) -> AuthoringScope {
        self.default_context_registry().context().authoring_scope()
    }

    /// The context a **document** of this environment is assisted under:
    /// the un-overlaid generation's own context.
    #[must_use]
    pub fn document_context(&self) -> ResolvedContext {
        self.default_context_registry().context().clone()
    }

    /// The keyed-axis pins for this session's library-version overrides.
    /// A malformed override spelling drops to the axis default rather than
    /// aborting resolution (micro-unification 3 in the module docs).
    #[must_use]
    pub fn keyed_versions(overrides: &LibraryVersionOverrides) -> KeyedVersions {
        KeyedVersions::from_overrides(overrides).unwrap_or_default()
    }

    /// The registry generation for this environment at `overlay` — the
    /// pack-overlay key threaded exactly as the old
    /// `registry_for_profile_if_built(profile, overlay)` door: a
    /// not-yet-installed overlay falls back to the un-overlaid generation,
    /// the state the process was in a moment ago.
    #[must_use]
    pub fn context_registry(&self, keyed: &KeyedVersions, overlay: u64) -> Arc<ContextRegistry> {
        registry_for_environment_if_built(&self.definition, &self.identity, keyed, overlay)
            .unwrap_or_else(|| {
                registry_for_environment_if_built(&self.definition, &self.identity, keyed, 0)
                    .expect("the un-overlaid generation always builds")
            })
    }

    /// The un-overlaid generation at default keyed axes — the plain
    /// "registry for this document" answer.
    #[must_use]
    pub fn default_context_registry(&self) -> Arc<ContextRegistry> {
        self.context_registry(&KeyedVersions::default(), 0)
    }
}

/// The generation a **profile-keyed** consumer reads (default keyed axes,
/// no pack overlay) — transitional plumbing for passes that still receive
/// a resolved [`DialectProfile`] instead of a dialect name (side-effect
/// classification, the fixed iRules handles, the LSP providers that take a
/// `&DialectProfile` argument). The profile's canonical name is a
/// canonical environment id, so this is an id-keyed lookup, not a
/// re-parse.
#[must_use]
pub fn context_for_profile(profile: &DialectProfile) -> Arc<ContextRegistry> {
    resolve_environment(profile.name).default_context_registry()
}

/// The promotion key: the environment's canonical id and both generation
/// axes an un-overlaid assembly answers under.
type PromotionKey = (String, u64, u64);

/// The surface-roster generation `environment`'s answers move with — `0`
/// for an environment no roster can reach.
///
/// [`ResolvedContext::inherited_surface_admits`] consults a roster only
/// for a family that *reimplements* an ancestor; a fork, a family with no
/// ancestry, and every environment with no core all answer `true` before
/// looking. Carrying the axis for those would rebuild each of their
/// promoted views — and, on a pack publication, the pack overlay behind
/// each — every time a roster moves, to arrive at the same answers.
///
/// [`ResolvedContext::inherited_surface_admits`]: crate::model::ResolvedContext::inherited_surface_admits
fn roster_axis_of(environment: &DocumentEnvironment) -> u64 {
    let reimplements = environment.definition.core.is_some_and(|core| {
        core.family.ancestry().is_some_and(|ancestry| {
            ancestry.lineage == tcl_dialect::model::family::Lineage::Reimplementation
        })
    });
    if reimplements {
        tcl_dialect::model::inherited_surface_generation()
    } else {
        0
    }
}

/// Every [`PromotionKey`] whose un-overlaid generation has been promoted
/// to a `&'static` view. The value is a leaked *clone of the generation
/// handle*, not a second assembly, so the `&'static` and the `Arc` name
/// one allocation and cannot drift apart.
static LEAKED_GENERATIONS: OnceLock<Mutex<FxHashMap<PromotionKey, &'static ContextRegistry>>> =
    OnceLock::new();

/// The un-overlaid, default-keyed generation for `name` as a `&'static`.
///
/// The analogue of [`crate::cache::registry_for_profile`]'s promotion,
/// but not id-keyed the way that one is: the environment axis is not the
/// closed set the profile axis was. A pack declaring an `environment`
/// block registers, a reload re-registers, and an `include from` roster
/// moves on its own axis without touching any environment — so an
/// id-keyed promotion would serve pre-reload placements, detection facts
/// and inherited availability until restart.
///
/// The key is therefore the whole of what an un-overlaid assembly
/// answers under: [`crate::model::assembly`]'s generation key minus the
/// keyed versions and the overlay, which are fixed here. Both axes only
/// move on a real change, so the promotion still leaks a clone of the
/// generation's `Arc` — eight bytes — never a copy of the assembly.
///
/// This is what lets the LSP providers keep their `&'static` registry
/// ergonomics while their *ingress* moves to the environment model: the
/// store behind [`ContextRegistry::commands`] is the very `Arc` the old
/// `(profile, overlay)` cache owns, so `static_context_for(name).commands()`
/// is the same allocation `registry_for_dialect(name)` returned.
#[must_use]
pub fn static_context_for(name: &str) -> &'static ContextRegistry {
    let environment = resolve_environment(name);
    let key: PromotionKey = (
        environment.id().to_owned(),
        environment.identity.generation,
        roster_axis_of(&environment),
    );
    let leaked = LEAKED_GENERATIONS.get_or_init(|| Mutex::new(FxHashMap::default()));
    if let Some(view) = leaked
        .lock()
        .expect("generation leak map mutex")
        .get(&key)
        .copied()
    {
        return view;
    }
    // Assembled outside the lock; a racing thread's promotion wins and ours
    // drops, exactly as the old registry promotion resolves the same race.
    let handle = environment.default_context_registry();
    let mut guard = leaked.lock().expect("generation leak map mutex");
    if let Some(view) = guard.get(&key).copied() {
        return view;
    }
    let leaked_handle: &'static Arc<ContextRegistry> = Box::leak(Box::new(handle));
    guard.insert(key, leaked_handle);
    leaked_handle
}

/// [`static_context_for`] keyed by an already-resolved profile — the
/// `&'static` twin of [`context_for_profile`].
#[must_use]
pub fn static_context_for_profile(profile: &DialectProfile) -> &'static ContextRegistry {
    static_context_for(profile.name)
}

/// The context a **document** of `name` is assisted under: the un-overlaid
/// generation's [`ResolvedContext`].
///
/// This is what an availability, option, floor, or subcommand question
/// about a *document* is asked of. **P3 collapsed it onto
/// `static_context_for(name).context()`**: the two used to differ for
/// exactly one environment, `tk`, whose additive `TK` bit was injected
/// over the derivation here; the ambient Tk placement derives it now, so
/// there is one context per generation and this face is a borrow of it —
/// no second leak map, no second value that could drift.
#[must_use]
pub fn static_document_context_for(name: &str) -> &'static ResolvedContext {
    static_context_for(name).context()
}

/// [`static_document_context_for`] keyed by an already-resolved profile.
#[must_use]
pub fn static_document_context_for_profile(profile: &DialectProfile) -> &'static ResolvedContext {
    static_document_context_for(profile.name)
}

/// The fixed iRules generation — the environment-model face of the old
/// `DialectProfile::irules()` handle for the hardcoded iRules lookups.
#[must_use]
pub fn irules_context() -> Arc<ContextRegistry> {
    resolve_environment("f5-irules").default_context_registry()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn names_resolve_as_the_old_ingress_did() {
        for (name, profile) in [
            ("tcl8.6", "tcl8.6"),
            ("f5-irules", "f5-irules"),
            ("irules", "f5-irules"),
            ("tcl-irule", "f5-irules"),
            ("f5-iapps", "f5-iapps"),
            ("xilinx-eda-tcl", "xilinx-eda-tcl"),
        ] {
            let environment = resolve_environment(name);
            assert_eq!(environment.id(), profile, "{name}");
            assert_eq!(environment.analyser_profile().name, profile, "{name}");
            assert_eq!(environment.unit_profile().name, profile, "{name}");
            assert!(!environment.is_tk(), "{name}");
        }
        for name in ["", "tcl", "no-such-dialect"] {
            let environment = resolve_environment(name);
            assert_eq!(environment.id(), "tcl", "{name}");
            assert!(environment.analyser_profile().is_fallback(), "{name}");
            assert!(environment.unit_profile().is_fallback(), "{name}");
            assert!(!environment.is_tk(), "{name}");
        }
        let tk = resolve_environment("tk");
        assert!(tk.is_tk());
        assert!(tk.analyser_profile().is_fallback());
        assert_eq!(tk.unit_profile().name, "tk");
    }

    /// The validator half: only declared names pass, and the lenient
    /// fallback of [`resolve_environment`] is *not* mistaken for one.
    #[test]
    fn the_validator_accepts_exactly_the_declared_names() {
        for name in ["tcl", "tcl8.6", "tk", "wish", "irules", "tcl-irule", "bpf"] {
            assert!(is_known_environment_name(name), "{name}");
            assert!(resolve_known_environment(name).is_some(), "{name}");
        }
        for name in ["", "no-such-dialect", "TCL8.6", "tcl8.7"] {
            assert!(!is_known_environment_name(name), "{name}");
            assert!(resolve_known_environment(name).is_none(), "{name}");
            // …but the lenient resolver still answers, as it always has.
            assert_eq!(resolve_environment(name).id(), "tcl", "{name}");
        }
    }

    /// The seam reproduces the LSP's old `profile_for_dialect`
    /// (`resolve_known` first, `by_name` as the sink) for every catalogue
    /// name and alias, the `tk` ingress, and the unknown-name sink.
    #[test]
    fn unit_profile_reproduces_the_old_lsp_ingress() {
        fn old(name: &str) -> &'static DialectProfile {
            // The retired LSP ingress, spelled inline now the validators
            // are deleted: `resolve_known` was the catalogue lookup plus
            // the `tk` promotion, and `by_name` the plain-Tcl sink.
            DialectProfile::find(name)
                .or_else(|| (name == "tk").then(DialectProfile::tk))
                .unwrap_or_else(DialectProfile::plain_tcl)
        }
        for profile in DialectProfile::all() {
            assert!(
                std::ptr::eq(
                    resolve_environment(profile.name).unit_profile(),
                    old(profile.name)
                ),
                "{}",
                profile.name
            );
            for &alias in profile.aliases {
                assert!(
                    std::ptr::eq(resolve_environment(alias).unit_profile(), old(alias)),
                    "{alias}"
                );
            }
        }
        for name in ["tk", "", "tcl", "no-such-dialect"] {
            assert!(
                std::ptr::eq(resolve_environment(name).unit_profile(), old(name)),
                "{name}"
            );
        }
    }

    /// The **document mask** the LSP's availability queries answer under
    /// equals the mask the old `ProfileQueries` read off the threaded
    /// profile, for every profile an ingress can produce — the catalogue,
    /// the additive `tk`, and the permissive sink every unknown name lands
    /// on.
    ///
    /// **P3**: this now holds *by derivation* for every environment, `tk`
    /// included. Waves 1-2 could only hold it by injection — the document
    /// context replaced the derived point with the threaded profile's,
    /// because `tk`'s derivation had no way to produce Tk. The ambient Tk
    /// placement produces it, so the injection door and the second leaked
    /// document-context value are both deleted, and the generation's own
    /// context *is* the document context.
    #[test]
    fn the_document_point_matches_the_threaded_profile() {
        let names: Vec<&str> = DialectProfile::all()
            .iter()
            .map(|profile| profile.name)
            .chain(["tk", "wish", "tcl", "", "no-such-dialect", "irules"])
            .collect();
        for name in names {
            let environment = resolve_environment(name);
            assert!(
                crate::model::surface::points_answer_alike(
                    &environment.document_authoring_scope().query(),
                    &environment.unit_profile().surface_query()
                ),
                "{name}"
            );
            assert!(
                crate::model::surface::points_answer_alike(
                    &environment.document_context().authoring_query(),
                    &environment.unit_profile().surface_query()
                ),
                "{name} context"
            );
            // One value, not two: no injected mask over the derivation.
            assert_eq!(
                environment.document_context().authoring_query(),
                environment
                    .default_context_registry()
                    .context()
                    .authoring_query(),
                "{name} derivation"
            );
        }
        // The `TK` bit is *derived*, from the `tk` environment's ambient
        // Tk placement.
        let tk = resolve_environment("tk");
        let tk_context = tk.default_context_registry();
        assert!(
            tk_context
                .context()
                .authoring_query()
                .packages
                .contains(&"Tk")
        );
        assert!(tk_context.context().placement_is_ambient("Tk"));
        // …and no plain-Tcl environment gains it from the lenient hosted
        // rule: *hosting* Tk is not *shipping* it.
        for plain in ["tcl", "tcl8.6", "tcl9.0"] {
            let environment = resolve_environment(plain);
            let generation = environment.default_context_registry();
            let context = generation.context();
            assert!(
                !context.authoring_query().packages.contains(&"Tk"),
                "{plain}"
            );
            assert!(context.can_host_package("Tk"), "{plain}");
            assert!(!context.placement_is_ambient("Tk"), "{plain}");
        }
    }

    /// Every name the retired validators accepted still resolves — the union
    /// of `DialectProfile::find` (catalogue names and aliases) and
    /// `DialectProfile::find`'s ingress spellings, which the LSP's
    /// `is_known_dialect_name` took as its acceptance set.
    #[test]
    fn the_validator_accepts_every_retired_validators_name() {
        for profile in DialectProfile::all() {
            assert!(is_known_environment_name(profile.name), "{}", profile.name);
            for &alias in profile.aliases {
                assert!(is_known_environment_name(alias), "{alias}");
            }
        }
        for name in [
            "bpf",
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
            "irules",
            "tcl-irule",
            "f5-iapps",
            "tk",
            "expect",
            "f5-tmsh",
            "f5-bigip",
            "spectcl",
            "tcl-spec",
            "tclspec",
        ] {
            assert!(is_known_environment_name(name), "{name}");
        }
    }

    #[test]
    fn context_registries_carry_the_expected_stores() {
        let environment = resolve_environment("tcl8.5");
        let generation = environment.default_context_registry();
        assert_eq!(
            generation.context().environment.id.as_str(),
            "tcl8.5",
            "the generation answers under the resolved environment"
        );
        let fallback = environment.context_registry(&KeyedVersions::default(), 0xDEAD);
        assert!(Arc::ptr_eq(generation.commands(), fallback.commands()));
    }
}
