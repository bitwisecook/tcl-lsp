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

//! The command semantics in effect for one dialect and realm after overlays and native factories apply.

use super::CommandRegistry;
use super::lookup::rooted_fallback_allowed;
use crate::hooks::LoweringHookId;
use crate::spec::CommandSpec;
use crate::traits::Traits;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock, PoisonError};
use tcl_dialect::model::SurfaceQuery;

/// Descriptor facts from the effective command spec selected by a registry.
///
/// This is deliberately registry-owned: consumers can ask generic trait and
/// lowering questions without constructing their own per-command indexes or
/// reproducing release/dialect selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveCommandSemantics {
    pub(super) traits: Traits,
    pub(super) lowering_hook: Option<LoweringHookId>,
}

const JIM_COMMANDS: &str = "* + - / after alarm alias append apply array binary binary::nextarg break catch cd class clock close collect concat continue curry defer dict ensemble env eof error errorInfo eval exec exists exit expr fconfigure file fileevent finalize flush for foreach format function getref gets glob glob.explode glob.glob glob.globdir global history if incr info interp join json::decode json::encode json::subencode kill lambda lambda.finalizer lappend lassign lindex linsert list llength lmap load load_ssl_certs local loop lrange lrepeat lreplace lreverse lsearch lset lsort lsubst namespace open os.fork os.gethostname os.getids os.umask os.uptime pack package parray pid pipe popen proc puts pwd rand range read readdir ref regexp regsub rename return scan seek set setref signal sleep socket source split stackdump stacktrace stderr stdin stdout string subst super switch syslog tailcall taint tcl::autocomplete tcl::prefix tcl::stdhint tell throw time timerate tree try unpack unset untaint upcall update uplevel upvar variable vwait wait while xtrace zlib";

pub(super) fn jim_fresh_command_names() -> &'static BTreeSet<String> {
    static NAMES: OnceLock<BTreeSet<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        JIM_COMMANDS
            .split_whitespace()
            .map(crate::rooted_command_descriptor_label)
            .collect()
    })
}

impl EffectiveCommandSemantics {
    /// The full effective command trait set.
    #[must_use]
    pub const fn traits(self) -> Traits {
        self.traits
    }

    /// Whether the effective command spec carries every flag in `traits`.
    #[must_use]
    pub const fn has_traits(self, traits: Traits) -> bool {
        self.traits.contains(traits)
    }

    /// The effective command's structured lowering hook, if any.
    #[must_use]
    pub const fn lowering_hook(self) -> Option<LoweringHookId> {
        self.lowering_hook
    }
}

/// Cached semantic projection of one complete [`CommandRegistry`] generation.
///
/// The projection is built in one pass over registry spellings. Its
/// per-command facts select the same effective row as
/// [`CommandRegistry::get_for_surface`] for the registry's own profile. The
/// binding domain includes every indexed descriptor across release rows and
/// the last registered unresolved-command handlers. Authored insertion and
/// profile changes invalidate this immutable projection.
#[derive(Debug, Default)]
pub struct EffectiveRegistrySemantics {
    pub(super) commands: FxHashMap<&'static str, EffectiveCommandSemantics>,
    binding_names: BTreeSet<String>,
    unresolved_command_handlers: BTreeSet<String>,
    binding_fingerprint: u64,
}

impl EffectiveRegistrySemantics {
    /// Rooted descriptor labels in the indexed binding domain across release
    /// rows. Literal colons and empty final components remain unchanged.
    #[must_use]
    pub const fn binding_names(&self) -> &BTreeSet<String> {
        &self.binding_names
    }

    /// Rooted descriptor labels for unresolved-command fallback carriers.
    #[must_use]
    pub const fn unresolved_command_handlers(&self) -> &BTreeSet<String> {
        &self.unresolved_command_handlers
    }

    /// Stable fingerprint of the command-binding domain.
    ///
    /// Callers must still compare the two sets for equality; this value exists
    /// so state hashes never walk the full registry universe.
    #[must_use]
    pub const fn binding_fingerprint(&self) -> u64 {
        self.binding_fingerprint
    }

    /// Facts for `name`'s effective command spec, or `None` when the spelling
    /// is unavailable in this registry's exact release/dialect surface.
    #[must_use]
    pub fn command(&self, name: &str) -> Option<EffectiveCommandSemantics> {
        self.commands.get(name).copied()
    }

    /// Effective command names carrying `traits`.
    ///
    /// This iterator is principally useful for drift tests and reports. Hot
    /// consumers should query [`Self::command`] at the call site instead of
    /// rebuilding another complete classification set.
    pub fn command_names_with_traits(
        &self,
        traits: Traits,
    ) -> impl Iterator<Item = &'static str> + '_ {
        self.commands
            .iter()
            .filter_map(move |(&name, facts)| facts.has_traits(traits).then_some(name))
    }
}

