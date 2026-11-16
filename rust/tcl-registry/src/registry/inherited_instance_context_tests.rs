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

use super::*;
use crate::spec::ObjectClassSpec;

fn context(
    factory_package: &'static str,
    provider_package: &'static str,
) -> crate::model::ContextRegistry {
    static OPTIONS: [crate::hover::OptionSpec; 1] = [crate::hover::OptionSpec {
        name: "-later",
        lifecycle: Lifecycle::introduced_in("5.0"),
        ..crate::hover::OptionSpec::DEFAULT
    }];
    static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
        name: "option-form",
        arity: Arity::exact(0).with_positionals(),
        ..crate::forms::CommandForm::DEFAULT
    }];
    static METHODS: [SubCommand; 3] = [
        SubCommand {
            name: "late",
            lifecycle: Lifecycle::introduced_in("5.0"),
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "last",
            lifecycle: Lifecycle::introduced_in("5.0"),
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "option",
            options: &OPTIONS,
            subcommand_forms: &FORMS,
            ..SubCommand::DEFAULT
        },
    ];
    static CHILD: ObjectClassSpec = ObjectClassSpec {
        class_name: "ContextChild",
        instance_methods: &[],
        superclasses: &["ContextParent"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Enabled,
    };
    static PARENT: ObjectClassSpec = ObjectClassSpec {
        class_name: "ContextParent",
        instance_methods: &METHODS,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    let mut registry = CommandRegistry::build_default();
    for (class, package) in [(&CHILD, factory_package), (&PARENT, provider_package)] {
        registry.insert(CommandSpec {
            name: class.class_name,
            required_package: Some(package),
            object_class: Some(class),
            ..CommandSpec::DEFAULT
        });
    }
    crate::model::ingress::static_context_for("tcl8.6").with_command_store(Arc::new(registry))
}

#[test]
fn inherited_context_method_prefix_form_and_options_keep_provider_axis() {
    for (factory_package, provider_package, later_available) in
        [("Itcl", "Tk", true), ("Tk", "Itcl", false)]
    {
        let context = context(factory_package, provider_package);
        let registry = context.commands();
        let factory = context
            .context()
            .resolve_spec(registry, "ContextChild")
            .unwrap();
        let provider = context
            .context()
            .resolve_spec(registry, "ContextParent")
            .unwrap();
        let floor = context
            .context()
            .placement_floor(provider_package)
            .unwrap()
            .as_str();
        assert_eq!(
            floor,
            if provider_package == "Tk" {
                "8.6"
            } else {
                "4.2"
            }
        );
        let keyword = registry
            .instance_method_selection_for_descriptor_in_context(factory, "la", &context)
            .unwrap();
        assert_eq!(
            matches!(keyword, crate::abbrev::KeywordMatch::Ambiguous(_)),
            later_available
        );
        assert_eq!(
            registry
                .instance_method_selection_for_descriptor_in_context(factory, "late", &context)
                .unwrap()
                .unique()
                .is_some(),
            later_available
        );
        let args = ["option", "-later"];
        let schema = registry
            .resolve_structured_instance_invocation_for_descriptor_in_context(
                factory,
                InvocationWords::literals(".w", &args),
                &context,
            )
            .unwrap()
            .unwrap();
        let (actual_provider, availability) =
            schema.authored_source_instance_method_provider().unwrap();
        assert!(std::ptr::eq(actual_provider, provider));
        assert!(std::ptr::eq(
            schema.authored_source_descriptors().command,
            factory
        ));
        assert_eq!(availability.package_version, Some(floor));
        assert_eq!(
            schema
                .semantics
                .options
                .available()
                .any(|option| option.name == "-later"),
            later_available
        );
        assert_eq!(
            schema.form.is_some(),
            later_available,
            "form arity parses options at the same provider floor"
        );
        let foreign = factory.clone();
        assert!(matches!(
            registry.instance_method_providers_for_descriptor_in_context(&foreign, &context),
            Err(InstanceMethodContextUnavailable::FactorySelection)
        ));
        assert!(matches!(
            CommandRegistry::build_default()
                .instance_method_providers_for_descriptor_in_context(factory, &context),
            Err(InstanceMethodContextUnavailable::RegistryMismatch)
        ));
    }
}

#[test]
fn inherited_context_unavailable_table_is_not_unknown_and_mixed_option_axes_refuse() {
    static MISSING: ObjectClassSpec = ObjectClassSpec {
        class_name: "IncompleteChild",
        instance_methods: &[],
        superclasses: &["UnownedSuperclass"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static RECEIVER_OPTIONS: [crate::hover::OptionSpec; 1] = [crate::hover::OptionSpec {
        name: "-receiver",
        ..crate::hover::OptionSpec::DEFAULT
    }];
    static FORM_OPTIONS: [crate::hover::OptionSpec; 1] = [crate::hover::OptionSpec {
        name: "-form",
        ..crate::hover::OptionSpec::DEFAULT
    }];
    static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
        name: "mixed",
        options: &FORM_OPTIONS,
        ..crate::forms::CommandForm::DEFAULT
    }];
    static METHODS: [SubCommand; 1] = [SubCommand {
        name: "configure",
        traits: Traits::CONFIGURES_INSTANCE_OPTIONS,
        subcommand_forms: &FORMS,
        ..SubCommand::DEFAULT
    }];
    static CHILD: ObjectClassSpec = ObjectClassSpec {
        class_name: "MixedChild",
        instance_methods: &[],
        superclasses: &["MixedParent"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static PARENT: ObjectClassSpec = ObjectClassSpec {
        class_name: "MixedParent",
        instance_methods: &METHODS,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "IncompleteChild",
        object_class: Some(&MISSING),
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "MixedChild",
        object_class: Some(&CHILD),
        required_package: Some("Tk"),
        options: &RECEIVER_OPTIONS,
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "MixedParent",
        object_class: Some(&PARENT),
        required_package: Some("Itcl"),
        ..CommandSpec::DEFAULT
    });
    let context =
        crate::model::ingress::static_context_for("tcl8.6").with_command_store(Arc::new(registry));
    let registry = context.commands();
    let factory = registry.get("IncompleteChild").unwrap();
    assert!(matches!(
        registry.instance_method_selection_for_descriptor_in_context(factory, "missing", &context),
        Err(InstanceMethodContextUnavailable::SuperclassSelection(
            "UnownedSuperclass"
        ))
    ));
    let factory = registry.get("MixedChild").unwrap();
    let args = ["configure"];
    assert!(matches!(
        registry.resolve_structured_instance_invocation_for_descriptor_in_context(
            factory,
            InvocationWords::literals(".w", &args),
            &context
        ),
        Err(InstanceMethodContextUnavailable::ReceiverOptionAxes)
    ));
}
