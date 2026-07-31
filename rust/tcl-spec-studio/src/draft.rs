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

//! The draft model — a JSON mirror of a `CommandSpec` or `SubCommand`.
//!
//! A draft is a JSON object keyed by Rust field name, holding exactly what
//! [`crate::schema`] says each field's editor produces. Seeding a draft from a
//! live registry spec ([`from_command_spec`]) is what makes the studio a
//! *browser* of the registry as well as an editor: pick a command, get its
//! real spec in the form, adjust it, render the `.rs` back out.
//!
//! ## Fields that cannot round-trip
//!
//! A handful of spec fields hold a function pointer (`arg_role_resolver`,
//! `const_fold`, `taint_sink_gate`, …) or a reference to a `&'static`
//! descriptor (`definition_body`, `case_list`, `object_class`, …). Rust can
//! tell that such a field is set, but not recover the *expression* that set
//! it. Seeding records those keys under [`UNRENDERABLE_KEY`] so the form can
//! flag them and the renderer can emit a `TODO` rather than silently dropping
//! behaviour the source command had.

use serde_json::{Map, Value, json};
use tcl_dialect::DialectSet;
use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::arity::Arity;
use tcl_registry::hooks::ArgTypeHint;
use tcl_registry::hover::{
    ArgValue, FormSpec, HoverSnippet, IntegerDomain, OptionArity, OptionSpec, OptionValue,
};
use tcl_registry::side_effects::SideEffect;
use tcl_registry::spec::{CommandSpec, SubCommand, SubSubCommand};
use tcl_registry::taint::{SetterConstraint, TaintColour};
use tcl_registry::traits::Traits;
use tcl_registry::types::{ReturnElements, VarElementsEffect, VarWriteTyping};

use crate::catalogue;

/// Draft key listing the fields a live spec sets but whose defining Rust
/// expression could not be recovered. Absent (or empty) when everything
/// round-tripped.
pub const UNRENDERABLE_KEY: &str = "__unrenderable";

/// Draft key naming the option-arity hooks that need supplying.
///
/// Distinct from a plain field key because the thing to fill in is nested in
/// an option row, not a top-level field: the renderer resolves it against the
/// `options` array so the note clears once every hook holds an expression.
pub const OPTION_HOOK_KEY: &str = "options.arity_hook";

/// Draft key holding the dialect the draft was seeded from, when it came from
/// the live registry.
pub const SOURCE_DIALECT_KEY: &str = "__sourceDialect";

/// A draft is a plain JSON object; the schema names its keys.
pub type Draft = Map<String, Value>;

fn opt_str(value: Option<&'static str>) -> Value {
    value.map_or(Value::Null, |s| json!(s))
}

fn str_list(values: &[&'static str]) -> Value {
    Value::Array(values.iter().map(|s| json!(s)).collect())
}

fn index_list(values: &[u8]) -> Value {
    Value::Array(values.iter().map(|n| json!(n)).collect())
}

fn opt_index(value: Option<u8>) -> Value {
    value.map_or(Value::Null, |n| json!(n))
}

fn dialects(value: Option<DialectSet>) -> Value {
    match value {
        None => Value::Null,
        Some(set) => Value::Array(set.member_names().into_iter().map(|n| json!(n)).collect()),
    }
}

fn traits(value: Traits) -> Value {
    Value::Array(
        catalogue::trait_keys(value)
            .into_iter()
            .map(|k| json!(k))
            .collect(),
    )
}

fn taint(value: Option<TaintColour>) -> Value {
    match value {
        None => Value::Null,
        Some(colour) => Value::Array(
            catalogue::taint_keys(colour)
                .into_iter()
                .map(|k| json!(k))
                .collect(),
        ),
    }
}

fn arity(value: Arity) -> Value {
    json!({
        "min": value.min,
        "max": if value.is_unlimited() { Value::Null } else { json!(value.max) },
        "step": value.step,
        "also_exact": value.also_exact.map_or(Value::Null, |n| json!(n)),
    })
}

fn appended_arity(value: AppendedArity) -> Value {
    match value {
        AppendedArity::Exactly(n) => json!({ "kind": "Exactly", "n": n }),
        AppendedArity::AtLeast(n) => json!({ "kind": "AtLeast", "n": n }),
        // `AppendedArity` is `#[non_exhaustive]`; an unrecognised variant
        // degrades to the arity-inert `Unknown` rather than being dropped.
        _ => json!({ "kind": "Unknown" }),
    }
}

fn role_map(entries: &[(u8, ArgRole)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, role)| json!({ "index": i, "role": catalogue::variant_name(role) }))
            .collect(),
    )
}

