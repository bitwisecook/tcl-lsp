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

use std::collections::BTreeMap;

use super::*;
use tcl_dialect::model::SpecSurface;

#[test]
fn original_abstract_grammar_rust_render_keeps_the_shared_named_descriptor() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    let registry = tcl_registry::CommandRegistry::build_default();
    let seeded = draft::from_command_spec(registry.get("oo::abstract").unwrap());
    let rendered = render(&seeded);
    assert!(rendered.contains("definition_body: Some(&crate::definer::TCLOO_ABSTRACT_GRAMMAR)"));
    assert!(rendered.contains("manufacturer_methods:"));
    assert_eq!(
        definition_body_expr(&serde_json::json!("tcloo-abstract"), ""),
        Some("Some(&crate::definer::TCLOO_ABSTRACT_GRAMMAR)".into())
    );
}

/// A draft that is nothing but a name — all these tests read is the stem.
fn named_draft(name: &str) -> (String, Draft) {
    let mut draft = draft::default_command_draft();
    draft.insert("name".to_owned(), serde_json::json!(name));
    (name.to_owned(), draft)
}

fn invalid_non_write_taint_draft() -> Draft {
    let registry = tcl_registry::CommandRegistry::build_default();
    let entry = registry.get("entry").expect("Tk entry spec");
    let seeded = draft::from_command_spec(entry);
    let mut option = seeded["options"]
        .as_array()
        .expect("entry options")
        .iter()
        .find(|option| as_str(&option["name"]) == "-textvariable")
        .expect("entry -textvariable")
        .clone();
    option["value"]["role"] = serde_json::json!("Value");
    option["value"]["also_role"] = Value::Null;
    option["value"]["taints_var_write"] = serde_json::json!(true);
    let mut draft = draft::default_command_draft();
    draft.insert("name".into(), serde_json::json!("probe"));
    draft.insert("options".into(), Value::Array(vec![option]));
    draft
}

/// A Tcl body is rendered as the `const` call that builds it, and its author's
/// `-evaluate` as the `.evaluated()` that carries the assertion; a spelling that
/// does not read renders nothing.
#[test]
fn a_tcl_body_backing_is_rendered_with_its_authors_assertion() {
    let rendered = |spelling: &str| runtime_backing_expr(&serde_json::json!(spelling));
    assert_eq!(rendered("none").as_deref(), Some("RuntimeBacking::None"));
    assert_eq!(
        rendered("tcl-body {-pack-text {proc p {} {}}}").as_deref(),
        Some("RuntimeBacking::pack_text(\"proc p {} {}\")")
    );
    assert_eq!(
        rendered("tcl-body {-pack-text {proc p {} {}} -evaluate}").as_deref(),
        Some("RuntimeBacking::pack_text(\"proc p {} {}\").evaluated()")
    );
    assert_eq!(
        rendered("tcl-body {-package-source lib/a.tcl}").as_deref(),
        Some("RuntimeBacking::package_source(\"lib/a.tcl\")")
    );
    assert_eq!(
        rendered("tcl-body {-evaluate -package-source lib/a.tcl}").as_deref(),
        Some("RuntimeBacking::package_source(\"lib/a.tcl\").evaluated()")
    );
    assert_eq!(rendered("tcl-body {-evaluate}"), None);
}

#[test]
fn invalid_taint_link_is_not_rendered_without_var_write_role() {
    let out = render(&invalid_non_write_taint_draft());
    assert!(!out.contains("taints_var_write: true"), "{out}");
}

