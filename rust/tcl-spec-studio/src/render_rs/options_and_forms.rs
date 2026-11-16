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

//! Option and form literals: arity hooks, lifecycles, option effects and values, knowledge, side effects and `CommandForm` rows.

use super::field_values::{
    appended_arity_expr, arg_value_expr, arity_expr, callback_taint_inputs_expr, dialect_set,
    flag_union, integer_domain_expr, role_map_expr, variable_receivers_expr,
};
use super::{as_array, as_bool, as_str, as_u64, list_expr, rust_string, str_slice};
use serde_json::Value;

/// The Rust expression supplied for a `Hook` arity, if the author filled it in.
fn hook_expr(arity: &Value) -> Option<&str> {
    arity
        .get("hook")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|h| !h.is_empty())
}

/// Every option row still missing its arity hook, by option name.
pub(super) fn unfilled_option_hooks(draft: &Value) -> Vec<&str> {
    as_array(draft.get("options").unwrap_or(&Value::Null))
        .iter()
        .filter(|opt| {
            let arity = &opt["value"]["arity"];
            as_str(&arity["kind"]) == "Hook" && hook_expr(arity).is_none()
        })
        .map(|opt| as_str(&opt["name"]))
        .collect()
}

/// Every option row whose lifecycle used a custom callback that seeding could
/// not recover and whose author has not yet supplied an expression.
pub(super) fn unfilled_option_deprecation_hooks(draft: &Value) -> Vec<&str> {
    as_array(draft.get("options").unwrap_or(&Value::Null))
        .iter()
        .filter(|opt| {
            opt[crate::draft::OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY]
                .as_bool()
                .unwrap_or(false)
                && opt["deprecation_fix"]
                    .as_str()
                    .is_none_or(|expr| expr.trim().is_empty())
        })
        .map(|opt| as_str(&opt["name"]))
        .collect()
}

/// Render a `Lifecycle` constructor from a draft's four lifecycle keys, or
/// `None` when the draft declares no lifecycle release or fix at all (the
/// `..DEFAULT` field is then simply omitted).
pub(super) fn lifecycle_expr(entry: &Value) -> Option<String> {
    let release = |key: &str| entry[key].as_str().filter(|v| !v.is_empty());
    let introduced = release("introduced_version");
    let deprecated = release("deprecated_version");
    let retired = release("retired_version");
    let deprecation_fix = entry["deprecation_fix"]
        .as_str()
        .map(str::trim)
        .filter(|expr| !expr.is_empty());
    if introduced.is_none()
        && deprecated.is_none()
        && retired.is_none()
        && deprecation_fix.is_none()
    {
        return None;
    }
    if let Some(deprecation_fix) = deprecation_fix {
        let release_expr = |value: Option<&str>| {
            value.map_or_else(
                || "None".to_owned(),
                |value| format!("Some({})", rust_string(value)),
            )
        };
        return Some(format!(
            "Lifecycle {{ introduced: {}, deprecated: {}, retired: {}, deprecation_fix: {deprecation_fix} }}",
            release_expr(introduced),
            release_expr(deprecated),
            release_expr(retired),
        ));
    }
    let mut expr = match introduced {
        Some(v) => format!("Lifecycle::introduced_in({})", rust_string(v)),
        None => match deprecated {
            Some(v) => format!("Lifecycle::deprecated_in({})", rust_string(v)),
            None => "Lifecycle::UNSPECIFIED".to_owned(),
        },
    };
    if introduced.is_some()
        && let Some(v) = deprecated
    {
        expr = format!("{expr}.deprecated_from({})", rust_string(v));
    }
    if let Some(v) = retired {
        expr = format!("{expr}.retired_from({})", rust_string(v));
    }
    Some(expr)
}

/// What a schema field whose key is one of the four `Lifecycle` editors
/// contributes to the emitted struct literal.
pub(super) enum LifecycleLine {
    /// Not a release key — render the field the ordinary way.
    Other,
    /// The first release key: emit this text (empty when the draft declares no
    /// release at all, leaving the field to `..DEFAULT`).
    Emit(String),
    /// A later release key — the first already accounted for it.
    Skip,
}

