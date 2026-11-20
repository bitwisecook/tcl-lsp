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

//! Arity rows with their windows and the per-argument rows of a command or subcommand.

use super::*;

/// `arity N`, `N..M`, `N..`, `..M`, `..`, plus `-step` / `-also` / `-positionals`.
pub(super) fn arity_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "arity") {
        return;
    }
    let word = arity_word(&draft["arity"]);
    match word.strip_suffix(" -positionals") {
        // An older loader drops the word, so the row is written as that
        // loader reads it and the lost counting basis is recorded.
        Some(counted) if !ctx.speaks(POSITIONAL_ARITY_VERSION) => {
            out.line(&format!("arity {counted}"));
            let reason = format!("requires SpecTcl {POSITIONAL_ARITY_VERSION} vocabulary");
            out.comment(&format!("TODO(spectcl): `arity -positionals` {reason}."));
            out.losses.push(Loss {
                key: "arity".to_owned(),
                reason,
            });
        }
        _ => out.line(&format!("arity {word}")),
    }
}

/// `script_lookup_scope`, or its `TODO` under a vocabulary that predates it.
pub(super) fn script_lookup_scope_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if ctx.speaks(SCRIPT_LOOKUP_SCOPE_VERSION) {
        enum_word(out, ctx, draft, "script_lookup_scope");
    } else {
        vocabulary_todo(
            out,
            ctx,
            draft,
            "script_lookup_scope",
            SCRIPT_LOOKUP_SCOPE_VERSION,
        );
    }
}

/// An arity draft value as the DSL's `RANGE ?-step N? ?-also N? ?-positionals?` words.
pub(super) fn arity_word(value: &Value) -> String {
    arity_shape(value)
}

/// The shape words of one arity value, shared by the plain row and every
/// window so a window's shape is spelled exactly as the plain row spells it.
fn arity_shape(value: &Value) -> String {
    let min = value["min"].as_u64().unwrap_or(0);
    let max = value["max"].as_u64();
    let step = value["step"].as_u64().unwrap_or(0);
    let also = value["also_exact"].as_u64();
    let range = match (min, max) {
        (m, Some(x)) if m == x && step == 0 => m.to_string(),
        (0, None) => "..".to_owned(),
        (m, None) => format!("{m}.."),
        (0, Some(x)) => format!("..{x}"),
        (m, Some(x)) => format!("{m}..{x}"),
    };
    let mut row = range;
    if step != 0 {
        let _ = write!(row, " -step {step}");
    }
    if let Some(also) = also {
        let _ = write!(row, " -also {also}");
    }
    if value["count"] == "positionals" {
        row.push_str(" -positionals");
    }
    row
}

/// `arity SHAPE -introduced V ?-deprecated V? ?-retired V?` — one row per
/// version window (`SpecTcl` 1.2).
///
/// Emitted after the plain `arity` row, matching the order the loader reads
/// them and the order `CommandSpec::DEFAULT` declares them. A spec with no
/// windows renders nothing, which is every spec whose signature never
/// changed.
pub(super) fn arity_window_rows(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "arity_windows") {
        return;
    }
    let windows = as_array(&draft["arity_windows"]).to_vec();
    for window in &windows {
        let mut row = format!("arity {}", arity_shape(&window["arity"]));
        let lifecycle = &window["lifecycle"];
        for (flag, key) in [
            ("-introduced", "introduced"),
            ("-deprecated", "deprecated"),
            ("-retired", "retired"),
        ] {
            if let Some(at) = lifecycle.get(key).and_then(Value::as_str) {
                let _ = write!(row, " {flag} {at}");
            }
        }
        out.line(&row);
    }
}

