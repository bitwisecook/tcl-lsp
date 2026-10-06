// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Itcl 4.3.2 (9098b1d7) recipe measured on C Tcl 8.6.18, 9.0.4 and 9.1.0.
//! Only inspected factory compiler hooks are stamped; private dependencies
//! remain exact installed lookup prerequisites, not execution permissions.

use crate::command_binding::{TrustedCommandLookup, TrustedPackageLoader};
use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;

const COMMANDS: &[&str] = &[
    "::itcl::Root",
    "::itcl::addcomponent",
    "::itcl::adddelegatedmethod",
    "::itcl::adddelegatedoption",
    "::itcl::addobjectoption",
    "::itcl::addoption",
    "::itcl::body",
    "::itcl::builtin::Info",
    "::itcl::builtin::Info::args",
    "::itcl::builtin::Info::body",
    "::itcl::builtin::Info::class",
    "::itcl::builtin::Info::classoptions",
    "::itcl::builtin::Info::component",
    "::itcl::builtin::Info::components",
    "::itcl::builtin::Info::context",
    "::itcl::builtin::Info::default",
    "::itcl::builtin::Info::delegated",
    "::itcl::builtin::Info::delegated::method",
    "::itcl::builtin::Info::delegated::methods",
    "::itcl::builtin::Info::delegated::option",
    "::itcl::builtin::Info::delegated::options",
    "::itcl::builtin::Info::delegated::typemethod",
    "::itcl::builtin::Info::delegated::typemethods",
    "::itcl::builtin::Info::delegated::unknown",
    "::itcl::builtin::Info::extendedclass",
    "::itcl::builtin::Info::function",
    "::itcl::builtin::Info::heritage",
    "::itcl::builtin::Info::hulltype",
    "::itcl::builtin::Info::hulltypes",
    "::itcl::builtin::Info::inherit",
    "::itcl::builtin::Info::instances",
    "::itcl::builtin::Info::method",
    "::itcl::builtin::Info::methods",
    "::itcl::builtin::Info::option",
    "::itcl::builtin::Info::options",
    "::itcl::builtin::Info::type",
    "::itcl::builtin::Info::typemethod",
    "::itcl::builtin::Info::typemethods",
    "::itcl::builtin::Info::types",
    "::itcl::builtin::Info::typevariable",
    "::itcl::builtin::Info::typevars",
    "::itcl::builtin::Info::unknown",
    "::itcl::builtin::Info::variable",
    "::itcl::builtin::Info::variables",
    "::itcl::builtin::Info::vars",
    "::itcl::builtin::Info::widget",
    "::itcl::builtin::Info::widgetadaptor",
    "::itcl::builtin::Info::widgetadaptors",
    "::itcl::builtin::Info::widgetclasses",
    "::itcl::builtin::Info::widgets",
    "::itcl::builtin::addoptioncomponent",
    "::itcl::builtin::callinstance",
    "::itcl::builtin::cget",
    "::itcl::builtin::chain",
    "::itcl::builtin::classunknown",
    "::itcl::builtin::configure",
    "::itcl::builtin::createhull",
    "::itcl::builtin::destroy",
    "::itcl::builtin::getinstancevar",
    "::itcl::builtin::ignorecomponentoption",
    "::itcl::builtin::ignoreoptioncomponent",
    "::itcl::builtin::info",
    "::itcl::builtin::installcomponent",
    "::itcl::builtin::isa",
    "::itcl::builtin::itcl_hull",
    "::itcl::builtin::itcl_initoptions",
    "::itcl::builtin::keepcomponentoption",
    "::itcl::builtin::mymethod",
    "::itcl::builtin::myproc",
    "::itcl::builtin::mytypemethod",
    "::itcl::builtin::mytypevar",
    "::itcl::builtin::myvar",
    "::itcl::builtin::renameoptioncomponent",
    "::itcl::builtin::setget",
    "::itcl::builtin::setupcomponent",
    "::itcl::builtin::unknown",
    "::itcl::class",
    "::itcl::clazz",
    "::itcl::code",
    "::itcl::configbody",
    "::itcl::delete",
    "::itcl::delete_helper",
    "::itcl::ensemble",
    "::itcl::extendedclass",
    "::itcl::filter",
    "::itcl::find",
    "::itcl::forward",
    "::itcl::import::stub",
    "::itcl::internal::commands::checksetitclhull",
    "::itcl::internal::commands::ensembles::1::classes",
    "::itcl::internal::commands::ensembles::1::objects",
    "::itcl::internal::commands::ensembles::2::class",
    "::itcl::internal::commands::ensembles::2::ensemble",
    "::itcl::internal::commands::ensembles::2::object",
    "::itcl::internal::commands::ensembles::3::class",
    "::itcl::internal::commands::ensembles::3::object",
    "::itcl::internal::commands::ensembles::4::add",
    "::itcl::internal::commands::ensembles::4::delete",
    "::itcl::internal::commands::ensembles::5::add",
    "::itcl::internal::commands::ensembles::5::delete",
    "::itcl::internal::commands::ensembles::6::add",
    "::itcl::internal::commands::ensembles::6::delete",
    "::itcl::internal::commands::ensembles::7::create",
    "::itcl::internal::commands::ensembles::7::exists",
    "::itcl::internal::commands::ensembles::8::method",
    "::itcl::internal::commands::ensembles::8::option",
    "::itcl::internal::commands::ensembles::8::typemethod",
    "::itcl::internal::commands::ensembles::unknown",
    "::itcl::internal::commands::genericclass",
    "::itcl::internal::commands::sethullwindowname",
    "::itcl::is",
    "::itcl::local",
    "::itcl::mixin",
    "::itcl::nwidget",
    "::itcl::parser::common",
    "::itcl::parser::component",
    "::itcl::parser::constructor",
    "::itcl::parser::delegate",
    "::itcl::parser::destructor",
    "::itcl::parser::filter",
    "::itcl::parser::forward",
    "::itcl::parser::handleClass",
    "::itcl::parser::hulltype",
    "::itcl::parser::inherit",
    "::itcl::parser::method",
    "::itcl::parser::methodvariable",
    "::itcl::parser::mixin",
    "::itcl::parser::option",
    "::itcl::parser::private",
    "::itcl::parser::proc",
    "::itcl::parser::protected",
    "::itcl::parser::public",
    "::itcl::parser::typecomponent",
    "::itcl::parser::typeconstructor",
    "::itcl::parser::typemethod",
    "::itcl::parser::typevariable",
    "::itcl::parser::variable",
    "::itcl::parser::widgetclass",
    "::itcl::scope",
    "::itcl::setcomponent",
    "::itcl::type",
    "::itcl::widget",
    "::itcl::widgetadaptor",
];

