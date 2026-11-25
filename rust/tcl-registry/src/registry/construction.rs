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

//! Building a registry: the default build, loading a surface, projecting for a profile, and inserting specs, reference texts and ambient packages.

use super::{
    CommandRegistry, argparse_specs, bpf_specs, expect_specs, iapps_specs, irules_specs,
    itcl_specs, next_registry_generation, spectcl_specs, sslictcl_specs, stdlib_specs, tcl_specs,
    tcllib_specs, thread_specs, ticklecharts_specs, tk_specs, tmsh_specs,
};
use crate::arity::Arity;
use crate::definer::DefinitionBodyGrammar;
use crate::spec::CommandSpec;
use rustc_hash::FxHashMap;
use std::sync::{Arc, OnceLock};
use tcl_dialect::model::{Family, PackageFloor, SurfaceLayer, SurfaceQuery};
use tcl_dialect::version_satisfies;

impl CommandRegistry {
    /// Build the default registry with core Tcl + stdlib + tcllib commands.
    #[must_use]
    pub fn build_default() -> Self {
        let mut registry = Self {
            by_name: FxHashMap::default(),
            overlay_specs: Vec::new(),
            loaded_layers: Vec::new(),
            profile: None,
            ambient_packages: Vec::new(),
            own_packages: Vec::new(),
            special_vars: Vec::new(),
            document_grammar: None,
            effective_semantics: OnceLock::new(),
            snapshot: OnceLock::new(),
            native_registrations: Arc::default(),
            overlay: None,
            pack_origins: FxHashMap::default(),
            reference_texts: FxHashMap::default(),
            generation: next_registry_generation(),
        };
        for spec in tcl_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in stdlib_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in tcllib_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in argparse_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in ticklecharts_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in itcl_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in thread_specs() {
            registry.insert_shipped_static(spec);
        }
        // Tk geometry/widget commands (`grid` / `pack` / `wm` / `button` / …)
        // are part of the always-known command universe: a `.tcl` script may
        // `package require Tk` at runtime, and the diagnostics treat them as
        // recognised under every Tcl dialect, so Tk is folded into the base
        // registry.  Mark the layer loaded so a later `load_surface(Tk)` is
        // a no-op rather than a double-insert.
        for spec in tk_specs() {
            registry.insert_shipped_static(spec);
        }
        registry.loaded_layers.push(SurfaceLayer::Package("Tk"));
        registry
    }

