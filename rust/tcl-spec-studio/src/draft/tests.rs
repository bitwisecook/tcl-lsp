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
use crate::schema;

/// `regsub -command` is Tcl 9.0 and later, and Jim from 0.84: the draft
/// names the open-ended Jim row `jim0.84+`, a bare Jim row `jim`.
#[test]
fn a_jim_row_open_ended_at_a_release_is_spelt_with_that_release() {
    use tcl_dialect::model::Family;
    const REGSUB_COMMAND: &[SpecSurface] = &[
        SpecSurface::core_in(Family::Tcl, &[("9.0", None)]),
        SpecSurface::core_in(Family::Jim, &[("0.84", None)]),
    ];
    assert_eq!(
        dialects(Some(REGSUB_COMMAND)),
        json!(["jim0.84+", "tcl9.0", "tcl9.1"])
    );
    assert_eq!(dialects(Some(SpecSurface::JIM)), json!(["jim"]));
}
use tcl_registry::definer::MemberVisibility;

#[test]
fn default_draft_has_every_schema_field() {
    let draft = default_command_draft();
    for field in schema::COMMAND_FIELDS {
        assert!(
            draft.contains_key(field.key),
            "default draft is missing {}",
            field.key
        );
    }
    let sub = default_subcommand_draft();
    for field in schema::SUBCOMMAND_FIELDS {
        assert!(
            sub.contains_key(field.key),
            "default subcommand draft is missing {}",
            field.key
        );
    }
}

#[test]
fn default_draft_records_no_unrecoverable_fields() {
    let draft = default_command_draft();
    assert_eq!(draft[UNRENDERABLE_KEY], json!([]));
}

/// The structural half of a declared `semantics` plan is plain data and
/// round-trips (`spectcl_roundtrip.rs`'s
/// `a_declared_semantics_plan_survives_the_round_trip` proves the DSL
/// side); what a bare `CommandSpec` cannot carry — a declared
/// implementation's body, or an option-level `-evaluate` decline — stays
/// unrecoverable, exactly like a shipped, compiled-in specialisation.
#[test]
fn a_declared_implementations_body_stays_unrecoverable() {
    use tcl_registry::value_transfer::{
        CompletionSupport, ContextDependency, DeclaredEvaluation, DeclaredImplementation,
        DeclaredSemantics, DeclaredStructure, DeclineReason, EvaluatorCapability, HostKind,
        ImplementationBudget, ImplementationIdentity, Needs, NoRouteReason, SemanticsDeclaration,
    };

    const NO_STRUCTURE: DeclaredStructure = DeclaredStructure {
        effects: &[],
        result: None,
        stores: None,
        iterate: None,
    };
    static IMPLEMENTATION: DeclaredSemantics = DeclaredSemantics {
        scope: "probe::grown",
        structure: NO_STRUCTURE,
        evaluation: DeclaredEvaluation::Implementation(DeclaredImplementation {
            capability: EvaluatorCapability {
                identity: ImplementationIdentity {
                    pack: "probe",
                    id: "probe.grown.v1",
                    content_hash: 0,
                },
                host: HostKind::BoundedTcl,
                target: Needs::NONE,
                inputs: &[],
                depends: &[ContextDependency::TclProfile],
                budget: ImplementationBudget {
                    commands: None,
                    wall_clock_ms: None,
                    value_bytes: None,
                },
                completion: CompletionSupport::NormalOnly,
            },
            slot: None,
            extension: None,
        }),
        option_declines: &[],
    };
    static DECLINING_OPTION: DeclaredSemantics = DeclaredSemantics {
        scope: "probe::grown",
        structure: NO_STRUCTURE,
        evaluation: DeclaredEvaluation::Route(EvalRoute::None {
            reason: NoRouteReason::Declared,
        }),
        option_declines: &[("-about", DeclineReason::NoRoute(NoRouteReason::Declared))],
    };

    let draft = from_command_spec(&CommandSpec {
        name: "probe::grown",
        semantics: SemanticsDeclaration::Declared(&IMPLEMENTATION),
        ..CommandSpec::DEFAULT
    });
    assert_eq!(draft["semantics"], Value::Null);
    assert_eq!(draft[UNRENDERABLE_KEY], json!(["semantics"]));

    let draft = from_command_spec(&CommandSpec {
        name: "probe::grown",
        semantics: SemanticsDeclaration::Declared(&DECLINING_OPTION),
        ..CommandSpec::DEFAULT
    });
    assert_eq!(draft["semantics"], Value::Null);
    assert_eq!(draft[UNRENDERABLE_KEY], json!(["semantics"]));
}

