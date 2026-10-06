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

//! Explicit package-entry fixtures, independent of catalogue availability.

use crate::command_binding::{SourceAnalysisEntry, TrustedPackageLoader};
use std::collections::BTreeMap;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

mod itcl;
mod snit;

/// Audited package implementations selected by the diagnostic fixture driver.
#[derive(Clone, Copy)]
pub(crate) enum Provider {
    /// Tcllib 2.0's pure Tcl tree implementation, package 2.1.3.
    Tree,
    /// Tcllib 2.0's graph Tcl implementation, package 2.4.4.
    Graph,
    /// Tcllib 2.0's matrix procedure implementation, package 2.2.
    Matrix,
    /// Tcllib 2.0's Snit 2.3.4 implementation.
    Snit,
    /// Itcl 4.3.2's actual class dispatcher on C Tcl 8.6.18.
    Itcl,
    /// C Tcl 8.6.18's tcltest 2.5.11 implementation.
    Tcltest,
    /// The Tk 8.6 widget implementation supplied by the fixture environment.
    Tk,
}

/// Build the same explicit entry for source lookup, lowering and diagnostics.
pub(crate) fn entry(
    registry: &CommandRegistry,
    providers: &[Provider],
) -> Arc<SourceAnalysisEntry> {
    let profile = registry.profile().expect("selected fixture dialect");
    let loaders = providers
        .iter()
        .map(|provider| {
            if matches!(provider, Provider::Tcltest) {
                return crate::lowering::stock_body_provider_loader(
                    tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
                    Some("2.5.11"),
                )
                .expect("audited C Tcl tcltest provider");
            }
            let (package, version, implementation, procedure_hooks) = match provider {
                Provider::Tree => ("struct::tree", "2.1.3", "tcllib-2.0:tree:pure-tcl", true),
                Provider::Graph => ("struct::graph", "2.4.4", "tcllib-2.0:graph:pure-tcl", true),
                Provider::Matrix => ("struct::matrix", "2.2", "tcllib-2.0:matrix:procedure", true),
                Provider::Snit => ("snit", "2.3.4", "tcllib-2.0:snit:procedure", true),
                Provider::Itcl => ("Itcl", "4.3.2", "itcl-4.3.2:9098b1d7:C8.6.18", false),
                Provider::Tcltest => unreachable!("stock body provider selected above"),
                Provider::Tk => ("Tk", "8.6.13", "tk-8.6.13:widget-provider", false),
            };
            // This enumeration is a surface declaration. The explicit implementation
            // selection above is the driver attestation that makes it an entry fact.
            let commands = registry
                .command_names()
                .filter_map(|name| {
                    let spec = registry.get(name)?;
                    (spec.owning_package() == Some(package)).then(|| name.to_owned())
                })
                .collect::<Vec<_>>();
            let compiler_hooks = if procedure_hooks {
                commands
                    .iter()
                    .map(|command| {
                        (
                            command.clone(),
                            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                        )
                    })
                    .collect()
            } else {
                // These exact constructors/registration commands are installed
                // through Tcl_CreateObjCommand in Tk/ttk 8.6.13. Other widget
                // return contracts do not imply a compiler registration.
                ["button", "entry", "bind", "listbox", "ttk::treeview"]
                    .into_iter()
                    .map(|command| {
                        (
                            command.to_owned(),
                            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
            };
            let mut loader = TrustedPackageLoader {
                required_core_family: Some(tcl_dialect::model::Family::Tcl),
                package: package.to_owned(),
                version: Some(version.to_owned()),
                implementation_id: implementation.to_owned(),
                command_surface: commands,
                compiler_hooks,
                definition_dispatchers: std::collections::BTreeSet::new(),
                optional_command_surface: Vec::new(),
                namespace_exports: BTreeMap::new(),
                optional_namespace_exports: BTreeMap::new(),
                lookup_dependencies: Vec::new(),
                installed_lookup_dependencies: Vec::new(),
                state_dependency_namespaces: Vec::new(),
                modelled_state_variables: Vec::new(),
            };
            if matches!(provider, Provider::Snit) {
                snit::attest(&mut loader);
            }
            if matches!(provider, Provider::Itcl) {
                let native_dialect = tcl_registry::InvocationDialect::of_profile(profile);
                assert!(
                    tcl_registry::definer::DefinitionDispatcher::ItclClass
                        .native_installation_is_audited(native_dialect)
                );
                let native_version = profile.effective_tcl_version(None).unwrap();
                loader.implementation_id = format!("itcl-4.3.2:9098b1d7:{native_version:?}");
                itcl::attest(&mut loader, native_version);
            }
            loader
        })
        .collect();
    Arc::new(SourceAnalysisEntry {
        trusted_package_loaders: loaders,
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..SourceAnalysisEntry::default()
    })
}

/// Analyse under one explicit selected package environment.
pub(crate) fn analyse(
    source: &str,
    dialect: &str,
    providers: &[Provider],
) -> crate::analyser::types::AnalysisResult {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    crate::analyser::Analyser::new()
        .with_source_analysis_entry(entry(registry, providers))
        .analyse(source, dialect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_surface_requires_selection_loading_and_an_unchanged_handler() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        for (provider, package, command, call) in [
            (
                Provider::Tree,
                "struct::tree",
                "::struct::tree",
                "::struct::tree mytree",
            ),
            (
                Provider::Matrix,
                "struct::matrix",
                "::struct::matrix",
                "::struct::matrix mymatrix",
            ),
            (Provider::Tk, "Tk", "::listbox", "listbox .l"),
        ] {
            let selected = entry(registry, &[provider]);
            let source = format!("package require {package}\n{call}");
            let offset = u32::try_from(source.find(call).unwrap()).unwrap();
            let loaded = crate::command_binding::SourceCommandBindings::analyse_with_options(
                &source,
                config,
                registry,
                selected.options(),
            );
            assert!(
                loaded
                    .invocation_at_source(command, offset)
                    .proved_execution_target()
                    .is_some_and(|target| target.registry_backed),
                "{source}"
            );
            let unselected =
                crate::command_binding::SourceCommandBindings::analyse(&source, config, registry);
            assert!(
                unselected
                    .invocation_at_source(command, offset)
                    .proved_execution_target()
                    .is_none()
            );
            let unloaded = crate::command_binding::SourceCommandBindings::analyse_with_options(
                call,
                config,
                registry,
                selected.options(),
            );
            assert!(
                unloaded
                    .invocation_at_source(command, 0)
                    .proved_execution_target()
                    .is_none()
            );
            let replaced_source = format!(
                "package require {package}\nproc {command} args {{return ordinary}}\n{call}",
            );
            let replaced_offset = u32::try_from(replaced_source.rfind(call).unwrap()).unwrap();
            let replaced = crate::command_binding::SourceCommandBindings::analyse_with_options(
                &replaced_source,
                config,
                registry,
                selected.options(),
            );
            assert!(
                replaced
                    .invocation_at_source(command, replaced_offset)
                    .proved_handler_target()
                    .is_none_or(|target| !target.registry_backed)
            );
        }
    }

    #[test]
    fn compilation_unit_retains_the_explicit_provider_entry() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let selected = entry(registry, &[Provider::Tree]);
        let source = "package require struct::tree\n::struct::tree mytree\n";
        let cu = crate::compilation_unit::CompilationUnit::build_with_source_entry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            &selected,
        );
        assert_eq!(
            cu.ir_module.source_entry.trusted_package_loaders,
            selected.trusted_package_loaders
        );
        assert_eq!(
            cu.ir_module.source_entry.invocation_dialect,
            selected.invocation_dialect
        );
    }

    #[test]
    fn snit_installed_dependencies_are_checked_after_the_real_loader_replacement() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let selected = entry(registry, &[Provider::Snit]);
        // Tcllib 2.0's loader overwrites this private proc before its first use;
        // the same replacement after loading withdraws the installed recipe.
        let source = "namespace eval ::snit {}; proc ::snit::Comp.Compile args {error PRELOAD-OLD}; package require snit; snit::type ::Counter {method bump {} {return 1}}; Counter create mine";
        let offset = u32::try_from(source.rfind("Counter create").unwrap()).unwrap();
        let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            selected.options(),
        );
        assert!(
            bindings
                .invocation_at_source("::Counter", offset)
                .nominal_definition_name_result(registry)
                .is_some()
        );
    }

    #[test]
    fn snit_dispatcher_recipe_is_conditional_nominal_and_dependency_guarded() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let selected = entry(registry, &[Provider::Snit]);
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let declaration =
            "package require snit\nsnit::type ::Counter {method bump {} {return 1}}\n";
        for (prelude, expected) in [
            ("", true),
            ("proc ::snit::Comp.Compile args {return altered}\n", false),
            ("rename ::snit::Comp.Compile ::snit::oldCompile\n", false),
            ("rename ::set ::oldSet\n", false),
            ("set ::snit::typeTemplate altered\n", false),
            ("proc ::Counter args {return ordinary}\n", false),
            (
                "namespace ensemble configure ::Counter -unknown ::replacement\n",
                false,
            ),
        ] {
            let source = format!("{declaration}{prelude}Counter create mine");
            let offset = u32::try_from(source.rfind("Counter create").unwrap()).unwrap();
            let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
                &source,
                config,
                registry,
                selected.options(),
            );
            let binding = bindings.invocation_at_source("::Counter", offset);
            assert_eq!(
                binding.nominal_definition_name_result(registry).is_some(),
                expected,
                "{source}"
            );
            assert!(
                binding
                    .targets
                    .iter()
                    .all(|target| { target.kind != crate::command_binding::BindingKind::Class })
            );
        }
        let source = format!("{declaration}Counter create mine");
        let offset = u32::try_from(source.rfind("Counter create").unwrap()).unwrap();
        let mut unattested = (*selected).clone();
        unattested.trusted_package_loaders[0]
            .definition_dispatchers
            .clear();
        for supplied in [None, Some(&unattested)] {
            let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
                &source,
                config,
                registry,
                supplied.map_or_else(Default::default, SourceAnalysisEntry::options),
            );
            assert!(
                bindings
                    .invocation_at_source("::Counter", offset)
                    .nominal_definition_name_result(registry)
                    .is_none()
            );
        }
    }

    #[test]
    fn snit_immediate_definition_callbacks_do_not_receive_dispatcher_receipts() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let selected = entry(registry, &[Provider::Snit]);
        let source = "package require snit\nsnit::type ::Counter {typeconstructor {error BOOM}}\nCounter create mine";
        let offset = u32::try_from(source.rfind("Counter create").unwrap()).unwrap();
        let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            selected.options(),
        );
        assert!(
            bindings
                .invocation_at_source("::Counter", offset)
                .nominal_definition_name_result(registry)
                .is_none()
        );
    }

    #[test]
    fn snit_shorthand_excludes_actual_declared_and_builtin_type_methods() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let selected = entry(registry, &[Provider::Snit]);
        for (arguments, expected) in [
            ("create mine", true),
            ("%AUTO%", true),
            ("mine", true),
            ("info", false),
            ("destroy", false),
            ("custom", false),
        ] {
            let source = format!(
                "package require snit\nsnit::type ::Counter {{typemethod custom {{}} {{return 3}}}}\nCounter {arguments}"
            );
            let offset = u32::try_from(source.rfind("Counter ").unwrap()).unwrap();
            let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
                registry,
                selected.options(),
            );
            assert_eq!(
                bindings
                    .invocation_at_source("::Counter", offset)
                    .nominal_definition_name_result(registry)
                    .is_some(),
                expected,
                "{source}"
            );
        }
    }
}
