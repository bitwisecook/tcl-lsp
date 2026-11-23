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
use serde_json::json;

fn drafted(name: &str) -> Draft {
    let mut d = draft::default_command_draft();
    d.insert("name".into(), json!(name));
    d
}

#[test]
fn original_abstract_grammar_roundtrip_keeps_created_class_and_factory_visibilities() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    let registry = tcl_registry::CommandRegistry::build_default();
    let selected = registry.get("oo::abstract").unwrap();
    // Roundtrip the selected semantic data without exporting Native hooks.
    let spec = tcl_registry::CommandSpec {
        name: "AbstractFactory",
        definition_body: selected.definition_body,
        manufacturer_methods: selected.manufacturer_methods,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let seeded = draft::from_command_spec(&spec);
    assert_eq!(seeded["definition_body"], json!("tcloo-abstract"));
    let (text, losses) = render_pack_reporting(&[seeded], "probe");
    assert!(losses.is_empty(), "{losses:?}");
    let loaded = crate::spectcl::evaluate_pack(&text);
    let imported = &loaded.command("AbstractFactory").unwrap().spec;
    let grammar = imported.definition_body.unwrap();
    for selector in ["create", "new", "createWithNamespace"] {
        assert_eq!(
            grammar.manufacturer(selector).unwrap().visibility,
            tcl_registry::definer::MemberVisibility::Unexported
        );
    }
    assert_eq!(imported.manufacturer_methods, spec.manufacturer_methods);
}