/// An implementation the registry derived from a command's reference body is
/// not a field the draft loses: the body and the author's `-evaluate` that
/// asked for it are on the `runtime_backing` row, which the draft carries, and
/// the next load derives it again. The test above is the control: one a pack
/// wrote stays lost.
#[test]
fn a_derived_implementation_is_not_lost_in_a_draft() {
    use tcl_registry::value_transfer::SemanticsDeclaration;

    let source = "speclib vendor 2.0 {\n    command vendor::double {\n        arity 1\n        \
                      runtime_backing tcl-body {-pack-text {proc vendor::double {x} {expr {$x * 2}}} -evaluate}\n    }\n}\n";
    let set = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::Setting,
            dependency_tier: None,
        },
        source.to_owned(),
    )]);
    let spec = set.packs[0].commands[0].spec;
    assert!(
        matches!(spec.semantics, SemanticsDeclaration::Declared(_)),
        "the load derives an implementation: {:?}",
        spec.semantics
    );
    let draft = from_command_spec(spec);
    assert_eq!(draft["semantics"], Value::Null);
    assert_eq!(draft[UNRENDERABLE_KEY], json!([]), "{draft:?}");
    let backing = draft["runtime_backing"].to_string();
    assert!(
        backing.contains("proc vendor::double") && backing.contains("-evaluate"),
        "the body and the assertion are on the row the draft carries: {backing}"
    );
}

#[test]
fn manufacturer_methods_round_trip_without_inventing_defaults() {
    const METHODS: &[ManufacturerMethod] = &[ManufacturerMethod {
        keyword: "create",
        visibility: MemberVisibility::Unexported,
        names_instance_at: Some(1),
        definition_body_at: None,
        constructor_args_from: 2,
    }];

    assert_eq!(default_command_draft()["manufacturer_methods"], json!([]));
    let draft = from_command_spec(&CommandSpec {
        name: "factory",
        manufacturer_methods: METHODS,
        ..CommandSpec::DEFAULT
    });
    assert_eq!(
        draft["manufacturer_methods"],
        json!([{
            "keyword": "create",
            "visibility": "Unexported",
            "names_instance_at": 1,
            "definition_body_at": null,
            "constructor_args_from": 2,
        }])
    );
    assert!(
        !draft[UNRENDERABLE_KEY]
            .as_array()
            .is_some_and(|keys| keys.iter().any(|key| key == "manufacturer_methods"))
    );

    let rendered = crate::render_rs::render(&draft);
    assert!(rendered.contains("manufacturer_methods: &["));
    assert!(rendered.contains("keyword: \"create\""));
    assert!(rendered.contains("visibility: crate::definer::MemberVisibility::Unexported"));
    assert!(rendered.contains("definition_body_at: None"));
}

#[test]
fn seeding_captures_scalar_and_flag_fields() {
    let spec = CommandSpec {
        name: "lappend",
        traits: Traits::BYTE_COMPILED | Traits::FIRST_ARG_VARNAME,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(tcl_registry::types::TclType::List),
        ..CommandSpec::DEFAULT
    };
    let draft = from_command_spec(&spec);
    assert_eq!(draft["name"], json!("lappend"));
    assert_eq!(
        draft["traits"],
        json!(["BYTE_COMPILED", "FIRST_ARG_VARNAME"])
    );
    assert_eq!(draft["arity"]["min"], json!(1));
    assert_eq!(draft["arity"]["max"], Value::Null);
    assert_eq!(
        draft["arg_roles"],
        json!([{ "index": 0, "role": "VarWrite" }])
    );
    assert_eq!(draft["assigns_variable_at"], json!(0));
    assert_eq!(draft["return_type"], json!("List"));
}

#[test]
fn dynamic_role_and_body_interpreter_metadata_round_trip_to_rust() {
    const SUBCOMMANDS: &[SubCommand] = &[SubCommand {
        name: "eval",
        arg_role_resolver_roles: &[ArgRole::Body],
        body_interpreter: BodyInterpreter::Argument(1),
        ..SubCommand::DEFAULT
    }];
    let spec = CommandSpec {
        name: "probe",
        arg_role_resolver_roles: &[ArgRole::VarWrite, ArgRole::Body],
        variable_write_min_args: Some(4),
        body_interpreter: BodyInterpreter::Argument(0),
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    };

    let draft = from_command_spec(&spec);
    assert_eq!(
        draft["arg_role_resolver_roles"],
        json!(["VarWrite", "Body"])
    );
    assert_eq!(draft["variable_write_min_args"], json!(4));
    assert_eq!(
        draft["body_interpreter"],
        json!({ "kind": "Argument", "index": 0 })
    );
    assert_eq!(
        draft["subcommands"][0]["arg_role_resolver_roles"],
        json!(["Body"])
    );
    assert_eq!(
        draft["subcommands"][0]["body_interpreter"],
        json!({ "kind": "Argument", "index": 1 })
    );

    let rendered = crate::render_rs::render(&draft);
    assert!(rendered.contains("arg_role_resolver_roles: &[ArgRole::VarWrite, ArgRole::Body],"));
    assert!(rendered.contains("variable_write_min_args: Some(4),"));
    assert!(rendered.contains("body_interpreter: BodyInterpreter::Argument(0),"));
    assert!(rendered.contains("body_interpreter: BodyInterpreter::Argument(1),"));
}