/// `return`'s `-errorstack` is the registry's real `OptionArity::Hook`.
///
/// Seeding cannot recover `errorstack_value` from a function pointer, so
/// the arity seeds with an empty `hook` slot, the note names that one
/// option rather than the whole `options` field, and supplying the
/// expression both renders it and clears the note.
#[test]
fn option_arity_hook_round_trips_once_supplied() {
    let reg = tcl_registry::registry::CommandRegistry::build_default();
    let spec = reg.get("return").expect("return is a core command");
    let mut d = draft::from_command_spec(spec);

    // Seeded: the hook slot exists and is empty.
    let opts = d["options"].as_array().expect("options array").clone();
    let hooked: Vec<&str> = opts
        .iter()
        .filter(|o| as_str(&o["value"]["arity"]["kind"]) == "Hook")
        .map(|o| as_str(&o["name"]))
        .collect();
    assert_eq!(
        hooked,
        vec!["-errorstack"],
        "one option consumes via a hook"
    );

    // Unfilled: the TODO names the option, not the whole field.
    let out = render(&d);
    assert!(
        out.contains("`-errorstack` consumes a computed number of words"),
        "note must name the option, got:\n{out}"
    );
    assert!(
        !out.contains("`options` is set on the source command"),
        "the whole options field must not be reported unreadable:\n{out}"
    );

    // Supplied: it renders as a real arity and the note is gone.
    let mut filled = opts;
    for opt in &mut filled {
        if as_str(&opt["value"]["arity"]["kind"]) == "Hook" {
            opt["value"]["arity"]["hook"] = json!("errorstack_value");
        }
    }
    d.insert("options".into(), Value::Array(filled));
    let out = render(&d);
    assert!(
        out.contains("arity: OptionArity::Hook(errorstack_value),"),
        "supplied hook must render:\n{out}"
    );
    assert!(
        !out.contains("consumes a computed number of words"),
        "note must clear once supplied:\n{out}"
    );
}

use serde_json::json;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::arity::Arity;
use tcl_registry::deprecation::{
    DeprecationFixContext, DeprecationFixHook, DeprecationFixPlan, DeprecationFixSafety,
};
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::representation::RepresentationEffect;
use tcl_registry::spec::{CommandSpec, OptionRelation, SubCommand};
use tcl_registry::traits::Traits;
use tcl_registry::types::TclType;

#[test]
fn renders_the_copyright_banner_and_spec_shell() {
    let mut draft = draft::default_command_draft();
    draft.insert("name".into(), json!("mycmd"));
    let out = render(&draft);
    assert!(out.starts_with("// tcl-lsp — a language server and toolchain for Tcl\n"));
    assert!(out.contains("// SPDX-License-Identifier: AGPL-3.0-or-later"));
    assert!(out.contains("use crate::prelude::*;"));
    assert!(out.contains("pub fn spec() -> CommandSpec {"));
    assert!(out.contains("name: \"mycmd\","));
    assert!(out.contains("..CommandSpec::DEFAULT"));
}

#[test]
fn omits_fields_left_at_their_default() {
    let mut d = draft::default_command_draft();
    d.insert("name".into(), json!("mycmd"));
    let out = render(&d);
    assert!(!out.contains("warn_missing_import"));
    assert!(!out.contains("body_kind"));
    assert!(!out.contains("unsafe_command"));
}

#[test]
fn round_trips_a_real_registry_spec() {
    let spec = CommandSpec {
        name: "lappend",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::BYTE_COMPILED | Traits::FIRST_ARG_VARNAME,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::List),
        ..CommandSpec::DEFAULT
    };
    let out = render(&draft::from_command_spec(&spec));
    assert!(out.contains("name: \"lappend\","));
    assert!(out.contains("arity: Arity::at_least(1),"));
    assert!(out.contains("arg_roles: &[(0, ArgRole::VarWrite)],"));
    assert!(out.contains("assigns_variable_at: Some(0),"));
    assert!(out.contains("return_type: Some(TclType::List),"));
    assert!(
        out.contains("traits: Traits::BYTE_COMPILED.union(Traits::FIRST_ARG_VARNAME),"),
        "a `|` chain is not const-evaluable inside the hoisted tables:\n{out}"
    );
    assert!(
        out.contains("surface![SpecSurface::ALL_TCL, SpecSurface::IRULES]"),
        "dialect aggregates should read like the hand-written specs:\n{out}"
    );
}

#[test]
fn renders_option_timing_and_object_method_prefix_policy() {
    static VALIDATION: &[tcl_registry::CallbackTaintInput] =
        &[tcl_registry::CallbackTaintInput::TK_PROPOSED_VALUE];
    static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
        tcl_registry::hover::OptionSpec {
            name: "-command",
            value: tcl_registry::hover::OptionValue::deferred_tainted_script(VALIDATION),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-textvariable",
            value: tcl_registry::hover::OptionValue::user_input_var(),
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
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
    ];
    static CLASS: tcl_registry::spec::ObjectClassSpec = tcl_registry::spec::ObjectClassSpec {
        class_name: "probe",
        instance_methods: &[],
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Enabled,
    };
    let spec = CommandSpec {
        name: "probe",
        options: OPTIONS,
        object_class: Some(&CLASS),
        ..CommandSpec::DEFAULT
    };

    let out = render(&draft::from_command_spec(&spec));
    assert!(
        out.contains("script_timing: ScriptTiming::Deferred,"),
        "{out}"
    );
    assert!(
        out.contains("script_timing: ScriptTiming::ReferenceOnly,"),
        "{out}"
    );
    assert!(
        out.contains("callback_taint_inputs: &[CallbackTaintInput::TK_PROPOSED_VALUE],"),
        "{out}"
    );
    assert!(
        out.contains("variable_scope: VariableScope::Global,"),
        "{out}"
    );
    assert!(
        out.contains("method_prefix_matching: PrefixMatching::Enabled,"),
        "{out}"
    );
}