fn prefix_map(entries: &[(u8, AppendedArity)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, a)| json!({ "index": i, "arity": appended_arity(*a) }))
            .collect(),
    )
}

fn arg_type_map(entries: &[(u8, ArgTypeHint)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, hint)| {
                json!({
                    "index": i,
                    "expected": hint.expected.map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
                    "shimmers": hint.shimmers,
                    "transparent_from": Value::Array(
                        hint.transparent_from
                            .iter()
                            .map(|t| json!(catalogue::variant_name(t)))
                            .collect(),
                    ),
                })
            })
            .collect(),
    )
}

fn arg_value(value: &ArgValue) -> Value {
    json!({
        "value": value.value,
        "detail": value.detail,
        "min_tcl": value.min_tcl.map_or(Value::Null, |v| json!(catalogue::variant_name(&v))),
        "code": value.code.map_or(Value::Null, |c| json!(c)),
    })
}

fn arg_value_map(entries: &[(u8, &'static [ArgValue])]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, values)| {
                json!({
                    "index": i,
                    "values": Value::Array(values.iter().map(arg_value).collect()),
                })
            })
            .collect(),
    )
}

fn integer_domain(value: Option<IntegerDomain>) -> Value {
    match value {
        None => Value::Null,
        Some(IntegerDomain::Any) => json!({ "kind": "Any" }),
        Some(IntegerDomain::Range(lo, hi)) => json!({ "kind": "Range", "lo": lo, "hi": hi }),
        Some(IntegerDomain::Port) => json!({ "kind": "Port" }),
    }
}

fn option_arity(value: OptionArity) -> (Value, bool) {
    match value {
        OptionArity::One => (json!({ "kind": "One" }), true),
        OptionArity::Fixed(n) => (json!({ "kind": "Fixed", "n": n }), true),
        // The hook is a function pointer: the *shape* round-trips, but the
        // expression that produced it does not. `hook` is the slot the author
        // fills in under the option's editor; it seeds empty, and the option
        // is complete once it holds an expression.
        OptionArity::Hook(_) => (json!({ "kind": "Hook", "hook": Value::Null }), false),
    }
}

/// Draft form of an option, plus whether it round-tripped completely.
fn option_spec(opt: &OptionSpec) -> (Value, bool) {
    let (value, complete) = match opt.value {
        OptionValue::Flag => (Value::Null, true),
        OptionValue::Takes(arg) => {
            let (arity_json, arity_complete) = option_arity(arg.arity);
            (
                json!({
                    "arity": arity_json,
                    "role": catalogue::variant_name(&arg.role),
                    "also_role": arg.also_role.map_or(Value::Null, |r| json!(catalogue::variant_name(&r))),
                    "body_kind": catalogue::variant_name(&arg.body_kind),
                    "values": Value::Array(arg.values.iter().map(arg_value).collect()),
                    "closed": arg.closed,
                    "integer": integer_domain(arg.integer),
                    "hint": arg.hint,
                    "appended_arity": appended_arity(arg.appended_arity),
                }),
                arity_complete,
            )
        }
    };
    (
        json!({
            "name": opt.name,
            "detail": opt.detail,
            "dialects": dialects(opt.dialects),
            "aliases": str_list(opt.aliases),
            "min_version": opt_str(opt.min_version),
            "value": value,
        }),
        complete,
    )
}

fn form_spec(form: &FormSpec) -> Value {
    json!({
        "kind": catalogue::variant_name(&form.kind),
        "synopsis": form.synopsis,
        "dialects": dialects(form.dialects),
    })
}

fn side_effect(effect: &SideEffect) -> Value {
    json!({
        "target": catalogue::variant_name(&effect.target),
        "reads": effect.reads,
        "writes": effect.writes,
        "connection_side": catalogue::variant_name(&effect.connection_side),
        "dialects": dialects(effect.dialects),
    })
}

