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

//! `option` rows with their values and integer domains, option conflicts and versioned argument values.

use super::argument_rows::appended_arity_word;
use super::properties::{enum_word, todo};
use super::rust_expressions::{
    deprecation_fix_row, option_conflict_rows, option_effect_block, option_effect_family_rows,
    versioned_arg_value_rows,
};
use super::tcl_words::{
    as_array, braced, emit_row, has_lifecycle, list_word, name_word, push_availability_flag,
    push_flag, push_lifecycle_flags, str_list_word, word,
};
use super::{Ctx, Loss, Out, str_of};
use crate::draft;
use crate::draft::{Draft, OPTION_DEPRECATION_FIX_HOOK_KEY};
use serde_json::Value;

/// Push `-values {…}` or `-values-from NAME` onto a row.
///
/// The inline form carries the value words and nothing else, so a table whose
/// rows have a detail, a version floor, or a completion code is hoisted to a
/// pack-level `values` declaration instead.
pub(super) fn push_values(
    row: &mut Vec<String>,
    lost: &mut bool,
    ctx: &mut Ctx<'_>,
    values: &Value,
    hint: &str,
) {
    let entries = as_array(values);
    if entries.is_empty() {
        return;
    }
    // Anything the inline `-values {…}` word cannot carry — a detail, either
    // version axis, or a completion code — hoists the whole set to a
    // pack-level table, which has a row per value and room for all of it.
    let plain = entries.iter().all(|entry| {
        str_of(&entry["detail"]).is_empty()
            && entry["min_tcl"].is_null()
            && entry["code"].is_null()
            && !has_lifecycle(entry)
    });
    if plain {
        let words: Vec<&str> = entries
            .iter()
            .map(|entry| str_of(&entry["value"]))
            .collect();
        push_flag(row, lost, "-values", list_word(&words));
    } else {
        let name = ctx.tables.intern(hint, values, &ctx.owner);
        row.push("-values-from".to_owned());
        row.push(name);
    }
}

/// One `option NAME …` row.
#[allow(clippy::too_many_lines)]
pub(super) fn option_row(out: &mut Out, ctx: &mut Ctx<'_>, option: &Value) {
    let name = str_of(&option["name"]);
    let mut row = vec!["option".to_owned(), name_word(name)];
    let mut lost = false;
    let value = &option["value"];
    if !value.is_null() {
        push_flag(&mut row, &mut lost, "-takes", word(str_of(&value["hint"])));

        let arity = &value["arity"];
        match str_of(&arity["kind"]) {
            "Fixed" => {
                row.push("-arity".to_owned());
                row.push(format!("{{Fixed {}}}", arity["n"].as_u64().unwrap_or(0)));
            }
            "OptionalOneOf" => {
                let values: Option<Vec<String>> = arity["values"]
                    .as_array()
                    .map_or(Some(Vec::new()), |values| {
                        values.iter().map(|v| word(str_of(v))).collect()
                    });
                match values {
                    Some(values) if !values.is_empty() => {
                        row.push("-arity".to_owned());
                        row.push(format!("{{OptionalOneOf {}}}", values.join(" ")));
                    }
                    _ => lost = true,
                }
            }
            "OptionalNotOption" => {
                row.push("-arity".to_owned());
                row.push("OptionalNotOption".to_owned());
            }
            "Hook" => {
                row.push("-arity-hook".to_owned());
                row.push("-native".to_owned());
                row.push(format!(
                    "{}::{}::arity",
                    ctx.scope,
                    name.trim_start_matches('-')
                ));
            }
            _ => {}
        }
        let role = str_of(&value["role"]);
        if role != "Value" {
            row.push("-role".to_owned());
            row.push(role.to_owned());
        }
        let also_role = value["also_role"].as_str();
        if let Some(also) = also_role {
            row.push("-also-role".to_owned());
            row.push(also.to_owned());
        }
        let body_kind = str_of(&value["body_kind"]);
        if body_kind != "Plain" {
            row.push("-body-kind".to_owned());
            row.push(body_kind.to_owned());
        }
        let script_timing = str_of(&value["script_timing"]);
        if !script_timing.is_empty() && script_timing != "SameInvocation" {
            row.push("-script-timing".to_owned());
            row.push(script_timing.to_owned());
        }
        let callback_inputs = as_array(&value["callback_taint_inputs"]);
        if !callback_inputs.is_empty()
            && let Some(inputs) = str_list_word(&Value::Array(callback_inputs.to_vec()))
        {
            row.push("-callback-taint-inputs".to_owned());
            row.push(inputs);
        }
        if str_of(&value["variable_scope"]) == "Global" {
            row.push("-variable-scope".to_owned());
            row.push("Global".to_owned());
        }
        push_values(
            &mut row,
            &mut lost,
            ctx,
            &value["values"],
            &format!("{}-{}", ctx.scope, name.trim_start_matches('-')),
        );
        if value["closed"].as_bool() == Some(true) {
            row.push("-closed".to_owned());
        }
        if let Some(domain) = integer_domain_word(&value["integer"]) {
            row.push("-integer".to_owned());
            row.push(domain);
        }
        let appended = &value["appended_arity"];
        if str_of(&appended["kind"]) != "Unknown" {
            row.push("-appends".to_owned());
            row.push(appended_arity_word(appended));
        }
        if value["taints_var_write"].as_bool() == Some(true)
            && (role == "VarWrite" || also_role == Some("VarWrite"))
        {
            row.push("-taints-var-write".to_owned());
        }
    }
    if let Some(aliases) = option["aliases"].as_array()
        && !aliases.is_empty()
    {
        push_flag(
            &mut row,
            &mut lost,
            "-aliases",
            str_list_word(&option["aliases"]),
        );
    }
    if let Some(dialects) = option["surface"].as_array() {
        push_availability_flag(
            &mut row,
            &mut lost,
            &Value::Array(dialects.clone()),
            ctx.availability,
        );
    }
    if let Some(n) = option["min_abbrev"].as_u64() {
        row.push("-min-abbrev".to_owned());
        row.push(n.to_string());
    }
    if !option["effect"].is_null() {
        match option_effect_block(&option["effect"]) {
            Some(block) => {
                row.push("-effect".to_owned());
                row.push(block);
                push_flag(
                    &mut row,
                    &mut lost,
                    "-family",
                    word(str_of(&option["effect"]["family"])),
                );
            }
            None => lost = true,
        }
    }
    push_lifecycle_flags(&mut row, &mut lost, option);
    // The data form of the quick-fix hook: the same flags the command-level
    // `deprecation_fix` statement takes, wrapped in one block word. The
    // callback variant has no data to write, so it stays a reported loss.
    let mut unwritable_fix = option[draft::OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY]
        .as_bool()
        .unwrap_or(false);
    if let Some(expr) = option["deprecation_fix"].as_str() {
        match deprecation_fix_row(expr)
            .map(|words| words[1..].join(" "))
            .and_then(|flags| braced(&flags))
        {
            Some(block) => {
                row.push("-deprecation-fix".to_owned());
                row.push(block);
            }
            None => unwritable_fix = true,
        }
    }
    let detail = str_of(&option["detail"]);
    if !detail.is_empty() {
        push_flag(&mut row, &mut lost, "-detail", braced(detail));
    }
    emit_row(out, "options", &row, lost, "-detail");
    if unwritable_fix {
        out.indented(|out| {
            out.comment(&format!(
                "TODO(spectcl): `{name}` carries a contextual deprecation fix; \
                 `-deprecation-fix {{…}}` writes the data form only."
            ));
            out.losses.push(Loss {
                key: OPTION_DEPRECATION_FIX_HOOK_KEY.to_owned(),
                reason: "the fix is a registry callback, not data".to_owned(),
            });
        });
    }
}

