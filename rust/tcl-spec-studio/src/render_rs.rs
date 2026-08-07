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

use serde_json::Value;

use crate::draft::{self, Draft, UNRENDERABLE_KEY};
use crate::schema::{self, FieldKind, FieldSchema};

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

/// Render a `&'static [u8]` literal.
fn index_slice(values: &[Value]) -> String {
    let items: Vec<String> = values.iter().map(|v| as_u64(v).to_string()).collect();
    format!("&[{}]", items.join(", "))
}

/// Render a bitflag union such as `Traits::A.union(Traits::B)`, or
/// `TYPE::empty()` when no bits are set.
///
/// `.union()` rather than `|`: the option / subcommand tables are hoisted into
/// `const` items, and `bitflags`' `BitOr` impl is not `const`, so a `|` chain
/// there fails to compile ("cannot call non-const operator in constants").
/// `union` is a `const fn`, so it is correct in both a `const` and the `fn`
/// body that returns the spec.
fn flag_union(ty: &str, values: &[Value]) -> String {
    let mut names = values.iter().map(|v| format!("{ty}::{}", as_str(v)));
    let Some(head) = names.next() else {
        return format!("{ty}::empty()");
    };
    names.fold(head, |acc, name| acc + ".union(" + &name + ")")
}

/// Render a `DialectSet` from the canonical dialect-name list a draft holds.
///
/// Emits the readable aggregate constants (`DialectSet::ALL_TCL`,
/// `DialectSet::TCL85_PLUS`) when the set matches one exactly, so the output
/// reads like the hand-written specs rather than a five-way union.
fn dialect_set(values: &[Value]) -> String {
    const AGGREGATES: &[(&str, &[&str])] = &[
        (
            "ALL_TCL",
            &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
        ),
        ("TCL85_PLUS", &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL86_PLUS", &["tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL90_PLUS", &["tcl9.0", "tcl9.1"]),
    ];
    let names: Vec<&str> = values.iter().map(as_str).collect();
    if names.is_empty() {
        return "DialectSet::empty()".to_owned();
    }
    let mut remaining = names.clone();
    let mut parts: Vec<String> = Vec::new();
    for (constant, members) in AGGREGATES {
        if members.iter().all(|m| remaining.contains(m)) {
            parts.push(format!("DialectSet::{constant}"));
            remaining.retain(|n| !members.contains(n));
            break;
        }
    }
    for name in remaining {
        // The canonical names are the registry's own (`DialectSet::parse` /
        // `member_names`); the constants are their Rust spellings. An
        // unrecognised name is emitted as a comment rather than a bare
        // identifier — `DialectSet::f5-tmsh` is not valid Rust, and rendering
        // it silently produced a file that only failed at `cargo build`.
        let Some(bit) = dialect_constant(name) else {
            parts.push(format!("/* unknown dialect {name:?} */"));
            continue;
        };
        parts.push(format!("DialectSet::{bit}"));
    }
    match parts.len() {
        0 => "DialectSet::empty()".to_owned(),
        1 => parts.remove(0),
        _ => {
            let head = parts.remove(0);
            let tail: Vec<String> = parts.into_iter().map(|p| format!(".union({p})")).collect();
            format!("{head}{}", tail.join(""))
        }
    }
}

/// The `DialectSet` constant for a canonical dialect name, or `None` when the
/// name is not one the registry knows.
fn dialect_constant(name: &str) -> Option<&'static str> {
    Some(match name {
        "tcl8.4" => "TCL84",
        "tcl8.5" => "TCL85",
        "tcl8.6" => "TCL86",
        "tcl9.0" => "TCL90",
        "tcl9.1" => "TCL91",
        "f5-irules" => "IRULES",
        "f5-iapps" => "IAPPS",
        "tk" => "TK",
        "expect" => "EXPECT",
        "bpf" => "BPF",
        "f5-tmsh" => "TMSH",
        "f5-bigip" => "BIGIP",
        _ => return None,
    })
}

