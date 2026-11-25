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

//! Finding commands and special variables: by name, by surface, by visibility from a namespace, and across every dialect.

use super::CommandRegistry;
use crate::arity::Arity;
use crate::spec::CommandSpec;
use crate::traits::Traits;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::{OnceLock, PoisonError, RwLock};
use tcl_dialect::model::{SpecProvider, SpecSurface, SurfaceQuery, surface_provided_by};

/// Command names registered by *every* dialect. Backs
/// [`CommandRegistry::known_in_any_dialect`] — the dialect-agnostic existence
/// check over every loaded dialect. `rootable` is the subset whose bare spec
/// can also denote a rooted singleton command; method-context-only spellings
/// such as `my` are deliberately absent from it.
/// Built from the same spec functions [`CommandRegistry::build_default`]
/// and [`CommandRegistry::load_surface`] draw from, so it stays in lock-step
/// with the registry's command universe, plus the core-surface specs a crate
/// above the registry has registered ([`crate::register_core_surface_specs`]).
#[derive(Clone)]
pub(super) struct AllDialectCommandNames {
    known: FxHashSet<&'static str>,
    rootable: FxHashSet<&'static str>,
    pub(super) providers: FxHashMap<&'static str, NameProviders>,
}

/// Who offers a command name across the command universe — see
/// [`CommandRegistry::providers_in_any_dialect`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameProviders {
    /// Some spec of this name states no surface, so it is offered to every
    /// dialect and no family is unrelated to it.
    pub unrestricted: bool,
    /// The providers the surface rows of every spec of this name name.
    pub providers: Vec<SpecProvider>,
    /// The surface rows of every spec of this name, for naming the dialects
    /// that offer it.
    pub rows: Vec<SpecSurface>,
}

impl NameProviders {
    fn add_spec(&mut self, spec: &CommandSpec) {
        match spec.surface {
            None => self.unrestricted = true,
            Some(rows) => {
                for row in rows {
                    if !self.providers.contains(&row.provider) {
                        self.providers.push(row.provider);
                    }
                    if !self.rows.contains(row) {
                        self.rows.push(*row);
                    }
                }
            }
        }
    }
}

impl AllDialectCommandNames {
    pub(super) fn add(&mut self, spec: &CommandSpec) {
        // Normalise away a leading `::` so a spec registered only in
        // its fully-qualified spelling (e.g.
        // `::tcl::unsupported::corotype`, which has no separate bare
        // registration) still matches `known_in_any_dialect`'s
        // already-bare query — the caller strips a literal `::` head
        // from the source text before calling in, so the set must be
        // bare-normalised too or the two never agree.
        let name = tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name);
        self.known.insert(name);
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT)
        {
            self.rootable.insert(name);
        }
        self.providers.entry(name).or_default().add_spec(spec);
    }
}

/// The names every compiled-in spec set offers.
fn compiled_command_names() -> &'static AllDialectCommandNames {
    static NAMES: OnceLock<AllDialectCommandNames> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names = AllDialectCommandNames {
            known: FxHashSet::default(),
            rootable: FxHashSet::default(),
            providers: FxHashMap::default(),
        };
        let mut add = |specs: Vec<CommandSpec>| {
            for spec in &specs {
                names.add(spec);
            }
        };
        add(crate::commands::bpf::bpf_command_specs());
        add(crate::commands::tcl::tcl_command_specs());
        add(crate::commands::stdlib::stdlib_command_specs());
        add(crate::commands::tcllib::tcllib_command_specs());
        add(crate::commands::argparse::argparse_command_specs());
        add(crate::commands::ticklecharts::ticklecharts_command_specs());
        add(crate::commands::itcl::itcl_command_specs());
        add(crate::commands::tk::tk_command_specs());
        add(crate::commands::irules::irules_command_specs());
        add(crate::commands::iapps::iapps_command_specs());
        add(crate::commands::expect::expect_command_specs());
        // The EDA vendor libraries are deliberately NOT added: they ship as
        // bundled `.tclspec` loadables (`docs/design/registry/spec-packs.md`), so this
        // crate does not know their names at compile time and W002 reports an
        // EDA command outside an EDA profile as an ordinary unknown command
        // rather than as "exists, but not here". The pack is what knows.
        // SpecTcl is deliberately NOT added. This set answers "is this name a
        // command in *some* dialect", and W002 turns a `true` into "exists,
        // but not here". SpecTcl's statement words are ordinary English nouns
        // (`arity`, `traits`, `value`, `detail`) that mean nothing outside a
        // pack body, so claiming them here would rewrite an honest
        // unknown-command report on a user's `proc arity` call into a
        // misleading dialect-availability one — the exact opposite of the
        // context-sensitivity the SpecTcl grammars exist to provide.
        names
    })
}