    /// Load a surface's commands into the registry (idempotent).
    pub fn load_surface(&mut self, layer: SurfaceLayer) {
        if self.loaded_layers.contains(&layer) {
            return;
        }
        let specs: &'static [CommandSpec] = match layer {
            SurfaceLayer::Package("bpf") => bpf_specs(),
            SurfaceLayer::Core(Family::F5Irules, _) => irules_specs(),
            SurfaceLayer::Package("iapps") => iapps_specs(),
            // The tmsh shell's own pack: the `tmsh::` surface shared with
            // iApps, without the iApp-only commands.
            SurfaceLayer::Package("tmsh") => tmsh_specs(),
            SurfaceLayer::Package("Tk") => tk_specs(),
            SurfaceLayer::Package("expect") => expect_specs(),
            // SpecTcl: the `.tclspec` DSL's own statement words. A pack file
            // is an ordinary Tcl script, so the base Tcl surface stays loaded
            // underneath (hook bodies are real Tcl); this layer adds the
            // declaration vocabulary on top of it.
            SurfaceLayer::Package("spectcl") => {
                self.document_grammar = Some(&crate::definer::SPECTCL_DOCUMENT_GRAMMAR);
                spectcl_specs()
            }
            // SslicTcl: the `.sslictcl` DSL's own declaration words. A
            // document is an ordinary Tcl script that is read and never
            // evaluated, so the base Tcl surface stays loaded underneath —
            // the grammar is what says a word is not an SslicTcl declaration.
            SurfaceLayer::Package("sslictcl") => {
                self.document_grammar = Some(&crate::definer::SSLICTCL_DOCUMENT_GRAMMAR);
                sslictcl_specs()
            }
            // A core release brings no pack of its own — it records which
            // language the registry is. The EDA shells are such a release
            // plus `required_package`-gated command libraries, which ship as
            // bundled `.tclspec` loadables (`docs/design/registry/spec-packs.md`).
            _ => &[],
        };
        for spec in specs {
            self.insert_shipped_static(spec);
        }
        self.loaded_layers.push(layer);
        self.invalidate_effective_semantics();
    }

    /// The member grammar of a document in this registry's dialect, when its
    /// command surface declares one — see the field's own documentation.
    ///
    /// A consumer asking "what may appear at the root of this file?" reads
    /// this; `None` means the root is an ordinary open command position.
    #[must_use]
    pub const fn document_grammar(&self) -> Option<&'static DefinitionBodyGrammar> {
        self.document_grammar
    }

    /// Load iRules dialect commands (convenience wrapper).
    pub fn load_irules(&mut self) {
        self.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    }

    /// Load BPF-Tcl dialect commands (convenience wrapper).
    pub fn load_bpf(&mut self) {
        self.load_surface(SurfaceLayer::Package("bpf"));
    }

    /// Whether this registry's dialect reads a bare leading-zero integer
    /// (`08`, `010`) as **octal**.
    ///
    /// Tcl 9.0 dropped the leading-zero octal rule (TIP 114): `08` parses as
    /// decimal 8 and `010` as decimal 10. Every earlier Tcl (8.4/8.5/8.6) and
    /// every 8.x-derived dialect (f5-irules ≈ 8.4, f5-iapps ≈ 8.5/8.6, the EDA
    /// dialects) keeps the octal rule, where `08`/`09` are *invalid* octal
    /// (treated as a string in `==`/`!=`) and `010` is 8.
    ///
    /// TIP 114 lands in tcl9.0 and stays in tcl9.1 (and any later 9.x), so the
    /// decimal rule applies to *every* Tcl 9 dialect, not tcl9.0 alone. The
    /// per-dialect registry built by `registry_for_dialect` records its Tcl
    /// version via [`Self::load_surface`], so a registry whose
    /// `loaded_layers` names a 9.x core release (tcl9.0, tcl9.1, …) is
    /// decimal; every other dialect (8.4/8.5/8.6 and the F5/EDA registries,
    /// which never load a Tcl-9 release row) reads leading zeros as octal.
    #[must_use]
    pub fn leading_zero_is_octal(&self) -> bool {
        self.octal_fold_policy().unwrap_or(true)
    }

    /// The release this registry's dialect runs, if it names one.
    ///
    /// The registry-level mirror of [`DialectProfile::runtime_version`], so a
    /// consumer holding a registry never spells out `profile().and_then(…)`.
    #[must_use]
    pub fn runtime_version(&self) -> Option<tcl_dialect::TclVersion> {
        self.profile()
            .and_then(tcl_dialect::DialectProfile::runtime_version)
    }

    /// The string/character model of the release this registry's dialect runs.
    #[must_use]
    pub fn character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        self.profile()
            .and_then(tcl_dialect::DialectProfile::character_model)
    }

    /// The numeral grammar of the dialect this registry serves, or the
    /// permissive 9.x default when no profile is loaded.
    ///
    /// The single way a registry-holding consumer names its release for number
    /// parsing — the hand-written `profile().map_or(Tcl90, …)` had six copies.
    #[must_use]
    pub fn numbers(&self) -> tcl_dialect::NumberSyntax {
        tcl_dialect::NumberSyntax::of_profile(self.profile())
    }

    /// The three-valued leading-zero fold policy for this registry's
    /// dialect. A profile-built registry (`registry_for_profile` /
    /// `registry_for_dialect`) answers from the profile's runtime base:
    /// `Some(true)` = 8.x octal, `Some(false)` = 9.x decimal (`bpf`
    /// included, D7), `None` = abstain — no Tcl runtime to have an opinion
    /// (`f5-bigip`, the unknown-dialect fallback; §11.1 of the
    /// dialect-profile model). A hand-assembled registry keeps the
    /// historical `loaded_layers` derivation (a version pack records its
    /// release, so a 9.x load reads decimal).
    #[must_use]
    pub fn octal_fold_policy(&self) -> Option<bool> {
        match self.profile {
            Some(p) => p.leading_zero_is_octal.as_bool(),
            None => Some(!self.loaded_core_is_tcl9()),
        }
    }

    /// Stamp the dialect profile this registry serves. Called by the
    /// per-profile cache (`registry_for_profile`) so behaviour queries
    /// (`octal_policy`, future runtime projections) answer from the
    /// profile rather than re-deriving from loaded packs.
    pub(crate) fn set_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        self.profile = Some(profile);
        self.refresh_own_packages();
        self.invalidate_effective_semantics();
    }

    /// Re-derive [`Self::own_packages`] from the profile and the floors this
    /// registry now guarantees.
    fn refresh_own_packages(&mut self) {
        let carried = self
            .profile
            .map_or(&[][..], |profile| profile.surface_packages);
        self.own_packages = carried
            .iter()
            .map(|package| PackageFloor {
                name: package.name,
                version: self.package_floor(package.name),
            })
            .collect();
    }

    /// Derive an exact registry view for `profile`, preserving this registry's
    /// authored overrides.
    ///
    /// The ordinary registry deliberately retains every release row so tools
    /// can ask cross-dialect questions. Compiler passes also have a few
    /// target-neutral lookups, however, so an explicit-profile compile needs a
    /// physically exact view: every spelling indexes only the one spec selected
    /// by the profile's surface query. Authored specs are selected separately
    /// from the compiled-in universe and replace a visible shipped row even
    /// when the authored override is deliberately surface-less. This preserves
    /// the public insertion contract used by custom `SpecTcl` commands without
    /// weakening release or dialect filtering for shipped commands. The
    /// permissive fallback has no selected release: it retains all admitted
    /// candidates so a later independently selected compiler point can resolve
    /// its original registration without borrowing a different release's row.
    #[must_use]
    pub fn project_for_profile(&self, profile: &'static tcl_dialect::DialectProfile) -> Self {
        let mut projected = Self::build_default();
        for &layer in profile.base_layers {
            projected.load_surface(layer);
        }
        for &(package, version) in &self.ambient_packages {
            projected.insert_ambient_package(package, version);
        }
        for &layer in &self.loaded_layers {
            if !projected.loaded_layers.contains(&layer) {
                projected.loaded_layers.push(layer);
            }
        }
        // Registered native-family rows are retained implementations too;
        // rebuilding the shipped catalogue must not discard them when the
        // driver selects an exact point. Availability still selects the row.
        for (&name, specs) in &self.by_name {
            let rows = projected.by_name.entry(name).or_default();
            for &spec in specs {
                if !rows.iter().any(|held| std::ptr::eq(*held, spec)) {
                    rows.push(spec);
                }
            }
        }
        projected.set_profile(profile);

        let own_packages = projected.own_packages.clone();
        let query = profile.surface_query().with_packages(&own_packages);
        projected.by_name = projected
            .by_name
            .iter()
            .filter_map(|(&name, specs)| {
                let selected: Vec<_> = if profile.is_fallback() {
                    specs
                        .iter()
                        .copied()
                        .filter(|spec| projected.spec_visible(spec, Some(query)))
                        .collect()
                } else {
                    projected
                        .best_visible(specs, Some(query))
                        .into_iter()
                        .collect()
                };
                (!selected.is_empty()).then_some((name, selected))
            })
            .collect();
        let mut overlays_by_name: FxHashMap<_, Vec<_>> = FxHashMap::default();
        for &spec in &self.overlay_specs {
            overlays_by_name.entry(spec.name).or_default().push(spec);
        }
        for (name, specs) in overlays_by_name {
            if let Some(spec) = projected.best_visible(&specs, Some(query)) {
                projected.by_name.insert(name, vec![spec]);
            }
        }
        projected.overlay_specs.clone_from(&self.overlay_specs);
        // The same authored world, projected: its overlay generation and the
        // pack each authored spec came from travel with the specs themselves.
        projected.overlay = self.overlay;
        projected.pack_origins.clone_from(&self.pack_origins);
        projected.reference_texts.clone_from(&self.reference_texts);
        projected.invalidate_effective_semantics();
        projected
    }

    /// The dialect profile this registry was built for, if any.
    #[must_use]
    pub fn profile(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.profile
    }

    /// `spec`'s signature shape at this registry's release
    /// ([`CommandSpec::arity_at`]); the unversioned shape where the profile
    /// names no release.
    #[must_use]
    pub fn release_arity(&self, spec: &CommandSpec) -> Arity {
        let release = self
            .profile
            .and_then(tcl_dialect::TclVersion::from_profile)
            .map(tcl_dialect::TclVersion::version_string);
        spec.arity_at(release)
    }

    /// The point a consumer resolves hooks and calls at when honouring
    /// **this registry's own** dialect: the attached profile's, or `None`
    /// — surface-blind — for a profile-less registry.
    ///
    /// This is what lets a version-pinned compile pipeline suppress a
    /// structured lowering or codegen hook for a
    /// command the emulated release does not have — `lmap` under a tcl8.4
    /// registry resolves to no spec, so the call reaches the runtime's
    /// availability gate as a generic dispatch instead of being inlined.
    #[must_use]
    pub fn own_surface_query(&self) -> Option<SurfaceQuery<'_>> {
        self.profile
            .map(|profile| profile.surface_query().with_packages(&self.own_packages))
    }

    /// Whether a loaded core layer is a Tcl 9.x release — the derivation a
    /// profile-less registry falls back to for the octal question.
    fn loaded_core_is_tcl9(&self) -> bool {
        self.loaded_layers.iter().any(|layer| {
            matches!(layer, SurfaceLayer::Core(Family::Tcl, release)
                if version_satisfies(release, "9.0-"))
        })
    }

    /// Insert an owned command spec, **leaking it** for the process lifetime.
    ///
    /// The registry indexes `&'static CommandSpec`, so an owned spec has to be
    /// given somewhere permanent to live. That is the right trade for the two
    /// callers that need it — a test building a registry by hand, and a
    /// `.tclspec` pack whose specs are already leaked by the loader — and the
    /// wrong one for the hundreds of shipped specs, which are indexed through
    /// the registry's private compiled-universe seam.
    ///
    /// A caller in a loop over user input wants `insert_static` and an arena
    /// it controls, not this.
    pub fn insert(&mut self, spec: CommandSpec) {
        self.insert_static(Box::leak(Box::new(spec)));
    }

    /// A copy of this registry with `specs` — a family's own compiled-in
    /// surface — indexed after every shipped spec under their names, and
    /// `overlaid`'s authored overlay indexed after them.
    ///
    /// `self` is the store before any pack overlay and `overlaid` is that
    /// store with the overlay installed (the same store when there is none).
    /// The compiled-in specs are not a pack's contribution, so a pack's row
    /// for the same name outranks them where registration order breaks a
    /// tie, as it outranks the shipped data it shadows. The copy shares every
    /// `&'static CommandSpec` with its sources, records only the overlay's
    /// specs as authored, and takes every other field from `overlaid`.
    /// Availability-aware queries rank by the query's core points before
    /// registration order is consulted.
    #[must_use]
    pub(crate) fn with_core_surface(
        &self,
        specs: &[&'static CommandSpec],
        overlaid: &Self,
    ) -> Self {
        let mut by_name = self.by_name.clone();
        for spec in specs.iter().chain(&overlaid.overlay_specs) {
            by_name.entry(spec.name).or_default().push(spec);
        }
        Self {
            by_name,
            overlay_specs: overlaid.overlay_specs.clone(),
            loaded_layers: overlaid.loaded_layers.clone(),
            profile: overlaid.profile,
            ambient_packages: overlaid.ambient_packages.clone(),
            own_packages: overlaid.own_packages.clone(),
            special_vars: overlaid.special_vars.clone(),
            document_grammar: overlaid.document_grammar,
            effective_semantics: OnceLock::new(),
            snapshot: OnceLock::new(),
            native_registrations: Arc::default(),
            overlay: overlaid.overlay,
            pack_origins: overlaid.pack_origins.clone(),
            reference_texts: overlaid.reference_texts.clone(),
            // A surface neither source has had.
            generation: next_registry_generation(),
        }
    }

    /// Insert an authored overlay spec the caller already owns permanently —
    /// no copy, no leak.
    ///
    /// This is what makes a per-pack-edit registry affordable. A registry is
    /// rebuilt from scratch for every distinct pack-set content the server
    /// sees, and a `CommandSpec` is 1,296 bytes: copying the ~2,400 shipped
    /// specs into each generation cost megabytes a keystroke. Sharing one
    /// leaked copy of every shipped spec reduces a generation to its index —
    /// the names, and one pointer each.
    pub fn insert_static(&mut self, spec: &'static CommandSpec) {
        self.by_name.entry(spec.name).or_default().push(spec);
        self.overlay_specs.push(spec);
        self.invalidate_effective_semantics();
    }

    /// Record that the installed `spec` came from a spec pack — the facts a
    /// site specialised on it stamps ([`Self::pack_origin`]). The installer
    /// calls this for each pack command it inserts.
    pub fn insert_pack_origin(
        &mut self,
        spec: &'static CommandSpec,
        origin: crate::pack_origin::PackOrigin,
    ) {
        self.pack_origins
            .insert(std::ptr::from_ref(spec).addr(), origin);
        self.invalidate_effective_semantics();
    }

    /// The spec pack `spec` was installed from, or `None` for a shipped spec
    /// and for one an embedder inserted itself.
    #[must_use]
    pub fn pack_origin(&self, spec: &CommandSpec) -> Option<&crate::pack_origin::PackOrigin> {
        self.pack_origins.get(&std::ptr::from_ref(spec).addr())
    }

    /// Record the definition text a pack command's `PackageSource` backing
    /// resolved to — what the loader read from the package at load, so that
    /// nothing downstream reads a file ([`Self::reference_body`]).
    pub fn insert_reference_text(&mut self, spec: &'static CommandSpec, text: Arc<str>) {
        self.reference_texts
            .insert(std::ptr::from_ref(spec).addr(), text);
        self.invalidate_effective_semantics();
    }

    /// The Tcl definition `spec`'s backing says defines the command: the text
    /// a `PackText` backing carries, or the file a `PackageSource` backing
    /// resolved to at load. `None` for every other backing, and for a
    /// `PackageSource` the loader had no package to read.
    #[must_use]
    pub fn reference_body(&self, spec: &CommandSpec) -> Option<&str> {
        use crate::runtime_backing::{BodySource, RuntimeBacking};
        match spec.runtime_backing {
            RuntimeBacking::TclBody {
                source: BodySource::PackText { text },
                ..
            } => Some(text),
            RuntimeBacking::TclBody {
                source: BodySource::PackageSource { .. },
                ..
            } => self
                .reference_texts
                .get(&std::ptr::from_ref(spec).addr())
                .map(AsRef::as_ref),
            _ => None,
        }
    }

    /// Every spec a pack installed whose backing is a Tcl body this registry
    /// holds the text of, with the text — the commands a compile may inline
    /// the definition of. A spec a later insertion shadowed is not among them.
    pub fn reference_bodies(&self) -> impl Iterator<Item = (&'static CommandSpec, &str)> {
        self.overlay_specs.iter().filter_map(|&spec| {
            let text = self.reference_body(spec)?;
            self.pack_origin(spec)?;
            self.get_exact(spec.name)
                .is_some_and(|live| std::ptr::eq(live, spec))
                .then_some((spec, text))
        })
    }

    /// Index one row from the compiled-in command universe without recording
    /// it as an authored overlay.
    fn insert_shipped_static(&mut self, spec: &'static CommandSpec) {
        self.by_name.entry(spec.name).or_default().push(spec);
        self.invalidate_effective_semantics();
    }

    /// Record that `package` is ambient in this registry's dialect at
    /// `version` — the `ambient_package` statement of a `SpecTcl` pack.
    pub fn insert_ambient_package(&mut self, package: &'static str, version: &'static str) {
        self.ambient_packages.push((package, version));
        self.refresh_own_packages();
        self.invalidate_effective_semantics();
    }

    /// Whether `package` is **ambient** — provided by the runtime, with no
    /// `package require` needed to reach it.
    ///
    /// The union of the two things that can make that true: the dialect
    /// profile shipping it (an F5 surface is part of the runtime, §7.1 axis
    /// C), and a loaded `SpecTcl` pack declaring it with `ambient_package`.
    /// This is the one place that union is taken — every consumer that used
    /// to ask the profile alone must ask here instead, or a pack's ambient
    /// declaration registers a floor while the same package still draws a
    /// "needs a `package require`" diagnostic.
    #[must_use]
    pub fn is_ambient_package(&self, package: &str) -> bool {
        self.profile
            .is_some_and(|profile| profile.is_ambient_package(package))
            || self
                .ambient_packages
                .iter()
                .any(|(name, _)| *name == package)
    }

    /// The version floor a pack declared for `package` as ambient, if any.
    ///
    /// The **highest** declared version wins when several packs name the same
    /// package: each is a claim that the runtime provides at least that
    /// version, and the strongest such claim is the one that holds. This is
    /// the same max-composition an explicit `package require` goes through,
    /// and for the same reason — a floor is a lower bound, so combining two
    /// lower bounds takes the greater.
    #[must_use]
    pub fn ambient_package_floor(&self, package: &str) -> Option<&'static str> {
        self.ambient_packages
            .iter()
            .filter(|(name, _)| *name == package)
            .map(|(_, version)| *version)
            .max_by(|a, b| crate::version::compare(a, b))
    }

    /// Every `(package, declared floor)` row loaded packs registered with
    /// `ambient_package`, in registration order. The enumeration face of
    /// [`Self::is_ambient_package`] / [`Self::ambient_package_floor`], for
    /// the model layer's context assembly
    /// ([`crate::model::assembly::ContextRegistry`]) — the resolved context
    /// carries these rows so pack-ambient floors answer from the context
    /// rather than from a live registry borrow.
    #[must_use]
    pub fn ambient_package_rows(&self) -> &[(&'static str, &'static str)] {
        &self.ambient_packages
    }

    /// Record a special variable a `SpecTcl` pack declares — the pack's
    /// `special_var` statement.
    pub fn insert_special_var(&mut self, spec: &'static crate::special_vars::SpecialVarSpec) {
        self.special_vars.push(spec);
        self.invalidate_effective_semantics();
    }
}
