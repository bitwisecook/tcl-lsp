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

//! Render a draft as a `tcl-registry` command-spec source file.
//!
//! The output is a complete, drop-in `rust/tcl-registry/src/commands/<pack>/`
//! module: the project's AGPL copyright banner, a module doc line, the
//! imports the emitted literals need, hoisted `const` arrays for the option /
//! form / subcommand tables, and a `spec()` returning the `CommandSpec`.
//!
//! Only fields that differ from `CommandSpec::DEFAULT` are emitted; the rest
//! come from the trailing `..CommandSpec::DEFAULT`, matching how every
//! hand-written spec in the registry is laid out.

use std::fmt::Write as _;

use serde_json::Value;

use crate::draft::{self, Draft, UNRENDERABLE_KEY};
use crate::schema::{self, FieldKind, FieldSchema};

mod definition_grammar;
mod field_values;
mod options_and_forms;
mod pack_module;

use definition_grammar::{
    clause_grammar_expr, definition_body_expr, runtime_backing_expr, semantic_operation_expr,
    tk_geometry_expr,
};
#[cfg(test)]
use field_values::dialect_constant;
pub(crate) use field_values::dialect_set;
use field_values::{
    arg_type_map_expr, arg_value_map_expr, arity_expr, body_interpreter_expr,
    callback_taint_map_expr, domain_map_expr, enum_slice, flag_union, index_slice, prefix_map_expr,
    presentation_map_expr, role_map_expr, variable_receivers_expr,
};
pub(crate) use options_and_forms::knowledge_expr;
use options_and_forms::{
    LifecycleLine, command_form_expr, dialects_line, form_expr, lifecycle_expr, lifecycle_line,
    option_expr, row_literal, side_effect_expr, unfilled_option_deprecation_hooks,
    unfilled_option_hooks,
};
pub use pack_module::{ModuleForm, render_pack_module};

/// The project's AGPL-3.0 copyright banner, as it appears at the top of every
/// original Rust source file in the repository.
pub const COPYRIGHT_BANNER: &str = "\
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
// SPDX-License-Identifier: AGPL-3.0-or-later";

/// Escape `s` as the body of a Rust double-quoted string literal.
///
/// Shared with [`crate::draft`], which renders the same literals when it seeds
/// a descriptor field as a Rust expression — one escaper, so a `"` in a
/// keyword word cannot compile in one path and not the other.
pub(crate) fn rust_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn as_str(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

fn as_u64(value: &Value) -> u64 {
    value.as_u64().unwrap_or_default()
}

fn as_bool(value: &Value) -> bool {
    value.as_bool().unwrap_or_default()
}

fn as_array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

/// Render a `&'static [&'static str]` literal.
fn str_slice(values: &[Value]) -> String {
    let items: Vec<String> = values.iter().map(|v| rust_string(as_str(v))).collect();
    format!("&[{}]", items.join(", "))
}

fn setter_constraint_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    format!(
        "{indent}SetterConstraint {{\n{inner}arg_index: {},\n{inner}required_prefix: {},\n{inner}code: DiagCode::{},\n{inner}message: {},\n{indent}}},",
        as_u64(&entry["arg_index"]),
        rust_string(as_str(&entry["required_prefix"])),
        as_str(&entry["code"]),
        rust_string(as_str(&entry["message"])),
    )
}

fn manufacturer_method_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let opt_index = |key: &str| {
        entry[key]
            .as_u64()
            .map_or_else(|| "None".to_owned(), |n| format!("Some({n})"))
    };
    format!(
        "{indent}crate::definer::ManufacturerMethod {{\n{inner}keyword: {},\n{inner}visibility: crate::definer::MemberVisibility::{},\n{inner}names_instance_at: {},\n{inner}definition_body_at: {},\n{inner}constructor_args_from: {},\n{indent}}},",
        rust_string(as_str(&entry["keyword"])),
        as_str(&entry["visibility"]),
        opt_index("names_instance_at"),
        opt_index("definition_body_at"),
        as_u64(&entry["constructor_args_from"]),
    )
}