fn setter_constraint(constraint: &SetterConstraint) -> Value {
    json!({
        "arg_index": constraint.arg_index,
        "required_prefix": constraint.required_prefix,
        "code": constraint.code.as_str(),
        "message": constraint.message,
    })
}

fn hover(value: Option<HoverSnippet>) -> Value {
    match value {
        None => Value::Null,
        Some(h) => json!({
            "summary": h.summary,
            "synopsis": str_list(h.synopsis),
            "snippet": h.snippet,
            "source": h.source,
            "examples": h.examples,
            "return_value": h.return_value,
        }),
    }
}

fn sub_subcommand(sub: &SubSubCommand) -> Value {
    json!({
        "name": sub.name,
        "detail": sub.detail,
        "synopsis": sub.synopsis,
        "dialects": dialects(sub.dialects),
    })
}

/// The Rust expression for a [`VarWriteTyping`], fully qualified.
///
/// Written out rather than derived from `Debug`: the `Fixed` payload is itself
/// an enum, and `Debug` prints its variant without a path (`Fixed(String)`),
/// which does not compile. The exhaustive `match` also means a new variant
/// breaks the build here rather than silently rendering a bad spec.
fn var_write_typing_expr(value: VarWriteTyping) -> String {
    match value {
        VarWriteTyping::ReturnValue => "VarWriteTyping::ReturnValue".to_owned(),
        VarWriteTyping::Fixed(t) => format!(
            "VarWriteTyping::Fixed({})",
            catalogue::qualified_variant("TclType", &t)
        ),
        VarWriteTyping::Destructured => "VarWriteTyping::Destructured".to_owned(),
        VarWriteTyping::ElementsOf { container_arg } => {
            format!("VarWriteTyping::ElementsOf {{ container_arg: {container_arg} }}")
        }
    }
}

/// The Rust expression for a [`ReturnElements`], wrapped in `Some(…)` for the
/// `Option`-typed field that holds it.
fn return_elements_expr(value: ReturnElements) -> String {
    let inner = match value {
        ReturnElements::ListOfArgs { from } => format!("ListOfArgs {{ from: {from} }}"),
        ReturnElements::DictOfPairs { from } => format!("DictOfPairs {{ from: {from} }}"),
        ReturnElements::ElementOf { container_arg } => {
            format!("ElementOf {{ container_arg: {container_arg} }}")
        }
        ReturnElements::SubListOf { container_arg } => {
            format!("SubListOf {{ container_arg: {container_arg} }}")
        }
    };
    format!("Some(ReturnElements::{inner})")
}

/// The Rust expression for a [`VarElementsEffect`], wrapped in `Some(…)`.
fn var_elements_effect_expr(value: VarElementsEffect) -> String {
    let inner = match value {
        VarElementsEffect::AppendsListElements { values_from } => {
            format!("AppendsListElements {{ values_from: {values_from} }}")
        }
        VarElementsEffect::SetsDictValue => "SetsDictValue".to_owned(),
        VarElementsEffect::ExtendsDictValuesByName { values_from } => {
            format!("ExtendsDictValuesByName {{ values_from: {values_from} }}")
        }
        VarElementsEffect::ListifiesDictValue => "ListifiesDictValue".to_owned(),
    };
    format!("Some(VarElementsEffect::{inner})")
}