/// The command universe: the compiled-in names plus the registered
/// core-surface specs' (a family's own commands, such as Jim's `loop`), built
/// once per registered generation.
pub(super) fn all_dialect_command_names() -> &'static AllDialectCommandNames {
    static WITH_CORE_SURFACE: RwLock<Option<(u64, &'static AllDialectCommandNames)>> =
        RwLock::new(None);
    let compiled = compiled_command_names();
    let registered = crate::cache::core_surface_generation();
    if registered == 0 {
        return compiled;
    }
    let held = WITH_CORE_SURFACE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .filter(|&(built, _)| built == registered)
        .map(|(_, names)| names);
    if let Some(names) = held {
        return names;
    }
    let (generation, specs) = crate::cache::core_surface_specs();
    let mut merged = compiled.clone();
    for spec in specs {
        merged.add(spec);
    }
    // One allocation per registered generation, held for the process.
    let merged: &'static AllDialectCommandNames = Box::leak(Box::new(merged));
    *WITH_CORE_SURFACE
        .write()
        .unwrap_or_else(PoisonError::into_inner) = Some((generation, merged));
    merged
}

/// Whether `spec` may serve as the bare-name fallback for a rooted spelling.
/// Tcl's global built-ins are authored under bare registry keys, so `::set`
/// must fall back to `set`; `TclOO` method-context commands are also authored
/// bare but do not exist as `::my`, `::self`, and so on. A namespaced spelling
/// such as `::oo::Helpers::self` remains an ordinary qualified fallback.
pub(super) fn rooted_fallback_allowed(rooted: &str, spec: &CommandSpec) -> bool {
    tcl_syntax::naming::unroot_rooted_key(rooted).is_none_or(|unrooted| {
        tcl_syntax::naming::is_qualified(unrooted.as_bytes())
            || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT)
    })
}

impl CommandRegistry {
    /// Every special variable this registry knows: the rows loaded packs
    /// declared, the latest first, then the shipped table
    /// ([`crate::special_vars::SPECIAL_VARS`]) less any name a pack row
    /// already answers for — the one door a consumer reads, so a variable a
    /// pack declares answers exactly as a shipped one does. A pack row
    /// shadows a shipped row of the same name, as an authored command spec
    /// shadows a shipped one.
    pub fn special_vars(
        &self,
    ) -> impl Iterator<Item = &'static crate::special_vars::SpecialVarSpec> + '_ {
        let declared = || self.special_vars.iter().rev().copied();
        declared().chain(
            crate::special_vars::SPECIAL_VARS
                .iter()
                .filter(move |shipped| !declared().any(|pack| pack.name == shipped.name)),
        )
    }