#[test]
fn native_object_method_operations_keep_grammar_identity_and_explicit_inline_loss() {
    use tcl_registry::definer::{DefinitionBodyGrammar, TCLOO_GRAMMAR};

    assert_eq!(
        draft::from_command_spec(&tcl_registry::CommandSpec {
            name: "shipped",
            definition_body: Some(&TCLOO_GRAMMAR),
            ..tcl_registry::CommandSpec::DEFAULT
        })["definition_body"],
        json!("tcloo")
    );
    let mut methods = TCLOO_GRAMMAR.builtin_object_methods.to_vec();
    let original = methods
        .iter_mut()
        .find(|method| method.operation.is_some())
        .expect("a shipped native object-method operation");
    original.detail = "custom object method";
    let grammar = Box::leak(Box::new(DefinitionBodyGrammar {
        builtin_object_methods: Box::leak(methods.clone().into_boxed_slice()),
        ..TCLOO_GRAMMAR
    }));
    let seeded = draft::from_command_spec(&tcl_registry::CommandSpec {
        name: "custom",
        definition_body: Some(grammar),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    assert!(seeded["definition_body"].is_object());
    assert!(
        seeded["definition_body"]["builtin_object_methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| !method["operation"].is_null())
    );
    let (text, losses) = render_pack_reporting(&[seeded], "probe");
    assert!(losses.iter().any(|loss| loss.key == "definition_body"));
    assert!(text.contains("Only an exact shipped grammar name preserves them"));
    let loaded = crate::spectcl::evaluate_pack(&text);
    assert!(
        loaded
            .command("custom")
            .unwrap()
            .spec
            .definition_body
            .is_none()
    );

    let mut methods = TCLOO_GRAMMAR.builtin_object_methods.to_vec();
    for method in &mut methods {
        method.operation = None;
    }
    let without_operation = Box::leak(Box::new(DefinitionBodyGrammar {
        builtin_object_methods: Box::leak(methods.into_boxed_slice()),
        ..TCLOO_GRAMMAR
    }));
    let seeded = draft::from_command_spec(&tcl_registry::CommandSpec {
        name: "metadata",
        definition_body: Some(without_operation),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    assert!(
        seeded["definition_body"].is_object(),
        "a missing native operation cannot be identified as the shipped grammar"
    );
}

#[test]
fn nested_native_compiler_proof_has_an_explicit_authoring_exclusion() {
    const WORKERS: &[tcl_registry::spec::SubSubCommand] = &[tcl_registry::spec::SubSubCommand {
        name: "worker",
        native_compilation: Some(tcl_registry::native_compilation::NativeCompilationSpec {
            grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
            operation: tcl_registry::SemanticOperationId::Invoke,
            body: tcl_registry::native_compilation::NativeBodyCompilation::Inherit,
        }),
        ..tcl_registry::spec::SubSubCommand::DEFAULT
    }];
    const MEMBERS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "codec",
        sub_subcommands: WORKERS,
        ..tcl_registry::SubCommand::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe::nested",
        subcommands: MEMBERS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let seeded = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&seeded), "probe");
    assert!(
        text.contains("TODO(spectcl): `nested_native_compilation`"),
        "{text}"
    );
    assert!(
        GAPS.iter()
            .any(|gap| gap.key == "nested_native_compilation" && gap.kind == GapKind::Excluded)
    );
    let loaded = crate::spectcl::evaluate_pack(&text);
    let command = &loaded.command("probe::nested").unwrap().spec;
    assert!(
        command.subcommands[0].sub_subcommands[0]
            .native_compilation
            .is_none()
    );
}

#[test]
// Implementation contract: naming.variable.registry-receiver-authoring-parity
// docs/design/analysis/name-resolution-proofs/registry-variable-receiver-authoring-parity.md
fn combined_variable_receiver_metadata_round_trips_commands_members_and_forms() {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined;
    use tcl_registry::{CommandSpec, SubCommand, forms::CommandForm};
    const FORMS: &[CommandForm] = &[
        CommandForm {
            name: "withdraw",
            variable_receivers: Some(&[]),
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "inherit",
            ..CommandForm::DEFAULT
        },
    ];
    const SUBS: &[SubCommand] = &[SubCommand {
        name: "nested",
        variable_receivers: Some(&[(0, Combined)]),
        subcommand_forms: FORMS,
        ..SubCommand::DEFAULT
    }];
    let spec = CommandSpec {
        name: "probe::names",
        variable_receivers: Some(&[(0, Combined), (2, Combined)]),
        command_forms: FORMS,
        subcommands: SUBS,
        ..CommandSpec::DEFAULT
    };
    let seeded = draft::from_command_spec(&spec);
    assert_eq!(seeded["variable_receivers"], serde_json::json!([0, 2]));
    let rust = crate::render_rs::render(&seeded);
    assert!(rust.contains("variable_receivers: Some(&[(0, tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined), (2, tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined)]),"), "{rust}");
    assert!(rust.contains("variable_receivers: Some(&[]),"));
    let text = render_pack(std::slice::from_ref(&seeded), "probe");
    assert!(text.contains("variable_receivers {0 2}"), "{text}");
    assert!(text.contains("variable_receivers {}"), "{text}");
    let loaded = crate::spectcl::evaluate_pack(&text);
    let parsed = &loaded
        .command("probe::names")
        .expect("naming round trip")
        .spec;
    assert_eq!(parsed.variable_receivers, spec.variable_receivers);
    assert_eq!(parsed.command_forms[0].variable_receivers, Some(&[][..]));
    assert_eq!(parsed.command_forms[1].variable_receivers, None);
    assert_eq!(
        parsed.subcommands[0].variable_receivers,
        spec.subcommands[0].variable_receivers
    );
    assert_eq!(
        parsed.subcommands[0].subcommand_forms[0].variable_receivers,
        Some(&[][..])
    );
    assert!(parsed.successful_handler.is_none());
    assert!(parsed.native_compilation.is_none());
}

#[test]
fn successful_handler_native_contract_has_an_explicit_authoring_gap() {
    let spec = tcl_registry::CommandSpec {
        name: "probe::native",
        successful_handler: Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::Leaf),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let seeded = draft::from_command_spec(&spec);
    assert!(
        seeded["successful_handler"]
            .as_str()
            .is_some_and(|value| value.contains("::Leaf"))
    );
    let text = render_pack(std::slice::from_ref(&seeded), "probe");
    assert!(
        text.contains("TODO(spectcl): `successful_handler`"),
        "{text}"
    );
    assert!(
        GAPS.iter()
            .any(|gap| gap.key == "successful_handler" && gap.kind == GapKind::Excluded)
    );
    let loaded = crate::spectcl::evaluate_pack(&text);
    assert!(
        loaded
            .command("probe::native")
            .unwrap()
            .spec
            .successful_handler
            .is_none()
    );
}

#[test]
fn source_deprecation_advice_survives_both_renderers_and_pack_reload() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let spec = tcl_registry::CommandSpec {
        name: "probe::legacy",
        source_deprecation_advice: Some(tcl_registry::SourceDeprecationAdvice::IruleMatchclass),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    assert_eq!(before["source_deprecation_advice"], "IruleMatchclass");
    let rust = crate::render_rs::render(&before);
    assert!(
        rust.contains("source_deprecation_advice: Some(SourceDeprecationAdvice::IruleMatchclass)"),
        "{rust}"
    );
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(
        text.contains("source_deprecation_advice IruleMatchclass"),
        "{text}"
    );
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let loaded = pack.command("probe::legacy").unwrap().spec;
    assert_eq!(
        loaded.source_deprecation_advice,
        spec.source_deprecation_advice
    );
    assert_eq!(
        draft::from_command_spec(loaded)["source_deprecation_advice"],
        before["source_deprecation_advice"]
    );
}

#[test]
fn positional_option_prefix_survives_both_renderers_and_pack_reload() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    const NESTED: &[tcl_registry::SubSubCommand] = &[tcl_registry::SubSubCommand {
        name: "configure",
        option_prefix_words: 3,
        options: Some(&[]),
        ..tcl_registry::SubSubCommand::DEFAULT
    }];
    const SUBS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "format",
        sub_subcommands: NESTED,
        option_prefix_words: 1,
        reserved_trailing_words: 2,
        ..tcl_registry::SubCommand::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe",
        option_prefix_words: 2,
        subcommands: SUBS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let rust = crate::render_rs::render(&before);
    assert!(rust.contains("option_prefix_words: 3,"), "{rust}");
    assert!(rust.contains("option_prefix_words: 2,"), "{rust}");
    assert!(rust.contains("option_prefix_words: 1,"), "{rust}");
    assert!(rust.contains("reserved_trailing_words: 2,"), "{rust}");
    let text = render_pack(std::slice::from_ref(&before), "probe");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let loaded = pack.command("probe").unwrap().spec;
    assert_eq!(
        loaded.subcommands[0].sub_subcommands[0].option_prefix_words,
        3
    );
    assert_eq!(loaded.option_prefix_words, 2);
    assert_eq!(loaded.subcommands[0].option_prefix_words, 1);
    assert_eq!(loaded.subcommands[0].reserved_trailing_words, 2);
}

#[test]
fn positional_arity_survives_both_renderers_and_pack_reload() {
    let spec = tcl_registry::CommandSpec {
        name: "probe::create",
        arity: tcl_registry::arity::Arity::new(0, 1).with_positionals(),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    assert_eq!(before["arity"]["count"], "positionals");
    let rust = crate::render_rs::render(&before);
    assert!(
        rust.contains("Arity::new(0, 1).with_positionals()"),
        "{rust}"
    );
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("arity ..1 -positionals"), "{text}");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let reloaded = pack.command("probe::create").unwrap().spec;
    assert_eq!(reloaded.arity, spec.arity);
    assert_eq!(draft::from_command_spec(reloaded)["arity"], before["arity"]);
}

#[test]
fn expression_coercion_survives_render_load_and_draft() {
    let spec = tcl_registry::CommandSpec {
        name: "probe::evaluate",
        representation_effect: Some(
            tcl_registry::representation::RepresentationEffect::CoerceExpressionValues {
                arguments_from: 0,
            },
        ),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(
        text.contains("representation_effect {CoerceExpressionValues 0}"),
        "{text}"
    );
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let after = draft::from_command_spec(pack.command("probe::evaluate").unwrap().spec);
    assert_eq!(
        after.get("representation_effect"),
        before.get("representation_effect")
    );
}

#[test]
fn ordinary_container_coercion_survives_render_load_and_draft() {
    use tcl_registry::representation::RepresentationEffect;
    for effect in [
        RepresentationEffect::CoerceOrdinaryList { operand: 0 },
        RepresentationEffect::CoerceOrdinaryDictionary { operand: 1 },
        RepresentationEffect::CoerceOrdinaryListPairs { variables_from: 0 },
        RepresentationEffect::CoerceOrdinaryListIndices {
            operand: 0,
            indices_from: 1,
        },
        RepresentationEffect::CoerceOrdinaryListRange {
            operand: 0,
            first: 1,
            last: 2,
        },
    ] {
        let spec = tcl_registry::CommandSpec {
            name: "probe::container",
            representation_effect: Some(effect),
            ..tcl_registry::CommandSpec::DEFAULT
        };
        let before = draft::from_command_spec(&spec);
        let text = render_pack(std::slice::from_ref(&before), "probe");
        let pack = crate::spectcl::evaluate_pack(&text);
        assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
        let reloaded = pack.command("probe::container").unwrap().spec;
        assert_eq!(reloaded.representation_effect, Some(effect));
        assert_eq!(
            draft::from_command_spec(reloaded)["representation_effect"],
            before["representation_effect"]
        );
    }
}

#[test]
fn invalid_taint_link_is_not_rendered_without_var_write_role() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let entry = registry.get("entry").expect("Tk entry spec");
    let seeded = draft::from_command_spec(entry);
    let mut option = seeded["options"]
        .as_array()
        .expect("entry options")
        .iter()
        .find(|option| str_of(&option["name"]) == "-textvariable")
        .expect("entry -textvariable")
        .clone();
    option["value"]["role"] = json!("Value");
    option["value"]["also_role"] = Value::Null;
    option["value"]["taints_var_write"] = json!(true);
    let mut draft = drafted("probe");
    draft.insert("options".into(), Value::Array(vec![option]));

    let text = render_pack(&[draft], "probe");
    assert!(!text.contains("-taints-var-write"), "{text}");
}

#[test]
fn a_default_draft_renders_a_pack_that_says_nothing_else() {
    let text = render_pack(&[drafted("mycommand")], "probe");
    assert!(text.contains(&format!("speclib probe {DSL_VERSION} {{")));
    assert!(text.contains("command mycommand {"));
    let statements: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('}'))
        .collect();
    assert_eq!(
        statements,
        vec![
            format!("speclib probe {DSL_VERSION} {{").as_str(),
            "command mycommand {"
        ],
        "a default draft must emit only what the author actually set:\n{text}"
    );
}

/// The 2.0 availability algebra, spelled the way the loader reads it
/// back — one row per contiguous run of releases, open at the top when
/// the run reaches the newest.
#[test]
fn a_dialect_set_is_written_in_the_availability_algebra() {
    let rows = |members: &[&str]| availability_rows(members);
    assert_eq!(
        rows(&["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]),
        Some(vec!["tcl 8.4-".to_owned()])
    );
    assert_eq!(
        rows(&["tcl8.6", "tcl9.0", "tcl9.1"]),
        Some(vec!["tcl 8.6-".to_owned()])
    );
    // The bound is exclusive, so a run stopping at 8.6 names 9.0.
    assert_eq!(
        rows(&["tcl8.4", "tcl8.5", "tcl8.6"]),
        Some(vec!["tcl 8.4-9.0".to_owned()])
    );
    assert_eq!(
        rows(&["tcl8.4", "tcl8.5"]),
        Some(vec!["tcl 8.4-8.6".to_owned()])
    );
    // A bare `X.Y` names that release line only.
    assert_eq!(rows(&["tcl8.5"]), Some(vec!["tcl 8.5".to_owned()]));
    // A hole in the ladder is two rows, which the loader unions.
    assert_eq!(
        rows(&["tcl8.4", "tcl8.6"]),
        Some(vec!["tcl 8.4".to_owned(), "tcl 8.6".to_owned()])
    );
    assert_eq!(
        rows(&["tcl8.6", "tcl9.0", "tcl9.1", "f5-irules"]),
        Some(vec!["tcl 8.6-".to_owned(), "f5-irules".to_owned()])
    );
    // `tk` is a package pin, and the algebra says so.
    assert_eq!(rows(&["tk"]), Some(vec!["package Tk".to_owned()]));

    // A dialect the closed provider list does not name keeps
    // `dialects`, and so does the empty (declared-unavailable) set.
    assert_eq!(rows(&["f5-iapps"]), None);
    assert_eq!(rows(&["tcl9.0", "bpf"]), None);
    assert_eq!(rows(&[]), None);

    // The flag form: one row bare, several wrapped into one list word.
    assert_eq!(
        available_flag(&["tcl8.6", "tcl9.0", "tcl9.1"]).as_deref(),
        Some("{tcl 8.6-}")
    );
    assert_eq!(
        available_flag(&["tcl8.6", "tcl9.0", "tcl9.1", "f5-irules"]).as_deref(),
        Some("{{tcl 8.6-} {f5-irules}}")
    );
}

/// The rendered rows, end to end, over a real shipped command: the
/// command scope takes the statement spelling and an option takes the
/// flag, and both load back to the very bits the draft held.
/// Every Tk command that refines its
/// subcommand forms round-trips through the pack DSL: the rendered
/// `refine` blocks reload to the same form tables the compiled specs
/// carry. Tk's form sites are the measured blocker the descriptor
/// existed to unblock, so they are what proves it.
#[test]
fn tk_form_refinements_round_trip_through_the_pack_dsl() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut checked = 0usize;
    for name in registry.command_names().collect::<Vec<_>>() {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        if spec.required_package != Some("Tk")
            || spec
                .subcommands
                .iter()
                .all(|sub| sub.subcommand_forms.is_empty())
        {
            continue;
        }
        let seeded = draft::from_command_spec(spec);
        let text = render_pack(std::slice::from_ref(&seeded), "tk-forms");
        let pack = crate::spectcl::evaluate_pack(&text);
        let reloaded = pack
            .commands
            .first()
            .unwrap_or_else(|| panic!("`{name}` reloads:\n{text}"))
            .spec;
        for original in spec.subcommands {
            if original.subcommand_forms.is_empty() {
                continue;
            }
            let back = reloaded
                .subcommand(original.name)
                .unwrap_or_else(|| panic!("`{name} {}` reloads", original.name));
            assert_eq!(
                format!("{:?}", back.subcommand_forms),
                format!("{:?}", original.subcommand_forms),
                "`{name} {}` form table",
                original.name
            );
            checked += 1;
        }
    }
    assert!(checked >= 40, "only {checked} Tk form sites round-tripped");
}

#[test]
fn availability_rows_reload_to_the_set_they_were_written_from() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let seeded = draft::from_command_spec(registry.get("lsort").expect("the lsort spec"));
    let text = render_pack(std::slice::from_ref(&seeded), "probe");
    // `{all-tcl f5-irules}` in the algebra, and `-nocase`'s 8.5+ gate.
    assert!(text.contains("available {tcl 8.4-} {f5-irules}"), "{text}");
    assert!(text.contains("-available {tcl 8.5-}"), "{text}");
    // …and not a legacy row anywhere: the prose may say the word
    // "dialects", so the check is on the row spellings themselves.
    assert!(!text.contains("dialects {all-tcl f5-irules}"), "{text}");
    assert!(!text.contains("-dialects tcl8.5+"), "{text}");

    let pack = crate::spectcl::evaluate_pack(&text);
    let spec = pack.commands.first().expect("the command").spec;
    let reloaded = draft::from_command_spec(spec);
    assert_eq!(reloaded["surface"], seeded["surface"]);
    let gate = |draft: &Draft, option: &str| -> Value {
        as_array(&draft["options"])
            .iter()
            .find(|o| str_of(&o["name"]) == option)
            .map_or(Value::Null, |o| o["surface"].clone())
    };
    for option in ["-nocase", "-indices", "-stride", "-ascii"] {
        assert_eq!(
            gate(&reloaded, option),
            gate(&seeded, option),
            "{option} lost its availability:\n{text}"
        );
    }
}

/// The header decides the spelling, and the two never disagree: a pack
/// declaring the newest vocabulary takes `available`, and one rendered
/// under an older declared vocabulary keeps the legacy word — so neither
/// earns the loader's per-site "newer than this pack declares" notice,
/// in either direction.
#[test]
fn the_header_and_the_body_agree_on_their_vocabulary() {
    assert_eq!(DSL_VERSION, tcl_spectcl::NEWEST_VOCABULARY_VERSION);
    let registry = tcl_registry::CommandRegistry::build_default();
    let seeded = draft::from_command_spec(registry.get("lsort").expect("the lsort spec"));

    let newest = render_pack(std::slice::from_ref(&seeded), "probe");
    assert!(
        newest.contains(&format!("speclib probe {DSL_VERSION} {{")),
        "{newest}"
    );
    assert!(newest.contains("available {tcl 8.4-}"), "{newest}");
    assert!(
        !crate::spectcl::evaluate_pack(&newest)
            .notices
            .iter()
            .any(|notice| notice
                .message
                .contains("vocabulary, but this pack declares")),
        "{:#?}",
        crate::spectcl::evaluate_pack(&newest).notices
    );

    let two_zero = render_pack_with_version(std::slice::from_ref(&seeded), "probe", "2.0");
    assert!(two_zero.contains("speclib probe 2.0 {"), "{two_zero}");
    assert!(two_zero.contains("available {tcl 8.4-}"), "{two_zero}");

    let older = render_pack_with_version(&[seeded], "probe", "1.2");
    assert!(older.contains("speclib probe 1.2 {"), "{older}");
    assert!(older.contains("dialects {all-tcl f5-irules}"), "{older}");
    assert!(!older.contains("available {tcl"), "{older}");
    assert!(!older.contains("-available "), "{older}");
    assert!(
        !crate::spectcl::evaluate_pack(&older)
            .notices
            .iter()
            .any(|notice| notice
                .message
                .contains("vocabulary, but this pack declares")),
        "{:#?}",
        crate::spectcl::evaluate_pack(&older).notices
    );
}

#[test]
fn arity_takes_its_six_documented_spellings() {
    for (min, max, step, also, expected) in [
        (3u64, Some(3u64), 0u64, None, "arity 3"),
        (1, Some(2), 0, None, "arity 1..2"),
        (1, None, 0, None, "arity 1.."),
        (0, Some(2), 0, None, "arity ..2"),
        // `arity ..` on its own is the default and is never emitted, so
        // the unbounded spelling is only visible with a modifier.
        (0, None, 2, None, "arity .. -step 2"),
        (3, None, 2, None, "arity 3.. -step 2"),
        (3, None, 2, Some(2), "arity 3.. -step 2 -also 2"),
    ] {
        let mut d = drafted("probe");
        d.insert(
            "arity".into(),
            json!({ "min": min, "max": max, "step": step, "also_exact": also }),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains(expected), "expected `{expected}` in:\n{text}");
    }
}

/// A braced word is the only faithful spelling, so text braces cannot hold
/// has none at all — the quoted form keeps its backslashes and normalises
/// `$x` to `${x}` on the way through the CST.
#[test]
fn only_a_braced_word_carries_prose_verbatim() {
    assert_eq!(braced("plain text").as_deref(), Some("{plain text}"));
    assert_eq!(
        braced("a {nested} word").as_deref(),
        Some("{a {nested} word}")
    );
    // `$` and `[` are literal inside braces, so they need nothing.
    assert_eq!(braced("$x [y]").as_deref(), Some("{$x [y]}"));
    // A lone opening brace, a trailing backslash, and a backslash-newline
    // (the one substitution Tcl still performs inside braces) have no
    // faithful spelling at all.
    assert_eq!(braced("a { lone"), None);
    assert_eq!(braced("trailing\\"), None);
    assert_eq!(braced("wrapped \\\nline"), None);
}

#[test]
fn a_native_hook_is_named_after_the_field_it_fills() {
    let mut d = drafted("probe");
    d.insert(
        draft::UNRENDERABLE_KEY.into(),
        json!(["arg_role_resolver", "script_timing_resolver", "const_fold"]),
    );
    let text = render_pack(std::slice::from_ref(&d), "probe");
    assert!(text.contains("arg_role_resolver -native probe::arg_role_resolver"));
    assert!(
        text.contains("script_timing_resolver -native probe::script_timing_resolver"),
        "{text}"
    );
    assert!(text.contains("const_fold -native probe::const_fold"));
}

#[test]
fn a_field_the_dsl_cannot_carry_becomes_a_todo_naming_it() {
    let mut d = drafted("probe");
    d.insert(draft::UNRENDERABLE_KEY.into(), json!(["frame_effect"]));
    let text = render_pack(std::slice::from_ref(&d), "probe");
    assert!(
        text.contains("TODO(spectcl): `frame_effect`"),
        "the TODO must name the field:\n{text}"
    );
    assert!(text.contains("frame_effect -level-word W -layout L"));
}

#[test]
fn resolver_capabilities_render_while_remaining_loader_gaps_stay_explicit() {
    let mut d = drafted("probe");
    d.insert("arg_role_resolver_roles".into(), json!(["Body"]));
    d.insert(draft::UNRENDERABLE_KEY.into(), json!(["arg_role_resolver"]));
    d.insert("variable_write_min_args".into(), json!(3));
    d.insert(
        "body_interpreter".into(),
        json!({ "kind": "Argument", "index": 1 }),
    );

    let text = render_pack(std::slice::from_ref(&d), "probe");
    assert!(
        text.contains("arg_role_resolver_roles Body"),
        "the capability set is ordinary loadable data:\n{text}"
    );
    for key in ["variable_write_min_args", "body_interpreter"] {
        assert!(
            text.contains(&format!("TODO(spectcl): `{key}`")),
            "the loader gap must remain explicit:\n{text}"
        );
    }
    let (two_zero, losses) = render_pack_reporting_with_version(&[d], "probe", "2.0");
    assert!(
        !two_zero.contains("arg_role_resolver_roles Body"),
        "2.0 must not emit a 2.1 statement:\n{two_zero}"
    );
    assert!(
        two_zero
            .contains("TODO(spectcl): `arg_role_resolver_roles` requires SpecTcl 2.1 vocabulary"),
        "{two_zero}"
    );
    assert!(
        losses.iter().any(|loss| {
            loss.key == "arg_role_resolver_roles"
                && loss.reason == "requires SpecTcl 2.1 vocabulary"
        }),
        "{losses:#?}"
    );
}

#[test]
fn multi_command_reporting_keeps_each_loss_with_its_command() {
    let clean = drafted("clean");
    let mut lossy = drafted("lossy");
    lossy.insert(draft::UNRENDERABLE_KEY.into(), json!(["frame_effect"]));

    let (_, reports) = render_pack_reporting_by_command(&[clean, lossy], "probe");
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].command, "clean");
    assert!(reports[0].losses.is_empty());
    assert_eq!(reports[1].command, "lossy");
    assert_eq!(reports[1].losses.len(), 1);
    assert_eq!(reports[1].losses[0].key, "frame_effect");
}