/// Collapse the schema's lifecycle keys back into the registry's one
/// `lifecycle` field.
///
/// `CommandSpec` / `SubCommand` declare **one** `lifecycle` field; the form
/// edits it as independent releases and a quick-fix hook. Walking the schema and
/// emitting each key verbatim produced `introduced_version: Some("21.1.0"),` —
/// a spec file that does not compile (`struct SubCommand has no field named
/// introduced_version`), caught by rendering the registry back into itself.
/// The line lands at the position of the first release key, so field order is
/// unchanged.
pub(super) fn lifecycle_line(key: &str, draft: &Value, indent: &str) -> LifecycleLine {
    let Some((first, rest)) = crate::coverage::LIFECYCLE_KEYS.split_first() else {
        return LifecycleLine::Other;
    };
    if *first == key {
        return LifecycleLine::Emit(
            lifecycle_expr(draft).map_or_default(|expr| format!("{indent}lifecycle: {expr},\n")),
        );
    }
    if rest.contains(&key) {
        return LifecycleLine::Skip;
    }
    LifecycleLine::Other
}

// Keep every OptionArg field in one renderer so adding registry metadata
// cannot silently miss one shorthand or generic literal path.
#[allow(clippy::too_many_lines)]
/// The Rust expression for a drafted [`tcl_registry::option_effect::EffectAxis`]
/// (`axis` and, for every axis but `case-sensitivity`, `value`).
fn effect_axis_expr(axis: &str, value: Option<&str>) -> Option<String> {
    match axis {
        "substitution" => {
            let kind = match value? {
                "backslashes" => "Backslashes",
                "commands" => "Commands",
                "variables" => "Variables",
                _ => return None,
            };
            Some(format!(
                "EffectAxis::Substitution(SubstitutionKind::{kind})"
            ))
        }
        "pattern-language" => {
            let kind = match value? {
                "glob" => "Glob",
                "regex" => "Regex",
                _ => return None,
            };
            Some(format!("EffectAxis::PatternLanguage(PatternType::{kind})"))
        }
        "case-sensitivity" => Some("EffectAxis::CaseSensitivity".to_owned()),
        "selection" => {
            let mode = match value? {
                "exact" => "Exact",
                "glob" => "Glob",
                "regexp" => "Regexp",
                "other" => "Other",
                _ => return None,
            };
            Some(format!("EffectAxis::Selection(CaseMatchMode::{mode})"))
        }
        _ => None,
    }
}

/// The Rust expression for a drafted `option` row's `effect` key, or `None`
/// for an unset or unreadable one.
fn option_effect_expr(entry: &Value) -> Option<String> {
    if entry.is_null() {
        return None;
    }
    let axis = || effect_axis_expr(as_str(&entry["axis"]), entry["value"].as_str());
    let kind = match as_str(&entry["kind"]) {
        "disables" => format!("OptionEffectKind::Disables({})", axis()?),
        "selects" => format!("OptionEffectKind::Selects({})", axis()?),
        "suppresses-role" => format!(
            "OptionEffectKind::SuppressesRole(ArgRole::{})",
            as_str(&entry["role"])
        ),
        "reserves-trailing-words" => format!(
            "OptionEffectKind::ReservesTrailingWords({})",
            as_u64(&entry["n"])
        ),
        "ends-options" => "OptionEffectKind::EndsOptions".to_owned(),
        _ => return None,
    };
    Some(format!(
        "OptionEffect {{ kind: {kind}, family: {} }}",
        rust_string(as_str(&entry["family"]))
    ))
}