#[test]
fn a_nested_enum_payload_keeps_its_own_type_path() {
    // `Debug` alone renders this as `Fixed(String)`, which does not compile.
    let spec = CommandSpec {
        name: "gets",
        var_write_typing: VarWriteTyping::Fixed(tcl_registry::types::TclType::String),
        ..CommandSpec::DEFAULT
    };
    let draft = from_command_spec(&spec);
    assert_eq!(
        draft["var_write_typing"],
        json!("VarWriteTyping::Fixed(TclType::String)")
    );
}

#[test]
fn struct_variant_payloads_render_as_rust_literals() {
    let spec = CommandSpec {
        name: "lappend",
        var_elements_effect: Some(VarElementsEffect::AppendsListElements { values_from: 1 }),
        return_elements: Some(ReturnElements::ListOfArgs { from: 0 }),
        ..CommandSpec::DEFAULT
    };
    let draft = from_command_spec(&spec);
    assert_eq!(
        draft["var_elements_effect"],
        json!("Some(VarElementsEffect::AppendsListElements { values_from: 1 })")
    );
    assert_eq!(
        draft["return_elements"],
        json!("Some(ReturnElements::ListOfArgs { from: 0 })")
    );
}

#[test]
fn seeding_flags_a_function_pointer_field_as_unrecoverable() {
    fn resolver(_args: &[&str]) -> Vec<(u8, ArgRole)> {
        Vec::new()
    }
    let spec = CommandSpec {
        name: "if",
        arg_role_resolver: Some(resolver),
        ..CommandSpec::DEFAULT
    };
    let draft = from_command_spec(&spec);
    assert_eq!(draft["arg_role_resolver"], Value::Null);
    assert_eq!(draft[UNRENDERABLE_KEY], json!(["arg_role_resolver"]));
}

#[test]
fn nested_unrecoverable_fields_stay_with_their_owner() {
    fn resolver(_args: &[&str]) -> Vec<(u8, ArgRole)> {
        Vec::new()
    }

    const NESTED: SubCommand = SubCommand {
        name: "nested",
        arg_role_resolver: Some(resolver),
        arg_role_resolver_roles: &[ArgRole::Body],
        ..SubCommand::DEFAULT
    };
    const CLASS: ObjectClassSpec = ObjectClassSpec {
        class_name: "Probe",
        instance_methods: &[NESTED],
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Strict,
    };
    let draft = from_command_spec(&CommandSpec {
        name: "probe",
        subcommands: &[NESTED],
        object_class: Some(&CLASS),
        ..CommandSpec::DEFAULT
    });

    assert_eq!(draft[UNRENDERABLE_KEY], json!([]));
    assert_eq!(
        draft["subcommands"][0][UNRENDERABLE_KEY],
        json!(["arg_role_resolver"])
    );
    assert_eq!(
        draft["object_class"]["instance_methods"][0][UNRENDERABLE_KEY],
        json!(["arg_role_resolver"])
    );
}

#[test]
fn tk_geometry_is_seeded_as_structured_data_not_a_rust_expression() {
    let spec = CommandSpec {
        name: "layout",
        tk_geometry: Some(tcl_registry::tk_geometry::TkGeometryManagerSpec {
            container_policy: tcl_registry::tk_geometry::TkGeometryContainerPolicy::Independent,
            container_option: Some("-inside"),
            direct_form: false,
            placement_subcommand: Some("arrange"),
            release_subcommands: &["release", "unmanage"],
        }),
        ..CommandSpec::DEFAULT
    };

    let draft = from_command_spec(&spec);
    assert_eq!(
        draft["tk_geometry"],
        json!({
            "container_policy": "Independent",
            "container_option": "-inside",
            "direct_form": false,
            "placement_subcommand": "arrange",
            "release_subcommands": ["release", "unmanage"],
        })
    );
    assert!(draft["tk_geometry"].is_object());
}

#[test]
fn seeding_flags_named_irules_descriptors_as_unrecoverable() {
    const FORM: tcl_registry::events::EventRequirementForm =
        tcl_registry::events::EventRequirementForm {
            argument_prefix: &["get"],
            requires: None,
            only_in: &["FIX_MESSAGE"],
        };
    let spec = CommandSpec {
        name: "eventful",
        event_requirement_forms: &[FORM],
        data_collection: Some(tcl_registry::events::HTTP_COLLECT),
        side_switch_target: Some(tcl_registry::side_effects::SideSwitchTarget::Client),
        event_handler_priority: Some(tcl_registry::events::BIGIP_EVENT_HANDLER_PRIORITY),
        irules_top_level_effect: Some(tcl_registry::events::IrulesTopLevelEffect::Priority),
        ..CommandSpec::DEFAULT
    };
    let draft = from_command_spec(&spec);
    for key in [
        "event_requirement_forms",
        "data_collection",
        "side_switch_target",
        "event_handler_priority",
        "irules_top_level_effect",
    ] {
        assert_eq!(draft[key], Value::Null, "{key} must request its Rust path");
    }
    assert_eq!(
        draft[UNRENDERABLE_KEY],
        json!([
            "event_requirement_forms",
            "data_collection",
            "side_switch_target",
            "event_handler_priority",
            "irules_top_level_effect"
        ])
    );
}