#[test]
fn shared_value_table_losses_are_reported_for_every_owning_command() {
    let values = json!([{
        "index": 0,
        "values": [
            { "value": "{", "detail": "Unwritable value.", "min_tcl": null, "code": null },
        ],
    }]);
    let mut first = drafted("first");
    first.insert("arg_values".into(), values.clone());
    let mut second = drafted("second");
    second.insert("arg_values".into(), values);

    let (_, reports) = render_pack_reporting_by_command(&[first, second], "probe");
    assert_eq!(reports.len(), 2);
    for report in reports {
        assert_eq!(report.losses.len(), 1, "{}", report.command);
        assert_eq!(report.losses[0].key, "arg_values");
    }
}

#[test]
fn shared_value_table_losses_include_nested_command_owners() {
    let values = json!([{
        "index": 0,
        "values": [
            { "value": "{", "detail": "Unwritable value.", "min_tcl": null, "code": null },
        ],
    }]);
    let mut subcommand = draft::default_subcommand_draft();
    subcommand.insert("name".into(), json!("nested"));
    subcommand.insert("arg_values".into(), values.clone());
    let mut method = draft::default_subcommand_draft();
    method.insert("name".into(), json!("method"));
    method.insert("arg_values".into(), values);

    let mut command = drafted("parent");
    command.insert("subcommands".into(), json!([subcommand]));
    command.insert(
        "object_class".into(),
        json!({
            "class_name": "Thing",
            "superclasses": [],
            "allow_unknown_methods": false,
            "method_prefix_matching": "Disabled",
            "instance_methods": [method],
        }),
    );

    let (text, reports) = render_pack_reporting_by_command(&[command], "probe");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].command, "parent");
    assert_eq!(reports[0].losses.len(), 1);
    assert_eq!(reports[0].losses[0].key, "arg_values");
    let table_names: Vec<_> = text
        .lines()
        .filter_map(|line| {
            line.split_once("-values-from ")
                .and_then(|(_, name)| name.split_whitespace().next())
        })
        .collect();
    assert_eq!(table_names.len(), 2, "{text}");
    assert_eq!(table_names[0], table_names[1], "{text}");
    assert!(
        text.contains(&format!("values {} {{", table_names[0])),
        "{text}"
    );
}