/// `{Range lo hi}` / `Any` / `Port`, with the `max` / `min` sentinels the DSL
/// documents.
fn integer_domain_word(value: &Value) -> Option<String> {
    match str_of(&value["kind"]) {
        "Any" => Some("Any".to_owned()),
        "Port" => Some("Port".to_owned()),
        "Range" => {
            let bound = |n: Option<i64>, sentinel: &str| -> String {
                match n {
                    Some(i64::MAX) => "max".to_owned(),
                    Some(i64::MIN) => "min".to_owned(),
                    Some(n) => n.to_string(),
                    None => sentinel.to_owned(),
                }
            };
            Some(format!(
                "{{Range {} {}}}",
                bound(value["lo"].as_i64(), "min"),
                bound(value["hi"].as_i64(), "max"),
            ))
        }
        _ => None,
    }
}

/// The `option …` / `option_conflict …` rows, which the ports keep together.
pub(super) fn option_block(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    if ctx.set(draft, "options") {
        out.gap();
        for option in as_array(&draft["options"]).to_vec() {
            option_row(out, ctx, &option);
        }
    }
    enum_word(out, ctx, draft, "option_placement");
    if ctx.set(draft, "option_relations")
        && let Some(expr) = draft["option_relations"].as_str()
    {
        match option_conflict_rows(expr, ctx.availability) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "option_relations"),
        }
    }
    if ctx.set(draft, "option_effect_families")
        && let Some(expr) = draft["option_effect_families"].as_str()
    {
        match option_effect_family_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "option_effect_families"),
        }
    }
}

/// The `versioned_arg_value N VALUE …` rows of a command or subcommand body —
/// one spelling, both levels, since 1.1 made the statement legal at either.
pub(super) fn versioned_arg_value_rows_for(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "versioned_arg_values") {
        return;
    }
    let Some(expr) = draft["versioned_arg_values"].as_str() else {
        return;
    };
    match versioned_arg_value_rows(expr) {
        Some(rows) => {
            for row in rows {
                out.row(&row, "");
            }
        }
        None => todo(out, "versioned_arg_values"),
    }
}
