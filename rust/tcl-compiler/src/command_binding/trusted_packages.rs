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

//! Trusted package loaders, the `package` transitions and the liveness of a provider's surface.

use super::binding_projection::{command_holder, source_binding};
use super::command_identity::CommandToken;
use super::executed_script_source::{ExecutedScriptMapping, ExecutedScriptSource};
#[cfg(any(test, debug_assertions))]
use super::original_command_table;
use super::source_origin::{SourceOriginId, SourceOriginKind};
use super::{
    BindingKind, MayBinding, ModuleCommandBindings, PackageProvision, ResolvedCommandTarget,
    trusted_package_transfer,
};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash;
use std::sync::Arc;
use tcl_registry::{CommandRegistry, StateTransition, TransitionSubject};

/// A selected standard-distribution loader contract supplied by the driver.
/// Package-name advertisements alone never create this provenance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustedPackageLoader {
    /// Native engine family audited by this provider, independent of its name.
    pub required_core_family: Option<tcl_dialect::model::Family>,
    /// Package whose unmodified loader is supplied at entry.
    pub package: String,
    /// Exact selected package version, when established by the provider.
    pub version: Option<String>,
    /// The surface is declared for the package rather than for one audited
    /// version, so any literal version requirement selects it.
    pub accepts_any_requested_version: bool,
    /// Stable provider/source implementation identity.
    pub implementation_id: String,
    /// Commands installed by this selected loader.
    pub command_surface: Vec<String>,
    /// Actual compiler registrations of installed commands, independently of handler semantics.
    /// Missing rows preserve uncertainty; a package name or factory shape is not a stamp.
    pub compiler_hooks:
        BTreeMap<String, tcl_runtime_api::native_compilation::NativeCompilerHookPresence>,
    /// Driver-attested definition recipes for these exact installed factories.
    /// The command, lookup and namespace-state dependencies must also remain live.
    /// This attestation does not prove constructor closure or an object class.
    pub definition_dispatchers: BTreeSet<(String, tcl_registry::definer::DefinitionDispatcher)>,
    /// Commands present in some audited version alternatives, never assumed absent.
    pub optional_command_surface: Vec<String>,
    /// Export patterns installed by this selected loader.
    pub namespace_exports: BTreeMap<String, Vec<String>>,
    /// Export patterns present in some audited version alternatives.
    pub optional_namespace_exports: BTreeMap<String, Vec<String>>,
    /// Lookup dependencies required before executing the selected loader.
    /// They remain prerequisites of its installed implementation afterwards.
    pub lookup_dependencies: Vec<TrustedCommandLookup>,
    /// Lookup dependencies of the installed implementation, checked after loading.
    /// Commands supplied by the loader itself cannot be preload prerequisites.
    pub installed_lookup_dependencies: Vec<TrustedCommandLookup>,
    /// Namespace storage whose callbacks influence the implementation.
    pub state_dependency_namespaces: Vec<String>,
    /// Cells with effects explicitly represented by the provider contract.
    pub modelled_state_variables: Vec<String>,
}

/// A driver-attested file read consumed only by a reached native source handler.
///
/// The decoded bytes and physical file identity are exact. A workspace filename
/// or known procedure declaration does not certify that this file is loaded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustedSourceModuleLoader {
    /// Native engine family audited by the driver.
    pub required_core_family: tcl_dialect::model::Family,
    /// Actual evaluated filename accepted by this file-read contract.
    pub path: String,
    /// Explicit encoding name, or the audited native default when absent.
    pub encoding: Option<String>,
    /// Exact decoded source and its driver-owned file implementation identity.
    pub source: ExecutedScriptSource,
    /// Rooted command names an auto-load index maps to this file. When one is
    /// called while absent under the stock `unknown` handler, the file is
    /// sourced in full and the call retried, as `auto_load` does.
    pub autoload: Vec<String>,
}