fn arity_expr(value: &Value) -> String {
    let min = as_u64(&value["min"]);
    let max = value["max"].as_u64();
    let step = as_u64(&value["step"]);
    let also = value["also_exact"].as_u64();
    // `stepped` is an associated *constructor* taking all three bounds, not a
    // builder method — `Arity::at_least(1).stepped(2)` does not compile.
    let base = if step == 0 {
        match max {
            None => {
                if min == 0 {
                    "Arity::any()".to_owned()
                } else {
                    format!("Arity::at_least({min})")
                }
            }
            Some(max) if max == min => format!("Arity::exact({min})"),
            Some(max) => format!("Arity::new({min}, {max})"),
        }
    } else {
        format!(
            "Arity::stepped({min}, {}, {step})",
            max.map_or_else(|| "Arity::UNLIMITED".to_owned(), |m| m.to_string())
        )
    };
    match also {
        None => base,
        Some(n) => format!("{base}.with_also_exact({n})"),
    }
}

fn appended_arity_expr(value: &Value) -> String {
    match as_str(&value["kind"]) {
        "Exactly" => format!("AppendedArity::Exactly({})", as_u64(&value["n"])),
        "AtLeast" => format!("AppendedArity::AtLeast({})", as_u64(&value["n"])),
        _ => "AppendedArity::Unknown".to_owned(),
    }
}

fn role_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, ArgRole::{})",
                as_u64(&entry["index"]),
                as_str(&entry["role"])
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn presentation_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, ArgPresentation::{})",
                as_u64(&entry["index"]),
                as_str(&entry["presentation"])
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn prefix_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, {})",
                as_u64(&entry["index"]),
                appended_arity_expr(&entry["arity"])
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn arg_type_map_expr(values: &[Value], indent: &str) -> String {
    if values.is_empty() {
        return "&[]".to_owned();
    }
    let inner = format!("{indent}    ");
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            let expected = entry["expected"]
                .as_str()
                .map_or_else(|| "None".to_owned(), |t| format!("Some(TclType::{t})"));
            let transparent: Vec<String> = as_array(&entry["transparent_from"])
                .iter()
                .map(|t| format!("TclType::{}", as_str(t)))
                .collect();
            format!(
                "{inner}({}, ArgTypeHint {{\n{inner}    expected: {expected},\n{inner}    shimmers: {},\n{inner}    transparent_from: &[{}],\n{inner}}}),",
                as_u64(&entry["index"]),
                as_bool(&entry["shimmers"]),
                transparent.join(", "),
            )
        })
        .collect();
    format!("&[\n{}\n{indent}]", items.join("\n"))
}

fn arg_value_expr(entry: &Value, indent: &str) -> String {
    let mut parts = vec![format!(
        "{indent}    value: {},",
        rust_string(as_str(&entry["value"]))
    )];
    let detail = as_str(&entry["detail"]);
    if !detail.is_empty() {
        parts.push(format!("{indent}    detail: {},", rust_string(detail)));
    }
    if let Some(min_tcl) = entry["min_tcl"].as_str() {
        parts.push(format!("{indent}    min_tcl: Some(TclVersion::{min_tcl}),"));
    }
    if let Some(code) = entry["code"].as_i64() {
        parts.push(format!("{indent}    code: Some({code}),"));
    }
    format!(
        "{indent}ArgValue {{\n{}\n{indent}    ..ArgValue::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

fn arg_value_map_expr(values: &[Value], indent: &str) -> String {
    if values.is_empty() {
        return "&[]".to_owned();
    }
    let inner = format!("{indent}    ");
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            let vals: Vec<String> = as_array(&entry["values"])
                .iter()
                .map(|v| arg_value_expr(v, &format!("{inner}        ")))
                .collect();
            format!(
                "{inner}({}, &[\n{}\n{inner}    ]),",
                as_u64(&entry["index"]),
                vals.join("\n")
            )
        })
        .collect();
    format!("&[\n{}\n{indent}]", items.join("\n"))
}