#[test]
fn values_carrying_details_are_hoisted_to_a_shared_table() {
    let mut d = drafted("probe");
    d.insert(
        "arg_values".into(),
        json!([{
            "index": 0,
            "values": [
                { "value": "alnum", "detail": "Any alphanumeric.", "min_tcl": null, "code": null },
                { "value": "dict", "detail": "A dict.", "min_tcl": "V9_0", "code": null },
            ],
        }]),
    );
    let text = render_pack(&[d], "probe");
    assert!(text.contains("values probe-arg0 {"), "{text}");
    assert!(
        text.contains("value alnum -detail {Any alphanumeric.}"),
        "{text}"
    );
    assert!(text.contains("value dict -min-tcl tcl9.0"), "{text}");
    assert!(text.contains("arg 0 -values-from probe-arg0"), "{text}");
}

#[test]
fn a_plain_value_set_stays_on_the_row() {
    let mut d = drafted("probe");
    d.insert(
        "arg_values".into(),
        json!([{
            "index": 1,
            "values": [
                { "value": "yes", "detail": "", "min_tcl": null, "code": null },
                { "value": "no", "detail": "", "min_tcl": null, "code": null },
            ],
        }]),
    );
    let text = render_pack(&[d], "probe");
    assert!(text.contains("arg 1 -values {yes no}"), "{text}");
    assert!(!text.contains("values probe"), "{text}");
}

#[test]
fn a_repeated_arg_layout_round_trips_through_its_rust_expression() {
    let rows = repeat_rows(
        "&[RepeatedArgLayout { role: ArgRole::LoopVarList, start: 0, stride: 2, \
             exclude_trailing: 1, optional_leading_word: false, conditional_binding: false }]",
    )
    .expect("the layout parses");
    assert_eq!(
        rows[0].join(" "),
        "repeat LoopVarList -stride 2 -exclude-trailing 1"
    );
}

/// R12: an inference edge has no `SpecTcl` statement, so the exporter must
/// refuse it rather than render it as the assertion it is not.
#[test]
fn an_inference_edge_is_not_exported_as_an_assertion() {
    const INFER: &str = "&[OptionRelation { kind: RelationKind::Requires, \
                             mode: RelationMode::Infer, \
                             subject: Some(OptionTerm::Option(\"-a\")), \
                             terms: &[OptionTerm::Option(\"-b\")], surface: None }]";
    // The same relation as an assertion still renders.
    const ASSERT: &str = "&[OptionRelation { kind: RelationKind::Requires, \
                              mode: RelationMode::Assert, \
                              subject: Some(OptionTerm::Option(\"-a\")), \
                              terms: &[OptionTerm::Option(\"-b\")], surface: None }]";
    assert!(option_conflict_rows(INFER, true).is_none());
    let rows = option_conflict_rows(ASSERT, true).expect("the relation parses");
    assert_eq!(rows[0].join(" "), "option_requires -a -b");
}

