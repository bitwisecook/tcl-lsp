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
use crate::{draft, render_rs, schema};
use serde_json::{Map, Value, json};
use tcl_core_types::DiagCode;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::MemberVisibility;
use tcl_registry::handle_binding::{HandleClassSource, HandleName};
use tcl_registry::hover::{FormKind, OptionArity, OptionValue};
use tcl_registry::side_effects::{ConnectionSide, SideEffectTarget};
use tcl_registry::symbol_def::DefinedSymbolKind;

/// The draft keys a coverage entry claims; empty for an exclusion.
fn claimed(surface: Surface) -> Vec<&'static str> {
    match surface {
        Surface::Key(k) | Surface::Expression(k) => vec![k],
        Surface::Keys(ks) => ks.to_vec(),
        Surface::Excluded(_) => Vec::new(),
    }
}

/// The keys of a JSON object, or a panic naming `what` when it is not one.
fn object_keys<'a>(what: &str, value: &'a Value) -> Vec<&'a str> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{what} seeds a JSON object"))
        .keys()
        .map(String::as_str)
        .collect()
}

/// Every `Key` / `Keys` entry names a key the seeded draft really holds.
fn assert_carried(what: &str, table: &[Field], keys: &[&str]) {
    for field in table {
        if matches!(field.surface, Surface::Excluded(_) | Surface::Expression(_)) {
            continue;
        }
        for key in claimed(field.surface) {
            assert!(
                keys.contains(&key),
                "{what}::{} claims draft key `{key}`, which the seeder never writes",
                field.name
            );
        }
    }
}

/// Every `Key` / `Keys` entry names a schema field, so the browser renders
/// an editor for it and `render_rs` emits it.
fn assert_schema_has(
    what: &str,
    table: &[Field],
    lookup: fn(&str) -> Option<&'static schema::FieldSchema>,
) {
    for field in table {
        if matches!(field.surface, Surface::Excluded(_)) {
            continue;
        }
        for key in claimed(field.surface) {
            assert!(
                lookup(key).is_some(),
                "{what}::{} claims schema key `{key}`, which schema.rs does not declare",
                field.name
            );
        }
    }
}

/// No schema field without a coverage entry — catches a schema key left
/// behind by a registry field that has since been removed.
fn assert_no_orphan_schema_keys(what: &str, table: &[Field], fields: &[schema::FieldSchema]) {
    let claimed_keys: Vec<&str> = table.iter().flat_map(|f| claimed(f.surface)).collect();
    for field in fields {
        assert!(
            claimed_keys.contains(&field.key),
            "schema declares {what} field `{}` that no coverage entry claims — remove it \
                 from schema.rs, or add back the registry field it stands for",
            field.key
        );
    }
}

#[test]
fn command_spec_is_fully_surfaced() {
    witness_command_spec(&CommandSpec::DEFAULT);
    let drafted = draft::default_command_draft();
    let keys: Vec<&str> = drafted.keys().map(String::as_str).collect();
    assert_carried("CommandSpec", COMMAND_SPEC, &keys);
    assert_schema_has("CommandSpec", COMMAND_SPEC, schema::command_field);
    assert_no_orphan_schema_keys("CommandSpec", COMMAND_SPEC, schema::COMMAND_FIELDS);
}

#[test]
fn sub_command_is_fully_surfaced() {
    witness_sub_command(&SubCommand::DEFAULT);
    let drafted = draft::default_subcommand_draft();
    let keys: Vec<&str> = drafted.keys().map(String::as_str).collect();
    assert_carried("SubCommand", SUB_COMMAND, &keys);
    assert_schema_has("SubCommand", SUB_COMMAND, schema::subcommand_field);
    assert_no_orphan_schema_keys("SubCommand", SUB_COMMAND, schema::SUBCOMMAND_FIELDS);
}

