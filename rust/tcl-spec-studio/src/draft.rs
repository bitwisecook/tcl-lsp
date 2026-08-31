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
//! `const_fold`, `taint_sink_gate`, …) or a reference to a **named** `&'static`
//! descriptor the registry shares between commands (`definition_body`,
//! `case_list`, `body_scope`, `frame_effect`, `bpf_op`,
//! `event_requires`, `command_forms`, and the semantic/effect descriptors).
//! Rust can tell that such a field is set, but not recover the *expression* —
//! the constant's path — that set it.
//! Seeding records those keys under [`UNRENDERABLE_KEY`] so the form can flag
//! them and the renderer can emit a `TODO` rather than silently dropping
//! behaviour the source command had.
//!
//! A descriptor that is **plain data** is a different case and does round-trip:
//! `repeated_args`, `binds_handle`, `byte_array_payload`, `defines_symbol`,
//! `oo_context_facts`, and a command's or subcommand's `versioned_arg_values`
//! are rendered
//! back out as full struct literals (every field spelled, never a defaulting
//! constructor), so drafting and re-rendering a command that sets one loses
//! nothing. [`crate::coverage`] is what keeps those literals complete.
//! `object_class` is the same case one level deeper — a class name, a flag,
//! superclass names, and a method table that *is* `&[SubCommand]` — so it is
//! seeded as a JSON object whose methods are ordinary subcommand drafts.

use serde_json::{Map, Value, json};
use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::arity::Arity;
use tcl_registry::definer::ManufacturerMethod;
use tcl_registry::deprecation::{DeprecationFixHook, DeprecationFixSafety};
use tcl_registry::forms::CommandForm;
use tcl_registry::handle_binding::{
    HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
};
use tcl_registry::hooks::ArgTypeHint;
use tcl_registry::hover::{
    ArgValue, CallbackTaintInput, FormSpec, HoverSnippet, IntegerDomain, OptionArity, OptionSpec,
    OptionValue,
};
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::presentation::ArgPresentation;
use tcl_registry::remote_method::{MethodWord, RemoteDispatch, RemoteFamily, RemoteMethodRole};
use tcl_registry::repeated::RepeatedArgLayout;
use tcl_registry::representation::RepresentationEffect;
use tcl_registry::side_effects::SideEffect;
use tcl_registry::spec::{
    BytePayloadSpec, CommandSpec, ObjectClassSpec, OoContextFact, OptionRelation, SubCommand,
    SubSubCommand, VersionedArgValue,
};
use tcl_registry::symbol_def::SymbolDef;
use tcl_registry::taint::{SetterConstraint, TaintColour};
use tcl_registry::traits::Traits;
use tcl_registry::types::{ReturnElements, VarElementsEffect, VarWriteTyping};

use crate::catalogue;
use crate::render_rs::rust_string;
use tcl_dialect::model::SpecSurface;

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

/// Draft key naming option lifecycle callbacks whose Rust function path must
/// be supplied by the author.
pub const OPTION_DEPRECATION_FIX_HOOK_KEY: &str = "options.deprecation_fix_hook";

/// Internal option-row marker for a callback hook that seeding could not name.
pub const OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY: &str = "__deprecationFixUnrecoverable";

/// Draft key holding the dialect the draft was seeded from, when it came from
/// the live registry.
pub const SOURCE_DIALECT_KEY: &str = "__sourceDialect";

/// A draft is a plain JSON object; the schema names its keys.
pub type Draft = Map<String, Value>;

fn opt_str(value: Option<&'static str>) -> Value {
    value.map_or(Value::Null, |s| json!(s))
}