/// The per-argument rows: seven schema keys, one statement per index.
///
/// Indices are visited in the order the tables themselves list them — roles
/// first, then the tables that only ever accompany them — so a spec that
/// declares its roles out of index order keeps that order on the way back in.
pub(super) fn arg_row_statements(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    let table =
        |key: &str| -> Vec<Value> { as_array(draft.get(key).unwrap_or(&Value::Null)).to_vec() };
    let roles = table("arg_roles");
    let types = table("arg_types");
    let values = table("arg_values");
    let presentation = table("arg_presentation");
    let prefixes = table("command_prefixes");
    let domains = table("arg_domains");
    let closed: Vec<u64> = as_array(draft.get("closed_value_args").unwrap_or(&Value::Null))
        .iter()
        .filter_map(Value::as_u64)
        .collect();

    let mut order: Vec<u64> = Vec::new();
    let note = |index: Option<u64>, order: &mut Vec<u64>| {
        if let Some(index) = index
            && !order.contains(&index)
        {
            order.push(index);
        }
    };
    for entry in roles
        .iter()
        .chain(&types)
        .chain(&values)
        .chain(&presentation)
        .chain(&prefixes)
        .chain(&domains)
    {
        note(entry["index"].as_u64(), &mut order);
    }
    for index in &closed {
        note(Some(*index), &mut order);
    }

    let at = |entries: &[Value], index: u64| -> Option<Value> {
        entries
            .iter()
            .find(|entry| entry["index"].as_u64() == Some(index))
            .cloned()
    };

    // An `arg` row names one role, and the loader keeps the first a position
    // is given, so a position the table gives two is written thinner.
    if order.iter().any(|index| {
        roles
            .iter()
            .filter(|entry| entry["index"].as_u64() == Some(*index))
            .count()
            > 1
    }) {
        let reason = "a position carries more than one role; an `arg` row names one".to_owned();
        out.comment(&format!("TODO(spectcl): `arg_roles`: {reason}."));
        out.losses.push(Loss {
            key: "arg_roles".to_owned(),
            reason,
        });
    }

    for index in order {
        let mut row = vec!["arg".to_owned(), index.to_string()];
        let mut lost = false;
        if let Some(entry) = at(&roles, index) {
            row.push("-role".to_owned());
            row.push(str_of(&entry["role"]).to_owned());
        }
        if let Some(entry) = at(&types, index) {
            if let Some(expected) = entry["expected"].as_str() {
                row.push("-type".to_owned());
                row.push(expected.to_owned());
            }
            if entry["shimmers"].as_bool() == Some(true) {
                row.push("-shimmers".to_owned());
            }
            let transparent = as_array(&entry["transparent_from"]);
            if !transparent.is_empty() {
                push_flag(
                    &mut row,
                    &mut lost,
                    "-transparent",
                    str_list_word(&entry["transparent_from"]),
                );
            }
        }
        if let Some(entry) = at(&values, index) {
            push_values(
                &mut row,
                &mut lost,
                ctx,
                &entry["values"],
                &format!("{}-arg{index}", ctx.scope),
            );
        }
        if closed.contains(&index) {
            row.push("-closed".to_owned());
        }
        if let Some(entry) = at(&presentation, index) {
            row.push("-layout".to_owned());
            row.push(str_of(&entry["presentation"]).to_owned());
        }
        if let Some(entry) = at(&prefixes, index) {
            row.push("-appends".to_owned());
            row.push(appended_arity_word(&entry["arity"]));
        }
        if let Some(word) = at(&domains, index)
            .and_then(|entry| super::option_rows::integer_domain_word(&entry["domain"]))
        {
            row.push("-domain".to_owned());
            row.push(word);
        }
        emit_row(out, "arg_values", &row, lost, "-detail");
    }
}

/// The registry-declared user-controlled substitutions for positional deferred
/// callbacks. This is deliberately separate from `arg` rows: it describes a
/// runtime injection into the stored callback, not a static syntax role.
pub(super) fn callback_taint_input_table(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "callback_taint_inputs") {
        return;
    }
    let rows: Vec<String> = as_array(&draft["callback_taint_inputs"])
        .iter()
        .filter_map(|entry| {
            let index = entry["index"].as_u64()?;
            let inputs = as_array(&entry["inputs"]);
            (!inputs.is_empty())
                .then(|| str_list_word(&Value::Array(inputs.to_vec())))
                .flatten()
                .map(|inputs| format!("{{{index} {inputs}}}"))
        })
        .collect();
    if !rows.is_empty() {
        out.line(&format!("callback_taint_inputs {{{}}}", rows.join(" ")));
    }
}

/// `{Exactly N}` / `{AtLeast N}` / `Unknown`.
pub(super) fn appended_arity_word(value: &Value) -> String {
    let kind = str_of(&value["kind"]);
    match value["n"].as_u64() {
        Some(n) => format!("{{{kind} {n}}}"),
        None => kind.to_owned(),
    }
}