/// A `SetterConstraint` to seed from — the type has no `DEFAULT`, and its
/// `code` is a real diagnostic code rather than a string.
const SETTER: SetterConstraint = SetterConstraint {
    arg_index: 0,
    required_prefix: "/",
    code: DiagCode::Irule3101,
    message: "",
};

/// The nested types have no `FieldSchema` of their own: each is edited
/// inside its parent field's row editor, so what has to hold is that the
/// draft's JSON object for the type carries a key per field.
#[test]
fn the_nested_row_types_are_fully_surfaced() {
    witness_sub_sub_command(&SubSubCommand::DEFAULT);
    let (sub_sub, _) = draft::sub_subcommand(&SubSubCommand::DEFAULT);
    assert_carried(
        "SubSubCommand",
        SUB_SUB_COMMAND,
        &object_keys("SubSubCommand", &sub_sub),
    );

    // An option row, with a value so the nested `OptionArg` object exists.
    let opt = OptionSpec {
        value: OptionValue::Takes(OptionArg {
            arity: OptionArity::One,
            ..OptionArg::DEFAULT
        }),
        ..OptionSpec::DEFAULT
    };
    witness_option_spec(&opt);
    witness_option_arg(&OptionArg::DEFAULT);
    let (row, _) = draft::option_spec(&opt);
    assert_carried("OptionSpec", OPTION_SPEC, &object_keys("OptionSpec", &row));
    assert_carried(
        "OptionArg",
        OPTION_ARG,
        &object_keys("OptionArg", &row["value"]),
    );

    witness_arg_value(&ArgValue::DEFAULT);
    let (value, _) = draft::arg_value(&ArgValue::DEFAULT);
    assert_carried("ArgValue", ARG_VALUE, &object_keys("ArgValue", &value));

    witness_form_spec(&FormSpec::DEFAULT);
    let (form, _) = draft::form_spec(&FormSpec::DEFAULT);
    assert_carried("FormSpec", FORM_SPEC, &object_keys("FormSpec", &form));

    let snippet = HoverSnippet::brief("", &[], "");
    witness_hover_snippet(&snippet);
    let hover = draft::hover(Some(snippet));
    assert_carried(
        "HoverSnippet",
        HOVER_SNIPPET,
        &object_keys("HoverSnippet", &hover),
    );

    witness_side_effect(&SideEffect::DEFAULT);
    let (effect, _) = draft::side_effect(&SideEffect::DEFAULT);
    assert_carried(
        "SideEffect",
        SIDE_EFFECT,
        &object_keys("SideEffect", &effect),
    );

    witness_option_constraint(&OptionRelation::DEFAULT);
    witness_setter_constraint(&SETTER);
    let constraint = draft::setter_constraint(&SETTER);
    assert_carried(
        "SetterConstraint",
        SETTER_CONSTRAINT,
        &object_keys("SetterConstraint", &constraint),
    );

    witness_arity(&Arity::any());
    let arity = draft::arity(Arity::any());
    assert_carried("Arity", ARITY, &object_keys("Arity", &arity));

    let hint = ArgTypeHint {
        expected: None,
        shimmers: false,
        transparent_from: &[],
    };
    witness_arg_type_hint(&hint);
    let hints = draft::arg_type_map(&[(0, hint)]);
    assert_carried(
        "ArgTypeHint",
        ARG_TYPE_HINT,
        &object_keys("ArgTypeHint", &hints[0]),
    );

    witness_lifecycle(&Lifecycle::UNSPECIFIED);
    let mut life = Map::new();
    let _ = draft::insert_lifecycle(&mut life, Lifecycle::UNSPECIFIED);
    let life_keys: Vec<&str> = life.keys().map(String::as_str).collect();
    assert_carried("Lifecycle", LIFECYCLE, &life_keys);
}