fn documentation_example_expr(value: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let notes: Vec<String> = as_array(&value["notes"])
        .iter()
        .map(|note| {
            format!(
                "{inner}    crate::documentation::DocumentationAnnotation::new({}, {}, {}),",
                note["line"].as_u64().unwrap_or(0),
                rust_string(as_str(&note["needle"])),
                rust_string(as_str(&note["label"])),
            )
        })
        .collect();
    let code = rust_string(as_str(&value["code"]));
    let notes = format!("&[\n{}\n{inner}]", notes.join("\n"));
    let carrier = &value["carrier"];
    if carrier.is_null() {
        format!(
            "{indent}crate::documentation::DocumentationExample::new(\n{inner}{code},\n{inner}{notes},\n{indent}),"
        )
    } else {
        format!(
            "{indent}crate::documentation::DocumentationExample::with_carrier(\n{inner}{code},\n{inner}crate::documentation::DocumentationCarrier::new({}, {}),\n{inner}{notes},\n{indent}),",
            carrier["line"].as_u64().unwrap_or(0),
            rust_string(as_str(&carrier["needle"])),
        )
    }
}

fn hover_expr(value: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let synopsis = str_slice(as_array(&value["synopsis"]));
    format!(
        "Some(HoverSnippet {{\n{inner}summary: {},\n{inner}synopsis: {synopsis},\n{inner}snippet: {},\n{inner}source: {},\n{inner}examples: {},\n{inner}return_value: {},\n{indent}}})",
        rust_string(as_str(&value["summary"])),
        rust_string(as_str(&value["snippet"])),
        rust_string(as_str(&value["source"])),
        rust_string(as_str(&value["examples"])),
        rust_string(as_str(&value["return_value"])),
    )
}

fn sub_subcommand_expr(entry: &Value, indent: &str) -> String {
    let mut parts = vec![format!(
        "{indent}    name: {},",
        rust_string(as_str(&entry["name"]))
    )];
    for key in ["detail", "synopsis"] {
        let text = as_str(&entry[key]);
        if !text.is_empty() {
            parts.push(format!("{indent}    {key}: {},", rust_string(text)));
        }
    }
    if let Some(prefix) = entry["option_prefix_words"]
        .as_u64()
        .filter(|prefix| *prefix > 0)
    {
        parts.push(format!("{indent}    option_prefix_words: {prefix},"));
    }
    parts.extend(dialects_line(entry, indent));
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    if entry
        .get("native_compilation")
        .is_some_and(|value| !value.is_null())
    {
        parts.push(format!(
            "{indent}    // Native worker compilation proof is excluded from this draft."
        ));
    }
    row_literal("SubSubCommand", &parts, indent)
}

/// Render a `&[...]` literal from `items`, one entry per line, or `None` when
/// the list is empty (the `..DEFAULT` fill already supplies an empty slice).
fn list_expr(
    items: &[Value],
    indent: &str,
    render_one: impl Fn(&Value, &str) -> String,
) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let inner = format!("{indent}    ");
    let rendered: Vec<String> = items.iter().map(|e| render_one(e, &inner)).collect();
    Some(format!("&[\n{}\n{indent}]", rendered.join("\n")))
}