#[test]
fn notes_a_field_it_could_not_recover() {
    fn resolver(_args: &[&str]) -> Vec<(u8, ArgRole)> {
        Vec::new()
    }
    let spec = CommandSpec {
        name: "if",
        arg_role_resolver: Some(resolver),
        ..CommandSpec::DEFAULT
    };
    let out = render(&draft::from_command_spec(&spec));
    assert!(out.contains("// TODO: `arg_role_resolver` is set on the source command"));
}

#[test]
fn a_supplied_rust_expression_is_emitted_verbatim() {
    let mut d = draft::default_command_draft();
    d.insert("name".into(), json!("if"));
    d.insert("arg_role_resolver".into(), json!("resolve_if_roles"));
    d.insert(
        "script_timing_resolver".into(),
        json!("Some(resolve_if_timing)"),
    );
    d.insert(
        "event_requirement_forms".into(),
        json!("&[FIX_TAG_GET_FORM]"),
    );
    d.insert("data_collection".into(), json!("Some(HTTP_COLLECT)"));
    d.insert(
        "side_switch_target".into(),
        json!("Some(SideSwitchTarget::Client)"),
    );
    d.insert(
        "event_handler_priority".into(),
        json!("Some(BIGIP_EVENT_HANDLER_PRIORITY)"),
    );
    let out = render(&d);
    assert!(out.contains("arg_role_resolver: resolve_if_roles,"));
    assert!(
        out.contains("script_timing_resolver: Some(resolve_if_timing),"),
        "{out}"
    );
    assert!(out.contains("event_requirement_forms: &[FIX_TAG_GET_FORM],"));
    assert!(out.contains("data_collection: Some(HTTP_COLLECT),"));
    assert!(out.contains("side_switch_target: Some(SideSwitchTarget::Client),"));
    assert!(out.contains("event_handler_priority: Some(BIGIP_EVENT_HANDLER_PRIORITY),"));
}

#[test]
fn renders_tk_geometry_from_the_structured_draft() {
    let mut draft = draft::default_command_draft();
    draft.insert("name".into(), json!("layout"));
    draft.insert(
        "tk_geometry".into(),
        json!({
            "container_policy": "Independent",
            "container_option": "-inside",
            "direct_form": false,
            "placement_subcommand": "arrange",
            "release_subcommands": ["release", "unmanage"],
        }),
    );

    let out = render(&draft);
    assert!(
        out.contains("tk_geometry: Some(TkGeometryManagerSpec {"),
        "{out}"
    );
    assert!(
        out.contains("container_policy: TkGeometryContainerPolicy::Independent"),
        "{out}"
    );
    assert!(out.contains("container_option: Some(\"-inside\")"), "{out}");
    assert!(out.contains("direct_form: false"), "{out}");
    assert!(
        out.contains("placement_subcommand: Some(\"arrange\")"),
        "{out}"
    );
    assert!(
        out.contains("release_subcommands: &[\"release\", \"unmanage\"]"),
        "{out}"
    );
}

/// Every dialect the catalogue offers must have a Rust constant to
/// render, or a spec naming it produces a file that will not compile.
#[test]
fn every_catalogued_dialect_renders_to_a_constant() {
    for entry in crate::catalogue::DIALECTS.iter() {
        assert!(
            dialect_constant(entry.key).is_some(),
            "no SpecSurface constant for the catalogued dialect {}",
            entry.key
        );
    }
}

#[test]
fn a_stepped_arity_uses_the_three_argument_constructor() {
    // `stepped` is an associated function, so a builder-style chain off
    // `at_least` would not compile.
    let spec = CommandSpec {
        name: "switch",
        arity: Arity::stepped(3, Arity::UNLIMITED, 2).with_also_exact(2),
        ..CommandSpec::DEFAULT
    };
    let out = render(&draft::from_command_spec(&spec));
    assert!(
        out.contains("arity: Arity::stepped(3, Arity::UNLIMITED, 2).with_also_exact(2),"),
        "{out}"
    );
}