/// The `value: OptionValue::Takes(OptionArg { … }),` line for an option
/// whose drafted `value` is set, or `None` for a bare flag.
fn option_value_expr(value: &Value, inner: &str) -> Option<String> {
    if !value.is_object() {
        return None;
    }
    let arg_indent = format!("{inner}    ");
    let mut arg_parts: Vec<String> = Vec::new();
    let arity = &value["arity"];
    let arity_expr = match as_str(&arity["kind"]) {
        "Fixed" => Some(format!("OptionArity::Fixed({})", as_u64(&arity["n"]))),
        "OptionalOneOf" => Some(format!(
            "OptionArity::OptionalOneOf(&[{}])",
            arity["values"].as_array().map_or_default(|values| values
                .iter()
                .map(|v| format!("{:?}", as_str(v)))
                .collect::<Vec<_>>()
                .join(", "))
        )),
        "OptionalNotOption" => Some("OptionArity::OptionalNotOption".to_owned()),
        // The hook is a function pointer, so seeding cannot recover it — but
        // the author can type it, and then it renders like any other arity.
        // Empty still falls through to the TODO below.
        "Hook" => hook_expr(arity).map(|h| format!("OptionArity::Hook({h})")),
        _ => Some("OptionArity::One".to_owned()),
    };
    match arity_expr {
        Some(expr) if expr != "OptionArity::One" => {
            arg_parts.push(format!("{arg_indent}    arity: {expr},"));
        }
        Some(_) => {}
        None => arg_parts.push(format!(
            "{arg_indent}    // TODO: this option consumed a computed number of words \
             (OptionArity::Hook); supply the hook.",
        )),
    }
    let role = as_str(&value["role"]);
    if !role.is_empty() && role != "Value" {
        arg_parts.push(format!("{arg_indent}    role: ArgRole::{role},"));
    }
    let also_role = value["also_role"].as_str();
    if let Some(also) = also_role {
        arg_parts.push(format!("{arg_indent}    also_role: Some(ArgRole::{also}),"));
    }
    let body_kind = as_str(&value["body_kind"]);
    if body_kind == "Structural" {
        arg_parts.push(format!("{arg_indent}    body_kind: BodyKind::Structural,"));
    }
    let script_timing = as_str(&value["script_timing"]);
    if !script_timing.is_empty() && script_timing != "SameInvocation" {
        arg_parts.push(format!(
            "{arg_indent}    script_timing: ScriptTiming::{script_timing},"
        ));
    }
    let callback_inputs = as_array(&value["callback_taint_inputs"]);
    if !callback_inputs.is_empty() {
        arg_parts.push(format!(
            "{arg_indent}    callback_taint_inputs: {},",
            callback_taint_inputs_expr(callback_inputs)
        ));
    }
    if as_str(&value["variable_scope"]) == "Global" {
        arg_parts.push(format!(
            "{arg_indent}    variable_scope: VariableScope::Global,"
        ));
    }
    let values = as_array(&value["values"]);
    if !values.is_empty() {
        let vals: Vec<String> = values
            .iter()
            .map(|v| arg_value_expr(v, &format!("{arg_indent}        ")))
            .collect();
        arg_parts.push(format!(
            "{arg_indent}    values: &[\n{}\n{arg_indent}    ],",
            vals.join("\n")
        ));
    }
    if as_bool(&value["closed"]) {
        arg_parts.push(format!("{arg_indent}    closed: true,"));
    }
    if let Some(domain) = integer_domain_expr(&value["integer"]) {
        arg_parts.push(format!("{arg_indent}    integer: Some({domain}),"));
    }
    let hint = as_str(&value["hint"]);
    if !hint.is_empty() {
        arg_parts.push(format!("{arg_indent}    hint: {},", rust_string(hint)));
    }
    let appended = appended_arity_expr(&value["appended_arity"]);
    if appended != "AppendedArity::Unknown" {
        arg_parts.push(format!("{arg_indent}    appended_arity: {appended},"));
    }
    if as_bool(&value["taints_var_write"]) && (role == "VarWrite" || also_role == Some("VarWrite"))
    {
        arg_parts.push(format!("{arg_indent}    taints_var_write: true,"));
    }
    Some(format!(
        "{inner}value: OptionValue::Takes(OptionArg {{\n{}\n{arg_indent}..OptionArg::DEFAULT\n{inner}}}),",
        arg_parts.join("\n")
    ))
}