/// Render one field's value expression, or `None` when it matches the
/// default and should be left to `..DEFAULT`.
fn field_expr(field: &FieldSchema, value: &Value, default: &Value, indent: &str) -> Option<String> {
    if value == default {
        return None;
    }
    if field.key == "variable_receivers" {
        return Some(variable_receivers_expr(value.as_array()?));
    }
    let expr = match field.kind {
        FieldKind::Bool => as_bool(value).to_string(),
        FieldKind::OptBool => format!("Some({})", value.as_bool()?),
        FieldKind::Count => as_u64(value).to_string(),
        FieldKind::OptCount | FieldKind::OptIndex => format!("Some({})", value.as_u64()?),
        FieldKind::Text | FieldKind::Prose => rust_string(as_str(value)),
        FieldKind::OptText => format!("Some({})", rust_string(value.as_str()?)),
        FieldKind::TextList => str_slice(as_array(value)),
        FieldKind::IndexList => index_slice(as_array(value)),
        FieldKind::OptIndexList => format!("Some({})", index_slice(value.as_array()?)),
        FieldKind::VariableReceivers => variable_receivers_expr(value.as_array()?),
        FieldKind::Enum {
            catalogue,
            optional,
        } => {
            let variant = value.as_str()?;
            let ty = enum_type_name(catalogue);
            let expr = tcl_spectcl::catalogue::rust_path(ty, variant);
            if optional {
                format!("Some({expr})")
            } else {
                expr
            }
        }
        FieldKind::FlagSet {
            catalogue,
            optional,
        } => {
            let ty = enum_type_name(catalogue);
            if optional {
                let items = value.as_array()?;
                if catalogue == "dialects" {
                    format!("Some({})", dialect_set(items))
                } else {
                    format!("Some({})", flag_union(ty, items))
                }
            } else {
                flag_union(ty, as_array(value))
            }
        }
        FieldKind::EnumList { catalogue } => enum_slice(enum_type_name(catalogue), as_array(value)),
        FieldKind::Arity => arity_expr(value),
        FieldKind::BodyInterpreter => body_interpreter_expr(value)?,
        FieldKind::RoleMap => role_map_expr(as_array(value)),
        FieldKind::PrefixMap => prefix_map_expr(as_array(value)),
        FieldKind::CallbackTaintMap => callback_taint_map_expr(as_array(value)),
        FieldKind::PresentationMap => presentation_map_expr(as_array(value)),
        FieldKind::DomainMap => domain_map_expr(as_array(value)),
        FieldKind::ArgTypeMap => arg_type_map_expr(as_array(value), indent),
        FieldKind::ArgValueMap => arg_value_map_expr(as_array(value), indent),
        FieldKind::Options => "OPTIONS".to_owned(),
        FieldKind::Forms => "FORMS".to_owned(),
        FieldKind::Refinements => list_expr(as_array(value), indent, command_form_expr)?,
        FieldKind::SubCommands => "SUBCOMMANDS".to_owned(),
        FieldKind::SideEffects => list_expr(as_array(value), indent, side_effect_expr)?,
        FieldKind::Knowledge => return None,
        FieldKind::SetterConstraints => list_expr(as_array(value), indent, setter_constraint_expr)?,
        FieldKind::ManufacturerMethods => {
            list_expr(as_array(value), indent, manufacturer_method_expr)?
        }
        FieldKind::SubSubCommands => list_expr(as_array(value), indent, sub_subcommand_expr)?,
        // The descriptor is hoisted to a `static` beside the command, the way
        // every shipped class spec is written; the field just names it.
        FieldKind::ObjectClass => "Some(&OBJECT_CLASS)".to_owned(),
        FieldKind::TkGeometry => tk_geometry_expr(value)?,
        FieldKind::ClauseGrammar => clause_grammar_expr(value, indent)?,
        FieldKind::DefinitionBody => definition_body_expr(value, indent)?,
        FieldKind::SemanticOperation => semantic_operation_expr(value)?,
        FieldKind::RuntimeBacking => runtime_backing_expr(value)?,
        FieldKind::Hover => {
            if value.is_null() {
                return None;
            }
            hover_expr(value, indent)
        }
        FieldKind::DocumentationExamples => {
            list_expr(as_array(value), indent, documentation_example_expr)?
        }
        FieldKind::RustExpr { .. } => {
            let expr = value.as_str()?.trim();
            if expr.is_empty() {
                return None;
            }
            expr.to_owned()
        }
    };
    Some(expr)
}

/// The Rust type name behind a schema catalogue id.
fn enum_type_name(catalogue: &str) -> &'static str {
    match catalogue {
        "argRole" => "ArgRole",
        "tclType" => "TclType",
        "bodyKind" => "BodyKind",
        "scriptLookupScope" => "ScriptLookupScope",
        "irulesSourceFlow" => "tcl_registry::events::IrulesSourceFlowSpec",
        "storageType" => "StorageType",
        "byteArrayEffect" => "ByteArrayEffect",
        "commandTableEffect" => "CommandTableEffect",
        "patternType" => "PatternType",
        "sourcePathOperation" => "SourcePathOperation",
        "sourceConfigurationReference" => "SourceConfigurationReferenceLayout",
        "sourceIndexBounds" => "SourceIndexBounds",
        "channelConfiguration" => "ChannelConfigurationSpec",
        "formatType" => "FormatType",
        "formKind" => "FormKind",
        "definedSymbolKind" => "DefinedSymbolKind",
        "sideEffectTarget" => "SideEffectTarget",
        "connectionSide" => "ConnectionSide",
        "loweringHook" => "LoweringHookId",
        "codegenHook" => "CodegenHookId",
        "inlineCodegenHook" => "InlineCodegenHookId",
        "analyserHook" => "AnalyserHookId",
        "returnTypeHook" => "ReturnTypeHookId",
        "traits" => "Traits",
        "taintColour" => "TaintColour",
        "dialects" => "&'static [SpecSurface]",
        "defaultFormFirstWord" => "DefaultFormFirstWord",
        "sourceDeprecationAdvice" => "SourceDeprecationAdvice",
        "prefixMatching" => "PrefixMatching",
        "optionPlacement" => "OptionPlacement",
        _ => "Unknown",
    }
}

