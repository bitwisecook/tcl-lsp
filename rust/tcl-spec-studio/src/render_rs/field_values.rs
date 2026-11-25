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

//! Literals for one field value: surfaces and dialect sets, slices and flag unions, arity, role and type maps and argument values.

use super::options_and_forms::lifecycle_expr;
use super::{as_array, as_bool, as_str, as_u64, rust_string};
use crate::draft;
use serde_json::Value;

/// A surface key's value as an `Option<&[SpecSurface]>` expression.
pub(super) fn surface_expr(value: &Value) -> String {
    match value.as_array() {
        Some(items) => format!("Some({})", dialect_set(items)),
        None => "None".to_owned(),
    }
}

/// Render a `&'static [u8]` literal.
pub(super) fn index_slice(values: &[Value]) -> String {
    let items: Vec<String> = values.iter().map(|v| as_u64(v).to_string()).collect();
    format!("&[{}]", items.join(", "))
}

pub(super) fn variable_receivers_expr(values: &[Value]) -> String {
    let Some(selected) = draft::selected_variable_receivers(values) else {
        return "VARIABLE_RECEIVERS_REQUIRE_UNIQUE_U8_INDICES_AND_VALID_FORMS".to_owned();
    };
    let rows = selected.iter().map(|(index, form)| {
        format!("({index}, tcl_registry::resolved_invocation::VariableReceiverOperandForm::{form:?})")
    }).collect::<Vec<_>>();
    format!("Some(&[{}])", rows.join(", "))
}

/// Render a slice of catalogue enum variants such as `&[ArgRole::Body]`.
pub(super) fn enum_slice(ty: &str, values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|value| tcl_spectcl::catalogue::rust_path(ty, as_str(value)))
        .collect();
    format!("&[{}]", items.join(", "))
}

pub(super) fn body_interpreter_expr(value: &Value) -> Option<String> {
    match value.get("kind")?.as_str()? {
        "Current" => Some("BodyInterpreter::Current".to_owned()),
        "ProcessShared" => Some("BodyInterpreter::ProcessShared".to_owned()),
        "Unknown" => Some("BodyInterpreter::Unknown".to_owned()),
        "ThreadArgument" => Some(format!(
            "BodyInterpreter::ThreadArgument({})",
            value.get("index")?.as_u64()?
        )),
        "Argument" => Some(format!(
            "BodyInterpreter::Argument({})",
            value.get("index")?.as_u64()?
        )),
        _ => None,
    }
}

/// Render a bitflag union such as `Traits::A.union(Traits::B)`, or
/// `TYPE::empty()` when no bits are set.
///
/// `.union()` rather than `|`: the option / subcommand tables are hoisted into
/// `const` items, and `bitflags`' `BitOr` impl is not `const`, so a `|` chain
/// there fails to compile ("cannot call non-const operator in constants").
/// `union` is a `const fn`, so it is correct in both a `const` and the `fn`
/// body that returns the spec.
pub(super) fn flag_union(ty: &str, values: &[Value]) -> String {
    let mut names = values
        .iter()
        .map(|v| tcl_spectcl::catalogue::rust_path(ty, as_str(v)));
    let Some(head) = names.next() else {
        return format!("{ty}::empty()");
    };
    names.fold(head, |acc, name| acc + ".union(" + &name + ")")
}

/// Render a surface row list from the canonical dialect-name list a draft
/// holds.
///
/// Emits the readable aggregate constants (`SpecSurface::ALL_TCL`,
/// `SpecSurface::TCL85_PLUS`) when the list matches one exactly, so the
/// output reads like the hand-written specs rather than a five-row list.
pub(crate) fn dialect_set(values: &[Value]) -> String {
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
        return "None".to_owned();
    }
    let mut remaining = names.clone();
    let mut parts: Vec<String> = Vec::new();
    for (constant, members) in AGGREGATES {
        if members.iter().all(|m| remaining.contains(m)) {
            parts.push((*constant).to_owned());
            remaining.retain(|n| !members.contains(n));
            break;
        }
    }
    for name in remaining {
        // An unrecognised name is emitted as a comment rather than a bare
        // identifier — `SpecSurface::f5-tmsh` is not valid Rust, and
        // rendering it silently produced a file that only failed at
        // `cargo build`.
        if let Some(release) = tcl_spectcl::catalogue::jim_and_later_release(name) {
            // No named constant: the row is open-ended at one Jim release.
            parts.push(format!(
                "core_in(tcl_dialect::model::Family::Jim, &[({release:?}, None)])"
            ));
            continue;
        }
        let Some(constant) = dialect_constant(name) else {
            parts.push(format!("/* unknown dialect {name:?} */"));
            continue;
        };
        parts.push(constant.to_owned());
    }
    match parts.len() {
        0 => "None".to_owned(),
        // A lone open-ended row is still a row list.
        1 if parts[0].starts_with("core_in(") => {
            format!("surface![SpecSurface::{}]", parts.remove(0))
        }
        1 => format!("SpecSurface::{}", parts.remove(0)),
        // Several named surfaces compose by listing their rows.
        _ => {
            let rows: Vec<String> = parts
                .into_iter()
                .map(|part| format!("SpecSurface::{part}"))
                .collect();
            format!("surface![{}]", rows.join(", "))
        }
    }
}