impl TrustedSourceModuleLoader {
    /// Retain an actual driver-validated file implementation and decoded bytes.
    /// The driver must apply the selected native encoding and file EOF protocol.
    #[must_use]
    pub fn new(
        required_core_family: tcl_dialect::model::Family,
        path: String,
        encoding: Option<String>,
        implementation_id: String,
        text: &Arc<str>,
    ) -> Self {
        let origin = Arc::new(SourceOriginId::loaded(
            Arc::from(path.as_str()),
            Arc::from(implementation_id),
            text,
        ));
        Self {
            required_core_family,
            path,
            encoding,
            source: ExecutedScriptSource {
                text: origin.source_image().clone(),
                origin,
                mapping: ExecutedScriptMapping::Contiguous { base: 0 },
            },
            autoload: Vec::new(),
        }
    }

    /// A sibling workspace document offered for auto-loading. It keeps the
    /// document's own authored origin, so what it declares is the same
    /// declaration the document's analysis retains, whichever loads it.
    #[must_use]
    pub fn workspace_document(path: String, text: &str, names: Vec<String>) -> Self {
        let image = tcl_lexer::SourceImage::document(text);
        Self {
            required_core_family: tcl_dialect::model::Family::Tcl,
            path,
            encoding: None,
            source: ExecutedScriptSource {
                text: image.clone(),
                origin: Arc::new(SourceOriginId::authored_image(image)),
                mapping: ExecutedScriptMapping::Contiguous { base: 0 },
            },
            autoload: names,
        }
    }

    /// The same contract, auto-loadable for each of `names` (rooted).
    #[must_use]
    pub fn with_autoload(mut self, names: Vec<String>) -> Self {
        self.autoload = names;
        self
    }

    /// Validate a selected native file read against this exact retained contract.
    #[must_use]
    pub fn matches_selected_file(
        &self,
        path: &str,
        encoding: Option<&str>,
        dialect: tcl_registry::InvocationDialect,
    ) -> bool {
        dialect.family() == Some(self.required_core_family)
            && self.path == path
            && self.encoding.as_deref() == encoding
            && self.source.base() == 0
            && matches!(self.source.origin.kind(), SourceOriginKind::Loaded {
                path: retained, implementation_id, source,
            } if retained.as_ref() == path && !implementation_id.is_empty() && source == &self.source.text)
    }
}

/// One implementation lookup consumed by an explicitly selected provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustedCommandLookup {
    /// Command word evaluated by the selected implementation.
    pub head: String,
    /// Original rooted namespace operand declared by the provider. Selection
    /// uses the retained native root and exact namespace rows, never a display.
    pub namespace: String,
    /// Registry implementation identity required by the provider.
    pub registry_identity: String,
    /// Whether this dependency exists in every audited version alternative.
    pub required: bool,
}

impl PackageProvision {
    pub(super) fn absent() -> Self {
        Self {
            present: false,
            version: None,
            implementation: None,
        }
    }
}

impl ModuleCommandBindings {
    pub(super) fn provider_surface_is_live(&self, loader: &TrustedPackageLoader) -> bool {
        #[cfg(any(test, debug_assertions))]
        let provider_timing = original_command_table::ProviderValidationTiming::at_query();
        let Some(root) = self.source_root_namespace_key() else {
            return false;
        };
        if self.tainted_provider_state.contains(&loader.package) {
            return false;
        }
        let required = loader.command_surface.iter().all(|command| {
            #[cfg(any(test, debug_assertions))]
            if provider_timing.is_some() {
                original_command_table::ProviderValidationTiming::required_row();
            }
            let proof = source_binding(self, command, &root);
            proof.proved_target().is_some_and(|target| {
                target.registry_backed
                    && target.prepended.is_empty()
                    && tcl_registry::command_descriptor_labels_match(&target.command, command)
                    && target.implementation_generation == 0
            })
        });
        required
            && self.provider_lookups_are_live(loader)
            && loader.optional_command_surface.iter().all(|command| {
                #[cfg(any(test, debug_assertions))]
                if provider_timing.is_some() {
                    original_command_table::ProviderValidationTiming::optional_row();
                }
                let slot = tcl_registry::rooted_command_descriptor_label(command);
                let keys = self.source_keys(command, &root);
                keys.len() == 1
                    && self.bindings.get(&keys[0]).is_some_and(|alternatives| {
                        alternatives.iter().all(|binding| match binding {
                            MayBinding::Missing => true,
                            MayBinding::Target(target) => {
                                target.registry_backed
                                    && target.terminal
                                    && target.prepended.is_empty()
                                    && target.command == slot
                                    && target.implementation_generation == 0
                            }
                            MayBinding::Imported(_) | MayBinding::Unknown => false,
                        })
                    })
            })
    }