impl CommandRegistry {
    /// The one-pass semantic projection of this exact registry generation.
    ///
    /// Repeated calls return the same `Arc`. Any later authored insertion or
    /// profile change invalidates the registry's cell, so the next call builds
    /// a fresh projection while existing readers safely retain the old,
    /// immutable generation.
    #[must_use]
    pub fn effective_semantics(&self) -> Arc<EffectiveRegistrySemantics> {
        Arc::clone(self.effective_semantics.get_or_init(|| {
            let query = self.own_surface_query();
            let mut commands = FxHashMap::default();
            let mut binding_names = BTreeSet::new();
            let mut unresolved_command_handlers = BTreeSet::new();

            for (&name, specs) in &self.by_name {
                let binding_name = crate::rooted_command_descriptor_label(name);
                binding_names.insert(binding_name.clone());
                if specs
                    .last()
                    .is_some_and(|spec| spec.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER))
                {
                    unresolved_command_handlers.insert(binding_name);
                }
                let selected = match query {
                    Some(query) => self.best_visible(specs, Some(query)),
                    None => specs.last().copied(),
                };
                let Some(spec) = selected else {
                    continue;
                };
                commands.insert(
                    name,
                    EffectiveCommandSemantics {
                        traits: spec.traits,
                        lowering_hook: spec.lowering_hook,
                    },
                );
            }

            // These native slots belong to TclOO's definition namespace,
            // rather than ordinary callable catalogue rows. Their owner and
            // release windows come from the admitted native grammar.
            if let Some(grammar) = self.default_construction_grammar("oo::class") {
                for member in grammar.members {
                    for receiver in [
                        crate::definer::DefinitionReceiver::Instance,
                        crate::definer::DefinitionReceiver::Class,
                    ] {
                        if let Some(lookup) =
                            grammar.definition_member_lookup_for_receiver(member, receiver, query)
                        {
                            binding_names.insert(lookup.implementation);
                        }
                    }
                }
            }

            if let Some(profile) = self.profile {
                binding_names.extend(
                    self.stock_native_implementation_slots(crate::InvocationDialect::of_profile(
                        profile,
                    ))
                    .into_iter()
                    .map(|lookup| lookup.slot.to_owned()),
                );
            }

            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            if let Some(profile) = self.profile {
                let dialect = crate::InvocationDialect::of_profile(profile);
                if let Some(support) = self
                    .native_class_factory_recipe(
                        "oo::configurable",
                        dialect,
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                    )
                    .and_then(crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
                {
                    binding_names.extend(
                        support
                            .binding_names()
                            .iter()
                            .map(|name| (*name).to_owned()),
                    );
                }
            }
            binding_names.hash(&mut hasher);
            unresolved_command_handlers.hash(&mut hasher);
            Arc::new(EffectiveRegistrySemantics {
                commands,
                binding_names,
                unresolved_command_handlers,
                binding_fingerprint: hasher.finish(),
            })
        }))
    }