/// Write a [`Lifecycle`] into a draft under the canonical keys used by every
/// registry surface. Returns whether every field round-tripped; a custom
/// callback cannot reveal its Rust function path and must be supplied by the
/// author.
pub(crate) fn insert_lifecycle(d: &mut Map<String, Value>, lifecycle: Lifecycle) -> bool {
    let (deprecation_fix, complete) = deprecation_fix_value(lifecycle.deprecation_fix);
    d.insert("introduced_version".into(), opt_str(lifecycle.introduced));
    d.insert("deprecated_version".into(), opt_str(lifecycle.deprecated));
    d.insert("retired_version".into(), opt_str(lifecycle.retired));
    d.insert("deprecation_fix".into(), deprecation_fix);
    complete
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

fn dialects(value: Option<&'static [SpecSurface]>) -> Value {
    match value {
        None => Value::Null,
        // The draft's vocabulary is catalogue dialect names; the registry
        // owns the one projection from rows onto them.
        Some(rows) => Value::Array(
            tcl_registry::model::surface::dialect_names_for_rows(rows)
                .into_iter()
                .map(|name| json!(name))
                .collect(),
        ),
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

pub(crate) fn arity(value: Arity) -> Value {
    json!({
        "min": value.min,
        "max": if value.is_unlimited() { Value::Null } else { json!(value.max) },
        "step": value.step,
        "also_exact": value.also_exact.map_or(Value::Null, |n| json!(n)),
    })
}

/// One [`tcl_registry::arity::ArityWindow`] as a draft value — the shape the
/// signature had over one span of the owning package's releases (#1627).
pub(crate) fn arity_windows(windows: &[tcl_registry::arity::ArityWindow]) -> Value {
    Value::Array(
        windows
            .iter()
            .map(|window| {
                json!({
                    "arity": arity(window.arity),
                    "lifecycle": {
                        "introduced": opt_str(window.lifecycle.introduced),
                        "deprecated": opt_str(window.lifecycle.deprecated),
                        "retired": opt_str(window.lifecycle.retired),
                    },
                })
            })
            .collect::<Vec<_>>(),
    )
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

fn presentation_map(entries: &[(u8, ArgPresentation)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, p)| json!({ "index": i, "presentation": catalogue::variant_name(p) }))
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

fn callback_taint_map(entries: &[(u8, &'static [CallbackTaintInput])]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(index, inputs)| {
                json!({
                    "index": index,
                    "inputs": inputs.iter().map(|input| input.spelling()).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

pub(crate) fn arg_type_map(entries: &[(u8, ArgTypeHint)]) -> Value {
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

/// Draft form of one literal argument value, plus whether it round-tripped.
///
/// The value rides **two** version axes and carries both: `min_tcl` is the
/// Tcl-core rung it needs, the lifecycle keys are the owning package's
/// releases (`hover.rs`'s `ArgValue`).
pub(crate) fn arg_value(value: &ArgValue) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("value".into(), json!(value.value));
    d.insert("detail".into(), json!(value.detail));
    d.insert(
        "min_tcl".into(),
        value
            .min_tcl
            .map_or(Value::Null, |v| json!(catalogue::variant_name(&v))),
    );
    let complete = insert_lifecycle(&mut d, value.lifecycle);
    d.insert("code".into(), value.code.map_or(Value::Null, |c| json!(c)));
    (Value::Object(d), complete)
}

/// Draft rows for one nested table, noting `key` when a row's lifecycle
/// carried a callback quick-fix whose Rust path seeding cannot name.
fn nested_rows<T>(
    items: &[T],
    key: &'static str,
    lost: &mut Unrecovered,
    row: impl Fn(&T) -> (Value, bool),
) -> Value {
    Value::Array(
        items
            .iter()
            .map(|item| {
                let (value, complete) = row(item);
                if !complete {
                    lost.note(key);
                }
                value
            })
            .collect(),
    )
}

fn arg_value_map(entries: &[(u8, &'static [ArgValue])], lost: &mut Unrecovered) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, values)| {
                json!({
                    "index": i,
                    "values": nested_rows(values, "arg_values", lost, arg_value),
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

fn deprecation_fix_safety_expr(safety: DeprecationFixSafety) -> &'static str {
    match safety {
        DeprecationFixSafety::SemanticsEquivalent => "DeprecationFixSafety::SemanticsEquivalent",
        DeprecationFixSafety::RequiresReview => "DeprecationFixSafety::RequiresReview",
    }
}

/// Recover a declarative lifecycle edit exactly. Callback function pointers
/// deliberately abstain: Rust does not expose the symbol path needed in a
/// rendered source file.
fn deprecation_fix_expr(hook: DeprecationFixHook) -> Option<String> {
    let expr = match hook {
        DeprecationFixHook::ReplaceMatchedWord {
            replacement,
            description,
            safety,
        } => format!(
            "DeprecationFixHook::ReplaceMatchedWord {{ replacement: {}, description: {}, safety: {} }}",
            rust_string(replacement),
            rust_string(description),
            deprecation_fix_safety_expr(safety),
        ),
        DeprecationFixHook::ReplaceArgument {
            index,
            replacement,
            description,
            safety,
        } => format!(
            "DeprecationFixHook::ReplaceArgument {{ index: {index}, replacement: {}, description: {}, safety: {} }}",
            rust_string(replacement),
            rust_string(description),
            deprecation_fix_safety_expr(safety),
        ),
        DeprecationFixHook::ReplaceInvocation {
            replacement,
            description,
            safety,
        } => format!(
            "DeprecationFixHook::ReplaceInvocation {{ replacement: {}, description: {}, safety: {} }}",
            rust_string(replacement),
            rust_string(description),
            deprecation_fix_safety_expr(safety),
        ),
        DeprecationFixHook::Custom { .. } => return None,
    };
    Some(format!("Some({expr})"))
}

fn deprecation_fix_value(hook: Option<DeprecationFixHook>) -> (Value, bool) {
    match hook {
        None => (Value::Null, true),
        Some(hook) => match deprecation_fix_expr(hook) {
            Some(expr) => (json!(expr), true),
            None => (Value::Null, false),
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OptionDraftCompleteness {
    pub(crate) arity_hook: bool,
    pub(crate) deprecation_fix_hook: bool,
}

/// Draft form of an option, plus whether it round-tripped completely.
pub(crate) fn option_spec(opt: &OptionSpec) -> (Value, OptionDraftCompleteness) {
    // A value row's own quick-fix hook is unrecoverable for the same reason the
    // option's is, and is filled in at the same place — inside this option's
    // row editor — so it rides the same completeness flag.
    let mut values_complete = true;
    let values: Vec<Value> = match opt.value {
        OptionValue::Takes(arg) => arg
            .values
            .iter()
            .map(|value| {
                let (row, complete) = arg_value(value);
                values_complete &= complete;
                row
            })
            .collect(),
        OptionValue::Flag => Vec::new(),
    };
    let (value, arity_hook) = match opt.value {
        OptionValue::Flag => (Value::Null, true),
        OptionValue::Takes(arg) => {
            let (arity_json, arity_complete) = option_arity(arg.arity);
            (
                json!({
                    "arity": arity_json,
                    "role": catalogue::variant_name(&arg.role),
                    "also_role": arg.also_role.map_or(Value::Null, |r| json!(catalogue::variant_name(&r))),
                    "body_kind": catalogue::variant_name(&arg.body_kind),
                    "script_timing": catalogue::variant_name(&arg.script_timing),
                    "callback_taint_inputs": arg.callback_taint_inputs.iter().map(|input| input.spelling()).collect::<Vec<_>>(),
                    "variable_scope": catalogue::variant_name(&arg.variable_scope),
                    "values": Value::Array(values),
                    "closed": arg.closed,
                    "integer": integer_domain(arg.integer),
                    "hint": arg.hint,
                    "appended_arity": appended_arity(arg.appended_arity),
                    "taints_var_write": arg.taints_var_write,
                }),
                arity_complete,
            )
        }
    };
    let (deprecation_fix, deprecation_fix_hook) =
        deprecation_fix_value(opt.lifecycle.deprecation_fix);
    (
        json!({
            "name": opt.name,
            "detail": opt.detail,
            "surface": dialects(opt.surface),
            "aliases": str_list(opt.aliases),
            "introduced_version": opt_str(opt.lifecycle.introduced),
            "deprecated_version": opt_str(opt.lifecycle.deprecated),
            "retired_version": opt_str(opt.lifecycle.retired),
            "deprecation_fix": deprecation_fix,
            OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY: !deprecation_fix_hook,
            "min_abbrev": opt_index(opt.min_abbrev),
            "value": value,
        }),
        OptionDraftCompleteness {
            arity_hook,
            deprecation_fix_hook: deprecation_fix_hook && values_complete,
        },
    )
}

pub(crate) fn form_spec(form: &FormSpec) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("kind".into(), json!(catalogue::variant_name(&form.kind)));
    d.insert("synopsis".into(), json!(form.synopsis));
    d.insert("surface".into(), dialects(form.surface));
    let complete = insert_lifecycle(&mut d, form.lifecycle);
    (Value::Object(d), complete)
}

pub(crate) fn side_effect(effect: &SideEffect) -> (Value, bool) {
    let mut d = Map::new();
    d.insert(
        "target".into(),
        json!(catalogue::variant_name(&effect.target)),
    );
    d.insert("reads".into(), json!(effect.reads));
    d.insert("writes".into(), json!(effect.writes));
    d.insert(
        "connection_side".into(),
        json!(catalogue::variant_name(&effect.connection_side)),
    );
    d.insert("surface".into(), dialects(effect.surface));
    let complete = insert_lifecycle(&mut d, effect.lifecycle);
    (Value::Object(d), complete)
}

pub(crate) fn setter_constraint(constraint: &SetterConstraint) -> Value {
    json!({
        "arg_index": constraint.arg_index,
        "required_prefix": constraint.required_prefix,
        "code": constraint.code.as_str(),
        "message": constraint.message,
    })
}

fn manufacturer_method(method: &ManufacturerMethod) -> Value {
    json!({
        "keyword": method.keyword,
        "visibility": catalogue::variant_name(&method.visibility),
        "names_instance_at": method.names_instance_at,
        "definition_body_at": method.definition_body_at,
        "constructor_args_from": method.constructor_args_from,
    })
}

/// Seed the `object_class` value from a live [`ObjectClassSpec`].
///
/// The five fields are plain data — a class name, a method table that *is*
/// `&[SubCommand]`, superclass names, and two policies — so the draft holds the
/// descriptor itself rather than an [`Unrecovered`] note. Each instance method
/// is drafted with the same [`subcommand_body`] the command's own subcommands
/// use, which is what lets the `SpecTcl` renderer write `method NAME { … }`
/// rows in the `subcommand` body grammar the DSL reuses for them.
fn object_class(class: Option<&'static ObjectClassSpec>, lost: &mut Unrecovered) -> Value {
    let Some(class) = class else {
        return Value::Null;
    };
    let methods: Vec<Value> = class
        .instance_methods
        .iter()
        .map(|method| {
            let mut method_lost = Unrecovered::default();
            let mut body = subcommand_body(method, &mut method_lost);
            for key in &method_lost.0 {
                lost.note(key);
            }
            body.insert(UNRENDERABLE_KEY.to_owned(), method_lost.into_value());
            Value::Object(body)
        })
        .collect();
    json!({
        "class_name": class.class_name,
        "instance_methods": Value::Array(methods),
        "superclasses": str_list(class.superclasses),
        "allow_unknown_methods": class.allow_unknown_methods,
        "method_prefix_matching": catalogue::variant_name(&class.method_prefix_matching),
    })
}

pub(crate) fn hover(value: Option<HoverSnippet>) -> Value {
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

pub(crate) fn sub_subcommand(sub: &SubSubCommand) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("name".into(), json!(sub.name));
    d.insert("detail".into(), json!(sub.detail));
    d.insert("synopsis".into(), json!(sub.synopsis));
    d.insert("surface".into(), dialects(sub.surface));
    let mut lost = Unrecovered::default();
    // `null` — declares nothing, inherits the subcommand's table — is a
    // different draft value from `[]`, which declares that there are no
    // options here at all (issue #1610).
    d.insert(
        "options".into(),
        sub.options
            .map_or(Value::Null, |options| option_rows(options, &mut lost)),
    );
    let complete = insert_lifecycle(&mut d, sub.lifecycle) && lost.is_empty();
    (Value::Object(d), complete)
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

/// The Rust expression for an `Option<u8>` field of a descriptor literal.
fn opt_u8_expr(value: Option<u8>) -> String {
    value.map_or_else(|| "None".to_owned(), |n| format!("Some({n})"))
}

/// The Rust expression for an `Option<&'static str>` field of a descriptor
/// literal.
fn opt_str_expr(value: Option<&'static str>) -> String {
    value.map_or_else(
        || "None".to_owned(),
        |s| format!("Some({})", rust_string(s)),
    )
}

/// The Rust expression for a [`Lifecycle`], spelled as a struct literal.
///
/// The literal names all three releases rather than chaining the
/// `introduced_in(…).deprecated_from(…)` builders, so a release the type gains
/// later cannot be dropped silently — [`crate::coverage`] checks the rendered
/// text mentions every field.
fn lifecycle_expr(value: Lifecycle) -> Option<String> {
    let deprecation_fix = match value.deprecation_fix {
        Some(hook) => deprecation_fix_expr(hook)?,
        None => "None".to_owned(),
    };
    Some(format!(
        "Lifecycle {{ introduced: {}, deprecated: {}, retired: {}, deprecation_fix: {} }}",
        opt_str_expr(value.introduced),
        opt_str_expr(value.deprecated),
        opt_str_expr(value.retired),
        deprecation_fix,
    ))
}

/// The Rust expression for one [`RepeatedArgLayout`].
///
/// Written as a full struct literal rather than one of the `every` / `strided`
/// constructors: a constructor hides the fields it defaults, and hiding a field
/// is exactly the drift this module exists to prevent.
fn repeated_arg_layout_expr(layout: RepeatedArgLayout) -> String {
    format!(
        "RepeatedArgLayout {{ role: {}, start: {}, stride: {}, exclude_trailing: {}, \
         optional_leading_word: {}, conditional_binding: {} }}",
        catalogue::qualified_variant("ArgRole", &layout.role),
        layout.start,
        layout.stride,
        layout.exclude_trailing,
        layout.optional_leading_word,
        layout.conditional_binding,
    )
}

/// The Rust expression for a `&'static [RepeatedArgLayout]` field.
fn repeated_args_expr(layouts: &[RepeatedArgLayout]) -> String {
    let items: Vec<String> = layouts
        .iter()
        .copied()
        .map(repeated_arg_layout_expr)
        .collect();
    format!("&[{}]", items.join(", "))
}

/// The Rust expression for a [`HandleName`].
fn handle_name_expr(name: HandleName) -> String {
    match name {
        HandleName::Word(i) => format!("HandleName::Word({i})"),
        HandleName::Implicit(v) => format!("HandleName::Implicit({})", rust_string(v)),
    }
}

/// The Rust expression for a [`HandleClassSource`].
fn handle_class_source_expr(source: HandleClassSource) -> String {
    match source {
        HandleClassSource::Word(i) => format!("HandleClassSource::Word({i})"),
        HandleClassSource::ConstructionValue(i) => {
            format!("HandleClassSource::ConstructionValue({i})")
        }
    }
}

/// The Rust expression for a [`HandleKeyword`].
fn handle_keyword_expr(keyword: HandleKeyword) -> String {
    format!(
        "HandleKeyword {{ at: {}, word: {} }}",
        keyword.at,
        rust_string(keyword.word)
    )
}

/// The Rust expression for a [`HandleBindingSpec`] reference, wrapped in
/// `Some(&…)`.
///
/// The whole descriptor is plain data (two indices, a fieldless-payload enum,
/// and an optional keyword), so it round-trips: `&`-borrowing a constant struct
/// literal promotes to the `&'static` the field wants (issue #1185).
fn handle_binding_expr(spec: &HandleBindingSpec) -> String {
    format!(
        "Some(&HandleBindingSpec {{ name_from: {}, class_from: {}, keyword: {} }})",
        handle_name_expr(spec.name_from),
        handle_class_source_expr(spec.class_from),
        spec.keyword.map_or_else(
            || "None".to_owned(),
            |k| format!("Some({})", handle_keyword_expr(k))
        ),
    )
}

/// The Rust expression for a [`RemoteMethodRole`] reference, wrapped in
/// `Some(&…)`.
///
/// Like [`handle_binding_expr`], the whole descriptor is plain data — two
/// fieldless-payload enums and a couple of indices — so `&`-borrowing the
/// struct literal promotes to the `&'static` the field wants and the value
/// round-trips through the draft (issue #1707).
fn remote_method_expr(role: RemoteMethodRole) -> String {
    let inner = match role {
        RemoteMethodRole::OpensHandle(spec) => format!(
            "RemoteMethodRole::OpensHandle(RemoteHandleSpec {{ family: {}, scope_arg: {}, \
             extension_arg: {}, exact_argc: {} }})",
            remote_family_expr(spec.family),
            spec.scope_arg,
            spec.extension_arg,
            spec.exact_argc,
        ),
        RemoteMethodRole::CallsMethod(spec) => format!(
            "RemoteMethodRole::CallsMethod(RemoteCallSpec {{ family: {}, handle_arg: {}, \
             method: {}, dispatch: {} }})",
            remote_family_expr(spec.family),
            spec.handle_arg,
            match spec.method {
                MethodWord::At(index) => format!("MethodWord::At({index})"),
                MethodWord::AfterOptions(index) => format!("MethodWord::AfterOptions({index})"),
            },
            match spec.dispatch {
                RemoteDispatch::Synchronous => "RemoteDispatch::Synchronous",
                RemoteDispatch::Notification => "RemoteDispatch::Notification",
            },
        ),
    };
    format!("Some(&{inner})")
}

/// The Rust expression for one [`RemoteFamily`] variant.
fn remote_family_expr(family: RemoteFamily) -> &'static str {
    match family {
        RemoteFamily::IRulesLxNode => "RemoteFamily::IRulesLxNode",
    }
}

/// The Rust expression for a [`BytePayloadSpec`], wrapped in `Some(…)`.
fn byte_payload_expr(spec: BytePayloadSpec) -> String {
    format!(
        "Some(BytePayloadSpec {{ replace_data_index: {}, message_flag_shift: {} }})",
        spec.replace_data_index, spec.message_flag_shift,
    )
}

/// The Rust expression for a [`SymbolDef`], wrapped in `Some(…)`.
fn symbol_def_expr(def: SymbolDef) -> String {
    format!(
        "Some(SymbolDef {{ name_arg: {}, detail_arg: {}, requires_arg: {}, kind: {} }})",
        def.name_arg,
        opt_u8_expr(def.detail_arg),
        opt_u8_expr(def.requires_arg),
        catalogue::qualified_variant("DefinedSymbolKind", &def.kind),
    )
}

/// The Rust expression for a `&'static [(&'static str, OoContextFact)]` field.
fn oo_context_facts_expr(facts: &[(&'static str, OoContextFact)]) -> String {
    let items: Vec<String> = facts
        .iter()
        .map(|(word, fact)| {
            format!(
                "({}, {})",
                rust_string(word),
                catalogue::qualified_variant("OoContextFact", fact)
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

/// The Rust expression for a `&'static [VersionedArgValue]` field.
fn versioned_arg_values_expr(gates: &[VersionedArgValue]) -> Option<String> {
    let items: Option<Vec<String>> = gates
        .iter()
        .map(|gate| {
            Some(format!(
                "VersionedArgValue {{ index: {}, value: {}, lifecycle: {} }}",
                gate.index,
                rust_string(gate.value),
                lifecycle_expr(gate.lifecycle)?,
            ))
        })
        .collect();
    Some(format!("&[{}]", items?.join(", ")))
}

fn representation_effect_expr(effect: RepresentationEffect) -> String {
    match effect {
        RepresentationEffect::None => "Some(RepresentationEffect::None)".to_owned(),
        RepresentationEffect::CopyOnWriteContainerMutation {
            variable_arg,
            minimum_arguments,
        } => format!(
            "Some(RepresentationEffect::CopyOnWriteContainerMutation {{ variable_arg: {variable_arg}, minimum_arguments: {minimum_arguments} }})"
        ),
    }
}

fn dialect_set_expr(set: &'static [SpecSurface]) -> String {
    let members = dialects(Some(set));
    crate::render_rs::dialect_set(
        members
            .as_array()
            .expect("a present dialect set always seeds an array"),
    )
}

/// The Rust expression for one [`tcl_registry::OptionTerm`].
fn relation_term_expr(term: tcl_registry::OptionTerm) -> String {
    use tcl_registry::OptionTerm as T;
    match term {
        T::Option(name) => format!("OptionTerm::Option({})", rust_string(name)),
        T::OptionValue(name, value) => format!(
            "OptionTerm::OptionValue({}, {})",
            rust_string(name),
            rust_string(value)
        ),
        T::Argument(index) => format!("OptionTerm::Argument({index})"),
        T::ArgumentValue(index, value) => {
            format!("OptionTerm::ArgumentValue({index}, {})", rust_string(value))
        }
    }
}

/// The Rust expression for a `&'static [OptionRelation]` field.
///
/// A full struct literal per relation, `lifecycle` included: a relation can
/// itself be gated — it only exists once both its operands do.
fn option_relations_expr(constraints: &[OptionRelation]) -> Option<String> {
    let items: Option<Vec<String>> = constraints
        .iter()
        .map(|relation| {
            let terms = relation
                .terms
                .iter()
                .copied()
                .map(relation_term_expr)
                .collect::<Vec<_>>()
                .join(", ");
            let subject = relation.subject.map_or_else(
                || "None".to_owned(),
                |term| format!("Some({})", relation_term_expr(term)),
            );
            let dialects = relation.surface.map_or_else(
                || "None".to_owned(),
                |set| format!("Some({})", dialect_set_expr(set)),
            );
            let message = relation.message.map_or_else(
                || "None".to_owned(),
                |text| format!("Some({})", rust_string(text)),
            );
            Some(format!(
                "OptionRelation {{ kind: RelationKind::{:?}, mode: RelationMode::{:?}, \
                 subject: {subject}, terms: &[{terms}], surface: {dialects}, \
                 lifecycle: {}, message: {message} }}",
                relation.kind,
                relation.mode,
                lifecycle_expr(relation.lifecycle)?,
            ))
        })
        .collect();
    Some(format!("&[{}]", items?.join(", ")))
}

/// A draft value for a descriptor field: the rendered Rust expression when the
/// field is set, `null` when it is at its default.
///
/// `null` is what the schema's `RustExpr` editor shows as "unset", and what the
/// renderer reads as "leave it to `..DEFAULT`" — so an unset descriptor stays
/// out of the emitted spec exactly as before.
fn expr_or_null(set: bool, render: impl FnOnce() -> String) -> Value {
    if set { json!(render()) } else { Value::Null }
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

    /// Nothing was lost — the "complete" answer a nested row reports upward
    /// when it seeds a sub-draft of its own (a second-level subcommand's
    /// option table) rather than sharing its parent's accumulator.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
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
    d.insert("arity_windows".into(), arity_windows(sub.arity_windows));
    d.insert("detail".into(), json!(sub.detail));
    d.insert("synopsis".into(), json!(sub.synopsis));
    d.insert("hover".into(), hover(sub.hover));
    d.insert("arg_roles".into(), role_map(sub.arg_roles));
    d.insert(
        "arg_presentation".into(),
        presentation_map(sub.arg_presentation),
    );
    d.insert(
        "repeated_args".into(),
        expr_or_null(!sub.repeated_args.is_empty(), || {
            repeated_args_expr(sub.repeated_args)
        }),
    );
    d.insert(
        "arg_role_resolver".into(),
        lost.expr("arg_role_resolver", sub.arg_role_resolver.is_some()),
    );
    d.insert("command_prefixes".into(), prefix_map(sub.command_prefixes));
    d.insert(
        "callback_taint_inputs".into(),
        callback_taint_map(sub.callback_taint_inputs),
    );
    d.insert(
        "command_prefix_resolver".into(),
        lost.expr(
            "command_prefix_resolver",
            sub.command_prefix_resolver.is_some(),
        ),
    );
    d.insert(
        "script_timing_resolver".into(),
        lost.expr(
            "script_timing_resolver",
            sub.script_timing_resolver.is_some(),
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
    d.insert(
        "representation_effect".into(),
        sub.representation_effect
            .map_or(Value::Null, |e| json!(representation_effect_expr(e))),
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
        "semantic_operation".into(),
        lost.expr("semantic_operation", sub.semantic_operation.is_some()),
    );
    d.insert(
        "completion".into(),
        lost.expr("completion", sub.completion.is_some()),
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
        "analyser_hook".into(),
        sub.analyser_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "command_table_effect".into(),
        sub.command_table_effect
            .map_or(Value::Null, |e| json!(catalogue::variant_name(&e))),
    );
    d.insert(
        "world_effects".into(),
        lost.expr("world_effects", sub.world_effects.is_some()),
    );
    d.insert(
        "state_transitions".into(),
        lost.expr("state_transitions", sub.state_transitions.is_some()),
    );
    d.insert(
        "dispatch_dependencies".into(),
        lost.expr("dispatch_dependencies", sub.dispatch_dependencies.is_some()),
    );
    d.insert(
        "result_stability".into(),
        lost.expr("result_stability", sub.result_stability.is_some()),
    );
    d.insert(
        "constraints".into(),
        lost.expr("constraints", sub.constraints.is_some()),
    );
    d.insert(
        "literal_argument_validator".into(),
        lost.expr(
            "literal_argument_validator",
            sub.literal_argument_validator.is_some(),
        ),
    );
}

/// The `command_forms` / `subcommand_forms` table as draft rows.
fn refinement_rows(forms: &[CommandForm], key: &'static str, lost: &mut Unrecovered) -> Value {
    Value::Array(
        forms
            .iter()
            .map(|form| {
                let (row, complete) = command_form(form, lost);
                if !complete {
                    lost.note(key);
                }
                row
            })
            .collect(),
    )
}

/// One invocation refinement (`CommandForm` / `SubCommandForm`) as draft rows
/// (design Q12/D2).
///
/// The overlay fields are tri-state — absent inherits, present replaces — so a
/// `null` here is a real declaration, not a missing one. The descriptor's
/// remaining native halves (completion, dispatch proofs, the literal-argument
/// validator, the compiler hook ids) have no recoverable expression, exactly
/// as they do not at command scope, so a form that carries one is noted lost
/// rather than silently thinned.
fn command_form(form: &CommandForm, lost: &mut Unrecovered) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("name".into(), json!(form.name));
    d.insert("arity".into(), arity(form.arity));
    d.insert(
        "selector".into(),
        form.literal_argument_prefix.map_or(Value::Null, |prefix| {
            json!({
                "words": str_list(prefix.words),
                "prefix_matching": catalogue::variant_name(&prefix.prefix_matching),
            })
        }),
    );
    d.insert("arg_roles".into(), role_map(form.arg_roles));
    d.insert("options".into(), option_rows(form.options, lost));
    d.insert(
        "option_relations".into(),
        if form.option_relations.is_empty() {
            Value::Null
        } else {
            option_relations_expr(form.option_relations).map_or(Value::Null, |expr| json!(expr))
        },
    );
    d.insert("surface".into(), dialects(form.surface));
    d.insert("traits".into(), form.traits.map_or(Value::Null, traits));
    d.insert(
        "mutator".into(),
        form.mutator.map_or(Value::Null, |flag| json!(flag)),
    );
    d.insert(
        "side_effects".into(),
        form.side_effects.map_or(Value::Null, |effects| {
            Value::Array(effects.iter().map(|effect| side_effect(effect).0).collect())
        }),
    );
    d.insert(
        "representation_effect".into(),
        form.representation_effect
            .map_or(Value::Null, |e| json!(representation_effect_expr(e))),
    );
    d.insert(
        "lowering_hook".into(),
        form.lowering_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "codegen_hook".into(),
        form.codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    // What is left of the descriptor is native: a compiler proof or a
    // function pointer, with no expression to recover.
    let native = form.semantic_operation.is_some()
        || form.completion.is_some()
        || form.result_stability.is_some()
        || form.world_effects.is_some()
        || form.state_transitions.is_some()
        || form.dispatch_dependencies.is_some()
        || form.literal_argument_validator.is_some();
    (Value::Object(d), !native)
}

fn option_rows(options: &[OptionSpec], lost: &mut Unrecovered) -> Value {
    Value::Array(
        options
            .iter()
            .map(|option| {
                let (row, complete) = option_spec(option);
                if !complete.arity_hook {
                    lost.note(OPTION_HOOK_KEY);
                }
                if !complete.deprecation_fix_hook {
                    lost.note(OPTION_DEPRECATION_FIX_HOOK_KEY);
                }
                row
            })
            .collect(),
    )
}

/// Options, values, availability, behaviour flags, taint, and effects.
/// The subcommand's option table and the E-R14 relation fields over it.
///
/// Split out of [`subcommand_rest`] so neither grows past the line budget:
/// the option surface is one topic and the rest of the descriptor is another.
fn subcommand_option_surface(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert("options".into(), option_rows(sub.options, lost));
    let option_relations = if sub.option_relations.is_empty() {
        Value::Null
    } else {
        option_relations_expr(sub.option_relations)
            .map_or_else(|| lost.expr("option_relations", true), |expr| json!(expr))
    };
    d.insert("option_relations".into(), option_relations);
    d.insert(
        "option_placement".into(),
        json!(catalogue::variant_name(&sub.option_placement)),
    );
}

fn subcommand_rest(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    subcommand_option_surface(d, sub, lost);
    d.insert("min_abbrev".into(), opt_index(sub.min_abbrev));
    d.insert(
        "prefix_matching".into(),
        json!(catalogue::variant_name(&sub.prefix_matching)),
    );
    d.insert("arg_values".into(), arg_value_map(sub.arg_values, lost));
    let versioned_arg_values = if sub.versioned_arg_values.is_empty() {
        Value::Null
    } else {
        versioned_arg_values_expr(sub.versioned_arg_values).map_or_else(
            || lost.expr("versioned_arg_values", true),
            |expr| json!(expr),
        )
    };
    d.insert("versioned_arg_values".into(), versioned_arg_values);
    d.insert(
        "subcommand_forms".into(),
        refinement_rows(sub.subcommand_forms, "subcommand_forms", lost),
    );
    d.insert("surface".into(), dialects(sub.surface));
    if !insert_lifecycle(d, sub.lifecycle) {
        lost.note("deprecation_fix");
    }
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
    d.insert(
        "side_effects".into(),
        nested_rows(sub.side_effects, "side_effects", lost, side_effect),
    );
    d.insert("destructive".into(), json!(sub.destructive));
    d.insert("returns_path".into(), json!(sub.returns_path));
    d.insert("is_unescape".into(), json!(sub.is_unescape));
    d.insert("cfg_rewrite_name".into(), opt_str(sub.cfg_rewrite_name));
    d.insert(
        "sub_subcommands".into(),
        nested_rows(sub.sub_subcommands, "sub_subcommands", lost, sub_subcommand),
    );
    d.insert(
        "defines_command_at".into(),
        opt_index(sub.defines_command_at),
    );
    d.insert(
        "max_leading_option_words".into(),
        opt_index(sub.max_leading_option_words),
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
    d.insert("surface".into(), dialects(spec.surface));
    d.insert("arity".into(), arity(spec.arity));
    d.insert("arity_windows".into(), arity_windows(spec.arity_windows));
    d.insert("arg_roles".into(), role_map(spec.arg_roles));
    d.insert(
        "arg_presentation".into(),
        presentation_map(spec.arg_presentation),
    );
    d.insert(
        "repeated_args".into(),
        expr_or_null(!spec.repeated_args.is_empty(), || {
            repeated_args_expr(spec.repeated_args)
        }),
    );
    d.insert(
        "arg_role_resolver".into(),
        lost.expr("arg_role_resolver", spec.arg_role_resolver.is_some()),
    );
    d.insert(
        "frame_effect".into(),
        lost.expr("frame_effect", spec.frame_effect.is_some()),
    );
    d.insert("bpf_op".into(), lost.expr("bpf_op", spec.bpf_op.is_some()));
    d.insert(
        "clause_shape_check".into(),
        lost.expr("clause_shape_check", spec.clause_shape_check.is_some()),
    );
    d.insert("command_prefixes".into(), prefix_map(spec.command_prefixes));
    d.insert(
        "callback_taint_inputs".into(),
        callback_taint_map(spec.callback_taint_inputs),
    );
    d.insert(
        "command_prefix_resolver".into(),
        lost.expr(
            "command_prefix_resolver",
            spec.command_prefix_resolver.is_some(),
        ),
    );
    d.insert(
        "script_timing_resolver".into(),
        lost.expr(
            "script_timing_resolver",
            spec.script_timing_resolver.is_some(),
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
        "return_type_hook".into(),
        spec.return_type_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
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
    d.insert(
        "representation_effect".into(),
        spec.representation_effect
            .map_or(Value::Null, |e| json!(representation_effect_expr(e))),
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
        "option_placement".into(),
        json!(catalogue::variant_name(&spec.option_placement)),
    );
    d.insert(
        "prefix_matching".into(),
        json!(catalogue::variant_name(&spec.prefix_matching)),
    );
    d.insert(
        "default_form_first_word".into(),
        spec.default_form_first_word
            .map_or(Value::Null, |w| json!(catalogue::variant_name(&w))),
    );
    d.insert("hover".into(), hover(spec.hover));
    d.insert(
        "forms".into(),
        nested_rows(spec.forms, "forms", lost, form_spec),
    );
    d.insert(
        "command_forms".into(),
        refinement_rows(spec.command_forms, "command_forms", lost),
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
        "semantic_operation".into(),
        lost.expr("semantic_operation", spec.semantic_operation.is_some()),
    );
    d.insert(
        "completion".into(),
        lost.expr("completion", spec.completion.is_some()),
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
        nested_rows(spec.side_effects, "side_effects", lost, side_effect),
    );
    d.insert(
        "world_effects".into(),
        lost.expr("world_effects", spec.world_effects.is_some()),
    );
    d.insert(
        "state_transitions".into(),
        lost.expr("state_transitions", spec.state_transitions.is_some()),
    );
    d.insert(
        "dispatch_dependencies".into(),
        lost.expr(
            "dispatch_dependencies",
            spec.dispatch_dependencies.is_some(),
        ),
    );
    d.insert(
        "result_stability".into(),
        lost.expr("result_stability", spec.result_stability.is_some()),
    );
    d.insert(
        "constraints".into(),
        lost.expr("constraints", spec.constraints.is_some()),
    );
    d.insert(
        "literal_argument_validator".into(),
        lost.expr(
            "literal_argument_validator",
            spec.literal_argument_validator.is_some(),
        ),
    );
    d.insert(
        "inferred_storage_type".into(),
        spec.inferred_storage_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
}

/// Options, enumerable values, and availability gating.
fn command_options(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("required_package".into(), opt_str(spec.required_package));
    d.insert(
        "tk_geometry".into(),
        spec.tk_geometry.map_or(Value::Null, |geometry| {
            use tcl_registry::tk_geometry::TkGeometryContainerPolicy;
            let container_policy = match geometry.container_policy {
                TkGeometryContainerPolicy::Exclusive => "Exclusive",
                TkGeometryContainerPolicy::Independent => "Independent",
            };
            json!({
                "container_policy": container_policy,
                "container_option": geometry.container_option,
                "direct_form": geometry.direct_form,
                "placement_subcommand": geometry.placement_subcommand,
                "release_subcommands": geometry.release_subcommands,
            })
        }),
    );
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
    d.insert(
        "event_requirement_forms".into(),
        lost.expr(
            "event_requirement_forms",
            !spec.event_requirement_forms.is_empty(),
        ),
    );
    d.insert(
        "data_collection".into(),
        lost.expr("data_collection", spec.data_collection.is_some()),
    );
    d.insert(
        "side_switch_target".into(),
        lost.expr("side_switch_target", spec.side_switch_target.is_some()),
    );
    d.insert(
        "event_handler_priority".into(),
        lost.expr(
            "event_handler_priority",
            spec.event_handler_priority.is_some(),
        ),
    );
    d.insert(
        "irules_top_level_effect".into(),
        lost.expr(
            "irules_top_level_effect",
            spec.irules_top_level_effect.is_some(),
        ),
    );
    d.insert("options".into(), option_rows(spec.options, lost));
    let option_relations = if spec.option_relations.is_empty() {
        Value::Null
    } else {
        option_relations_expr(spec.option_relations)
            .map_or_else(|| lost.expr("option_relations", true), |expr| json!(expr))
    };
    d.insert("option_relations".into(), option_relations);
    d.insert(
        "reserved_trailing_words".into(),
        json!(spec.reserved_trailing_words),
    );
    d.insert("arg_values".into(), arg_value_map(spec.arg_values, lost));
    let versioned_arg_values = if spec.versioned_arg_values.is_empty() {
        Value::Null
    } else {
        versioned_arg_values_expr(spec.versioned_arg_values).map_or_else(
            || lost.expr("versioned_arg_values", true),
            |expr| json!(expr),
        )
    };
    d.insert("versioned_arg_values".into(), versioned_arg_values);
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
fn command_manufacturer_methods(d: &mut Draft, spec: &CommandSpec) {
    d.insert(
        "manufacturer_methods".into(),
        Value::Array(
            spec.manufacturer_methods
                .iter()
                .map(manufacturer_method)
                .collect(),
        ),
    );
}

fn command_advanced(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert(
        "pattern_type".into(),
        spec.pattern_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "pattern_arg_resolver".into(),
        lost.expr("pattern_arg_resolver", spec.pattern_arg_resolver.is_some()),
    );
    d.insert(
        "format_string_type".into(),
        spec.format_string_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert("tcllib_package".into(), opt_str(spec.tcllib_package));
    if !insert_lifecycle(d, spec.lifecycle) {
        lost.note("deprecation_fix");
    }
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
        spec.byte_array_payload
            .map_or(Value::Null, |p| json!(byte_payload_expr(p))),
    );
    d.insert(
        "byte_array_effect".into(),
        json!(catalogue::variant_name(&spec.byte_array_effect)),
    );
    d.insert(
        "definition_body".into(),
        lost.expr("definition_body", spec.definition_body.is_some()),
    );
    command_manufacturer_methods(d, spec);
    d.insert(
        "case_list".into(),
        lost.expr("case_list", spec.case_list.is_some()),
    );
    d.insert(
        "oo_context_facts".into(),
        expr_or_null(!spec.oo_context_facts.is_empty(), || {
            oo_context_facts_expr(spec.oo_context_facts)
        }),
    );
    d.insert(
        "self_receiver_words".into(),
        str_list(spec.self_receiver_words),
    );
    command_object_facts(d, spec, lost);
}

/// The object / definition half of the advanced group: what a command
/// *defines* (a class, an outline symbol, a handle, a remote method) and the
/// context it defines it in.
///
/// Split out of [`command_advanced`] only to keep each within the hundred-line
/// budget; the two are one pass over one group.
fn command_object_facts(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("object_class".into(), object_class(spec.object_class, lost));
    d.insert(
        "defines_symbol".into(),
        spec.defines_symbol
            .map_or(Value::Null, |s| json!(symbol_def_expr(s))),
    );
    d.insert(
        "body_scope".into(),
        lost.expr("body_scope", spec.body_scope.is_some()),
    );
    d.insert(
        "binds_handle".into(),
        spec.binds_handle
            .map_or(Value::Null, |b| json!(handle_binding_expr(b))),
    );
    d.insert(
        "remote_method".into(),
        spec.remote_method
            .map_or(Value::Null, |role| json!(remote_method_expr(*role))),
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
}