    pub(super) fn record_provider_state_writes(
        &mut self,
        facts: &tcl_registry::InvocationFacts,
        invocation: tcl_registry::InvocationWords<'_>,
        registry: &CommandRegistry,
        operands: &crate::variable_bindings::OriginalVariableInvocation,
    ) {
        let mut places =
            crate::variable_bindings::source_variable_write_places_with_original_operands(
                facts,
                invocation.arguments(),
                &self.source_variables,
                registry,
                operands,
            );
        let mut observer_places = Vec::new();
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            if let StateTransition::Trace(tcl_registry::TraceTransition::Add {
                target: tcl_registry::TraceTarget::Variable(subject),
                ..
            }) = &fact.transition
            {
                observer_places.push(
                    subject
                        .argument_index()
                        .map_or_else(crate::place::unknown_top, |index| {
                            operands.trace_subject_access(index, &self.source_variables, registry)
                        }),
                );
            }
        }
        places.extend(observer_places.iter().cloned());
        self.record_provider_write_places(&places, &observer_places, registry);
    }

    pub(super) fn record_provider_write_places(
        &mut self,
        places: &[crate::place::Place],
        observer_places: &[crate::place::Place],
        registry: &CommandRegistry,
    ) {
        if places.is_empty() {
            return;
        }
        let protocol = self
            .baseline
            .dialect
            .and_then(|dialect| dialect.package_protocol)
            .unwrap_or(tcl_dialect::PackageProtocol::Tcl);
        let dependencies = tcl_registry::model::binding::package_resolver_dependencies(protocol).iter().filter_map(|dependency| {
            let name = match dependency {
                tcl_registry::model::binding::PackageResolverDependency::GlobalVariable(name) => (*name).to_owned(),
                tcl_registry::model::binding::PackageResolverDependency::GlobalArrayElement { array, element } => format!("{array}({element})"),
                tcl_registry::model::binding::PackageResolverDependency::WorkingDirectory
                | tcl_registry::model::binding::PackageResolverDependency::Filesystem => return None,
            };
            Some(crate::var_resolve::resolve_place(&name, &self.source_variables, false, registry))
        }).collect::<Vec<_>>();
        if places.iter().any(|place| {
            dependencies
                .iter()
                .any(|dependency| crate::place::overlap(place, dependency))
        }) {
            Arc::make_mut(&mut self.revoked_loaders)
                .extend(self.baseline.trusted_loaders.keys().cloned());
        }
        if !self.loader_handler_unknown
            && tcl_registry::model::binding::autoload_index_dependencies()
                .iter()
                .filter_map(|dependency| match dependency {
                    tcl_registry::model::binding::PackageResolverDependency::GlobalVariable(
                        name,
                    ) => Some(crate::var_resolve::resolve_place(
                        name,
                        &self.source_variables,
                        false,
                        registry,
                    )),
                    _ => None,
                })
                .any(|dependency| {
                    places
                        .iter()
                        .any(|place| crate::place::overlap(place, &dependency))
                })
        {
            self.loader_handler_unknown = true;
        }
        for loader in self.baseline.trusted_loaders.values() {
            if loader.state_dependency_namespaces.is_empty() {
                continue;
            }
            let modelled = loader
                .modelled_state_variables
                .iter()
                .map(|name| {
                    crate::var_resolve::resolve_place(name, &self.source_variables, false, registry)
                })
                .collect::<Vec<_>>();
            let affected = places.iter().any(|place| {
                if place.dynamic
                    || place.kind == crate::place::PlaceKind::UpvarAlias
                    || (place.kind == crate::place::PlaceKind::Unknown
                        && place.ns == crate::place::LOCAL_NS)
                {
                    return true;
                }
                let actual = tcl_syntax::naming::key_segments(&place.ns);
                if !loader.state_dependency_namespaces.iter().any(|namespace| {
                    let dependency = tcl_syntax::naming::key_segments(namespace);
                    actual.starts_with(&dependency)
                        || (place.kind == crate::place::PlaceKind::Unknown
                            && dependency.starts_with(&actual))
                }) {
                    return false;
                }
                observer_places.contains(place)
                    || !modelled.iter().any(|allowed| {
                        allowed.kind == place.kind
                            && allowed.ns == place.ns
                            && allowed.name == place.name
                            && allowed.index == place.index
                    })
            });
            if affected {
                Arc::make_mut(&mut self.tainted_provider_state).insert(loader.package.clone());
            }
        }
    }

    fn provider_lookups_are_live(&self, loader: &TrustedPackageLoader) -> bool {
        self.provider_lookup_dependencies_are_live(
            loader
                .lookup_dependencies
                .iter()
                .chain(&loader.installed_lookup_dependencies),
        )
    }

    pub(super) fn provider_lookup_dependencies_are_live<'a>(
        &self,
        mut dependencies: impl Iterator<Item = &'a TrustedCommandLookup>,
    ) -> bool {
        dependencies.all(|dependency| {
            #[cfg(any(test, debug_assertions))]
            original_command_table::ProviderValidationTiming::dependency_row();
            let Some(namespace_key) = self.namespace_for_rooted_operand(&dependency.namespace)
            else {
                return false;
            };
            let proof = source_binding(self, &dependency.head, &namespace_key);
            if proof.proved_target().is_some_and(|target| {
                target.registry_backed
                    && target.prepended.is_empty()
                    && tcl_registry::command_descriptor_labels_match(
                        &target.command,
                        &dependency.registry_identity,
                    )
                    && target.implementation_generation == 0
            }) {
                return true;
            }
            !dependency.required
                && self.definitely_absent(&dependency.head, &namespace_key)
                && self
                    .source_keys(&dependency.head, &namespace_key)
                    .iter()
                    .all(|slot| {
                        !self.bindings.contains_key(slot)
                            && !slot.authored_spelling().is_some_and(|slot| {
                                self.baseline.semantics.binding_names().contains(slot)
                            })
                    })
        })
    }

    /// The implementation that alone provides `package`, when it is present.
    pub(super) fn provided_implementation(&self, package: &str) -> Option<&str> {
        if self.opaque_domain {
            return None;
        }
        let [provision] = self.packages.get(package)?.iter().collect::<Vec<_>>()[..] else {
            return None;
        };
        provision
            .present
            .then_some(provision.implementation.as_deref())
            .flatten()
    }

    pub(super) fn loaded_provider(&self, package: &str) -> Option<&TrustedPackageLoader> {
        if self.opaque_domain {
            return None;
        }
        let provisions = self.packages.get(package)?;
        if provisions.len() != 1 {
            return None;
        }
        let provision = provisions.first()?;
        let loader = self.baseline.trusted_loaders.get(package)?;
        if loader.required_core_family.is_some_and(|family| {
            self.baseline
                .dialect
                .and_then(tcl_registry::InvocationDialect::family)
                != Some(family)
        }) {
            return None;
        }
        (provision.present
            && provision.implementation.as_deref() == Some(loader.implementation_id.as_str()))
        .then_some(loader)
    }
}

