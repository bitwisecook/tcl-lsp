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

//! The `semantics` block and the `effects` and `side_effect` rows of a scope.

use super::properties::unrecovered;
use super::tcl_words::{as_array, emit_row, push_availability_flag, push_lifecycle_flags};
use super::{Ctx, Out, str_of};
use crate::draft::Draft;
use serde_json::Value;

/// `semantics { effects …; result -semantic T; stores …; iterate … }` and
/// `evaluate …`: [`crate::draft::semantics_value`]'s captured plan, or a
/// `-native SCOPE::FIELD` placeholder — the route picker's read-only
/// fallback — when seeding could not recover it (a shipped, compiled-in
/// specialisation; a declared implementation's body; an option-level
/// `-evaluate` decline). The body box itself belongs to
/// `evaluate -implementation`'s own author-facing editor, over the route
/// this function reads back rather than a reconstruction from `CommandSpec`.
pub(super) fn semantics_block(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if unrecovered(draft, "semantics") {
        out.line(&ctx.native("semantics").join(" "));
        return;
    }
    if !ctx.set(draft, "semantics") {
        return;
    }
    let value = &draft["semantics"];
    if value.as_str() == Some("none") {
        out.line("semantics none");
        return;
    }
    let effects: Vec<&str> = as_array(&value["effects"])
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let result = value["result"].as_str();
    let stores = value.get("stores").filter(|v| !v.is_null());
    let iterate = value.get("iterate").filter(|v| !v.is_null());
    if !effects.is_empty() || result.is_some() || stores.is_some() || iterate.is_some() {
        let mut body = Out::at(out.indent + 1);
        if !effects.is_empty() {
            body.line(&format!("effects {{{}}}", effects.join(" ")));
        }
        if let Some(result) = result {
            body.line(&format!("result -semantic {result}"));
        }
        if let Some(stores) = stores {
            let targets: Vec<String> = as_array(&stores["targets"])
                .iter()
                .map(ToString::to_string)
                .collect();
            body.line(&format!(
                "stores -targets {{{}}} -outcome {}",
                targets.join(" "),
                str_of(&stores["outcome"])
            ));
        }
        if let Some(iterate) = iterate {
            let mut inner = Out::at(body.indent + 1);
            inner.line(&format!("binder -arg {}", iterate["binder"]));
            inner.line(&format!(
                "iterable -arg {} -kind {}",
                iterate["iterable"],
                str_of(&iterate["kind"])
            ));
            if let Some(arg) = iterate["body"].as_u64() {
                inner.line(&format!("body -arg {arg}"));
            }
            if let Some(yields) = iterate["yields"].as_str() {
                inner.line(&format!("yield -semantic {yields}"));
            }
            if let Some(cardinality) = iterate["cardinality"].as_u64() {
                inner.line(&format!("cardinality -arg {cardinality}"));
            }
            if iterate["zero_iterations_bind"].as_bool() == Some(true) {
                inner.line("zero_iterations -bindings bind");
            }
            body.line("iterate {");
            body.block(&inner);
            body.line("}");
        }
        out.line("semantics {");
        out.block(&body);
        out.line("}");
    }
    match value["evaluation"]["kind"].as_str() {
        Some("none") if value["evaluation"]["reason"].as_str() == Some("declared") => {
            out.line("evaluate none");
        }
        Some("direct") => {
            out.line(&format!(
                "evaluate -direct {}",
                str_of(&value["evaluation"]["id"])
            ));
        }
        Some("expression") => {
            out.line(&format!(
                "evaluate -expression {}",
                str_of(&value["evaluation"]["language"])
            ));
        }
        // `none` with any reason other than `declared` — `unauthored` (no
        // `evaluate` statement at all) chief among them, since a
        // command/subcommand scope's own `evaluate none` cannot produce any
        // other — and no `evaluation` key at all: nothing was said, so
        // nothing is written.
        _ => {}
    }
}

pub(super) fn side_effect_rows(out: &mut Out, draft: &Draft, availability: bool) {
    let rows = draft.get("side_effects").unwrap_or(&Value::Null);
    side_effect_rows_of(out, rows, availability);
    effects_row(
        out,
        draft.get("side_effects_knowledge").unwrap_or(&Value::Null),
        !as_array(rows).is_empty(),
    );
}

/// A `refine` body's effect rows: `side_effects none` replaces the parent's
/// effects with none known, and an `effects` row states anything stronger.
pub(super) fn form_side_effect_rows(out: &mut Out, form: &Value, availability: bool) {
    let knowledge = &form["side_effects_knowledge"];
    match form["side_effects"].as_array() {
        Some(effects) if effects.is_empty() => {
            if knowledge["level"].as_str() == Some("complete") {
                effects_row(out, knowledge, false);
            } else {
                out.line("side_effects none");
            }
        }
        Some(_) => {
            side_effect_rows_of(out, &form["side_effects"], availability);
            effects_row(out, knowledge, true);
        }
        None => {}
    }
}

/// The `effects` row a scope's knowledge needs beside its `side_effect`
/// rows (issue #1712). Rows alone already read back as a lower bound and no
/// rows as unknown, so only the other states are written.
fn effects_row(out: &mut Out, knowledge: &Value, has_rows: bool) {
    match knowledge["level"].as_str() {
        Some("partial") if !has_rows => out.line("effects -partial"),
        Some("complete") => {
            let mut row = vec!["effects".to_owned()];
            for words in as_array(&knowledge["complete"]) {
                let words: Vec<&str> = as_array(words).iter().filter_map(Value::as_str).collect();
                row.push("-complete".to_owned());
                row.push(format!("{{{}}}", words.join(" ")));
            }
            emit_row(out, "side_effects_knowledge", &row, false, "");
        }
        _ => {}
    }
}

/// The `side_effect …` rows of one effect table, wherever it hangs.
fn side_effect_rows_of(out: &mut Out, effects: &Value, availability: bool) {
    for effect in as_array(effects) {
        let mut row = vec![
            "side_effect".to_owned(),
            str_of(&effect["target"]).to_owned(),
        ];
        if effect["reads"].as_bool() == Some(true) {
            row.push("-reads".to_owned());
        }
        if effect["writes"].as_bool() == Some(true) {
            row.push("-writes".to_owned());
        }
        let side = str_of(&effect["connection_side"]);
        if side != "None" {
            row.push("-side".to_owned());
            row.push(side.to_owned());
        }
        let mut lost = false;
        if let Some(dialects) = effect["surface"].as_array() {
            push_availability_flag(
                &mut row,
                &mut lost,
                &Value::Array(dialects.clone()),
                availability,
            );
        }
        push_lifecycle_flags(&mut row, &mut lost, effect);
        emit_row(out, "side_effects", &row, lost, "");
    }
}