pub(super) fn attest(loader: &mut TrustedPackageLoader, version: tcl_dialect::TclVersion) {
    loader.command_surface = COMMANDS.iter().map(|name| (*name).to_owned()).collect();
    if version >= tcl_dialect::TclVersion::V9_0 {
        loader.command_surface.push("::itcl::build-info".to_owned());
    }
    loader.compiler_hooks.insert(
        "::itcl::class".to_owned(),
        NativeCompilerHookPresence::Absent,
    );
    loader.definition_dispatchers.insert((
        "::itcl::class".to_owned(),
        tcl_registry::definer::DefinitionDispatcher::ItclClass,
    ));
    loader.installed_lookup_dependencies = loader
        .command_surface
        .iter()
        .map(|name| lookup(name))
        .collect();
    loader.lookup_dependencies = [
        "::namespace",
        "::info",
        "::set",
        "::list",
        "::proc",
        "::rename",
        "::return",
        "::error",
        "::catch",
        "::trace",
        "::oo::class",
        "::oo::object",
        "::oo::define",
        "::oo::objdefine",
    ]
    .into_iter()
    .map(lookup)
    .collect();
    // The provider's own objects, parser state and mutable metaclass all live
    // here. Opaque object mutation also withdraws the installation recipe.
    loader.state_dependency_namespaces = vec!["::itcl".to_owned()];
}

fn lookup(name: &str) -> TrustedCommandLookup {
    TrustedCommandLookup {
        head: name.to_owned(),
        namespace: "::itcl".to_owned(),
        registry_identity: name.to_owned(),
        required: true,
    }
}

#[cfg(test)]
mod tests {
    use crate::command_binding::{BindingKind, SourceCommandBindings};
    use tcl_registry::definer::DefinitionDispatcher;

    fn bindings(source: &str, selected: bool) -> SourceCommandBindings {
        bindings_for(source, selected, "tcl8.6")
    }