pub(super) fn interpreter_provided_packages(
    release: tcl_dialect::TclVersion,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> BTreeMap<String, BTreeSet<PackageProvision>> {
    release
        .core_provided_packages()
        .iter()
        .filter_map(|package| {
            let key = tcl_registry::native_package::NativePackageNameKey::from_native_units(
                package.name.as_bytes(),
                policy,
            );
            let key = String::from_utf8(key.bytes().to_vec()).ok()?;
            Some((
                key,
                BTreeSet::from([PackageProvision {
                    present: true,
                    version: Some(package.version.to_owned()),
                    implementation: None,
                }]),
            ))
        })
        .collect()
}

/// Authored source requiring a package the registry declares trusts a loader
/// of exactly that surface. Runtime and inherited entries never assert one.
fn declared_surface_loader(
    state: &ModuleCommandBindings,
    package: &str,
) -> Option<TrustedPackageLoader> {
    let baseline = &state.baseline;
    let family = baseline
        .dialect
        .and_then(tcl_registry::InvocationDialect::family)
        .filter(|family| *family == tcl_dialect::model::Family::Tcl)?;
    if baseline.native_entry.is_some()
        || baseline.unknown_entry
        || baseline.source_point_entry.is_some()
        || baseline.invocation_realm != tcl_dialect::model::InvocationRealm::RuleLoader
    {
        return None;
    }
    let registry = baseline.registry_owner.as_ref()?.registry();
    let profile = registry.profile()?;
    if profile.is_irules() || profile.package_protocol() != Some(tcl_dialect::PackageProtocol::Tcl)
    {
        return None;
    }
    let policy = state.original_command_world.selected_policy()?;
    let mut loader = crate::lowering::declared_surface_package_loader(registry, package, policy)?;
    loader.required_core_family = Some(family);
    Some(loader)
}

/// The trusted loader a `package require` of `package` selects: a supplied
/// one, else one the registry declares.
pub(super) fn trusted_package_loader(
    state: &ModuleCommandBindings,
    package: &str,
) -> Option<TrustedPackageLoader> {
    state
        .baseline
        .trusted_loaders
        .get(package)
        .cloned()
        .or_else(|| declared_surface_loader(state, package))
}

pub(super) fn apply_package_transition(
    state: &mut ModuleCommandBindings,
    transition: &tcl_registry::model::binding::PackageTransition,
) {
    use tcl_registry::model::binding::PackageTransition;
    match transition {
        PackageTransition::Provide { package, version } => {
            apply_package_provision(state, package, version.as_ref());
        }
        PackageTransition::Require {
            package,
            requirements,
            exact,
        } => {
            let Some(package) = package.literal() else {
                state.mark_opaque_binding_mutation();
                return;
            };
            if state
                .packages
                .get(package)
                .is_some_and(|provisions| provisions.iter().all(|provision| provision.present))
            {
                return;
            }
            if state
                .packages
                .get(package)
                .is_some_and(|provisions| provisions.iter().any(|provision| provision.present))
            {
                // A previously provided version and a loadable version are
                // distinct paths. Do not replace both with loader provenance.
                state.mark_opaque_binding_mutation();
                return;
            }
            let loader = trusted_package_loader(state, package);
            let selected = loader
                .filter(|loader| trusted_loader_is_selected(state, loader, requirements, *exact));
            let Some(loader) = selected else {
                state.mark_opaque_binding_mutation();
                return;
            };
            install_trusted_package_provider(state, package, loader);
        }
        PackageTransition::Ifneeded {
            package,
            script_provided,
            ..
        } if *script_provided => {
            if let Some(package) = package.literal() {
                Arc::make_mut(&mut state.revoked_loaders).insert(package.to_owned());
            } else {
                state.loader_handler_unknown = true;
            }
        }
        PackageTransition::Forget { packages } => {
            for package in packages {
                if let Some(package) = package.literal() {
                    Arc::make_mut(&mut state.packages).remove(package);
                    Arc::make_mut(&mut state.revoked_loaders).insert(package.to_owned());
                } else {
                    state.loader_handler_unknown = true;
                }
            }
        }
        PackageTransition::UnknownHandler { handler: Some(_) }
        | PackageTransition::Prefer { mode: Some(_) } => {
            state.loader_handler_unknown = true;
        }
        PackageTransition::SourceLoad { .. } => {
            state.mark_opaque_binding_mutation();
            state.loader_handler_unknown = true;
        }
        PackageTransition::IndexedAutoload { command } => {
            if !autoload_defines_nothing(state, command.literal()) {
                state.mark_opaque_binding_mutation();
                state.loader_handler_unknown = true;
            }
        }
        PackageTransition::DiscoveryDependencyChanged { .. } => {
            Arc::make_mut(&mut state.revoked_loaders)
                .extend(state.baseline.trusted_loaders.keys().cloned());
        }
        PackageTransition::Ifneeded { .. }
        | PackageTransition::UnknownHandler { handler: None }
        | PackageTransition::Prefer { mode: None } => {}
    }
}

/// Whether the stock `auto_load` of `command` can define nothing: the walk
/// starts from an authored entry, nothing it has seen may have changed an
/// autoload index input ([`tcl_registry::model::binding::autoload_index_dependencies`])
/// or the loader handlers, and no supplied source module is indexed under the
/// command's name. The analysis sees no other index, so the call is the
/// handler's "invalid command name" error and the command table stays known.
/// A name the registry catalogues (another dialect's, a vendor's or a
/// package's command) may be provided by the host the document runs in, so it
/// keeps the unbounded fallback.
pub(super) fn autoload_defines_nothing(
    state: &ModuleCommandBindings,
    command: Option<&str>,
) -> bool {
    if state.loader_handler_unknown
        || state.baseline.native_entry.is_some()
        || state.baseline.unknown_entry
        || state.baseline.host_extensible_command_table
        || command.is_some_and(|command| {
            state.baseline.registry_owner.as_ref().is_some_and(|owner| {
                let registry = owner.registry();
                registry.get(command).is_some() || registry.known_in_any_dialect(command)
            })
        })
    {
        return false;
    }
    let mut indexed = state
        .baseline
        .trusted_source_modules
        .iter()
        .flat_map(|loader| &loader.autoload);
    let Some(command) = command else {
        return indexed.next().is_none();
    };
    let rooted = tcl_registry::rooted_command_descriptor_label(command);
    let (_, tail) = tcl_syntax::naming::key_holder_and_tail(&rooted);
    !indexed.any(|name| tcl_syntax::naming::key_holder_and_tail(name).1 == tail)
}

fn apply_package_provision(
    state: &mut ModuleCommandBindings,
    package: &TransitionSubject,
    version: Option<&TransitionSubject>,
) {
    let Some(package) = package.literal() else {
        state.loader_handler_unknown = true;
        return;
    };
    if let Some(version) = version {
        let advertised = PackageProvision {
            present: true,
            version: version.literal().map(str::to_owned),
            implementation: None,
        };
        let entry = Arc::make_mut(&mut state.packages)
            .entry(package.to_owned())
            .or_insert_with(|| BTreeSet::from([PackageProvision::absent()]));
        *entry = entry
            .iter()
            .map(|provision| {
                if provision.present {
                    provision.clone()
                } else {
                    advertised.clone()
                }
            })
            .collect();
        Arc::make_mut(&mut state.revoked_loaders).insert(package.to_owned());
    }
}

pub(super) fn trusted_loader_is_selected(
    state: &ModuleCommandBindings,
    loader: &TrustedPackageLoader,
    requirements: &[TransitionSubject],
    exact: bool,
) -> bool {
    if loader.required_core_family.is_some_and(|family| {
        state
            .baseline
            .dialect
            .and_then(tcl_registry::InvocationDialect::family)
            != Some(family)
    }) {
        return false;
    }
    if state.loader_handler_unknown || state.revoked_loaders.contains(&loader.package) {
        return false;
    }
    if !trusted_package_transfer::preload_dependencies_hold(state, loader) {
        return false;
    }
    if requirements.is_empty() {
        return true;
    }
    if loader.accepts_any_requested_version {
        return requirements
            .iter()
            .all(|requirement| requirement.literal().is_some());
    }
    let (Some(version), Some(release)) = (loader.version.as_deref(), state.baseline.release) else {
        return false;
    };
    requirements
        .iter()
        .all(|requirement| requirement.literal().is_some())
        && requirements
            .iter()
            .filter_map(TransitionSubject::literal)
            .any(|requirement| {
                if exact {
                    tcl_dialect::compare_versions_for(version, requirement, release).is_eq()
                } else {
                    tcl_dialect::version_satisfies_for(version, requirement, release)
                }
            })
}

fn install_trusted_package_namespace_exports(
    state: &mut ModuleCommandBindings,
    loader: &TrustedPackageLoader,
) {
    for (namespace, exports) in &loader.namespace_exports {
        Arc::make_mut(&mut state.namespaces).insert(namespace.clone());
        if exports
            .iter()
            .any(|pattern| !pattern.is_ascii() || pattern.as_bytes().contains(&0))
        {
            Arc::make_mut(&mut state.unknown_export_namespaces).insert(namespace.clone());
        } else {
            Arc::make_mut(&mut state.namespace_exports).insert(
                namespace.clone(),
                BTreeSet::from([exports
                    .iter()
                    .map(|pattern| tcl_core_types::NameBytes::from(pattern.as_bytes()))
                    .collect()]),
            );
        }
    }
    for (namespace, optional) in &loader.optional_namespace_exports {
        if optional
            .iter()
            .any(|pattern| !pattern.is_ascii() || pattern.as_bytes().contains(&0))
        {
            Arc::make_mut(&mut state.unknown_export_namespaces).insert(namespace.clone());
        }
        let exports = Arc::make_mut(&mut state.namespace_exports)
            .entry(namespace.clone())
            .or_insert_with(|| BTreeSet::from([Vec::new()]));
        let with_optional = exports
            .iter()
            .map(|patterns| {
                let mut patterns = patterns.clone();
                patterns.extend(
                    optional
                        .iter()
                        .filter(|pattern| pattern.is_ascii() && !pattern.as_bytes().contains(&0))
                        .map(|pattern| tcl_core_types::NameBytes::from(pattern.as_bytes())),
                );
                patterns
            })
            .collect::<Vec<_>>();
        exports.extend(with_optional);
    }
}

fn install_trusted_package_provider(
    state: &mut ModuleCommandBindings,
    package: &str,
    loader: TrustedPackageLoader,
) {
    // Proof: naming.package.attested-source-loader-transfer
    // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
    let Some(publication_keys) = trusted_package_transfer::publication_keys(state, &loader) else {
        state.mark_opaque_binding_mutation();
        return;
    };
    for command in &loader.command_surface {
        let slot = tcl_registry::rooted_command_descriptor_label(command);
        let (namespace, _) = tcl_syntax::naming::key_holder_and_tail(&slot);
        Arc::make_mut(&mut state.namespaces).insert(namespace.to_owned());
        state.install(
            publication_keys[command].clone(),
            MayBinding::Target(ResolvedCommandTarget {
                command: slot.clone(),
                prepended: Vec::new(),
                registry_backed: true,
                kind: BindingKind::Builtin,
                implementation_generation: 0,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: Some(CommandToken {
                    runtime: None,
                    origin: slot,
                    declaration: 0,
                    allocation: None,
                }),
                implementation_allocation: None,
            }),
        );
    }
    for command in &loader.optional_command_surface {
        let slot = tcl_registry::rooted_command_descriptor_label(command);
        Arc::make_mut(&mut state.namespaces).insert(command_holder(&slot));
        let original = MayBinding::Target(ResolvedCommandTarget {
            command: slot.clone(),
            prepended: Vec::new(),
            registry_backed: true,
            kind: BindingKind::Builtin,
            implementation_generation: 0,
            terminal: true,
            target_lookup: tcl_registry::AliasTargetLookup::Global,
            token: Some(CommandToken {
                runtime: None,
                origin: slot.clone(),
                declaration: 0,
                allocation: None,
            }),
            implementation_allocation: None,
        });
        state.replace(
            publication_keys[command].clone(),
            BTreeSet::from([original, MayBinding::Missing]),
        );
    }
    install_trusted_package_namespace_exports(state, &loader);
    Arc::make_mut(&mut state.packages).insert(
        package.to_owned(),
        BTreeSet::from([PackageProvision {
            present: true,
            version: loader.version,
            implementation: Some(loader.implementation_id),
        }]),
    );
}