/// The clause-grammar descriptor is plain data all the way down: the
/// draft's JSON objects for the grammar, a row, a slot and the default
/// clause each carry a key per field. `if`'s grammar exercises every one —
/// a head, a keyword row, a tail, and a default clause.
#[test]
fn the_clause_grammar_types_are_fully_surfaced() {
    let grammar = tcl_registry::default_registry()
        .get("if")
        .and_then(|spec| spec.clause_grammar)
        .expect("`if` declares a clause grammar");
    witness_clause_grammar_spec(grammar);
    witness_clause_row(&grammar.head);
    witness_clause_slot(&grammar.head.slots()[0]);
    witness_default_clause(&grammar.default_clause.expect("`if`'s else is the default"));
    let drafted = draft::clause_grammar(Some(grammar));
    assert_carried(
        "ClauseGrammarSpec",
        CLAUSE_GRAMMAR_SPEC,
        &object_keys("ClauseGrammarSpec", &drafted),
    );
    assert_carried(
        "ClauseRow",
        CLAUSE_ROW,
        &object_keys("ClauseRow", &drafted["head"]),
    );
    assert_carried(
        "ClauseSlot",
        CLAUSE_SLOT,
        &object_keys("ClauseSlot", &drafted["head"]["slots"][0]),
    );
    assert_carried(
        "DefaultClause",
        DEFAULT_CLAUSE,
        &object_keys("DefaultClause", &drafted["default_clause"]),
    );
}

/// Every exclusion states a reason, and names a type whose fields really
/// are reached through one shared-constant editor.
#[test]
fn every_exclusion_carries_a_reason() {
    witness_manufacturer_method(
        tcl_registry::definer::TCLOO_GRAMMAR
            .manufacturer("create")
            .expect("TclOO manufactures through `create`"),
    );
    witness_object_class_spec(&ObjectClassSpec {
        class_name: "",
        instance_methods: &[],
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Strict,
    });
    witness_case_list_spec(&CaseListSpec::SWITCH);
    // `ObjectClassSpec` is deliberately NOT in this list: its five fields
    // are plain data the draft carries and the `SpecTcl` renderer writes,
    // so they are `Surface::Key`s inside the `object_class` editor rather
    // than exclusions.
    for field in OBJECT_CLASS_SPEC {
        assert!(
            matches!(field.surface, Surface::Key(_)),
            "ObjectClassSpec::{} is modelled as data, not excluded",
            field.name
        );
    }
    for (what, table) in [("CaseListSpec", CASE_LIST_SPEC)] {
        for field in table {
            let Surface::Excluded(reason) = field.surface else {
                panic!("{what}::{} is not an exclusion", field.name);
            };
            assert!(!reason.is_empty(), "{what}::{} has no reason", field.name);
        }
        // The whole descriptor is one editor on some command field, so the
        // schema must still offer a way to name it.
        assert!(!table.is_empty(), "{what} has no coverage entries");
    }
    for key in ["definition_body", "object_class", "case_list", "body_scope"] {
        assert!(
            schema::command_field(key).is_some(),
            "`{key}` must stay an editable field even though its descriptor is a constant"
        );
    }
    assert_eq!(
        schema::command_field("object_class").map(|field| field.kind),
        Some(schema::FieldKind::ObjectClass),
        "`object_class` is modelled as data, not as a Rust expression"
    );
    assert_eq!(
        schema::command_field("definition_body").map(|field| field.kind),
        Some(schema::FieldKind::DefinitionBody),
        "`definition_body` is modelled as data, not as a Rust expression"
    );
}