/// The surface constant for a canonical dialect name, or `None` when the
/// name is not one the registry knows.
pub(super) fn dialect_constant(name: &str) -> Option<&'static str> {
    Some(match name {
        "tcl8.4" => "TCL84",
        "tcl8.5" => "TCL85",
        "tcl8.6" => "TCL86",
        "tcl9.0" => "TCL90",
        "tcl9.1" => "TCL91",
        "f5-irules" => "IRULES",
        "jim" => "JIM",
        "f5-iapps" => "IAPPS",
        "tk" => "TK",
        "expect" => "EXPECT",
        "bpf" => "BPF",
        "f5-tmsh" => "TMSH",
        "f5-bigip" => "BIGIP",
        "spectcl" => "SPECTCL",
        "sslictcl" => "SSLICTCL",
        _ => return None,
    })
}

pub(super) fn arity_expr(value: &Value) -> String {
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
    let expression = match also {
        None => base,
        Some(n) => format!("{base}.with_also_exact({n})"),
    };
    if value["count"] == "positionals" {
        format!("{expression}.with_positionals()")
    } else {
        expression
    }
}

pub(super) fn appended_arity_expr(value: &Value) -> String {
    match as_str(&value["kind"]) {
        "Exactly" => format!("AppendedArity::Exactly({})", as_u64(&value["n"])),
        "AtLeast" => format!("AppendedArity::AtLeast({})", as_u64(&value["n"])),
        _ => "AppendedArity::Unknown".to_owned(),
    }
}

pub(super) fn role_map_expr(values: &[Value]) -> String {
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

pub(super) fn presentation_map_expr(values: &[Value]) -> String {
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

pub(super) fn domain_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .filter_map(|entry| {
            Some(format!(
                "({}, {})",
                as_u64(&entry["index"]),
                integer_domain_expr(&entry["domain"])?
            ))
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

pub(super) fn prefix_map_expr(values: &[Value]) -> String {
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

fn callback_taint_input_expr(value: &Value) -> String {
    match as_str(value) {
        "%P" => "CallbackTaintInput::TK_PROPOSED_VALUE".to_owned(),
        "%s" => "CallbackTaintInput::TK_CURRENT_VALUE".to_owned(),
        "%S" => "CallbackTaintInput::TK_EDIT_TEXT".to_owned(),
        "%A" => "CallbackTaintInput::TK_EVENT_CHAR".to_owned(),
        "%K" => "CallbackTaintInput::TK_EVENT_KEYSYM".to_owned(),
        spelling => format!("CallbackTaintInput::TkPercent({})", rust_string(spelling)),
    }
}

pub(super) fn callback_taint_inputs_expr(values: &[Value]) -> String {
    let inputs: Vec<String> = values.iter().map(callback_taint_input_expr).collect();
    format!("&[{}]", inputs.join(", "))
}

pub(super) fn callback_taint_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, {})",
                as_u64(&entry["index"]),
                callback_taint_inputs_expr(as_array(&entry["inputs"]))
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

pub(super) fn arg_type_map_expr(values: &[Value], indent: &str) -> String {
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

pub(super) fn arg_value_expr(entry: &Value, indent: &str) -> String {
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
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    if let Some(code) = entry["code"].as_i64() {
        parts.push(format!("{indent}    code: Some({code}),"));
    }
    format!(
        "{indent}ArgValue {{\n{}\n{indent}    ..ArgValue::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

pub(super) fn arg_value_map_expr(values: &[Value], indent: &str) -> String {
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

pub(super) fn integer_domain_expr(value: &Value) -> Option<String> {
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