pub(super) fn option_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let mut parts = vec![format!(
        "{inner}name: {},",
        rust_string(as_str(&entry["name"]))
    )];

    parts.extend(option_value_expr(&entry["value"], &inner));

    let detail = as_str(&entry["detail"]);
    if !detail.is_empty() {
        parts.push(format!("{inner}detail: {},", rust_string(detail)));
    }
    if let Some(dialects) = entry["surface"].as_array() {
        parts.push(format!("{inner}surface: Some({}),", dialect_set(dialects)));
    }
    let aliases = as_array(&entry["aliases"]);
    if !aliases.is_empty() {
        parts.push(format!("{inner}aliases: {},", str_slice(aliases)));
    }
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{inner}lifecycle: {lifecycle},"));
    }
    if let Some(min_abbrev) = entry["min_abbrev"].as_u64() {
        parts.push(format!("{inner}min_abbrev: Some({min_abbrev}),"));
    }
    if let Some(effect) = option_effect_expr(&entry["effect"]) {
        parts.push(format!("{inner}effect: Some({effect}),"));
    }
    format!(
        "{indent}OptionSpec {{\n{}\n{inner}..OptionSpec::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

/// A row literal: only the fields that differ from the type's `DEFAULT`, then
/// the spread — the same shape every hand-written spec has used since the
/// registry-wide `..DEFAULT` conversion.
pub(super) fn row_literal(name: &str, parts: &[String], indent: &str) -> String {
    format!(
        "{indent}{name} {{\n{}\n{indent}    ..{name}::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

/// `Some(<dialect set>)` for a present dialect gate, or nothing for an absent
/// one — the field's own default.
pub(super) fn dialects_line(entry: &Value, indent: &str) -> Option<String> {
    let dialects = entry["surface"].as_array()?;
    Some(format!(
        "{indent}    surface: Some({}),",
        dialect_set(dialects)
    ))
}

pub(super) fn form_expr(entry: &Value, indent: &str) -> String {
    let mut parts = Vec::new();
    let kind = as_str(&entry["kind"]);
    if kind != "Default" {
        parts.push(format!("{indent}    kind: FormKind::{kind},"));
    }
    parts.push(format!(
        "{indent}    synopsis: {},",
        rust_string(as_str(&entry["synopsis"]))
    ));
    parts.extend(dialects_line(entry, indent));
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    row_literal("FormSpec", &parts, indent)
}

/// The `Knowledge` literal a facts domain's rows and its draft knowledge
/// object (`{level, complete}`) state together (issue #1712).
///
/// `None` is the `Unknown` default, which the literal leaves to `DEFAULT`.
pub(crate) fn knowledge_expr(rows: Option<String>, knowledge: &Value) -> Option<String> {
    let path = "crate::knowledge::Knowledge";
    let level =
        knowledge["level"]
            .as_str()
            .unwrap_or(if rows.is_some() { "partial" } else { "unknown" });
    let rows = rows.unwrap_or_else(|| "&[]".to_owned());
    match level {
        "partial" => Some(format!("{path}::Partial({rows})")),
        "complete" => {
            let surfaces: Vec<String> = as_array(&knowledge["complete"])
                .iter()
                .filter_map(complete_surface_expr)
                .collect();
            Some(format!(
                "{path}::Complete({rows}, &[{}])",
                surfaces.join(", ")
            ))
        }
        _ => None,
    }
}

/// One `effects -complete` window as a `SpecSurface` literal.
fn complete_surface_expr(words: &Value) -> Option<String> {
    let words: Vec<&str> = as_array(words).iter().filter_map(Value::as_str).collect();
    let surface = "tcl_dialect::model::SpecSurface";
    match words.as_slice() {
        ["package", name] => Some(format!("{surface}::package({})", rust_string(name))),
        ["package", name, from, until] => Some(format!(
            "{surface}::package_in({}, &[({}, Some({}))])",
            rust_string(name),
            rust_string(from),
            rust_string(until)
        )),
        [from, until] => {
            let first = from.parse::<tcl_dialect::model::Release>().ok()?;
            let last = until.parse::<tcl_dialect::model::Release>().ok()?;
            let family = first.family();
            let end = family.next_release(last).map_or_else(
                || "None".to_owned(),
                |next| format!("Some({})", rust_string(next.as_str())),
            );
            Some(format!(
                "{surface}::core_in(tcl_dialect::model::Family::{family:?}, &[({}, {end})])",
                rust_string(first.as_str())
            ))
        }
        _ => None,
    }
}

pub(super) fn side_effect_expr(entry: &Value, indent: &str) -> String {
    let mut parts = vec![format!(
        "{indent}    target: SideEffectTarget::{},",
        as_str(&entry["target"])
    )];
    for (key, field) in [("reads", "reads"), ("writes", "writes")] {
        if as_bool(&entry[key]) {
            parts.push(format!("{indent}    {field}: true,"));
        }
    }
    let side = as_str(&entry["connection_side"]);
    if side != "None" {
        parts.push(format!(
            "{indent}    connection_side: ConnectionSide::{side},"
        ));
    }
    parts.extend(dialects_line(entry, indent));
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    row_literal("SideEffect", &parts, indent)
}

/// One `CommandForm` literal from an invocation-refinement draft row.
///
/// Only the fields the row actually declares are written; everything else
/// rides `..CommandForm::DEFAULT`, which is what makes an omitted overlay
/// inherit rather than blank the parent's fact.
pub(super) fn command_form_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let mut parts = vec![format!(
        "{inner}name: {},",
        rust_string(as_str(&entry["name"]))
    )];
    let arity = arity_expr(&entry["arity"]);
    if arity != "Arity::any()" {
        parts.push(format!("{inner}arity: {arity},"));
    }
    if let Some(selector) = entry["selector"].as_object() {
        let words = str_slice(as_array(&selector["words"]));
        let constructor = if as_str(&selector["prefix_matching"]) == "Strict" {
            "exact"
        } else {
            "unique"
        };
        parts.push(format!(
            "{inner}literal_argument_prefix: Some(LiteralArgumentPrefix::{constructor}({words})),"
        ));
    }
    let roles = as_array(&entry["arg_roles"]);
    if !roles.is_empty() {
        parts.push(format!("{inner}arg_roles: {},", role_map_expr(roles)));
    }
    if let Some(receivers) = entry["variable_receivers"].as_array() {
        parts.push(format!(
            "{inner}variable_receivers: {},",
            variable_receivers_expr(receivers)
        ));
    }
    let options = as_array(&entry["options"]);
    if !options.is_empty() {
        let opt_indent = format!("{inner}    ");
        let rendered: Vec<String> = options
            .iter()
            .map(|option| option_expr(option, &opt_indent))
            .collect();
        parts.push(format!(
            "{inner}options: &[\n{}\n{inner}],",
            rendered.join("\n")
        ));
    }
    if let Some(relations) = entry["option_relations"].as_str() {
        parts.push(format!("{inner}option_relations: {relations},"));
    }
    if let Some(rows) = entry["surface"].as_array() {
        parts.push(format!("{inner}surface: Some({}),", dialect_set(rows)));
    }
    if let Some(traits) = entry["traits"].as_array() {
        parts.push(format!(
            "{inner}traits: Some({}),",
            flag_union("Traits", traits)
        ));
    }
    if let Some(mutator) = entry["mutator"].as_bool() {
        parts.push(format!("{inner}mutator: Some({mutator}),"));
    }
    if let Some(effects) = entry["side_effects"].as_array() {
        let rows = list_expr(effects, &inner, side_effect_expr);
        let knowledge = &entry["side_effects_knowledge"];
        // A form's empty row list replaces its parent's effects with none,
        // so its default level is partial rather than unknown.
        let expr = if knowledge.is_null() {
            knowledge_expr(Some(rows.unwrap_or_else(|| "&[]".to_owned())), knowledge)
        } else {
            knowledge_expr(rows, knowledge)
        };
        parts.push(format!(
            "{inner}side_effects: Some({}),",
            expr.unwrap_or_else(|| "crate::knowledge::Knowledge::Unknown".to_owned())
        ));
    }
    if let Some(expr) = entry["representation_effect"].as_str() {
        parts.push(format!("{inner}representation_effect: Some({expr}),"));
    }
    for (key, ty) in [
        ("lowering_hook", "LoweringHookId"),
        ("codegen_hook", "CodegenHookId"),
    ] {
        if let Some(id) = entry[key].as_str() {
            parts.push(format!("{inner}{key}: Some({ty}::{id}),"));
        }
    }
    row_literal("CommandForm", &parts, indent)
}