#[test]
fn a_multiline_summary_stays_inside_the_doc_comment() {
    // The Summary field is a textarea, so a newline with no ". " is
    // reachable; a bare continuation line would not compile.
    let mut d = draft::default_command_draft();
    d.insert("name".into(), json!("mycmd"));
    d.insert(
        "hover".into(),
        json!({
            "summary": "does a thing\nand keeps going",
            "synopsis": [],
            "snippet": "",
            "source": "",
            "examples": "",
            "return_value": "",
        }),
    );
    let out = render(&d);
    for line in out.lines().take_while(|l| !l.starts_with("use ")) {
        assert!(
            line.is_empty() || line.starts_with("//"),
            "every header line must be a comment, got: {line:?}\n{out}"
        );
    }
    assert!(
        out.contains("//! `mycmd` — does a thing\n//! and keeps going.\n"),
        "{out}"
    );
}

/// The three release keys collapse back into the one `lifecycle` field the
/// registry declares — at the command level and inside a subcommand.
///
/// Emitting the schema keys verbatim produced `introduced_version:
/// Some("21.1.0"),` inside a `SubCommand` literal, which is not a field:
/// the studio's advertised drop-in file did not compile. Found by
/// rendering `persist` back into `tcl-registry` and building it, per the
/// procedure in `tests/render_sweep.rs`.
#[test]
fn a_lifecycle_renders_as_one_field_not_three_keys() {
    const SUBS: &[SubCommand] = &[SubCommand {
        name: "add",
        lifecycle: tcl_registry::lifecycle::Lifecycle::introduced_in("21.1.0"),
        ..SubCommand::DEFAULT
    }];
    let spec = CommandSpec {
        name: "persist",
        lifecycle: tcl_registry::lifecycle::Lifecycle::introduced_in("11.0.0")
            .deprecated_from("17.0.0")
            .retired_from("21.0.0"),
        subcommands: SUBS,
        ..CommandSpec::DEFAULT
    };
    let out = render(&draft::from_command_spec(&spec));
    assert!(
        out.contains(
            "lifecycle: Lifecycle::introduced_in(\"11.0.0\")\
                 .deprecated_from(\"17.0.0\").retired_from(\"21.0.0\"),"
        ),
        "{out}"
    );
    assert!(
        out.contains("lifecycle: Lifecycle::introduced_in(\"21.1.0\"),"),
        "a subcommand's lifecycle must collapse too:\n{out}"
    );
    for key in crate::coverage::LIFECYCLE_KEYS {
        assert!(
            !out.contains(&format!("{key}:")),
            "`{key}` is a form key, not a struct field — it must never reach the spec:\n{out}"
        );
    }
}

/// A command with no lifecycle emits nothing, leaving it to `..DEFAULT`.
#[test]
fn an_unspecified_lifecycle_emits_no_field() {
    let mut d = draft::default_command_draft();
    d.insert("name".into(), json!("mycmd"));
    assert!(!render(&d).contains("lifecycle"));
}

#[test]
fn registry_owned_diagnostic_and_representation_metadata_round_trips() {
    const CONSTRAINTS: &[OptionRelation] = &[OptionRelation {
        terms: &[
            tcl_registry::OptionTerm::Option("-encoding"),
            tcl_registry::OptionTerm::Option("-nopkg"),
        ],
        ..OptionRelation::DEFAULT
    }];
    let spec = CommandSpec {
        name: "source",
        representation_effect: Some(RepresentationEffect::copy_on_write_container(0, 2)),
        option_relations: CONSTRAINTS,
        lifecycle: Lifecycle {
            introduced: None,
            deprecated: Some("8.6"),
            retired: None,
            deprecation_fix: Some(DeprecationFixHook::ReplaceMatchedWord {
                replacement: "children",
                description: "Use interp children",
                safety: DeprecationFixSafety::SemanticsEquivalent,
            }),
        },
        ..CommandSpec::DEFAULT
    };
    let out = render(&draft::from_command_spec(&spec));
    assert!(out.contains(
            "representation_effect: Some(RepresentationEffect::CopyOnWriteContainerMutation { variable_arg: 0, minimum_arguments: 2 }),"
        ));
    assert!(
        out.contains(
            "option_relations: &[OptionRelation { kind: RelationKind::MutuallyExclusive, \
                 mode: RelationMode::Assert, subject: None, \
                 terms: &[OptionTerm::Option(\"-encoding\"), \
                 OptionTerm::Option(\"-nopkg\")], surface: None, \
                 lifecycle: Lifecycle { introduced: None, deprecated: None, \
                 retired: None, deprecation_fix: None }, message: None }],"
        ),
        "{out}"
    );
    assert!(out.contains(
            "deprecation_fix: Some(DeprecationFixHook::ReplaceMatchedWord { replacement: \"children\", description: \"Use interp children\", safety: DeprecationFixSafety::SemanticsEquivalent })"
        ));
}