#[test]
fn an_option_constraint_keeps_its_dialect_gate() {
    const EXPR: &str = "&[OptionRelation { kind: RelationKind::MutuallyExclusive, \
                            subject: None, terms: &[OptionTerm::Option(\"-glob\"), \
                            OptionTerm::Option(\"-regexp\")], \
                            surface: Some(surface![SpecSurface::ALL_TCL, \
                            SpecSurface::IRULES]) }]";
    // Under 2.0 the gate is written in the availability algebra …
    let rows = option_conflict_rows(EXPR, true).expect("the constraint parses");
    assert_eq!(
        rows[0].join(" "),
        "option_conflict {-glob -regexp} -available {{tcl 8.4-} {f5-irules}}"
    );
    // … and under an older declared vocabulary, in the legacy word.
    let rows = option_conflict_rows(EXPR, false).expect("the constraint parses");
    assert_eq!(
        rows[0].join(" "),
        "option_conflict {-glob -regexp} -dialects {all-tcl f5-irules}"
    );
}

#[test]
fn a_rendered_surface_reads_back_as_the_members_it_was_written_from() {
    for members in [
        vec!["tcl8.6", "tcl9.0", "tcl9.1"],
        vec![
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
        ],
        vec!["jim"],
        vec!["jim0.84+"],
        vec!["tcl9.0", "tcl9.1", "jim0.84+"],
    ] {
        let values: Vec<Value> = members.iter().map(|member| Value::from(*member)).collect();
        let rendered = crate::render_rs::dialect_set(&values);
        assert_eq!(
            dialect_names(&rendered).as_deref(),
            Some(
                members
                    .iter()
                    .map(|m| (*m).to_owned())
                    .collect::<Vec<_>>()
                    .as_slice()
            ),
            "{rendered}"
        );
    }
    // An interval that closes is not the open-ended row.
    assert!(
        dialect_names(
            "surface![SpecSurface::core_in(tcl_dialect::model::Family::Jim, \
                 &[(\"0.79\", Some(\"0.84\"))])]"
        )
        .is_none()
    );
}

/// **The E-R14 export gate.** Each of the three new relation kinds, and
/// each of the four term shapes, must come back out as the statement an
/// author would have written — otherwise a pack that round-trips through
/// the studio silently loses a relation.
#[test]
fn every_relation_kind_and_term_shape_round_trips_to_its_statement() {
    for (expr, expected) in [
        (
            "&[OptionRelation { kind: RelationKind::Requires, \
                 subject: Some(OptionTerm::Option(\"-command\")), \
                 terms: &[OptionTerm::Option(\"-channel\")], surface: None, \
                 message: None }]",
            "option_requires -command -channel",
        ),
        (
            "&[OptionRelation { kind: RelationKind::RequiresOneOf, subject: None, \
                 terms: &[OptionTerm::Option(\"-channel\"), OptionTerm::Argument(0)], \
                 surface: None, message: Some(\"Neither -channel nor text specified\") }]",
            "option_requires_one_of {} {-channel {arg 0}} \
                 -message {Neither -channel nor text specified}",
        ),
        (
            "&[OptionRelation { kind: RelationKind::Forbids, \
                 subject: Some(OptionTerm::OptionValue(\"-order\", \"in\")), \
                 terms: &[OptionTerm::OptionValue(\"-type\", \"bfs\")], \
                 surface: None, message: None }]",
            "option_forbids {-order in} {{-type bfs}}",
        ),
        (
            "&[OptionRelation { kind: RelationKind::Forbids, \
                 subject: Some(OptionTerm::Option(\"-channel\")), \
                 terms: &[OptionTerm::ArgumentValue(1, \"text\")], \
                 surface: None, message: None }]",
            "option_forbids -channel {{arg 1 text}}",
        ),
    ] {
        let rows = option_conflict_rows(expr, true).expect("the relation parses");
        assert_eq!(rows[0].join(" "), expected, "{expr}");
    }
}

#[test]
fn relation_term_values_survive_nested_list_render_and_load() {
    use tcl_registry::{OptionRelation, OptionTerm, relation::RelationKind};
    const RELATIONS: &[OptionRelation] = &[OptionRelation {
        kind: RelationKind::Forbids,
        subject: Some(OptionTerm::OptionValue("-subject", "two words")),
        terms: &[
            OptionTerm::OptionValue("-space", "two words"),
            OptionTerm::OptionValue("-slash", r"one\two"),
            OptionTerm::ArgumentValue(0, ""),
            OptionTerm::ArgumentValue(1, "{nested}"),
            OptionTerm::ArgumentValue(2, "$name [command];"),
            OptionTerm::ArgumentValue(3, "é"),
            OptionTerm::ArgumentValue(4, "line\nnext"),
            OptionTerm::ArgumentValue(5, "tail\\"),
        ],
        ..OptionRelation::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe::relations",
        option_relations: RELATIONS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let reloaded = pack.command("probe::relations").unwrap().spec;
    assert_eq!(reloaded.option_relations, RELATIONS, "{text}");
    assert_eq!(
        draft::from_command_spec(reloaded).get("option_relations"),
        before.get("option_relations"),
        "{text}",
    );
}

#[test]
fn a_simple_subcommand_is_written_on_one_line() {
    let mut d = drafted("probe");
    let mut sub = draft::default_subcommand_draft();
    sub.insert("name".into(), json!("at"));
    sub.insert(
        "arity".into(),
        json!({ "min": 1, "max": 1, "step": 0, "also_exact": null }),
    );
    sub.insert("detail".into(), json!("Get header name by index."));
    d.insert("subcommands".into(), json!([Value::Object(sub)]));
    let text = render_pack(&[d], "probe");
    assert!(
        text.contains("subcommand at { arity 1 ; detail {Get header name by index.} }"),
        "{text}"
    );
}

/// The class's three one-word facts ride on the statement; only the
/// method table goes in the block, and a method is written with the
/// subcommand body grammar under a `method` keyword.
#[test]
fn an_object_class_writes_its_flags_on_the_row_and_its_methods_in_the_block() {
    let mut d = drafted("probe");
    let mut method = draft::default_subcommand_draft();
    method.insert("name".into(), json!("configure"));
    method.insert(
        "arity".into(),
        json!({ "min": 1, "max": 1, "step": 0, "also_exact": null }),
    );
    method.insert("detail".into(), json!("Reconfigure the widget."));
    d.insert(
        "object_class".into(),
        json!({
            "class_name": "::probe::Widget",
            "instance_methods": [Value::Object(method)],
            "superclasses": ["::probe::Base", "::probe::Mixin"],
            "allow_unknown_methods": true,
            "method_prefix_matching": "Enabled",
        }),
    );
    let text = render_pack(&[d], "probe");
    assert!(text.contains("object_class ::probe::Widget"), "{text}");
    assert!(
        text.contains("-superclass {::probe::Base ::probe::Mixin}"),
        "{text}"
    );
    assert!(text.contains("-allow-unknown"), "{text}");
    assert!(text.contains("-method-prefix-matching Enabled"), "{text}");
    assert!(
        text.contains("method configure { arity 1 ; detail {Reconfigure the widget.} }"),
        "{text}"
    );
}

/// A class with no methods of its own — `SpiceGenTcl`'s `R`, an empty
/// subclass — is one row, not an empty block.
#[test]
fn an_object_class_with_no_methods_is_a_single_row() {
    let mut d = drafted("probe");
    d.insert(
        "object_class".into(),
        json!({
            "class_name": "::probe::R",
            "instance_methods": [],
            "superclasses": ["::probe::Resistor"],
            "allow_unknown_methods": false,
            "method_prefix_matching": "Strict",
        }),
    );
    let text = render_pack(&[d], "probe");
    assert!(
        text.contains("object_class ::probe::R -superclass ::probe::Resistor\n"),
        "{text}"
    );
    assert!(!text.contains("-allow-unknown"), "{text}");
}

/// The gate in miniature: a spec carrying an `object_class` renders,
/// loads, and drafts back **equal** — no notices, no gap.
#[test]
fn an_object_class_survives_render_load_draft() {
    static METHODS: &[tcl_registry::spec::SubCommand] = &[tcl_registry::spec::SubCommand {
        name: "Add",
        arity: tcl_registry::arity::Arity::at_least(1),
        detail: "Add a series by literal type name.",
        synopsis: "$chart Add seriesType ?-opt value ...?",
        mutator: true,
        ..tcl_registry::spec::SubCommand::DEFAULT
    }];
    static CLASS: tcl_registry::spec::ObjectClassSpec = tcl_registry::spec::ObjectClassSpec {
        class_name: "probe::chart",
        instance_methods: METHODS,
        superclasses: &["probe::base"],
        allow_unknown_methods: true,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Enabled,
    };
    let spec = tcl_registry::spec::CommandSpec {
        name: "probe::chart",
        arity: tcl_registry::arity::Arity::at_least(1),
        object_class: Some(&CLASS),
        ..tcl_registry::spec::CommandSpec::DEFAULT
    };

    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("-method-prefix-matching Enabled"), "{text}");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let reloaded = pack.command("probe::chart").expect("the command reloads");
    let after = draft::from_command_spec(reloaded.spec);
    assert_eq!(
        after.get("object_class"),
        before.get("object_class"),
        "{text}"
    );
    assert_eq!(
        after.get(draft::UNRENDERABLE_KEY),
        before.get(draft::UNRENDERABLE_KEY),
        "{text}"
    );
    assert!(
        gap("object_class").is_none(),
        "`object_class` is no longer a renderer gap"
    );
}

#[test]
fn source_flow_grammar_survives_rust_dsl_loader_and_snapshot_round_trip() {
    use tcl_registry::events::IrulesSourceFlowSpec;
    for flow in [
        IrulesSourceFlowSpec::DropsConnection,
        IrulesSourceFlowSpec::ReturnsDnsPacket,
        IrulesSourceFlowSpec::EventControl,
    ] {
        let spec = tcl_registry::CommandSpec {
            name: "probe::flow",
            irules_source_flow: Some(flow),
            ..tcl_registry::CommandSpec::DEFAULT
        };
        let before = draft::from_command_spec(&spec);
        let variant = format!("{flow:?}");
        assert_eq!(
            before.get("irules_source_flow").and_then(Value::as_str),
            Some(variant.as_str())
        );
        let rust = crate::render_rs::render(&before);
        assert!(
            rust.contains(&format!(
                "irules_source_flow: Some(tcl_registry::events::IrulesSourceFlowSpec::{variant})"
            )),
            "{rust}"
        );
        let text = render_pack(std::slice::from_ref(&before), "probe");
        assert!(
            text.contains(&format!("irules_source_flow {variant}")),
            "{text}"
        );
        let pack = crate::spectcl::evaluate_pack(&text);
        assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
        let loaded = pack.command("probe::flow").unwrap().spec;
        assert_eq!(loaded.irules_source_flow, Some(flow));
        let after = draft::from_command_spec(loaded);
        assert_eq!(
            after.get("irules_source_flow"),
            before.get("irules_source_flow")
        );
        let invalid = text.replace(
            &format!("irules_source_flow {variant}"),
            "irules_source_flow ImaginedFlow",
        );
        let pack = crate::spectcl::evaluate_pack(&invalid);
        assert!(!pack.notices.is_empty());
        assert_eq!(
            pack.command("probe::flow").unwrap().spec.irules_source_flow,
            None
        );
    }
}

#[test]
fn executable_lookup_scope_survives_rust_and_dsl_round_trip() {
    // Implementation contract: naming.callback.lookup-scope-owner (docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md).
    static MEMBERS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "trigger",
        script_lookup_scope: Some(tcl_registry::ScriptLookupScope::TriggerFrame),
        ..tcl_registry::SubCommand::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe::callback",
        script_lookup_scope: Some(tcl_registry::ScriptLookupScope::GlobalFrame),
        subcommands: MEMBERS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let rust = crate::render_rs::render(&before);
    assert!(
        rust.contains("script_lookup_scope: Some(ScriptLookupScope::GlobalFrame)"),
        "{rust}"
    );
    assert!(
        rust.contains("script_lookup_scope: Some(ScriptLookupScope::TriggerFrame)"),
        "{rust}"
    );
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("script_lookup_scope GlobalFrame"), "{text}");
    assert!(text.contains("script_lookup_scope TriggerFrame"), "{text}");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let after = draft::from_command_spec(pack.command("probe::callback").unwrap().spec);
    assert_eq!(
        after.get("script_lookup_scope"),
        before.get("script_lookup_scope")
    );
    assert_eq!(after.get("subcommands"), before.get("subcommands"));
    let invalid = text.replace(
        "script_lookup_scope GlobalFrame",
        "script_lookup_scope ImaginedFrame",
    );
    let pack = crate::spectcl::evaluate_pack(&invalid);
    assert!(!pack.notices.is_empty());
    assert_eq!(
        pack.command("probe::callback")
            .unwrap()
            .spec
            .script_lookup_scope,
        None
    );
}