/// The definition-body grammar is plain data all the way down: the draft's
/// JSON objects for the grammar, a member row, its optional word and one
/// accepted spelling of it, its slot, its wrapper shift, a built-in object
/// method and a member-body command each carry a key per field (a
/// manufacturer row is the `manufacturer_methods` row the command field
/// already drafts). `TclOO`'s grammar exercises all but the member-body
/// command, which snit's does.
#[test]
fn the_definition_body_types_are_fully_surfaced() {
    use tcl_registry::definer::{MethodReach, SNIT_GRAMMAR, TCLOO_GRAMMAR};
    witness_definition_body_grammar(&TCLOO_GRAMMAR);
    let method = TCLOO_GRAMMAR.member("method").expect("method");
    witness_member_spec(method);
    let optional = method.optional_argument.expect("method's export flag");
    witness_optional_member_argument(&optional);
    witness_member_option_value(&optional.values[0]);
    let filter = TCLOO_GRAMMAR.member("filter").expect("filter");
    witness_slot_spec(&filter.slot.expect("filter is a slot"));
    let self_ = TCLOO_GRAMMAR.member("self").expect("self");
    witness_wrapper_shift(&self_.wrapper_shift.expect("self moves its member"));
    witness_builtin_object_method(
        TCLOO_GRAMMAR
            .builtin_object_method("variable", MethodReach::SelfDispatch)
            .expect("`my variable` reaches oo::object's unexported member"),
    );
    witness_member_body_command(
        SNIT_GRAMMAR
            .member_body_command("install")
            .expect("snit injects `install` into every member body"),
    );

    let tcloo = draft::definition_body_block(&TCLOO_GRAMMAR);
    assert_carried(
        "DefinitionBodyGrammar",
        DEFINITION_BODY_GRAMMAR,
        &object_keys("DefinitionBodyGrammar", &tcloo),
    );
    let members = tcloo["members"].as_array().expect("member rows");
    let row = |keyword: &str| {
        members
            .iter()
            .find(|row| row["keyword"] == keyword)
            .unwrap_or_else(|| panic!("`{keyword}` row"))
    };
    assert_carried(
        "MemberSpec",
        MEMBER_SPEC,
        &object_keys("MemberSpec", row("method")),
    );
    assert_carried(
        "OptionalMemberArgument",
        OPTIONAL_MEMBER_ARGUMENT,
        &object_keys(
            "OptionalMemberArgument",
            &row("method")["optional_argument"],
        ),
    );
    assert_carried(
        "MemberOptionValue",
        MEMBER_OPTION_VALUE,
        &object_keys(
            "MemberOptionValue",
            &row("method")["optional_argument"]["values"][0],
        ),
    );
    assert_carried(
        "SlotSpec",
        SLOT_SPEC,
        &object_keys("SlotSpec", &row("filter")["slot"]),
    );
    assert_carried(
        "WrapperShift",
        WRAPPER_SHIFT,
        &object_keys("WrapperShift", &row("self")["wrapper_shift"]),
    );
    assert_carried(
        "BuiltinObjectMethod",
        BUILTIN_OBJECT_METHOD,
        &object_keys("BuiltinObjectMethod", &tcloo["builtin_object_methods"][0]),
    );
    let snit = draft::definition_body_block(&SNIT_GRAMMAR);
    assert_carried(
        "MemberBodyCommand",
        MEMBER_BODY_COMMAND,
        &object_keys("MemberBodyCommand", &snit["member_body_commands"][0]),
    );
}

// The plain-data descriptors really round-trip.
//
// Each literal below is written twice: once as Rust the compiler accepts,
// and once as the string the studio renders for it. If the two ever
// disagree the studio is emitting a spec that does not compile — and a
// descriptor field added without a renderer line shows up as a missing
// `name:` in the expected text.

const WITNESS_BINDING: HandleBindingSpec = HandleBindingSpec {
    name_from: HandleName::Word(0),
    class_from: HandleClassSource::Word(2),
    keyword: Some(HandleKeyword {
        at: 1,
        word: "using",
    }),
};

const WITNESS_REPEAT: RepeatedArgLayout = RepeatedArgLayout {
    role: ArgRole::VarWrite,
    start: 1,
    stride: 2,
    exclude_trailing: 1,
    optional_leading_word: true,
    conditional_binding: false,
};

const WITNESS_SYMBOL: SymbolDef = SymbolDef {
    name_arg: 0,
    detail_arg: Some(1),
    requires_arg: None,
    kind: DefinedSymbolKind::Test,
};