fn contextual_fix(_context: DeprecationFixContext<'_>) -> Option<DeprecationFixPlan> {
    None
}

#[test]
fn contextual_deprecation_hook_is_never_silently_dropped() {
    let spec = CommandSpec {
        name: "legacy",
        lifecycle: Lifecycle {
            introduced: None,
            deprecated: Some("9.0"),
            retired: None,
            deprecation_fix: Some(DeprecationFixHook::Custom {
                resolver: contextual_fix,
            }),
        },
        ..CommandSpec::DEFAULT
    };
    let draft = draft::from_command_spec(&spec);
    assert!(
        draft[crate::draft::UNRENDERABLE_KEY]
            .as_array()
            .is_some_and(|keys| keys.iter().any(|key| key == "deprecation_fix"))
    );
    let out = render(&draft);
    assert!(
        out.contains("TODO: `deprecation_fix` is set on the source command"),
        "{out}"
    );
}

#[test]
fn strings_are_escaped() {
    assert_eq!(rust_string("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
}

#[test]
fn suggested_path_handles_keywords_and_namespaces() {
    assert_eq!(
        suggested_path("lappend", "tcl"),
        "rust/tcl-registry/src/commands/tcl/lappend.rs"
    );
    assert_eq!(
        suggested_path("if", "tcl"),
        "rust/tcl-registry/src/commands/tcl/if_.rs"
    );
    assert_eq!(
        suggested_path("HTTP::header", "irules"),
        "rust/tcl-registry/src/commands/irules/http__header.rs"
    );
    // A leading `::` must not leave the module name starting with `_`, and
    // a run of punctuation that is not a namespace separator collapses to
    // one.
    assert_eq!(
        suggested_path("::tcl::dict::lappend", "tcl"),
        "rust/tcl-registry/src/commands/tcl/tcl__dict__lappend.rs"
    );
    assert_eq!(
        suggested_path("tcl::mathop::+", "tcl"),
        "rust/tcl-registry/src/commands/tcl/tcl__mathop.rs"
    );
}

/// The registry files `IP::ttl` at `ip__ttl.rs` and `ip_ttl` at
/// `ip_ttl.rs`, and ships both commands in `irules`. Collapsing the `::`
/// to a single underscore put them at one path, where `pack_export` wrote
/// one file over the other and the `mod.rs` declared the module once.
#[test]
fn a_namespace_separator_is_a_double_underscore_as_the_registry_spells_it() {
    for (name, stem) in [
        ("IP::ttl", "ip__ttl"),
        ("ip_ttl", "ip_ttl"),
        ("IP::addr", "ip__addr"),
        ("ip_addr", "ip_addr"),
        ("HTTP::header", "http__header"),
        ("AAA::acct_result", "aaa__acct_result"),
        ("access2::access2_proc", "access2__access2_proc"),
    ] {
        assert_eq!(
            suggested_path(name, "irules"),
            format!("rust/tcl-registry/src/commands/irules/{stem}.rs"),
            "{name}"
        );
    }
    assert_ne!(
        suggested_path("IP::ttl", "irules"),
        suggested_path("ip_ttl", "irules"),
    );
}

/// The convention is not a table written here: it is what the registry's
/// thousand-odd command files already do, so the claim is checked against
/// every shipped command.
#[test]
fn every_shipped_namespaced_command_keeps_its_separator() {
    let mut checked = 0_usize;
    for pack in tcl_registry::installed().surfaces() {
        for spec in pack.specs() {
            if !tcl_syntax::naming::unroot_rooted_key(spec.name)
                .unwrap_or(spec.name)
                .contains("::")
            {
                continue;
            }
            assert!(
                suggested_path(spec.name, pack.id).contains("__"),
                "{} lost its namespace separator: {}",
                spec.name,
                suggested_path(spec.name, pack.id)
            );
            checked += 1;
        }
    }
    assert!(checked > 500, "only {checked} namespaced commands checked");
}

/// `irules` ships four pairs that met at one path under the old rule —
/// `IP::addr`/`ip_addr`, `IP::protocol`/`ip_protocol`, `IP::tos`/`ip_tos`,
/// `IP::ttl`/`ip_ttl` — and the registry files each pair in two modules.
/// An export of that pack wrote one of each pair over the other, and the
/// `mod.rs` declared the module once.
///
/// The pack as a whole is the assertion, not the four pairs: no two of its
/// commands may propose one file. (`tcl` and `tcllib` cannot make the same
/// claim — they ship the operator commands, whose names are punctuation
/// and carry no identifier to be distinct in. That residual is what
/// `pack_export` reports.)
#[test]
fn no_two_shipped_irules_commands_propose_the_same_file() {
    let pack = tcl_registry::installed()
        .surface("irules")
        .expect("the irules pack");
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for spec in pack.specs() {
        // One command may ship under both its bare and its fully-qualified
        // spelling, and the registry files those in one module.
        let command = tcl_syntax::naming::unroot_rooted_key(spec.name)
            .unwrap_or(spec.name)
            .to_owned();
        let path = suggested_path(spec.name, pack.id);
        if let Some(other) = seen.insert(path.clone(), command.clone())
            && other != command
        {
            panic!("{other} and {command} both render to {path}");
        }
    }
    assert_eq!(
        suggested_path("IP::ttl", "irules"),
        "rust/tcl-registry/src/commands/irules/ip__ttl.rs"
    );
    assert_eq!(
        suggested_path("ip_ttl", "irules"),
        "rust/tcl-registry/src/commands/irules/ip_ttl.rs"
    );
}

/// The generated file lands *inside* `tcl-registry`, which does not alias
/// itself: `use tcl_registry::CommandSpec;` there is `E0432`.
#[test]
fn the_pack_module_imports_the_spec_from_the_crate_it_is_written_into() {
    let module = render_pack_module(&[named_draft("greet")], "mylib", ModuleForm::Whole);
    assert!(
        !module.contains("use tcl_registry::"),
        "the module names the crate it is written into: {module}"
    );
    assert!(module.contains("use crate::spec::CommandSpec;"), "{module}");
}

/// A stem can now carry a `::`'s double underscore, which is exactly the
/// lint every hand-written pack suppresses.
#[test]
fn the_pack_module_allows_the_registry_module_naming() {
    let module = render_pack_module(&[named_draft("HTTP::header")], "mylib", ModuleForm::Whole);
    let allow = module.find("#![allow(non_snake_case)]").expect("the allow");
    let first_mod = module.find("\nmod ").expect("a mod line");
    assert!(allow < first_mod, "the inner attribute must lead: {module}");
    assert!(module.contains("mod http__header;"), "{module}");
}

/// A shipped pack's `mod.rs` already declares hundreds of commands, so the
/// whole-file render is a drop-in that deletes them.
#[test]
fn a_shipped_pack_renders_as_an_addition_rather_than_a_whole_file() {
    let module = render_pack_module(&[named_draft("greet")], "tcl", ModuleForm::Addition);
    assert!(module.contains("NOT A DROP-IN FILE"), "{module}");
    assert!(!module.contains("pub fn tcl_command_specs()"), "{module}");
    assert!(module.contains("mod greet;"), "{module}");
    assert!(module.contains("greet::spec(),"), "{module}");
    assert!(module.contains("tcl_command_specs()"), "{module}");
}

/// A `speclib` name is free text, and the directory is seeded from it, so
/// `my-lib` must not reach the output as `my-lib_command_specs`.
#[test]
fn the_collector_is_an_identifier_whatever_the_directory_is_called() {
    let module = render_pack_module(&[named_draft("greet")], "my-lib", ModuleForm::Whole);
    assert!(module.contains("pub fn my_lib_command_specs()"), "{module}");
}

#[test]
fn an_enum_edited_as_an_expression_keeps_its_type_path() {
    let spec = CommandSpec {
        name: "lappend",
        var_elements_effect: Some(tcl_registry::types::VarElementsEffect::SetsDictValue),
        ..CommandSpec::DEFAULT
    };
    let out = render(&draft::from_command_spec(&spec));
    assert!(
        out.contains("var_elements_effect: Some(VarElementsEffect::SetsDictValue),"),
        "the field is an Option, so the expression carries its own Some:\n{out}"
    );
}