/// Render a `SubCommand` literal from a subcommand draft.
fn subcommand_literal(sub: &Value, default: &Draft, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let mut lines: Vec<String> = Vec::new();
    for field in schema::SUBCOMMAND_FIELDS {
        match lifecycle_line(field.key, sub, &inner) {
            LifecycleLine::Emit(line) => {
                if !line.is_empty() {
                    lines.push(line.trim_end_matches('\n').to_owned());
                }
                continue;
            }
            LifecycleLine::Skip => continue,
            LifecycleLine::Other => {}
        }
        // Nested tables are inlined here rather than hoisted — a subcommand's
        // option list belongs with the subcommand that declares it.
        let value = sub.get(field.key).unwrap_or(&Value::Null);
        let default_value = default.get(field.key).unwrap_or(&Value::Null);
        let expr = match field.kind {
            FieldKind::Options => {
                let items = crate::render_rs::as_array(value);
                if items.is_empty() {
                    None
                } else {
                    let opt_indent = format!("{inner}    ");
                    let rendered: Vec<String> =
                        items.iter().map(|o| option_expr(o, &opt_indent)).collect();
                    Some(format!("&[\n{}\n{inner}]", rendered.join("\n")))
                }
            }
            FieldKind::SideEffects => knowledge_expr(
                field_expr(field, value, default_value, &inner),
                sub.get("side_effects_knowledge").unwrap_or(&Value::Null),
            ),
            FieldKind::Knowledge if field.key == "taint_knowledge" => {
                knowledge_expr(Some("()".to_owned()), value)
            }
            _ => field_expr(field, value, default_value, &inner),
        };
        if let Some(expr) = expr {
            lines.push(format!("{inner}{}: {expr},", rust_field_name(field.key)));
        }
    }
    for note in unrenderable_notes(sub, &inner) {
        lines.push(note);
    }
    format!(
        "{indent}SubCommand {{\n{}\n{inner}..SubCommand::DEFAULT\n{indent}}},",
        lines.join("\n")
    )
}