const WITNESS_PAYLOAD: BytePayloadSpec = BytePayloadSpec {
    replace_data_index: 3,
    message_flag_shift: true,
};

const WITNESS_GATE: VersionedArgValue = VersionedArgValue {
    index: 0,
    value: "utf-8",
    lifecycle: Lifecycle::UNSPECIFIED,
};

const WITNESS_CONSTRAINTS: &[OptionRelation] = &[OptionRelation {
    terms: &[
        tcl_registry::OptionTerm::Option("-glob"),
        tcl_registry::OptionTerm::Option("-regexp"),
    ],
    ..OptionRelation::DEFAULT
}];

const WITNESS_SUBS: &[SubCommand] = &[SubCommand {
    name: "encoding",
    versioned_arg_values: &[WITNESS_GATE],
    ..SubCommand::DEFAULT
}];

const WITNESS_MANUFACTURERS: &[ManufacturerMethod] = &[ManufacturerMethod {
    keyword: "create",
    visibility: MemberVisibility::Exported,
    names_instance_at: Some(1),
    definition_body_at: Some(2),
    constructor_args_from: 2,
}];

fn witness_spec() -> CommandSpec {
    CommandSpec {
        name: "witness",
        repeated_args: &[WITNESS_REPEAT],
        binds_handle: Some(&WITNESS_BINDING),
        byte_array_payload: Some(WITNESS_PAYLOAD),
        defines_symbol: Some(WITNESS_SYMBOL),
        oo_context_facts: &[("class", OoContextFactWitness::FACT)],
        manufacturer_methods: WITNESS_MANUFACTURERS,
        option_relations: WITNESS_CONSTRAINTS,
        subcommands: WITNESS_SUBS,
        ..CommandSpec::DEFAULT
    }
}

/// `OoContextFact` has one variant; naming it through a constant keeps the
/// witness readable next to the string it must render as.
struct OoContextFactWitness;
impl OoContextFactWitness {
    const FACT: tcl_registry::spec::OoContextFact =
        tcl_registry::spec::OoContextFact::DefiningClass;
}

#[test]
fn descriptor_literals_name_every_field() {
    let d = draft::from_command_spec(&witness_spec());
    assert_eq!(
        d["binds_handle"],
        json!(
            "Some(&HandleBindingSpec { name_from: HandleName::Word(0), \
                 class_from: HandleClassSource::Word(2), \
                 keyword: Some(HandleKeyword { at: 1, word: \"using\" }) })"
        )
    );
    assert_eq!(
        d["repeated_args"],
        json!(
            "&[RepeatedArgLayout { role: ArgRole::VarWrite, start: 1, stride: 2, \
                 exclude_trailing: 1, optional_leading_word: true, \
                 conditional_binding: false }]"
        )
    );
    assert_eq!(
        d["defines_symbol"],
        json!(
            "Some(SymbolDef { name_arg: 0, detail_arg: Some(1), requires_arg: None, \
                 kind: DefinedSymbolKind::Test })"
        )
    );
    assert_eq!(
        d["byte_array_payload"],
        json!("Some(BytePayloadSpec { replace_data_index: 3, message_flag_shift: true })")
    );
    assert_eq!(
        d["oo_context_facts"],
        json!("&[(\"class\", OoContextFact::DefiningClass)]")
    );
    assert_eq!(
        d["subcommands"][0]["versioned_arg_values"],
        json!(
            "&[VersionedArgValue { index: 0, value: \"utf-8\", lifecycle: Lifecycle { \
                 introduced: None, deprecated: None, retired: None, deprecation_fix: None } }]"
        )
    );
}