#[test]
fn option_script_timings_survive_render_load_draft() {
    static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
        tcl_registry::hover::OptionSpec {
            name: "-command",
            value: tcl_registry::hover::OptionValue::deferred_script(),
            detail: "Run after activation.",
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-remove",
            value: tcl_registry::hover::OptionValue::Takes(tcl_registry::hover::OptionArg {
                role: tcl_registry::ArgRole::CommandPrefix,
                script_timing: tcl_registry::hover::ScriptTiming::ReferenceOnly,
                hint: "prefix",
                ..tcl_registry::hover::OptionArg::DEFAULT
            }),
            detail: "Match a prior registration without invoking it.",
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
    ];
    let spec = tcl_registry::spec::CommandSpec {
        name: "probe::button",
        options: OPTIONS,
        ..tcl_registry::spec::CommandSpec::DEFAULT
    };

    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("-script-timing Deferred"), "{text}");
    assert!(text.contains("-script-timing ReferenceOnly"), "{text}");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let reloaded = pack.command("probe::button").expect("the command reloads");
    let after = draft::from_command_spec(reloaded.spec);
    assert_eq!(after.get("options"), before.get("options"), "{text}");
}

#[test]
fn callback_taint_inputs_survive_option_and_positional_round_trip() {
    static VALIDATION: &[tcl_registry::CallbackTaintInput] = &[
        tcl_registry::CallbackTaintInput::TK_PROPOSED_VALUE,
        tcl_registry::CallbackTaintInput::TK_EDIT_TEXT,
    ];
    static EVENT: &[tcl_registry::CallbackTaintInput] = &[
        tcl_registry::CallbackTaintInput::TK_EVENT_CHAR,
        tcl_registry::CallbackTaintInput::TK_EVENT_KEYSYM,
    ];
    static POSITIONAL: &[(u8, &[tcl_registry::CallbackTaintInput])] = &[(2, EVENT)];
    static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[tcl_registry::hover::OptionSpec {
        name: "-validatecommand",
        value: tcl_registry::hover::OptionValue::deferred_tainted_script(VALIDATION),
        ..tcl_registry::hover::OptionSpec::DEFAULT
    }];
    let spec = tcl_registry::spec::CommandSpec {
        name: "probe::callback",
        traits: tcl_registry::Traits::DEFERS_BODY,
        arity: tcl_registry::Arity::exact(3),
        arg_roles: &[(2, tcl_registry::ArgRole::Body)],
        callback_taint_inputs: POSITIONAL,
        options: OPTIONS,
        ..tcl_registry::spec::CommandSpec::DEFAULT
    };

    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("-callback-taint-inputs {%P %S}"), "{text}");
    assert!(
        text.contains("callback_taint_inputs {{2 {%A %K}}}"),
        "{text}"
    );
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let after = draft::from_command_spec(
        pack.command("probe::callback")
            .expect("the command reloads")
            .spec,
    );
    assert_eq!(
        after.get("callback_taint_inputs"),
        before.get("callback_taint_inputs"),
        "{text}"
    );
    assert_eq!(after.get("options"), before.get("options"), "{text}");
}

#[test]
fn an_option_variable_scope_survives_render_load_draft() {
    static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[tcl_registry::hover::OptionSpec {
        name: "-textvariable",
        value: tcl_registry::hover::OptionValue::user_input_var(),
        detail: "Global linked input.",
        ..tcl_registry::hover::OptionSpec::DEFAULT
    }];
    let spec = tcl_registry::spec::CommandSpec {
        name: "probe::entry",
        options: OPTIONS,
        ..tcl_registry::spec::CommandSpec::DEFAULT
    };

    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("-variable-scope Global"), "{text}");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let reloaded = pack.command("probe::entry").expect("the command reloads");
    let after = draft::from_command_spec(reloaded.spec);
    assert_eq!(after.get("options"), before.get("options"), "{text}");
}