/// `TODO` lines for the fields a seeded draft could not recover.
fn unrenderable_notes(draft: &Value, indent: &str) -> Vec<String> {
    as_array(draft.get(UNRENDERABLE_KEY).unwrap_or(&Value::Null))
        .iter()
        .filter_map(|key| {
            let key = key.as_str()?;
            // Option-arity hooks live in an option row, not a top-level field,
            // so they resolve against `options` and name the exact options
            // still outstanding. Filling every one clears the note.
            if key == crate::draft::OPTION_HOOK_KEY {
                let pending = unfilled_option_hooks(draft);
                if pending.is_empty() {
                    return None;
                }
                return Some(format!(
                    "{indent}// TODO: {} consumes a computed number of words \
                     (OptionArity::Hook); supply the hook under that option.",
                    pending
                        .iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
            }
            if key == crate::draft::OPTION_DEPRECATION_FIX_HOOK_KEY {
                let pending = unfilled_option_deprecation_hooks(draft);
                if pending.is_empty() {
                    return None;
                }
                return Some(format!(
                    "{indent}// TODO: {} uses a contextual deprecation quick-fix callback; \
                     supply the `Some(DeprecationFixHook::Custom {{ resolver: ... }})` expression \
                     under that option.",
                    pending
                        .iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
            }
            // Only note a field the author has not since filled in.
            let filled = draft
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty());
            (!filled).then(|| {
                format!(
                    "{indent}// TODO: `{key}` is set on the source command but its defining \
                     expression could not be recovered — supply it here."
                )
            })
        })
        .collect()
}

/// The set of extra imports the emitted literals need beyond the prelude.
fn extra_imports(draft: &Draft) -> Vec<&'static str> {
    let mut imports: Vec<&'static str> = Vec::new();
    let uses_inline_hook = |d: &Value| d.get("inline_codegen_hook").is_some_and(|v| !v.is_null());
    let mut needs_inline = uses_inline_hook(&Value::Object(draft.clone()));
    let mut needs_diag =
        !as_array(draft.get("setter_constraints").unwrap_or(&Value::Null)).is_empty();
    for sub in as_array(draft.get("subcommands").unwrap_or(&Value::Null)) {
        needs_inline = needs_inline || uses_inline_hook(sub);
        needs_diag = needs_diag
            || !as_array(sub.get("setter_constraints").unwrap_or(&Value::Null)).is_empty();
    }
    if needs_inline {
        imports.push("use crate::hooks::InlineCodegenHookId;");
    }
    if needs_diag {
        imports.push("use tcl_core_types::DiagCode;");
    }
    imports
}

/// A hoisted `const NAME: &[TYPE] = &[…];` table, or an empty string when the
/// draft declares no entries for it.
fn const_table(
    name: &str,
    ty: &str,
    items: &[Value],
    render_one: impl Fn(&Value) -> String,
) -> String {
    if items.is_empty() {
        return String::new();
    }
    let rendered: Vec<String> = items.iter().map(render_one).collect();
    format!(
        "const {name}: &[{ty}] = &[\n{}\n];\n\n",
        rendered.join("\n")
    )
}

/// The hoisted `static OBJECT_CLASS: ObjectClassSpec = …;` (and, when the
/// instance methods are not simply the command's own subcommands, the
/// `const OBJECT_CLASS_METHODS` table it points at), or an empty string when
/// the draft declares no class.
///
/// Eleven of the thirteen shipped class specs set `instance_methods:
/// SUBCOMMANDS` — a widget's instance methods *are* its subcommands — so that
/// case is written the way it is written by hand rather than duplicating the
/// table.
fn object_class_statics(draft: &Draft, default_sub: &Draft) -> String {
    let Some(class) = draft.get("object_class").filter(|v| !v.is_null()) else {
        return String::new();
    };
    let methods = as_array(&class["instance_methods"]);
    let subcommands = as_array(draft.get("subcommands").unwrap_or(&Value::Null));
    let (table, methods_expr) = if methods.is_empty() {
        (String::new(), "&[]".to_owned())
    } else if methods == subcommands {
        (String::new(), "SUBCOMMANDS".to_owned())
    } else {
        (
            const_table("OBJECT_CLASS_METHODS", "SubCommand", methods, |m| {
                subcommand_literal(m, default_sub, "    ")
            }),
            "OBJECT_CLASS_METHODS".to_owned(),
        )
    };
    // Older saved Studio drafts predate this field.  Preserve the registry's
    // conservative object-method default instead of emitting an invalid empty
    // enum variant when one is reopened and rendered.
    let method_prefix_matching = match as_str(&class["method_prefix_matching"]) {
        "Enabled" => "Enabled",
        _ => "Strict",
    };
    format!(
        "{table}static OBJECT_CLASS: ObjectClassSpec = ObjectClassSpec {{\n    class_name: \
         {},\n    instance_methods: {methods_expr},\n    superclasses: {},\n    \
         allow_unknown_methods: {},\n    method_prefix_matching: PrefixMatching::{},\n}};\n\n",
        rust_string(as_str(&class["class_name"])),
        str_slice(as_array(&class["superclasses"])),
        as_bool(&class["allow_unknown_methods"]),
        method_prefix_matching,
    )
}

/// The module doc line, from the draft's hover summary when it has one.
///
/// The summary is free text a user typed into a textarea, so it may contain
/// newlines. Every line has to carry its own `//!`: interpolating a multi-line
/// summary verbatim leaves the continuation lines as bare Rust and the
/// advertised drop-in file does not compile.
fn module_doc(name: &str, summary: &str) -> String {
    if summary.trim().is_empty() {
        return format!("//! `{name}`.\n");
    }
    let first = summary
        .split_once(". ")
        .map_or(summary, |(head, _)| head)
        .trim()
        .trim_end_matches('.');
    let mut lines = first.lines().map(str::trim).filter(|l| !l.is_empty());
    let Some(head) = lines.next() else {
        return format!("//! `{name}`.\n");
    };
    let mut out = format!("//! `{name}` — {}", lower_first(head));
    for line in lines {
        out.push_str("\n//! ");
        out.push_str(line);
    }
    out.push_str(".\n");
    out
}

/// Render `draft` as a complete `tcl-registry` command-spec source file.
#[must_use]
pub fn render(draft: &Draft) -> String {
    let default = draft::default_command_draft();
    let default_sub = draft::default_subcommand_draft();
    let name = draft.get("name").map_or("", |v| as_str(v));
    let summary = draft
        .get("hover")
        .and_then(|h| h.get("summary"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let mut out = String::new();
    out.push_str(COPYRIGHT_BANNER);
    out.push_str("\n\n");
    out.push_str(&module_doc(name, summary));
    out.push('\n');
    for import in extra_imports(draft) {
        out.push_str(import);
        out.push('\n');
    }
    out.push_str("use crate::prelude::*;\n\n");

    out.push_str(&const_table(
        "FORMS",
        "FormSpec",
        as_array(draft.get("forms").unwrap_or(&Value::Null)),
        |f| form_expr(f, "    "),
    ));
    out.push_str(&const_table(
        "OPTIONS",
        "OptionSpec",
        as_array(draft.get("options").unwrap_or(&Value::Null)),
        |o| option_expr(o, "    "),
    ));
    out.push_str(&const_table(
        "SUBCOMMANDS",
        "SubCommand",
        as_array(draft.get("subcommands").unwrap_or(&Value::Null)),
        |s| subcommand_literal(s, &default_sub, "    "),
    ));
    out.push_str(&object_class_statics(draft, &default_sub));

    out.push_str("/// Command spec for `");
    out.push_str(name);
    out.push_str("`.\npub fn spec() -> CommandSpec {\n    CommandSpec {\n");
    let whole = Value::Object(draft.clone());
    for field in schema::COMMAND_FIELDS {
        match lifecycle_line(field.key, &whole, "        ") {
            LifecycleLine::Emit(line) => {
                out.push_str(&line);
                continue;
            }
            LifecycleLine::Skip => continue,
            LifecycleLine::Other => {}
        }
        let value = draft.get(field.key).unwrap_or(&Value::Null);
        let default_value = default.get(field.key).unwrap_or(&Value::Null);
        let expr = match field.kind {
            FieldKind::SideEffects => knowledge_expr(
                field_expr(field, value, default_value, "        "),
                draft.get("side_effects_knowledge").unwrap_or(&Value::Null),
            ),
            // The taint marker carries no facts of its own: its knowledge is
            // the field (issue #1712).
            FieldKind::Knowledge if field.key == "taint_knowledge" => {
                knowledge_expr(Some("()".to_owned()), value)
            }
            _ => field_expr(field, value, default_value, "        "),
        };
        if let Some(expr) = expr {
            out.push_str("        ");
            out.push_str(rust_field_name(field.key));
            out.push_str(": ");
            out.push_str(&expr);
            out.push_str(",\n");
        }
    }
    for note in unrenderable_notes(&Value::Object(draft.clone()), "        ") {
        out.push_str(&note);
        out.push('\n');
    }
    out.push_str("        ..CommandSpec::DEFAULT\n    }\n}\n");
    out
}

/// Lower-case the first character, for splicing a summary into a doc line.
fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

/// The conventional file path for a command spec, relative to the repository
/// root — e.g. `rust/tcl-registry/src/commands/tcl/lappend_.rs`.
///
/// The stem follows the convention the registry's 1000-odd command files
/// already keep: a namespace `::` is a **double** underscore, every other run
/// of punctuation is a single one. `IP::ttl` is `ip__ttl.rs` and `ip_ttl` is
/// `ip_ttl.rs` — both are real iRules commands, and collapsing the `::` to one
/// underscore filed them at the same path, where the second silently replaced
/// the first. Names that collide with a Rust keyword get the registry's
/// trailing underscore (`lappend_.rs`).
#[must_use]
pub fn suggested_path(name: &str, pack: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
        "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final",
        "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ];
    // A fully-qualified name (`::tcl::dict::lappend`) would otherwise start with
    // the separator's underscores, and a run of other punctuation would double
    // up into the `::` spelling — neither reads like the module names already
    // in the registry.
    let mut stem = String::new();
    let mut chars = name.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() {
            stem.extend(ch.to_lowercase());
        } else if ch == ':' && chars.peek() == Some(&':') {
            chars.next();
            if !stem.ends_with("__") {
                stem.push_str("__");
            }
        } else if !stem.ends_with('_') {
            stem.push('_');
        }
    }
    let mut stem = stem.trim_matches('_').to_owned();
    if stem.is_empty() {
        stem.push_str("command");
    }
    // The registry also appends `_` to names that shadow a Tcl-side module,
    // which is why `lappend` is `lappend_.rs`; keyword collisions are the
    // mechanical part we can decide here.
    if KEYWORDS.contains(&stem.as_str()) {
        stem.push('_');
    }
    format!("rust/tcl-registry/src/commands/{pack}/{stem}.rs")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod authored_construction_hint_render_controls;

/// The Rust field a draft key renders to: the draft names the taint
/// marker by its completeness, the spec by its domain.
fn rust_field_name(key: &str) -> &str {
    if key == "taint_knowledge" {
        "taint"
    } else {
        key
    }
}