    /// Descriptor roster for a separate authored source model under the
    /// actual full availability context. This supplies no native interpreter
    /// presence, compiler implementation slots or name policy. The source
    /// driver must independently retain its positively selected Logical input.
    #[must_use]
    pub fn authored_source_semantics_in_context(
        &self,
        context: &crate::model::ResolvedContext,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Arc<EffectiveRegistrySemantics> {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let mut commands = FxHashMap::default();
        let mut binding_names = BTreeSet::new();
        let mut unresolved_command_handlers = BTreeSet::new();
        for &name in self.by_name.keys() {
            let Some(spec) = context.resolve_spec_in_realm(self, name, realm) else {
                continue;
            };
            let binding_name = crate::rooted_command_descriptor_label(name);
            // Source metadata remains known for hosted packages, but their
            // names are not initial bindings until this context activates them.
            let binding_available = spec
                .owning_package()
                .is_none_or(|package| context.package_provider_active(package));
            if binding_available && rooted_fallback_allowed(&binding_name, spec) {
                binding_names.insert(binding_name.clone());
            }
            if binding_available && spec.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER) {
                unresolved_command_handlers.insert(binding_name);
            }
            commands.insert(
                name,
                EffectiveCommandSemantics {
                    traits: spec.traits,
                    lowering_hook: spec.lowering_hook,
                },
            );
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        binding_names.hash(&mut hasher);
        unresolved_command_handlers.hash(&mut hasher);
        Arc::new(EffectiveRegistrySemantics {
            commands,
            binding_names,
            unresolved_command_handlers,
            binding_fingerprint: hasher.finish(),
        })
    }

    /// Entry command table for a concrete execution dialect. Jim's current
    /// default build has a distinct command roster; catalogue union entries
    /// cannot establish the presence of C-only commands or autoload handlers.
    #[must_use]
    pub fn effective_semantics_for_dialect(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Arc<EffectiveRegistrySemantics> {
        self.effective_semantics_for_dialect_in_realm(
            dialect,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
    }

    /// Entry command table under an explicit availability phase. Runtime
    /// presence does not license rule-loader source or native compiler hooks.
    #[must_use]
    pub fn effective_semantics_for_dialect_in_realm(
        &self,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Arc<EffectiveRegistrySemantics> {
        if let Some(cached) = self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .dialect_semantics
            .get(&(dialect, realm))
        {
            return Arc::clone(cached);
        }
        let semantics = self.effective_semantics_for_dialect_in_realm_uncached(dialect, realm);
        self.native_registrations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .dialect_semantics
            .insert((dialect, realm), Arc::clone(&semantics));
        semantics
    }

    fn effective_semantics_for_dialect_in_realm_uncached(
        &self,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Arc<EffectiveRegistrySemantics> {
        let query = dialect.authoring_query().map(|mut query| {
            if let Some(profile) = self
                .profile
                .filter(|profile| crate::InvocationDialect::of_profile(profile) == dialect)
            {
                query.packages =
                    tcl_dialect::model::PackageFloors::Borrowed(profile.surface_packages);
            }
            query.with_realm(realm)
        });
        let jim = dialect.family() == Some(tcl_dialect::model::Family::Jim);
        // A roster observed for one release/build cannot prove any other build.
        let audited_jim = dialect.core_point
            == Some(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            ));

        let jim_names = jim_fresh_command_names();
        let mut binding_names = if jim && audited_jim {
            jim_names.clone()
        } else {
            BTreeSet::new()
        };
        let mut commands = FxHashMap::default();
        let mut unresolved_command_handlers = BTreeSet::new();
        for (&name, specs) in &self.by_name {
            let Some(query) = query else {
                continue;
            };
            let Some(spec) = self.selected_entry_descriptor(specs, dialect, query) else {
                continue;
            };
            let binding_name = crate::rooted_command_descriptor_label(name);
            if jim {
                if !audited_jim || !jim_names.contains(&binding_name) {
                    continue;
                }
            } else {
                // Unversioned C admits only commands present across the whole
                // supported ladder. A catalogue union supplies assistance only.
                if dialect.family() == Some(tcl_dialect::model::Family::Tcl)
                    && dialect.tcl_version.is_none()
                    && !tcl_dialect::TclVersion::ALL.into_iter().all(|release| {
                        self.best_visible(
                            specs,
                            Some(SurfaceQuery::core(
                                tcl_dialect::model::Family::Tcl,
                                release.version_string(),
                            )),
                        )
                        .is_some()
                    })
                {
                    continue;
                }
                if rooted_fallback_allowed(&binding_name, spec) {
                    binding_names.insert(binding_name.clone());
                }
            }
            commands.insert(
                name,
                EffectiveCommandSemantics {
                    traits: spec.traits,
                    lowering_hook: spec.lowering_hook,
                },
            );
            if spec.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER) {
                unresolved_command_handlers.insert(binding_name);
            }
        }
        if !jim
            && let Some(spec) = self.get_for_surface("oo::class", query)
            && let Some(grammar) = spec.definition_body
        {
            for member in grammar.members {
                for receiver in [
                    crate::definer::DefinitionReceiver::Instance,
                    crate::definer::DefinitionReceiver::Class,
                ] {
                    if let Some(lookup) =
                        grammar.definition_member_lookup_for_receiver(member, receiver, query)
                    {
                        binding_names.insert(lookup.implementation);
                    }
                }
            }
        }
        binding_names.extend(
            self.stock_native_implementation_slots(dialect)
                .into_iter()
                .map(|lookup| lookup.slot.to_owned()),
        );
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.extend_native_factory_binding_names(&mut binding_names, dialect, realm);
        binding_names.hash(&mut hasher);
        unresolved_command_handlers.hash(&mut hasher);
        dialect.hash(&mut hasher);
        realm.hash(&mut hasher);
        Arc::new(EffectiveRegistrySemantics {
            commands,
            binding_names,
            unresolved_command_handlers,
            binding_fingerprint: hasher.finish(),
        })
    }

    fn extend_native_factory_binding_names(
        &self,
        names: &mut BTreeSet<String>,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) {
        if let Some(support) = self
            .native_class_factory_recipe("oo::configurable", dialect, realm)
            .and_then(crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
        {
            names.extend(support.binding_names().iter().copied().map(str::to_owned));
        }
    }

    /// Select an entry descriptor under the actual surface and realm, admitting
    /// package-owned commands only from the matching profile's ambient packages.
    fn selected_entry_descriptor(
        &self,
        specs: &[&'static CommandSpec],
        dialect: crate::InvocationDialect,
        query: SurfaceQuery<'_>,
    ) -> Option<&'static CommandSpec> {
        let spec = self.best_visible(specs, Some(query))?;
        if spec.owning_package().is_some_and(|package| {
            !self.profile.is_some_and(|profile| {
                crate::InvocationDialect::of_profile(profile) == dialect
                    && profile.is_ambient_package(package)
            })
        }) {
            return None;
        }
        Some(spec)
    }
}
