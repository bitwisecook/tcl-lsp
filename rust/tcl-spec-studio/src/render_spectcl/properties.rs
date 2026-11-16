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

//! One property statement per field kind: scalars, flags, text, enums, sets, lists, counts, expression words and hooks, and the `TODO` for a field that did not survive.

use super::gaps::gap;
use super::tcl_words::{
    as_array, available_statement, dialect_set_word, emit_row, list_word, push_flag, push_word,
    str_list_word, word,
};
use super::{Ctx, Loss, Out};
use crate::draft;
use crate::draft::Draft;
use serde_json::Value;

/// Whether `draft` records `key` as one whose defining expression seeding
/// could not recover.
pub(super) fn unrecovered(draft: &Draft, key: &str) -> bool {
    draft
        .get(draft::UNRENDERABLE_KEY)
        .map(as_array)
        .is_some_and(|keys| keys.iter().any(|k| k.as_str() == Some(key)))
}

/// The `# TODO(spectcl):` line for a field that did not survive.
pub(super) fn todo(out: &mut Out, key: &str) {
    let Some(gap) = gap(key) else {
        out.comment(&format!(
            "TODO(spectcl): `{key}` is set on the source spec and has no DSL spelling."
        ));
        out.losses.push(Loss {
            key: key.to_owned(),
            reason: "set on the source spec, with no DSL spelling".to_owned(),
        });
        return;
    };
    let spelling = if gap.spelling.is_empty() {
        String::new()
    } else {
        format!(" The DSL spells it `{}`, but ", gap.spelling)
    };
    let joiner = if spelling.is_empty() { " — " } else { "" };
    out.comment(&format!(
        "TODO(spectcl): `{key}` is set on the source spec.{spelling}{joiner}{}.",
        gap.kind.why()
    ));
    out.losses.push(Loss {
        key: key.to_owned(),
        reason: gap.kind.why().to_owned(),
    });
}

/// The `# TODO(spectcl):` line for text no Tcl word can carry verbatim.
///
/// A braced word is the only faithful spelling (see [`word`]), so a string
/// with an unbalanced brace or a backslash-newline has to be dropped rather
/// than written down wrong.
pub(super) fn unwritable(out: &mut Out, key: &str) {
    out.comment(&format!(
        "TODO(spectcl): `{key}` holds text no Tcl word carries verbatim — an \
         unbalanced brace or a backslash-newline. A braced word is the only \
         faithful spelling, so the statement is dropped rather than written \
         down wrong."
    ));
    out.losses.push(Loss {
        key: key.to_owned(),
        reason: "the text has an unbalanced brace or a backslash-newline, which no \
                 Tcl word carries verbatim"
            .to_owned(),
    });
}

/// Emit `key` when it is set, as a one-value property statement.
pub(super) fn scalar(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str, value: &str) {
    if ctx.set(draft, key) {
        out.line(&format!("{key} {value}"));
    }
}

/// Emit a boolean property in the DSL's bare-word / explicit-`no` spelling.
pub(super) fn flag(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if !ctx.set(draft, key) {
        return;
    }
    if draft[key].as_bool() == Some(false) {
        out.line(&format!("{key} no"));
    } else {
        out.line(key);
    }
}

/// Emit a text / opt-text property.
pub(super) fn text(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(value) = draft[key].as_str()
    {
        match word(value) {
            Some(spelling) => out.line(&format!("{key} {spelling}")),
            None => unwritable(out, key),
        }
    }
}

/// Emit an enum property, whose draft value is already the catalogue spelling.
///
/// A variant carrying a payload (`Rebinarifies { data_index: 2 }`) is *not*
/// one word, and no loader vocabulary lists it, so it becomes a `TODO` rather
/// than a statement the loader would drop half of.
pub(super) fn enum_word(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(value) = draft[key].as_str()
    {
        if value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            out.line(&format!("{key} {value}"));
        } else {
            todo(out, key);
        }
    }
}

/// The DSL word a draft key is written as. Availability is the one key whose
/// two names differ: the draft carries the spec field name (`surface`), the
/// 1.x word it degrades to is `dialects`.
fn dsl_word(key: &str) -> &str {
    if key == "surface" { "dialects" } else { key }
}

/// Emit a flag-set property — a trait, taint-colour, or dialect set.
pub(super) fn set_word(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    // Availability is written in the 2.0 algebra wherever it carries the
    // set; `dialects` stays for the bits it has no provider for, and for
    // `safe_on_uninit`, which holds a dialect set but is not an
    // availability question and has no `available` reader.
    if key == "surface"
        && ctx.availability
        && ctx.set(draft, key)
        && draft[key].is_array()
        && let Some(rows) = available_statement(&draft[key])
    {
        out.line(&format!("available {rows}"));
        return;
    }
    if ctx.set(draft, key) && draft[key].is_array() {
        let spelling = if is_dialect_set(key) {
            dialect_set_word(&draft[key])
        } else {
            str_list_word(&draft[key])
        };
        match spelling {
            Some(spelling) => out.line(&format!("{} {spelling}", dsl_word(key))),
            None => unwritable(out, key),
        }
    }
}