#[test]
fn tk_geometry_preserves_a_custom_container_option() {
    let spec = tcl_registry::spec::CommandSpec {
        name: "probe::layout",
        tk_geometry: Some(tcl_registry::tk_geometry::TkGeometryManagerSpec {
            container_policy: tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive,
            container_option: Some("-inside"),
            direct_form: false,
            placement_subcommand: Some("arrange"),
            release_subcommands: &["release", "unmanage"],
        }),
        ..tcl_registry::spec::CommandSpec::DEFAULT
    };

    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("tk_geometry Exclusive"), "{text}");
    assert!(text.contains("-container-option -inside"), "{text}");
    assert!(text.contains("-placement-subcommand arrange"), "{text}");
    assert!(
        text.contains("-release-subcommands {release unmanage}"),
        "{text}"
    );
    assert!(!text.contains("-container-option -in\n"), "{text}");

    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let reloaded = pack.command("probe::layout").expect("command reloads");
    assert_eq!(
        reloaded
            .spec
            .tk_geometry
            .and_then(|geometry| geometry.container_option),
        Some("-inside")
    );
    let geometry = reloaded.spec.tk_geometry.expect("geometry descriptor");
    assert!(!geometry.direct_form);
    assert_eq!(geometry.placement_subcommand, Some("arrange"));
    assert_eq!(geometry.release_subcommands, ["release", "unmanage"]);
}

#[test]
fn trace_subject_receiver_drafts_reject_invalid_forms_and_positions_without_defaulting() {
    // Implementation contract: naming.variable.trace-source-receiver-purpose (docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md).
    let spec = tcl_registry::CommandSpec {
        name: "probe::trace",
        ..tcl_registry::CommandSpec::DEFAULT
    };
    for rows in [
        serde_json::json!([{"index":1,"form":"Unknown"}]),
        serde_json::json!([{"form":"TraceSubject"}]),
        serde_json::json!([{"index":256,"form":"TraceSubject"}]),
        serde_json::json!([{"index":1,"form":"TraceSubject","extra":true}]),
        serde_json::json!([0,{"index":0,"form":"TraceSubject"}]),
        serde_json::json!(["1"]),
    ] {
        let mut invalid = draft::from_command_spec(&spec);
        invalid.insert("variable_receivers".into(), rows);
        let rust = crate::render_rs::render(&invalid);
        assert!(rust.contains("VARIABLE_RECEIVERS_REQUIRE_UNIQUE_U8_INDICES_AND_VALID_FORMS"));
        let text = render_pack(std::slice::from_ref(&invalid), "probe");
        assert!(text.contains("invalid-receiver-form"));
        let loaded = crate::spectcl::evaluate_pack(&text);
        assert!(!loaded.notices.is_empty());
    }
}

#[test]
// Implementation contract: naming.variable.trace-source-receiver-purpose
// docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
fn trace_subject_receiver_metadata_round_trips_commands_members_and_forms() {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::{Combined, TraceSubject};
    const FORMS: &[tcl_registry::forms::CommandForm] = &[tcl_registry::forms::CommandForm {
        name: "trace-query",
        variable_receivers: Some(&[(1, TraceSubject)]),
        ..tcl_registry::forms::CommandForm::DEFAULT
    }];
    const SUBS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "query",
        variable_receivers: Some(&[(0, TraceSubject)]),
        subcommand_forms: FORMS,
        ..tcl_registry::SubCommand::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe::trace",
        variable_receivers: Some(&[(0, Combined), (1, TraceSubject)]),
        command_forms: FORMS,
        subcommands: SUBS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let seeded = draft::from_command_spec(&spec);
    assert_eq!(
        seeded["variable_receivers"],
        serde_json::json!([0, {"index":1,"form":"TraceSubject"}])
    );
    let rust = crate::render_rs::render(&seeded);
    assert!(
        rust.contains("VariableReceiverOperandForm::TraceSubject"),
        "{rust}"
    );
    let text = render_pack(std::slice::from_ref(&seeded), "probe");
    assert!(
        text.contains("variable_receivers {0 {1 trace-subject}}"),
        "{text}"
    );
    let loaded = crate::spectcl::evaluate_pack(&text);
    let parsed = &loaded.command("probe::trace").unwrap().spec;
    assert_eq!(parsed.variable_receivers, spec.variable_receivers);
    assert_eq!(
        parsed.command_forms[0].variable_receivers,
        spec.command_forms[0].variable_receivers
    );
    assert_eq!(
        parsed.subcommands[0].variable_receivers,
        spec.subcommands[0].variable_receivers
    );
    assert_eq!(
        parsed.subcommands[0].subcommand_forms[0].variable_receivers,
        spec.subcommands[0].subcommand_forms[0].variable_receivers
    );
    assert!(parsed.successful_handler.is_none());
}
#[test]
fn authored_source_path_operations_survive_render_load_and_draft() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    static MEMBERS: &[tcl_registry::SubCommand] = &[
        tcl_registry::SubCommand {
            name: "combine",
            source_path_operation: Some(tcl_registry::SourcePathOperation::Join),
            ..tcl_registry::SubCommand::DEFAULT
        },
        tcl_registry::SubCommand {
            name: "filename",
            source_path_operation: Some(tcl_registry::SourcePathOperation::ScriptPath),
            ..tcl_registry::SubCommand::DEFAULT
        },
    ];
    let spec = tcl_registry::CommandSpec {
        name: "probe::paths",
        subcommands: MEMBERS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("source_path_operation Join"), "{text}");
    assert!(text.contains("source_path_operation ScriptPath"), "{text}");
    let rust = crate::render_rs::render(&before);
    assert!(rust.contains("SourcePathOperation::Join"), "{rust}");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let after = draft::from_command_spec(pack.command("probe::paths").unwrap().spec);
    assert_eq!(
        after.get("subcommands"),
        before.get("subcommands"),
        "{text}"
    );
    let invalid = text.replace(
        "source_path_operation Join",
        "source_path_operation Unknown",
    );
    let invalid_pack = crate::spectcl::evaluate_pack(&invalid);
    assert!(!invalid_pack.notices.is_empty());
    assert_eq!(
        invalid_pack
            .command("probe::paths")
            .unwrap()
            .spec
            .subcommand("combine")
            .unwrap()
            .source_path_operation,
        None,
    );
}
#[test]
#[allow(clippy::too_many_lines)]
fn authored_source_configuration_reference_survives_render_load_and_draft() {
    use tcl_registry::InvocationWord::{Dynamic, Literal};
    use tcl_registry::{
        AuthoredSourceConfigurationReference, CommandRegistry, CommandSpec,
        ConfigurationObjectFamily, InvocationWord, InvocationWords,
        SourceConfigurationReferenceLayout, SubCommand,
    };
    static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[tcl_registry::hover::OptionSpec {
        name: "--",
        ..tcl_registry::hover::OptionSpec::DEFAULT
    }];
    static MEMBERS: &[SubCommand] = &[
        SubCommand {
            name: "compare",
            options: OPTIONS,
            source_configuration_reference: Some(
                SourceConfigurationReferenceLayout::DataGroupComparison,
            ),
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "plain",
            ..SubCommand::DEFAULT
        },
    ];
    let spec = CommandSpec {
        name: "probe::references",
        subcommands: MEMBERS,
        ..CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    assert!(crate::schema::command_field("source_configuration_reference").is_none());
    assert!(before["subcommands"][1]["source_configuration_reference"].is_null());
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(
        text.contains("source_configuration_reference DataGroupComparison"),
        "{text}"
    );
    let rust = crate::render_rs::render(&before);
    assert!(
        rust.contains("Some(SourceConfigurationReferenceLayout::DataGroupComparison)"),
        "{rust}"
    );
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let reloaded = pack.command("probe::references").unwrap().spec;
    assert_eq!(draft::from_command_spec(reloaded), before, "{text}");

    let mut original_registry = CommandRegistry::build_default();
    original_registry.insert(spec);
    let mut reloaded_registry = CommandRegistry::build_default();
    reloaded_registry.insert(reloaded.clone());
    let project = |registry: &CommandRegistry, args: &[InvocationWord<'_>]| {
        registry
            .resolve_structured_invocation(
                InvocationWords::structured(Literal("probe::references"), args),
                None,
            )
            .resolved()
            .unwrap()
            .authored_source_configuration_reference_operand()
    };
    for (args, argument, selection_unknown) in [
        (
            vec![
                Literal("compare"),
                Dynamic,
                Literal("equals"),
                Literal("dg"),
            ],
            3,
            true,
        ),
        (
            vec![
                Literal("compare"),
                Literal("--"),
                Dynamic,
                Literal("equals"),
                Literal("dg"),
            ],
            4,
            false,
        ),
    ] {
        let expected = Some(AuthoredSourceConfigurationReference {
            argument,
            comparison_item: Some(argument - 2),
            family: ConfigurationObjectFamily::DataGroup,
            selection_unknown,
        });
        assert_eq!(project(&original_registry, &args), expected);
        assert_eq!(project(&reloaded_registry, &args), expected);
    }
    for args in [
        [Literal("plain"), Dynamic, Literal("equals"), Literal("dg")],
        [
            Literal("compare"),
            Dynamic,
            Literal("unknown"),
            Literal("dg"),
        ],
    ] {
        assert_eq!(project(&original_registry, &args), None);
        assert_eq!(project(&reloaded_registry, &args), None);
    }
    let invalid = text.replace(
        "source_configuration_reference DataGroupComparison",
        "source_configuration_reference Unknown",
    );
    let invalid_pack = crate::spectcl::evaluate_pack(&invalid);
    assert!(!invalid_pack.notices.is_empty());
    assert!(
        invalid_pack
            .command("probe::references")
            .unwrap()
            .spec
            .subcommands[0]
            .source_configuration_reference
            .is_none()
    );
}

#[test]
fn authored_channel_configuration_survives_render_load_and_draft() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    static MEMBERS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "configure",
        channel_configuration: Some(tcl_registry::ChannelConfigurationSpec::EncodingTranslation),
        ..tcl_registry::SubCommand::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe::configure",
        channel_configuration: Some(tcl_registry::ChannelConfigurationSpec::EncodingTranslation),
        subcommands: MEMBERS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert_eq!(
        text.matches("channel_configuration EncodingTranslation")
            .count(),
        2,
        "{text}"
    );
    let rust = crate::render_rs::render(&before);
    assert_eq!(
        rust.matches("ChannelConfigurationSpec::EncodingTranslation")
            .count(),
        2,
        "{rust}"
    );
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let parsed = pack.command("probe::configure").unwrap().spec;
    let after = draft::from_command_spec(parsed);
    assert_eq!(
        after.get("channel_configuration"),
        before.get("channel_configuration")
    );
    assert_eq!(after.get("subcommands"), before.get("subcommands"));
    assert!(parsed.successful_handler.is_none());
    assert!(parsed.subcommands[0].successful_handler.is_none());
    let invalid = crate::spectcl::evaluate_pack(&text.replace(
        "channel_configuration EncodingTranslation",
        "channel_configuration Unknown",
    ));
    assert!(!invalid.notices.is_empty());
    let rejected = invalid.command("probe::configure").unwrap().spec;
    assert!(rejected.channel_configuration.is_none());
    assert!(rejected.subcommands[0].channel_configuration.is_none());
}