    /// The special variable named `name`, ignoring dialect — the registry
    /// face of [`crate::special_vars::special_var`], pack rows included.
    #[must_use]
    pub fn special_var(&self, name: &str) -> Option<&'static crate::special_vars::SpecialVarSpec> {
        self.special_vars().find(|spec| spec.name == name)
    }

    /// The special variable named `name` when `dialect` provides it — the
    /// registry face of [`crate::special_vars::special_var_in_dialect`].
    #[must_use]
    pub fn special_var_in_dialect(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static crate::special_vars::SpecialVarSpec> {
        self.special_var(name)
            .filter(|spec| spec.available_in(dialect))
    }

    /// The special variables `dialect` provides, pack rows first — the
    /// registry face of [`crate::special_vars::special_vars_for_dialect`].
    pub fn special_vars_for_dialect<'a>(
        &'a self,
        dialect: Option<SurfaceQuery<'a>>,
    ) -> impl Iterator<Item = &'static crate::special_vars::SpecialVarSpec> + 'a {
        self.special_vars()
            .filter(move |spec| spec.available_in(dialect))
    }

    /// Startup binding and read-trace facts for the bare global `name`.
    /// Pack-declared variables use the same release query as built-in ones.
    #[must_use]
    pub fn startup_existence(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> crate::special_vars::StartupExistence {
        self.special_var(name)
            .map_or_else(Default::default, |spec| spec.startup_existence(dialect))
    }

    /// Whether the runtime observes a write to `name` in `dialect`.
    /// Pack declarations participate alongside the built-in registry.
    #[must_use]
    pub fn is_externally_read(&self, name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
        self.special_var_in_dialect(name, dialect)
            .is_some_and(|spec| spec.externally_read)
    }

    /// Whether `name` exists as a command in *any* dialect, independent of
    /// which dialects this registry instance loaded.
    ///
    /// Looks up the global by-name index across every dialect's specs.
    /// Like [`Self::taint_source`], this is deliberately dialect-agnostic:
    /// an iRules command such as `when` is "known" even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules specs — the
    /// W002 disabled-in-dialect check needs to distinguish "exists, but not
    /// in this dialect" (→ DISALLOWED) from "exists nowhere" (→ W123's
    /// concern). A leading `::` follows [`Self::get`]'s bare-name fallback,
    /// including its exclusion for method-context-only singleton commands.
    #[must_use]
    pub fn known_in_any_dialect(&self, name: &str) -> bool {
        let names = all_dialect_command_names();
        match tcl_syntax::naming::unroot_rooted_key(name) {
            Some(unrooted) => names.rootable.contains(unrooted),
            None => names.known.contains(name),
        }
    }

    /// Canonical authored names across the compiled universe and actual store.
    /// Availability, source identity and runtime binding remain separate queries.
    pub fn command_names_in_any_dialect(&self) -> impl Iterator<Item = &'static str> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        all_dialect_command_names()
            .rootable
            .iter()
            .copied()
            .chain(
                self.by_name
                    .values()
                    .flatten()
                    .copied()
                    .chain(self.overlay_specs.iter().copied())
                    .filter(|spec| {
                        tcl_syntax::naming::is_qualified(spec.name.as_bytes())
                            || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT)
                    })
                    .map(|spec| spec.name),
            )
            .map(|name| tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
    }

    /// Source availability providers from the compiled universe and this
    /// actual store's authored rows. Neither collection establishes a lookup.
    #[must_use]
    pub fn source_availability_providers(&self, command: &str) -> Option<NameProviders> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let name = tcl_syntax::naming::unroot_rooted_key(command).unwrap_or(command);
        if !all_dialect_command_names().rootable.contains(name)
            && !self
                .by_name
                .values()
                .flatten()
                .chain(self.overlay_specs.iter())
                .any(|spec| {
                    tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name) == name
                        && (tcl_syntax::naming::is_qualified(name.as_bytes())
                            || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT))
                })
        {
            return None;
        }
        let mut providers = self.providers_in_any_dialect(command).cloned();
        for spec in self
            .by_name
            .values()
            .flatten()
            .chain(self.overlay_specs.iter())
        {
            if tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name) == name {
                providers
                    .get_or_insert_with(NameProviders::default)
                    .add_spec(spec);
            }
        }
        providers
    }

    /// Who offers `name` across every compiled-in dialect: the providers its
    /// specs' surface rows name, and whether any spec of the name is offered
    /// to every dialect. `None` exactly when
    /// [`Self::known_in_any_dialect`] is `false`.
    ///
    /// The other half of the W002 question. "Exists in some dialect, not this
    /// one" says the name is disabled here only when this document's
    /// environment stands in some relation to the dialect that has it; a name
    /// only an unrelated environment offers (Expect's `system` in a Jim
    /// document) is unknown here, not disabled here. The universe is the one
    /// [`Self::known_in_any_dialect`] reads, so a runtime pack's own names are
    /// absent from both.
    #[must_use]
    pub fn providers_in_any_dialect(&self, name: &str) -> Option<&'static NameProviders> {
        if !self.known_in_any_dialect(name) {
            return None;
        }
        let unrooted = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
        all_dialect_command_names().providers.get(unrooted)
    }

    /// Look up a command spec by name (dialect-agnostic).
    ///
    /// A leading `::` (global qualifier) falls back to the bare name, so an
    /// explicitly-global call to a built-in (`::foreach`, `::for`, …)
    /// resolves to the same spec as its unqualified form. The fallback rejects
    /// a method-context-only singleton: Tcl has a contextual `my` but no
    /// global `::my`. Real qualified commands such as
    /// `::oo::Helpers::self` retain their namespaced fallback.
    ///
    /// The return is `&'static` because the registry stores only interned
    /// static specs — the identity [`crate::model::SpecKey`] keys the realm
    /// binding layer; the generation work re-keys dynamic pack
    /// specs when non-`'static` specs join.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&'static CommandSpec> {
        if let Some(spec) = self.by_name.get(name).and_then(|v| v.last().copied()) {
            return Some(spec);
        }
        let unrooted = tcl_syntax::naming::unroot_rooted_key(name)?;
        let spec = self.by_name.get(unrooted)?.last().copied()?;
        rooted_fallback_allowed(name, spec).then_some(spec)
    }

    /// Exact spelling lookup; unlike [`Self::get`], does not apply rooted
    /// global-name fallback.
    #[must_use]
    pub fn get_exact(&self, name: &str) -> Option<&'static CommandSpec> {
        self.by_name.get(name).and_then(|v| v.last().copied())
    }

    /// Whether this source catalogue declares a command at a constructed
    /// rooted or unrooted reporting key. This compatibility interface removes
    /// exactly one leading root marker; callers with a known rooted key use
    /// [`Self::declares_command_at_rooted_key`].
    ///
    /// The remaining bytes must equal the selected descriptor's exact label.
    /// Written command spelling lookup, imports and repeated-colon
    /// canonicalisation cannot supply that equality. Bare operator imports
    /// and method-context-only commands retain their separate declaration
    /// conventions; their qualified declarations remain available.
    ///
    /// Package declarations remain in the catalogue with their
    /// [`CommandSpec::required_package`] or `tcllib_package` metadata. This
    /// query does not establish package loading, an installed command table,
    /// a current Native binding or namespace/import/export authority.
    #[must_use]
    pub fn declares_command_at(&self, qualified_name: &str) -> bool {
        let key = tcl_syntax::naming::unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.declares_command_at_unrooted_key(key)
    }

    /// Source-catalogue declaration metadata for a rooted constructed key.
    /// Removes exactly the root supplied by the constructed-key producer;
    /// every remaining byte, including leading literal colons, survives.
    /// An unrooted argument declines. No written-name or Native lookup runs.
    #[must_use]
    pub fn declares_command_at_rooted_key(&self, key: &str) -> bool {
        tcl_syntax::naming::unroot_rooted_key(key)
            .is_some_and(|key| self.declares_command_at_unrooted_key(key))
    }

    fn declares_command_at_unrooted_key(&self, key: &str) -> bool {
        self.get_exact(key).is_some_and(|spec| {
            let descriptor = crate::CanonicalCommandDescriptorKey::from_registry_label(spec.name);
            if descriptor.label() != key {
                return false;
            }
            let qualified = !tcl_syntax::naming::key_holder_and_tail(key).0.is_empty();
            if !qualified && spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT) {
                return false;
            }
            qualified || !spec.traits.contains(Traits::OPERATOR_COMMAND)
        })
    }

    /// The typed BPF-Tcl lowering descriptor for `name`, when `name` is a
    /// BPF-dialect command (see [`crate::bpf_op`]).  The BPF-Tcl front-end
    /// dispatches on this — never on the command name.
    #[must_use]
    pub fn bpf_op(&self, name: &str) -> Option<&'static crate::bpf_op::BpfOpSpec> {
        self.get(name).and_then(|s| s.bpf_op)
    }

    /// BPF metadata for an independently selected exact descriptor label.
    /// This performs no written-name lookup and grants no runtime binding.
    #[must_use]
    pub fn bpf_op_for_descriptor(
        &self,
        key: crate::CanonicalCommandDescriptorKey<'_>,
    ) -> Option<&'static crate::bpf_op::BpfOpSpec> {
        self.get_exact(key.label()).and_then(|spec| spec.bpf_op)
    }

    /// Native lowering metadata for an independently selected exact descriptor.
    /// The supplied availability selects stamp windows only; this provides no
    /// original source, runtime binding or Native compilation admission.
    #[must_use]
    pub fn native_lowering_for_descriptor(
        &self,
        key: crate::CanonicalCommandDescriptorKey<'_>,
        query: Option<&SurfaceQuery<'_>>,
    ) -> Option<crate::NativeLowering> {
        self.get_exact(key.label())
            .map(|spec| spec.native_lowering_at(query))
    }

    /// Validated condition/body positions for a closed BPF language command.
    /// The frontend must separately retain literal original words and reject
    /// expansions. This syntax projection provides no Tcl execution evidence.
    #[must_use]
    pub fn bpf_conditional_operands(
        &self,
        name: &str,
        arguments: &[&str],
    ) -> Option<Vec<(Option<usize>, usize)>> {
        (self.bpf_op(name)?.kind == crate::bpf_op::BpfOpKind::Conditional)
            .then(|| crate::commands::tcl::bpf_conditional_operands(arguments))?
    }

    /// Find the registered spelling for a typed BPF-Tcl operation.
    ///
    /// This is the reverse lookup companion to [`Self::bpf_op`].  Tools that
    /// generate BPF-Tcl source can select an operation by its registry-owned
    /// [`crate::bpf_op::BpfOpKind`] instead of embedding a command spelling.
    /// It deliberately returns the first registered spelling: aliases must
    /// share an operation descriptor, and the canonical command specs are
    /// registered before any compatibility aliases.
    #[must_use]
    pub fn bpf_command_for(&self, kind: crate::bpf_op::BpfOpKind) -> Option<&'static str> {
        self.by_name
            .values()
            .flat_map(|specs| specs.iter())
            .find_map(|spec| (spec.bpf_op.is_some_and(|op| op.kind == kind)).then_some(spec.name))
    }

    /// Look up a command spec filtered by dialect, picking the
    /// **most-specific** visible spec (`best_visible` — §5.3's single
    /// selection rule).
    ///
    /// As with [`Self::get`], a leading `::` falls back to the bare name while
    /// rejecting method-context-only singleton spellings.
    ///
    /// A registry built for a dialect profile additionally applies that
    /// profile's operator-head exclusion ([`Self::spec_visible`]) whenever
    /// the query is about the profile's own surface — so an f5-irules query
    /// on the f5-irules registry sees exactly the specs iRules ships, no
    /// matter which consumer asks (dialect-profile-model.md §9.2).
    #[must_use]
    pub fn get_for_surface(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static CommandSpec> {
        if let Some(specs) = self.by_name.get(name) {
            return self.best_visible(specs, dialect);
        }
        let unrooted = tcl_syntax::naming::unroot_rooted_key(name)?;
        let spec = self.best_visible(self.by_name.get(unrooted)?, dialect)?;
        rooted_fallback_allowed(name, spec).then_some(spec)
    }

    /// Whether this registry's **own dialect** has a command spelled `head`.
    ///
    /// The public face of [`Self::spec_for_this_registry`], for callers that
    /// only need the yes/no: a constant-folder deciding whether it may
    /// pre-compute a call, say. Folding is a rewrite that skips the runtime's
    /// availability gate entirely, so a fold that does not ask this question
    /// silently *adds* the command to releases that never had it — a
    /// `tcl8.4` compile folded `[dict create a 1 a 2]` to a literal instead of
    /// raising `invalid command name "dict"`.
    ///
    /// A profile-less registry (`build_default`) answers dialect-agnostically,
    /// exactly as [`Self::get`] does.
    #[must_use]
    pub fn has_command_in_this_dialect(&self, head: &str) -> bool {
        self.spec_for_this_registry(head).is_some()
    }

    /// The release arity of the spec this registry's own dialect selects for
    /// `head`, when that spec came from a pack the user wrote
    /// ([`Provenance::is_user_authored`]) — a description of the user's own
    /// command rather than of one a runtime, package or vendor ships.
    ///
    /// [`Provenance::is_user_authored`]: tcl_dialect::model::Provenance::is_user_authored
    #[must_use]
    pub fn user_authored_pack_arity(&self, head: &str) -> Option<Arity> {
        let spec = self.spec_for_this_registry(head)?;
        self.pack_origin(spec)?
            .provenance
            .is_user_authored()
            .then(|| self.release_arity(spec))
    }

    /// Whether `spec` is visible to any of `providers` — the coarse,
    /// version-blind twin of [`Self::spec_visible`] the static grammars use.
    ///
    /// First-paint highlighting has no resolved release to ask about, so it
    /// asks which providers the profile's language spans. The operator-head
    /// exclusion still applies: math operators are not command heads under
    /// iRules at any release.
    #[must_use]
    pub fn spec_visible_to(&self, spec: &CommandSpec, providers: &[SpecProvider]) -> bool {
        spec.surface
            .is_none_or(|rows| surface_provided_by(rows, providers))
            && self.profile.is_none_or(|profile| {
                profile.operators_as_commands
                    || !spec
                        .traits
                        .contains(crate::traits::Traits::OPERATOR_COMMAND)
            })
    }

    /// The full availability test for a point query on this registry: the
    /// spec's own surface gate, plus — when this registry was built for a
    /// profile and the query is that profile's own point — the
    /// operator-command exclusion (§9), for a profile whose math operators
    /// are not command heads.
    ///
    /// There is no disable list: availability is fully explicit in each
    /// spec's surface rows, so a sandbox-banned command such as `exec`
    /// simply never names the iRules family.
    ///
    /// Public because generators projecting a command surface for an
    /// explicit point need the same exclusion semantics `get_for_surface`
    /// applies internally.
    #[must_use]
    pub fn spec_visible(&self, spec: &CommandSpec, dialect: Option<SurfaceQuery<'_>>) -> bool {
        if !dialect.map_or_else(
            || spec.supports_dialect(None),
            |query| crate::irules_policy::runtime_surface_admits(spec, query),
        ) {
            return false;
        }
        let Some(profile) = self.profile else {
            return true;
        };
        if dialect.is_none_or(|query| !query.same_point(&profile.surface_query())) {
            // The query is about some other surface's availability; this
            // profile's operator-exclusion does not apply to it.
            return true;
        }
        // iRules availability is stated positively by each spec, so there
        // is no subtractive ban list — the only profile-level exclusion left
        // is the operator-command one (math operators are not command heads
        // under iRules).
        profile.operators_as_commands
            || !spec
                .traits
                .contains(crate::traits::Traits::OPERATOR_COMMAND)
    }

    /// Return all registered command names.
    pub fn command_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().copied()
    }

    /// Exact selected labels from the current Registry descriptor roster.
    pub fn command_descriptor_keys(
        &self,
    ) -> impl Iterator<Item = crate::CanonicalCommandDescriptorKey<'_>> {
        self.command_names()
            .map(crate::CanonicalCommandDescriptorKey::from_registry_label)
    }

    /// Return command names whose command-level descriptor selects `operation`
    /// at some release.
    ///
    /// This uses the same target-neutral descriptor precedence as structured
    /// invocation resolution, over the stamps a command carries plain and in
    /// every window. It is intended for whole-module trust proofs that need to
    /// quantify over every registry spelling of one semantic operation, at
    /// whichever release the module is compiled for.
    pub fn command_names_for_semantic_operation(
        &self,
        operation: crate::SemanticOperationId,
    ) -> impl Iterator<Item = &str> {
        self.by_name.iter().filter_map(move |(name, specs)| {
            specs
                .iter()
                .any(|spec| {
                    spec.descriptor_combinations()
                        .any(|(semantic, codegen, inline_codegen)| {
                            crate::resolved_invocation::descriptor_operation(
                                semantic,
                                spec.lowering_hook,
                                codegen,
                                inline_codegen,
                            ) == Some(operation)
                        })
                })
                .then_some(*name)
        })
    }

    /// Exact pure descriptor keys reached by the existing operation roster.
    /// This retains its all-release scope and does not resolve written names.
    pub fn command_descriptor_keys_for_semantic_operation(
        &self,
        operation: crate::SemanticOperationId,
    ) -> impl Iterator<Item = crate::CanonicalCommandDescriptorKey<'_>> {
        self.command_names_for_semantic_operation(operation)
            .map(crate::CanonicalCommandDescriptorKey::from_registry_label)
    }

    /// Whether `pkg` is a package the registry knows about — i.e. at
    /// least one registered command declares it as its
    /// [`required_package`](crate::CommandSpec::required_package).
    ///
    /// Used by the W120 (missing-`package require`) check: a `package
    /// require` of an *unknown* third-party package may itself pull in
    /// arbitrary commands (e.g. a wrapper that `package require Tk`s
    /// internally), so the analyser cannot prove a Tk/extension command
    /// is unprovided and must suppress W120.
    #[must_use]
    pub fn provides_package(&self, pkg: &str) -> bool {
        self.by_name
            .values()
            .flat_map(|specs| specs.iter())
            .any(|spec| {
                spec.required_package.is_some_and(|descriptor| {
                    crate::native_package::matches_package_descriptor(pkg.as_bytes(), descriptor)
                })
            })
    }

    /// Return every registered [`CommandSpec`] for `name` (all dialects),
    /// in registration order. Empty when the name is unknown.
    ///
    /// This is the raw data view — resolution (which spec *wins* under a
    /// dialect) goes through [`Self::get_for_surface`]'s most-specific
    /// rule.
    #[must_use]
    pub fn specs(&self, name: &str) -> &[&'static CommandSpec] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }
}