/// Emit the data form of the Tk geometry-manager descriptor.
pub(super) fn tk_geometry_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "tk_geometry") {
        return;
    }
    let Some(fields) = draft["tk_geometry"].as_object() else {
        todo(out, "tk_geometry");
        return;
    };
    let Some(policy @ ("Independent" | "Exclusive")) =
        fields.get("container_policy").and_then(Value::as_str)
    else {
        todo(out, "tk_geometry");
        return;
    };
    let container_option = fields
        .get("container_option")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let direct_form = fields
        .get("direct_form")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let placement_subcommand = fields
        .get("placement_subcommand")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let release_subcommands: Vec<String> = fields
        .get("release_subcommands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let mut row = vec!["tk_geometry".to_owned(), policy.to_owned()];
    let mut lost = false;
    if let Some(container_option) = container_option {
        row.push("-container-option".to_owned());
        push_word(&mut row, &mut lost, word(&container_option));
    }
    if direct_form {
        row.push("-direct-form".to_owned());
    }
    if let Some(placement_subcommand) = placement_subcommand {
        row.push("-placement-subcommand".to_owned());
        push_word(&mut row, &mut lost, word(&placement_subcommand));
    }
    if !release_subcommands.is_empty() {
        push_flag(
            &mut row,
            &mut lost,
            "-release-subcommands",
            list_word(&release_subcommands),
        );
    }
    emit_row(out, "tk_geometry", &row, lost, "");
}

/// Whether a flag-set field holds dialect members, which take the shorthands.
fn is_dialect_set(key: &str) -> bool {
    matches!(key, "surface" | "safe_on_uninit")
}

/// Emit a `&'static [&'static str]` property.
pub(super) fn text_list(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    set_word(out, ctx, draft, key);
}

/// Emit an index-list property, keeping the tri-state (`{}` is *declared
/// empty*, absent is unset).
pub(super) fn index_list(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if !ctx.set(draft, key) {
        return;
    }
    let Some(items) = draft[key].as_array() else {
        return;
    };
    let indices: Vec<String> = items.iter().map(ToString::to_string).collect();
    out.line(&format!("{key} {{{}}}", indices.join(" ")));
}

/// Authored receiver grammar retains its form alongside the argv position.
pub(super) fn variable_receiver_words(items: &[Value]) -> String {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm;
    let Some(selected) = draft::selected_variable_receivers(items) else {
        return "{invalid-index invalid-receiver-form}".to_owned();
    };
    selected
        .iter()
        .map(|(index, form)| match form {
            VariableReceiverOperandForm::Combined => index.to_string(),
            VariableReceiverOperandForm::TraceSubject => format!("{{{index} trace-subject}}"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn variable_receiver_list(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if ctx.set(draft, "variable_receivers")
        && let Some(items) = draft["variable_receivers"].as_array()
    {
        out.line(&format!(
            "variable_receivers {{{}}}",
            variable_receiver_words(items)
        ));
    }
}

/// Emit a count property.
pub(super) fn count(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(n) = draft[key].as_u64()
    {
        out.line(&format!("{key} {n}"));
    }
}

/// Emit an expression-valued property whose DSL spelling is one word.
pub(super) fn expr_word(
    out: &mut Out,
    ctx: &Ctx<'_>,
    draft: &Draft,
    key: &str,
    spell: impl Fn(&str) -> Option<String>,
) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        match draft[key].as_str().and_then(spell) {
            Some(spelling) => out.line(&format!("{key} {spelling}")),
            None => todo(out, key),
        }
    }
}

/// Emit an expression-valued property whose DSL spelling is a flag row.
pub(super) fn expr_row(
    out: &mut Out,
    ctx: &Ctx<'_>,
    draft: &Draft,
    key: &str,
    spell: impl Fn(&str) -> Option<Vec<String>>,
) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        match draft[key].as_str().and_then(spell) {
            Some(row) => out.row(&row, "-description"),
            None => todo(out, key),
        }
    }
}

/// Emit a `RustExpr` field that the DSL cannot carry, as a `TODO`.
pub(super) fn gap_todo(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        todo(out, key);
    }
}

/// Record a field whose spelling postdates the explicitly requested output
/// vocabulary.
pub(super) fn vocabulary_todo(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str, since: &str) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        let reason = format!("requires SpecTcl {since} vocabulary");
        out.comment(&format!("TODO(spectcl): `{key}` {reason}."));
        out.losses.push(Loss {
            key: key.to_owned(),
            reason,
        });
    }
}

/// Emit a hook field in the documented `-native ID` form.
///
/// Not a loss: the loader installs the family's abstention for a `-native`
/// hook exactly as it does for a Tcl body, and seeding records the reloaded
/// field the same way it recorded the shipped one — "set, expression not
/// recovered".
pub(super) fn native_hook(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if unrecovered(draft, key) {
        out.line(&ctx.native(key).join(" "));
    }
}

/// Emit a closed-catalogue compiler hook, whose draft value *is* the variant
/// name the DSL takes.
pub(super) fn catalogue_hook(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(id) = draft[key].as_str()
    {
        out.line(&format!("{key} -native {id}"));
    }
}