#[test]
fn authored_source_index_bounds_survive_render_load_and_draft() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    static MEMBERS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "insert",
        source_index_bounds: Some(tcl_registry::SourceIndexBounds::StringInsert),
        ..tcl_registry::SubCommand::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe::indices",
        source_index_bounds: Some(tcl_registry::SourceIndexBounds::ListIndex),
        subcommands: MEMBERS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert!(text.contains("source_index_bounds ListIndex"), "{text}");
    assert!(text.contains("source_index_bounds StringInsert"), "{text}");
    let rust = crate::render_rs::render(&before);
    assert!(rust.contains("SourceIndexBounds::ListIndex"), "{rust}");
    assert!(rust.contains("SourceIndexBounds::StringInsert"), "{rust}");
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let parsed = pack.command("probe::indices").unwrap().spec;
    let after = draft::from_command_spec(parsed);
    assert_eq!(
        after.get("source_index_bounds"),
        before.get("source_index_bounds")
    );
    assert_eq!(after.get("subcommands"), before.get("subcommands"));
    assert!(parsed.successful_handler.is_none());
    assert!(parsed.subcommands[0].successful_handler.is_none());
    let invalid = crate::spectcl::evaluate_pack(&text.replace(
        "source_index_bounds StringInsert",
        "source_index_bounds Unknown",
    ));
    assert!(!invalid.notices.is_empty());
    assert_eq!(
        invalid.command("probe::indices").unwrap().spec.subcommands[0].source_index_bounds,
        None
    );
}

#[test]
fn command_closed_value_prefix_policy_round_trips_both_studio_renderers() {
    use tcl_registry::hover::ArgValue;
    static VALUES: &[(u8, &[ArgValue])] = &[(
        0,
        &[ArgValue {
            value: "script",
            ..ArgValue::DEFAULT
        }],
    )];
    for accepts_prefix in [false, true] {
        let spec = tcl_registry::CommandSpec {
            name: "prefix_probe",
            arg_values: VALUES,
            closed_value_args: &[0],
            arg_values_accept_prefix: accepts_prefix,
            ..tcl_registry::CommandSpec::DEFAULT
        };
        let before = draft::from_command_spec(&spec);
        assert_eq!(
            before["arg_values_accept_prefix"],
            serde_json::json!(accepts_prefix)
        );
        let rust_source = crate::render_rs::render(&before);
        if accepts_prefix {
            assert!(rust_source.contains("arg_values_accept_prefix: true"));
        }
        let text = render_pack(std::slice::from_ref(&before), "prefix_probe_pack");
        let loaded = crate::spectcl::evaluate_pack(&text);
        assert!(loaded.notices.is_empty(), "{:?}\n{text}", loaded.notices);
        let command = loaded.command("prefix_probe").unwrap();
        assert_eq!(command.spec.arg_values_accept_prefix, accepts_prefix);
        assert_eq!(
            draft::from_command_spec(command.spec)["arg_values_accept_prefix"],
            before["arg_values_accept_prefix"]
        );
    }
}

#[test]
fn caller_alias_transition_survives_native_draft_both_renderers_and_reload() {
    use tcl_registry::CommandTableEffect;
    static MEMBERS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "alias",
        command_table_effect: Some(CommandTableEffect::CreatesCallerAliases),
        ..tcl_registry::SubCommand::DEFAULT
    }];
    let spec = tcl_registry::CommandSpec {
        name: "probe::aliases",
        command_table_effect: Some(CommandTableEffect::CreatesCallerAliases),
        subcommands: MEMBERS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let before = draft::from_command_spec(&spec);
    let text = render_pack(std::slice::from_ref(&before), "probe");
    assert_eq!(
        text.matches("command_table_effect CreatesCallerAliases")
            .count(),
        2,
        "{text}"
    );
    let rust = crate::render_rs::render(&before);
    assert_eq!(
        rust.matches("CommandTableEffect::CreatesCallerAliases")
            .count(),
        2,
        "{rust}"
    );
    let pack = crate::spectcl::evaluate_pack(&text);
    assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
    let parsed = pack.command("probe::aliases").unwrap().spec;
    assert_eq!(parsed.command_table_effect, spec.command_table_effect);
    assert_eq!(
        parsed.subcommands[0].command_table_effect,
        MEMBERS[0].command_table_effect
    );
    let after = draft::from_command_spec(parsed);
    assert_eq!(
        after.get("command_table_effect"),
        before.get("command_table_effect")
    );
    assert_eq!(after.get("subcommands"), before.get("subcommands"));
    let invalid = crate::spectcl::evaluate_pack(&text.replace(
        "command_table_effect CreatesCallerAliases",
        "command_table_effect Unknown",
    ));
    assert!(!invalid.notices.is_empty());
    let rejected = invalid.command("probe::aliases").unwrap().spec;
    assert!(rejected.command_table_effect.is_none());
    assert!(rejected.subcommands[0].command_table_effect.is_none());
}