/// Records the keys whose defining expression could not be recovered.
#[derive(Debug, Default)]
struct Unrecovered(Vec<&'static str>);

impl Unrecovered {
    /// Note `key` as unrecoverable when `present` holds, and return the JSON
    /// placeholder (always `null` — the author supplies the expression).
    fn expr(&mut self, key: &'static str, present: bool) -> Value {
        if present {
            self.0.push(key);
        }
        Value::Null
    }

    fn note(&mut self, key: &'static str) {
        if !self.0.contains(&key) {
            self.0.push(key);
        }
    }

    fn into_value(self) -> Value {
        Value::Array(self.0.into_iter().map(|k| json!(k)).collect())
    }
}

/// Seed a draft from a live [`SubCommand`].
#[must_use]
pub fn from_subcommand(sub: &SubCommand) -> Draft {
    let mut lost = Unrecovered::default();
    let draft = subcommand_body(sub, &mut lost);
    let mut draft = draft;
    draft.insert(UNRENDERABLE_KEY.to_owned(), lost.into_value());
    draft
}

fn subcommand_body(sub: &SubCommand, lost: &mut Unrecovered) -> Draft {
    let mut d = Map::new();
    subcommand_identity(&mut d, sub, lost);
    subcommand_types(&mut d, sub);
    subcommand_hooks(&mut d, sub, lost);
    subcommand_rest(&mut d, sub, lost);
    d
}

/// Name, traits, arity, documentation, and argument layout.
fn subcommand_identity(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert("name".into(), json!(sub.name));
    d.insert("traits".into(), traits(sub.traits));
    d.insert("arity".into(), arity(sub.arity));
    d.insert("detail".into(), json!(sub.detail));
    d.insert("synopsis".into(), json!(sub.synopsis));
    d.insert("hover".into(), hover(sub.hover));
    d.insert("arg_roles".into(), role_map(sub.arg_roles));
    d.insert(
        "arg_role_resolver".into(),
        lost.expr("arg_role_resolver", sub.arg_role_resolver.is_some()),
    );
    d.insert("command_prefixes".into(), prefix_map(sub.command_prefixes));
    d.insert(
        "command_prefix_resolver".into(),
        lost.expr(
            "command_prefix_resolver",
            sub.command_prefix_resolver.is_some(),
        ),
    );
}

/// Return type, element structure, and per-argument type hints.
fn subcommand_types(d: &mut Draft, sub: &SubCommand) {
    d.insert(
        "return_type".into(),
        sub.return_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "var_write_typing".into(),
        json!(var_write_typing_expr(sub.var_write_typing)),
    );
    d.insert(
        "return_elements".into(),
        sub.return_elements
            .map_or(Value::Null, |e| json!(return_elements_expr(e))),
    );
    d.insert(
        "var_elements_effect".into(),
        sub.var_elements_effect
            .map_or(Value::Null, |e| json!(var_elements_effect_expr(e))),
    );
    d.insert("arg_types".into(), arg_type_map(sub.arg_types));
    d.insert("pure".into(), json!(sub.pure));
    d.insert("mutator".into(), json!(sub.mutator));
}

/// Constant folders and the compiler / analyser hook IDs.
fn subcommand_hooks(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert(
        "const_fold".into(),
        lost.expr("const_fold", sub.const_fold.is_some()),
    );
    d.insert(
        "const_fold_versioned".into(),
        lost.expr("const_fold_versioned", sub.const_fold_versioned.is_some()),
    );
    d.insert(
        "lowering_hook".into(),
        sub.lowering_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "codegen_hook".into(),
        sub.codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "inline_codegen_hook".into(),
        sub.inline_codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "wasm_codegen_hook".into(),
        sub.wasm_codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "analyser_hook".into(),
        sub.analyser_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "command_table_effect".into(),
        sub.command_table_effect
            .map_or(Value::Null, |e| json!(catalogue::variant_name(&e))),
    );
}

/// Options, values, availability, behaviour flags, taint, and effects.
fn subcommand_rest(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    let mut options = Vec::new();
    for opt in sub.options {
        let (json_value, complete) = option_spec(opt);
        if !complete {
            lost.note(OPTION_HOOK_KEY);
        }
        options.push(json_value);
    }
    d.insert("options".into(), Value::Array(options));
    d.insert("arg_values".into(), arg_value_map(sub.arg_values));
    d.insert(
        "subcommand_forms".into(),
        lost.expr("subcommand_forms", !sub.subcommand_forms.is_empty()),
    );
    d.insert("dialects".into(), dialects(sub.dialects));
    d.insert("safe_on_uninit".into(), dialects(sub.safe_on_uninit));
    d.insert("loop_list_header".into(), json!(sub.loop_list_header));
    d.insert("creates_scope_alias".into(), json!(sub.creates_scope_alias));
    d.insert(
        "inferred_storage_type".into(),
        sub.inferred_storage_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "body_kind".into(),
        json!(catalogue::variant_name(&sub.body_kind)),
    );
    d.insert(
        "byte_array_effect".into(),
        json!(catalogue::variant_name(&sub.byte_array_effect)),
    );
    d.insert(
        "closed_value_args".into(),
        index_list(sub.closed_value_args),
    );
    d.insert(
        "arg_values_accept_prefix".into(),
        json!(sub.arg_values_accept_prefix),
    );
    d.insert(
        "body_arg_implicit_args".into(),
        json!(sub.body_arg_implicit_args),
    );
    d.insert("taint_transform".into(), taint(sub.taint_transform));
    d.insert(
        "taint_double_encode_colour".into(),
        taint(sub.taint_double_encode_colour),
    );
    d.insert("taint_output_sink".into(), opt_str(sub.taint_output_sink));
    d.insert("credential_arg".into(), opt_index(sub.credential_arg));
    d.insert("sensitive_headers".into(), str_list(sub.sensitive_headers));
    d.insert(
        "pattern_type".into(),
        sub.pattern_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "format_string_type".into(),
        sub.format_string_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert("xc_operation".into(), opt_str(sub.xc_operation));
    d.insert(
        "side_effects".into(),
        Value::Array(sub.side_effects.iter().map(side_effect).collect()),
    );
    d.insert("destructive".into(), json!(sub.destructive));
    d.insert("returns_path".into(), json!(sub.returns_path));
    d.insert("is_unescape".into(), json!(sub.is_unescape));
    d.insert("cfg_rewrite_name".into(), opt_str(sub.cfg_rewrite_name));
    d.insert(
        "sub_subcommands".into(),
        Value::Array(sub.sub_subcommands.iter().map(sub_subcommand).collect()),
    );
    d.insert(
        "defines_command_at".into(),
        opt_index(sub.defines_command_at),
    );
}

/// Seed a draft from a live [`CommandSpec`].
///
/// Every field the schema names is present in the result, so the form never
/// has to guess a default. Fields whose defining expression could not be
/// recovered are listed under [`UNRENDERABLE_KEY`].
#[must_use]
pub fn from_command_spec(spec: &CommandSpec) -> Draft {
    let mut lost = Unrecovered::default();
    let mut d = Map::new();
    command_identity(&mut d, spec, &mut lost);
    command_types(&mut d, spec, &mut lost);
    command_docs(&mut d, spec, &mut lost);
    command_hooks(&mut d, spec, &mut lost);
    command_options(&mut d, spec, &mut lost);
    command_taint(&mut d, spec, &mut lost);
    command_advanced(&mut d, spec, &mut lost);
    d.insert(UNRENDERABLE_KEY.to_owned(), lost.into_value());
    d
}

/// Name, traits, availability, arity, and argument layout.
fn command_identity(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("name".into(), json!(spec.name));
    d.insert("traits".into(), traits(spec.traits));
    d.insert("dialects".into(), dialects(spec.dialects));
    d.insert("arity".into(), arity(spec.arity));
    d.insert("arg_roles".into(), role_map(spec.arg_roles));
    d.insert(
        "arg_role_resolver".into(),
        lost.expr("arg_role_resolver", spec.arg_role_resolver.is_some()),
    );
    d.insert(
        "frame_effect".into(),
        lost.expr("frame_effect", spec.frame_effect.is_some()),
    );
    d.insert(
        "clause_shape_check".into(),
        lost.expr("clause_shape_check", spec.clause_shape_check.is_some()),
    );
    d.insert("command_prefixes".into(), prefix_map(spec.command_prefixes));
    d.insert(
        "command_prefix_resolver".into(),
        lost.expr(
            "command_prefix_resolver",
            spec.command_prefix_resolver.is_some(),
        ),
    );
}

/// Return type, element structure, and per-argument type hints.
fn command_types(d: &mut Draft, spec: &CommandSpec, _lost: &mut Unrecovered) {
    d.insert(
        "return_type".into(),
        spec.return_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "var_write_typing".into(),
        json!(var_write_typing_expr(spec.var_write_typing)),
    );
    d.insert(
        "return_elements".into(),
        spec.return_elements
            .map_or(Value::Null, |e| json!(return_elements_expr(e))),
    );
    d.insert(
        "var_elements_effect".into(),
        spec.var_elements_effect
            .map_or(Value::Null, |e| json!(var_elements_effect_expr(e))),
    );
    d.insert("arg_types".into(), arg_type_map(spec.arg_types));
}

/// Subcommands, documentation, and invocation forms.
fn command_docs(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    let subcommands: Vec<Value> = spec
        .subcommands
        .iter()
        .map(|sub| {
            let mut sub_lost = Unrecovered::default();
            let mut body = subcommand_body(sub, &mut sub_lost);
            for key in &sub_lost.0 {
                lost.note(key);
            }
            body.insert(UNRENDERABLE_KEY.to_owned(), sub_lost.into_value());
            Value::Object(body)
        })
        .collect();
    d.insert("subcommands".into(), Value::Array(subcommands));
    d.insert(
        "allow_unknown_subcommands".into(),
        json!(spec.allow_unknown_subcommands),
    );
    d.insert(
        "default_form_first_word".into(),
        spec.default_form_first_word
            .map_or(Value::Null, |w| json!(catalogue::variant_name(&w))),
    );
    d.insert("hover".into(), hover(spec.hover));
    d.insert(
        "forms".into(),
        Value::Array(spec.forms.iter().map(form_spec).collect()),
    );
    d.insert(
        "command_forms".into(),
        lost.expr("command_forms", !spec.command_forms.is_empty()),
    );
    d.insert(
        "assigns_variable_at".into(),
        opt_index(spec.assigns_variable_at),
    );
    d.insert("safe_on_uninit".into(), dialects(spec.safe_on_uninit));
}

/// Constant folders, compiler / analyser hooks, and effects.
fn command_hooks(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert(
        "const_fold".into(),
        lost.expr("const_fold", spec.const_fold.is_some()),
    );
    d.insert(
        "const_fold_versioned".into(),
        lost.expr("const_fold_versioned", spec.const_fold_versioned.is_some()),
    );
    d.insert(
        "lowering_hook".into(),
        spec.lowering_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "codegen_hook".into(),
        spec.codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "inline_codegen_hook".into(),
        spec.inline_codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "wasm_codegen_hook".into(),
        spec.wasm_codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "analyser_hook".into(),
        spec.analyser_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "command_table_effect".into(),
        spec.command_table_effect
            .map_or(Value::Null, |e| json!(catalogue::variant_name(&e))),
    );
    d.insert(
        "side_effects".into(),
        Value::Array(spec.side_effects.iter().map(side_effect).collect()),
    );
    d.insert(
        "inferred_storage_type".into(),
        spec.inferred_storage_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
}

/// Options, enumerable values, and availability gating.
fn command_options(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    let mut options = Vec::new();
    for opt in spec.options {
        let (json_value, complete) = option_spec(opt);
        if !complete {
            lost.note(OPTION_HOOK_KEY);
        }
        options.push(json_value);
    }
    d.insert("required_package".into(), opt_str(spec.required_package));
    d.insert("excluded_events".into(), str_list(spec.excluded_events));
    d.insert("unsafe_command".into(), json!(spec.unsafe_command));
    d.insert(
        "closed_value_args".into(),
        index_list(spec.closed_value_args),
    );
    d.insert(
        "event_requires".into(),
        lost.expr("event_requires", spec.event_requires.is_some()),
    );
    d.insert("options".into(), Value::Array(options));
    d.insert(
        "reserved_trailing_words".into(),
        json!(spec.reserved_trailing_words),
    );
    d.insert("arg_values".into(), arg_value_map(spec.arg_values));
    d.insert(
        "body_kind".into(),
        json!(catalogue::variant_name(&spec.body_kind)),
    );
    d.insert(
        "body_arg_implicit_args".into(),
        json!(spec.body_arg_implicit_args),
    );
}

/// Taint and credential metadata.
fn command_taint(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("taint_output_sink".into(), opt_str(spec.taint_output_sink));
    d.insert(
        "taint_output_sink_subcommands".into(),
        str_list(spec.taint_output_sink_subcommands),
    );
    d.insert("taint_log_sink".into(), opt_str(spec.taint_log_sink));
    d.insert(
        "taint_network_sink_args".into(),
        spec.taint_network_sink_args.map_or(Value::Null, index_list),
    );
    d.insert(
        "taint_code_sink_args".into(),
        spec.taint_code_sink_args.map_or(Value::Null, index_list),
    );
    d.insert(
        "taint_interp_eval_subcommands".into(),
        str_list(spec.taint_interp_eval_subcommands),
    );
    d.insert("taint_source".into(), taint(spec.taint_source));
    d.insert("taint_transform".into(), taint(spec.taint_transform));
    d.insert(
        "taint_double_encode_colour".into(),
        taint(spec.taint_double_encode_colour),
    );
    d.insert(
        "taint_sink_safe_colour".into(),
        taint(spec.taint_sink_safe_colour),
    );
    d.insert(
        "taint_sink_gate".into(),
        lost.expr("taint_sink_gate", spec.taint_sink_gate.is_some()),
    );
    d.insert(
        "credential_options".into(),
        str_list(spec.credential_options),
    );
    d.insert("sensitive_headers".into(), str_list(spec.sensitive_headers));
    d.insert(
        "setter_constraints".into(),
        Value::Array(
            spec.setter_constraints
                .iter()
                .map(setter_constraint)
                .collect(),
        ),
    );
}

/// Deprecation, translation, and the descriptor references.
fn command_advanced(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert(
        "pattern_type".into(),
        spec.pattern_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "format_string_type".into(),
        spec.format_string_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert("tcllib_package".into(), opt_str(spec.tcllib_package));
    d.insert("min_version".into(), opt_str(spec.min_version));
    d.insert("max_version".into(), opt_str(spec.max_version));
    d.insert(
        "warn_missing_import".into(),
        json!(spec.warn_missing_import),
    );
    d.insert(
        "is_namespace_exported".into(),
        json!(spec.is_namespace_exported),
    );
    d.insert(
        "xc_translatable".into(),
        spec.xc_translatable.map_or(Value::Null, |b| json!(b)),
    );
    d.insert("xc_operation".into(), opt_str(spec.xc_operation));
    d.insert(
        "deprecated_replacement".into(),
        opt_str(spec.deprecated_replacement),
    );
    d.insert(
        "deprecated_replacement_drop_in".into(),
        json!(spec.deprecated_replacement_drop_in),
    );
    d.insert(
        "byte_array_payload".into(),
        lost.expr("byte_array_payload", spec.byte_array_payload.is_some()),
    );
    d.insert(
        "byte_array_effect".into(),
        json!(catalogue::variant_name(&spec.byte_array_effect)),
    );
    d.insert(
        "definition_body".into(),
        lost.expr("definition_body", spec.definition_body.is_some()),
    );
    d.insert(
        "case_list".into(),
        lost.expr("case_list", spec.case_list.is_some()),
    );
    d.insert(
        "oo_context_facts".into(),
        lost.expr("oo_context_facts", !spec.oo_context_facts.is_empty()),
    );
    d.insert(
        "object_class".into(),
        lost.expr("object_class", spec.object_class.is_some()),
    );
    d.insert(
        "defines_symbol".into(),
        lost.expr("defines_symbol", spec.defines_symbol.is_some()),
    );
    d.insert(
        "body_scope".into(),
        lost.expr("body_scope", spec.body_scope.is_some()),
    );
    d.insert(
        "creates_instance_at".into(),
        opt_index(spec.creates_instance_at),
    );
    d.insert(
        "defines_command_at".into(),
        opt_index(spec.defines_command_at),
    );
    d.insert(
        "context_gate".into(),
        lost.expr("context_gate", spec.context_gate.is_some()),
    );
    d.insert(
        "implementation_namespace".into(),
        opt_str(spec.implementation_namespace),
    );
}

/// A draft of a brand-new command: every field at its `CommandSpec::DEFAULT`
/// value.
#[must_use]
pub fn default_command_draft() -> Draft {
    from_command_spec(&CommandSpec::DEFAULT)
}

/// A draft of a brand-new subcommand: every field at its
/// `SubCommand::DEFAULT` value.
#[must_use]
pub fn default_subcommand_draft() -> Draft {
    from_subcommand(&SubCommand::DEFAULT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;

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
}