fn integer_domain_expr(value: &Value) -> Option<String> {
    match as_str(&value["kind"]) {
        "Any" => Some("IntegerDomain::Any".to_owned()),
        "Range" => Some(format!(
            "IntegerDomain::Range({}, {})",
            value["lo"].as_i64().unwrap_or_default(),
            value["hi"].as_i64().unwrap_or_default()
        )),
        "Port" => Some("IntegerDomain::Port".to_owned()),
        _ => None,
    }
}

/// The Rust expression supplied for a `Hook` arity, if the author filled it in.
fn hook_expr(arity: &Value) -> Option<&str> {
    arity
        .get("hook")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|h| !h.is_empty())
}

/// Every option row still missing its arity hook, by option name.
fn unfilled_option_hooks(draft: &Value) -> Vec<&str> {
    as_array(draft.get("options").unwrap_or(&Value::Null))
        .iter()
        .filter(|opt| {
            let arity = &opt["value"]["arity"];
            as_str(&arity["kind"]) == "Hook" && hook_expr(arity).is_none()
        })
        .map(|opt| as_str(&opt["name"]))
        .collect()
}

/// Render a `Lifecycle` constructor from a draft's three lifecycle keys, or
/// `None` when the draft declares no lifecycle release at all (the
/// `..DEFAULT` field is then simply omitted).
fn lifecycle_expr(entry: &Value) -> Option<String> {
    let release = |key: &str| entry[key].as_str().filter(|v| !v.is_empty());
    let introduced = release("introduced_version");
    let deprecated = release("deprecated_version");
    let retired = release("retired_version");
    if introduced.is_none() && deprecated.is_none() && retired.is_none() {
        return None;
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

/// What a schema field whose key is one of the three `Lifecycle` releases
/// contributes to the emitted struct literal.
enum LifecycleLine {
    /// Not a release key — render the field the ordinary way.
    Other,
    /// The first release key: emit this text (empty when the draft declares no
    /// release at all, leaving the field to `..DEFAULT`).
    Emit(String),
    /// A later release key — the first already accounted for it.
    Skip,
}

/// Collapse the schema's three release keys back into the registry's one
/// `lifecycle` field.
///
/// `CommandSpec` / `SubCommand` declare **one** `lifecycle` field; the form
/// edits it as three independent releases (#1210). Walking the schema and
/// emitting each key verbatim produced `introduced_version: Some("21.1.0"),` —
/// a spec file that does not compile (`struct SubCommand has no field named
/// introduced_version`), caught by rendering the registry back into itself.
/// The line lands at the position of the first release key, so field order is
/// unchanged.
fn lifecycle_line(key: &str, draft: &Value, indent: &str) -> LifecycleLine {
    let Some((first, rest)) = crate::coverage::LIFECYCLE_KEYS.split_first() else {
        return LifecycleLine::Other;
    };
    if *first == key {
        return LifecycleLine::Emit(
            lifecycle_expr(draft)
                .map(|expr| format!("{indent}lifecycle: {expr},\n"))
                .unwrap_or_default(),
        );
    }
    if rest.contains(&key) {
        return LifecycleLine::Skip;
    }
    LifecycleLine::Other
}

fn option_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let mut parts = vec![format!(
        "{inner}name: {},",
        rust_string(as_str(&entry["name"]))
    )];

    let value = &entry["value"];
    if value.is_object() {
        let arg_indent = format!("{inner}    ");
        let mut arg_parts: Vec<String> = Vec::new();
        let arity = &value["arity"];
        let arity_expr = match as_str(&arity["kind"]) {
            "Fixed" => Some(format!("OptionArity::Fixed({})", as_u64(&arity["n"]))),
            // The hook is a function pointer, so seeding cannot recover it —
            // but the author can type it, and then it renders like any other
            // arity. Empty still falls through to the TODO below.
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
        if let Some(also) = value["also_role"].as_str() {
            arg_parts.push(format!("{arg_indent}    also_role: Some(ArgRole::{also}),"));
        }
        let body_kind = as_str(&value["body_kind"]);
        if body_kind == "Structural" {
            arg_parts.push(format!("{arg_indent}    body_kind: BodyKind::Structural,"));
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
        parts.push(format!(
            "{inner}value: OptionValue::Takes(OptionArg {{\n{}\n{arg_indent}..OptionArg::DEFAULT\n{inner}}}),",
            arg_parts.join("\n")
        ));
    }

    let detail = as_str(&entry["detail"]);
    if !detail.is_empty() {
        parts.push(format!("{inner}detail: {},", rust_string(detail)));
    }
    if let Some(dialects) = entry["dialects"].as_array() {
        parts.push(format!("{inner}dialects: Some({}),", dialect_set(dialects)));
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
    format!(
        "{indent}OptionSpec {{\n{}\n{inner}..OptionSpec::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

fn form_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let dialects = entry["dialects"].as_array().map_or_else(
        || "None".to_owned(),
        |d| format!("Some({})", dialect_set(d)),
    );
    format!(
        "{indent}FormSpec {{\n{inner}kind: FormKind::{},\n{inner}synopsis: {},\n{inner}dialects: {dialects},\n{indent}}},",
        as_str(&entry["kind"]),
        rust_string(as_str(&entry["synopsis"])),
    )
}

fn side_effect_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let dialects = entry["dialects"].as_array().map_or_else(
        || "None".to_owned(),
        |d| format!("Some({})", dialect_set(d)),
    );
    format!(
        "{indent}SideEffect {{\n{inner}target: SideEffectTarget::{},\n{inner}reads: {},\n{inner}writes: {},\n{inner}connection_side: ConnectionSide::{},\n{inner}dialects: {dialects},\n{indent}}},",
        as_str(&entry["target"]),
        as_bool(&entry["reads"]),
        as_bool(&entry["writes"]),
        as_str(&entry["connection_side"]),
    )
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
    let inner = format!("{indent}    ");
    let dialects = entry["dialects"].as_array().map_or_else(
        || "None".to_owned(),
        |d| format!("Some({})", dialect_set(d)),
    );
    format!(
        "{indent}SubSubCommand {{\n{inner}name: {},\n{inner}detail: {},\n{inner}synopsis: {},\n{inner}dialects: {dialects},\n{indent}}},",
        rust_string(as_str(&entry["name"])),
        rust_string(as_str(&entry["detail"])),
        rust_string(as_str(&entry["synopsis"])),
    )
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
    let expr = match field.kind {
        FieldKind::Bool => as_bool(value).to_string(),
        FieldKind::OptBool => format!("Some({})", value.as_bool()?),
        FieldKind::Count => as_u64(value).to_string(),
        FieldKind::OptIndex => format!("Some({})", value.as_u64()?),
        FieldKind::Text | FieldKind::Prose => rust_string(as_str(value)),
        FieldKind::OptText => format!("Some({})", rust_string(value.as_str()?)),
        FieldKind::TextList => str_slice(as_array(value)),
        FieldKind::IndexList => index_slice(as_array(value)),
        FieldKind::OptIndexList => format!("Some({})", index_slice(value.as_array()?)),
        FieldKind::Enum {
            catalogue,
            optional,
        } => {
            let variant = value.as_str()?;
            let ty = enum_type_name(catalogue);
            let expr = format!("{ty}::{variant}");
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
        FieldKind::Arity => arity_expr(value),
        FieldKind::RoleMap => role_map_expr(as_array(value)),
        FieldKind::PrefixMap => prefix_map_expr(as_array(value)),
        FieldKind::PresentationMap => presentation_map_expr(as_array(value)),
        FieldKind::ArgTypeMap => arg_type_map_expr(as_array(value), indent),
        FieldKind::ArgValueMap => arg_value_map_expr(as_array(value), indent),
        FieldKind::Options => "OPTIONS".to_owned(),
        FieldKind::Forms => "FORMS".to_owned(),
        FieldKind::SubCommands => "SUBCOMMANDS".to_owned(),
        FieldKind::SideEffects => list_expr(as_array(value), indent, side_effect_expr)?,
        FieldKind::SetterConstraints => list_expr(as_array(value), indent, setter_constraint_expr)?,
        FieldKind::SubSubCommands => list_expr(as_array(value), indent, sub_subcommand_expr)?,
        FieldKind::Hover => {
            if value.is_null() {
                return None;
            }
            hover_expr(value, indent)
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
        "storageType" => "StorageType",
        "byteArrayEffect" => "ByteArrayEffect",
        "commandTableEffect" => "CommandTableEffect",
        "patternType" => "PatternType",
        "formatType" => "FormatType",
        "formKind" => "FormKind",
        "definedSymbolKind" => "DefinedSymbolKind",
        "sideEffectTarget" => "SideEffectTarget",
        "connectionSide" => "ConnectionSide",
        "loweringHook" => "LoweringHookId",
        "codegenHook" => "CodegenHookId",
        "inlineCodegenHook" => "InlineCodegenHookId",
        "wasmCodegenHook" => "WasmCodegenHookId",
        "analyserHook" => "AnalyserHookId",
        "traits" => "Traits",
        "taintColour" => "TaintColour",
        "dialects" => "DialectSet",
        "defaultFormFirstWord" => "DefaultFormFirstWord",
        "prefixMatching" => "PrefixMatching",
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
            _ => field_expr(field, value, default_value, &inner),
        };
        if let Some(expr) = expr {
            lines.push(format!("{inner}{}: {expr},", field.key));
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
        if let Some(expr) = field_expr(field, value, default_value, "        ") {
            out.push_str("        ");
            out.push_str(field.key);
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
/// Names that collide with a Rust keyword get the registry's trailing
/// underscore (`lappend_.rs`), and a namespaced iRules command flattens its
/// `::` separators.
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
    // the separator's underscore, and repeated punctuation would double it up —
    // neither reads like the module names already in the registry.
    let mut stem = String::new();
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            stem.extend(ch.to_lowercase());
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
mod tests {
    use super::*;

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
    use tcl_registry::spec::{CommandSpec, SubCommand};
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
            dialects: Some(tcl_dialect::DialectSet::ALL_TCL.union(tcl_dialect::DialectSet::IRULES)),
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
            out.contains("DialectSet::ALL_TCL.union(DialectSet::IRULES)"),
            "dialect aggregates should read like the hand-written specs:\n{out}"
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
        let out = render(&d);
        assert!(out.contains("arg_role_resolver: resolve_if_roles,"));
    }

    /// Every dialect the catalogue offers must have a Rust constant to render,
    /// or a spec carrying that bit produces a file that will not compile.
    #[test]
    fn every_catalogued_dialect_renders_to_a_constant() {
        for entry in crate::catalogue::DIALECTS {
            assert!(
                dialect_constant(entry.key).is_some(),
                "no DialectSet constant for the catalogued dialect {}",
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
            "rust/tcl-registry/src/commands/irules/http_header.rs"
        );
        // A leading `::` must not leave the module name starting with `_`, and
        // a run of separators collapses to one.
        assert_eq!(
            suggested_path("::tcl::dict::lappend", "tcl"),
            "rust/tcl-registry/src/commands/tcl/tcl_dict_lappend.rs"
        );
        assert_eq!(
            suggested_path("tcl::mathop::+", "tcl"),
            "rust/tcl-registry/src/commands/tcl/tcl_mathop.rs"
        );
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
}