    fn bindings_for(source: &str, selected: bool, environment: &str) -> SourceCommandBindings {
        let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
        let providers = if selected {
            &[super::super::Provider::Itcl][..]
        } else {
            &[][..]
        };
        let entry = super::super::entry(registry, providers);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            entry.options(),
        )
    }

    #[test]
    fn itcl_recipe_retains_each_audited_native_engine_and_installed_roster() {
        for environment in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
            let source = "package require Itcl; itcl::class C {}; C named";
            let offset = u32::try_from(source.rfind("C named").unwrap()).unwrap();
            let analysis = bindings_for(source, true, environment);
            let call = analysis.invocation_at_source("C", offset);
            assert_eq!(
                call.nominal_definition_name_result(registry),
                Some(tcl_registry::definer::DefinitionDispatcher::ItclClass),
                "{environment}"
            );
            assert!(call.proved_class_definition_factory().is_none());
            let entry = super::super::entry(registry, &[super::super::Provider::Itcl]);
            let roster = &entry.trusted_package_loaders[0].command_surface;
            assert_eq!(
                roster.iter().any(|name| name == "::itcl::build-info"),
                environment != "tcl8.6"
            );
            assert!(
                bindings_for(source, false, environment)
                    .invocation_at_source("C", offset)
                    .nominal_definition_name_result(registry)
                    .is_none()
            );
        }
        for environment in ["tcl8.4", "tcl8.5", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
            assert!(
                !tcl_registry::definer::DefinitionDispatcher::ItclClass
                    .native_installation_is_audited(tcl_registry::InvocationDialect::of_profile(
                        registry.profile().unwrap()
                    ))
            );
        }
    }

    #[test]
    fn native_itcl_installs_its_own_dispatcher_without_object_class_proofs() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for source in [
            "package require Itcl; itcl::class C {}; C named",
            "package require Itcl; itcl::class C {method status {} {return ok}}; C #auto",
            "package require Itcl; itcl::class C {proc ping {} {return static}}; C ping",
            "package require Itcl; itcl::class C {constructor {} {error stopped}}; C named",
            "package require Itcl; itcl::class C {constructor {} {rename $this {}; return}}; C named",
        ] {
            let offset = u32::try_from(source.rfind("C ").unwrap()).unwrap();
            let bindings = bindings(source, true);
            let call = bindings.invocation_at_source("C", offset);
            assert_eq!(
                call.proved_execution_target().unwrap().kind,
                BindingKind::Command,
                "{source}"
            );
            assert_eq!(
                call.nominal_definition_name_result(registry),
                Some(DefinitionDispatcher::ItclClass),
                "{source}"
            );
            assert!(call.proved_class_definition_factory().is_none(), "{source}");
        }
    }

    #[test]
    fn itcl_dispatcher_requires_loaded_attested_provider_and_current_dependencies() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, selected) in [
            ("itcl::class C {}; C named", true),
            ("package require Itcl; itcl::class C {}; C named", false),
            (
                "package require Itcl; rename ::itcl::parser::method ::itcl::parser::saved; proc ::itcl::parser::method args {}; itcl::class C {method status {} {return ok}}; C named",
                true,
            ),
            (
                "rename info saved_info; package require Itcl; itcl::class C {}; C named",
                true,
            ),
            (
                "package require Itcl; oo::define ::itcl::clazz method unknown args {return hijacked}; itcl::class C {}; C named",
                true,
            ),
        ] {
            let offset = u32::try_from(source.rfind("C ").unwrap()).unwrap();
            let bindings = bindings(source, selected);
            assert!(
                bindings
                    .invocation_at_source("C", offset)
                    .nominal_definition_name_result(registry)
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn itcl_private_execution_observers_and_unbounded_definitions_decline_installation() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for definition in [
            "proc observe args {}; trace add execution ::itcl::parser::method enter observe; itcl::class C {method status {} {return ok}}",
            "itcl::class C {method status {} {}; method status {} {}}",
            "itcl::class C {constructor {a(k)} {}}",
            "itcl::class C {variable state [error stopped]}",
            "itcl::class C.Name {}",
        ] {
            let source = format!("package require Itcl; {definition}; C named");
            let offset = u32::try_from(source.rfind("C named").unwrap()).unwrap();
            let bindings = bindings(&source, true);
            assert!(
                bindings
                    .invocation_at_source("C", offset)
                    .nominal_definition_name_result(registry)
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn itcl_qualified_class_proc_slot_is_separate_from_instance_name_dispatch() {
        let source = "package require Itcl; namespace eval N {}; itcl::class N::C {proc ping {} {return static}}; N::C::ping";
        let offset = u32::try_from(source.rfind("N::C::ping").unwrap()).unwrap();
        let bindings = bindings(source, true);
        let call = bindings.invocation_at_source("N::C::ping", offset);
        assert!(call.proved_target().is_some(), "{call:?}");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        assert!(call.nominal_definition_name_result(registry).is_none());
    }
}