/// The `binds_handle` expression above, written as the Rust a rendered
/// module contains: an inline `&`-borrow of a constant struct literal,
/// inside a `fn` returning the spec.
///
/// The point is that this *compiles*. `binds_handle` wants a `&'static`,
/// and only rvalue static promotion makes an inline literal satisfy it —
/// so if the renderer's chosen shape ever stopped being promotable, the
/// build breaks here rather than in the file an author downloads.
fn promoted_binding_spec() -> CommandSpec {
    CommandSpec {
        binds_handle: Some(&HandleBindingSpec {
            name_from: HandleName::Word(0),
            class_from: HandleClassSource::Word(2),
            keyword: Some(HandleKeyword {
                at: 1,
                word: "using",
            }),
        }),
        ..CommandSpec::DEFAULT
    }
}

#[test]
fn the_rendered_descriptor_expression_is_the_rust_that_compiles() {
    assert_eq!(
        draft::from_command_spec(&promoted_binding_spec())["binds_handle"],
        draft::from_command_spec(&witness_spec())["binds_handle"],
        "the promotable literal above and the rendered expression must be the same tokens"
    );
}

/// Every `Expression` entry's field name really appears in the literal the
/// renderer emits — the mechanical half of the assertions above, so a new
/// descriptor field that nobody wired into `draft.rs` fails here too.
#[test]
fn every_expression_field_reaches_the_rendered_spec() {
    let source = render_rs::render(&draft::from_command_spec(&witness_spec()));
    for (what, table) in [
        ("RepeatedArgLayout", REPEATED_ARG_LAYOUT),
        ("HandleBindingSpec", HANDLE_BINDING_SPEC),
        ("HandleKeyword", HANDLE_KEYWORD),
        ("SymbolDef", SYMBOL_DEF),
        ("BytePayloadSpec", BYTE_PAYLOAD_SPEC),
        ("VersionedArgValue", VERSIONED_ARG_VALUE),
        ("OptionRelation", OPTION_CONSTRAINT),
        ("ManufacturerMethod", MANUFACTURER_METHOD),
    ] {
        for field in table {
            let Surface::Expression(key) = field.surface else {
                panic!("{what}::{} is not rendered as an expression", field.name);
            };
            assert!(
                schema::command_field(key).is_some() || schema::subcommand_field(key).is_some(),
                "{what}::{} names `{key}`, which is not a schema field",
                field.name
            );
            assert!(
                source.contains(&format!("{}: ", field.name)),
                "{what}::{} never reaches the rendered spec:\n{source}",
                field.name
            );
        }
    }
    // The whole point: a seeded descriptor renders instead of leaving a
    // `TODO` for the author to retype.
    assert!(
        !source.contains("`binds_handle` is set on the source command"),
        "binds_handle must round-trip, not be reported unrecoverable:\n{source}"
    );
}

/// A descriptor the registry really declares survives the round trip —
/// `set`'s handle binding, seeded from the live registry.
#[test]
fn a_live_registry_binding_round_trips() {
    let registry = tcl_registry::registry::CommandRegistry::build_default();
    let spec = registry.get("set").expect("set is a core command");
    assert!(
        spec.binds_handle.is_some(),
        "`set` declares the handle binding this test exists to round-trip"
    );
    let source = render_rs::render(&draft::from_command_spec(spec));
    assert!(
        source.contains("binds_handle: Some(&HandleBindingSpec { name_from: HandleName::Word(0), "),
        "the live binding must render as a literal:\n{source}"
    );
}

/// Guard the imports the assertions above rely on staying meaningful.
#[test]
fn witness_constants_are_the_shapes_they_claim() {
    assert_eq!(WITNESS_BINDING.class_from.index(), 2);
    // start 1, stride 2, one trailing word excluded: 5 words cover 1 and 3.
    assert_eq!(WITNESS_REPEAT.indices(5), vec![1, 3]);
    assert_eq!(SETTER.code, DiagCode::Irule3101);
    assert_eq!(FormKind::Default, FormSpec::DEFAULT.kind);
    assert_eq!(ConnectionSide::None, SideEffect::DEFAULT.connection_side);
    assert_eq!(SideEffectTarget::Unknown, SideEffect::DEFAULT.target);
}
