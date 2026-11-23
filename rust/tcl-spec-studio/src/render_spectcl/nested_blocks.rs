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

//! The blocks nested in a command body: `refine` forms and `subcommand` or `method` blocks.

use super::argument_rows::{
    arg_row_statements, arity_row, arity_window_rows, arity_word, callback_taint_input_table,
    script_lookup_scope_row,
};
use super::definition_grammar::{
    clause_grammar_block, hook_window_rows, semantic_operation_row, semantic_operation_window_rows,
};
use super::effects::{form_side_effect_rows, semantics_block, side_effect_rows, taint_row};
use super::option_rows::{option_block, option_row, versioned_arg_value_rows_for};
use super::properties::{
    catalogue_hook, count, enum_word, expr_row, expr_word, flag, gap_todo, native_hook, set_word,
    text, text_list, todo, unrecovered, unwritable, variable_receiver_list,
    variable_receiver_words, vocabulary_todo,
};
use super::rust_expressions::{
    byte_array_effect_word, deprecation_fix_row, option_conflict_rows, repeat_rows,
    representation_effect_word, return_elements_word, var_elements_effect_word,
    var_write_typing_word,
};
use super::tcl_words::{
    as_array, available_statement, braced, dialect_set_word, emit_row, list_word, name_word,
    push_availability_flag, push_flag, push_lifecycle_flags, str_list_word,
};
use super::{Ctx, Loss, ONE_LINE_COLUMN, Out, hover_block, str_of};
use crate::draft;
use crate::draft::Draft;
use serde_json::Value;

/// The `refine NAME { … }` blocks under `key` (design Q12/D2).
///
/// A form states the same words its owning scope does, about one call shape.
/// Only the overlays the row actually declares are written: an absent one
/// inherits, and writing it back as an empty row would change its meaning.
pub(super) fn refine_blocks(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft, key: &str) {
    for form in as_array(draft.get(key).unwrap_or(&Value::Null)).to_vec() {
        let Some(row) = form.as_object() else {
            continue;
        };
        out.gap();
        out.line(&format!("refine {} {{", name_word(str_of(&row["name"]))));
        let mut lost = false;
        out.indented(|out| {
            let arity = arity_word(&form["arity"]);
            if arity != ".." {
                out.line(&format!("arity {arity}"));
            }
            if let Some(selector) = form["selector"].as_object() {
                let mut words = vec![
                    "selector".to_owned(),
                    list_word(
                        &as_array(&selector["words"])
                            .iter()
                            .map(|word| str_of(word).to_owned())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap_or_else(|| "{}".to_owned()),
                ];
                if str_of(&selector["prefix_matching"]) == "Strict" {
                    words.push("-exact".to_owned());
                }
                out.line(&words.join(" "));
            }
            for role in as_array(&form["arg_roles"]) {
                out.line(&format!(
                    "arg {} -role {}",
                    role["index"],
                    str_of(&role["role"])
                ));
            }
            if let Some(receivers) = form["variable_receivers"].as_array() {
                out.line(&format!(
                    "variable_receivers {{{}}}",
                    variable_receiver_words(receivers)
                ));
            }
            for option in as_array(&form["options"]).to_vec() {
                option_row(out, ctx, &option);
            }
            if let Some(expr) = form["option_relations"].as_str() {
                match option_conflict_rows(expr, ctx.availability) {
                    Some(rows) => {
                        for row in rows {
                            out.row(&row, "");
                        }
                    }
                    None => lost = true,
                }
            }
            if let Some(rows) = form["surface"].as_array() {
                let value = Value::Array(rows.clone());
                match ctx
                    .availability
                    .then(|| available_statement(&value))
                    .flatten()
                {
                    Some(spelling) => out.line(&format!("available {spelling}")),
                    None => match dialect_set_word(&value) {
                        Some(spelling) => out.line(&format!("dialects {spelling}")),
                        None => lost = true,
                    },
                }
            }
            if let Some(traits) = form["traits"].as_array() {
                match str_list_word(&Value::Array(traits.clone())) {
                    Some(spelling) => out.line(&format!("traits {spelling}")),
                    None => lost = true,
                }
            }
            if let Some(mutator) = form["mutator"].as_bool() {
                out.line(&format!("mutator {}", if mutator { "yes" } else { "no" }));
            }
            form_header_write_row(out, &form["header_write"]);
            form_side_effect_rows(out, &form, ctx.availability);
            if let Some(expr) = form["representation_effect"].as_str() {
                match representation_effect_word(expr) {
                    Some(spelling) => out.line(&format!("representation_effect {spelling}")),
                    None => lost = true,
                }
            }
            for hook in ["lowering_hook", "codegen_hook"] {
                if let Some(id) = form[hook].as_str() {
                    out.line(&format!("{hook} -native {id}"));
                }
            }
        });
        if lost {
            out.indented(|out| todo(out, "refine"));
        }
        out.line("}");
    }
    thinned_refinements(out, draft, key);
}

fn form_header_write_row(out: &mut Out, write: &Value) {
    if write.is_object() {
        let suffix = if write["returns_empty"].as_bool() == Some(true) {
            " empty-result"
        } else {
            ""
        };
        out.line(&format!(
            "header_write {} {}{suffix}",
            write["name"], write["value"]
        ));
    }
}

/// A form's opaque halves (its completion, effect and transition descriptors)
/// have no draft value to write, so the rendered forms are thinner than the
/// source's and the table says so.
fn thinned_refinements(out: &mut Out, draft: &Draft, key: &str) {
    if unrecovered(draft, key) {
        let reason = "a refinement carries descriptors a draft does not hold".to_owned();
        out.comment(&format!("TODO(spectcl): `{key}`: {reason}."));
        out.losses.push(Loss {
            key: key.to_owned(),
            reason,
        });
    }
}

/// One `subcommand NAME { … }` block, or the `method NAME { … }` row of an
/// `object_class` — `keyword` names which, since the DSL reuses the subcommand
/// body grammar unchanged for instance methods.
///
/// A subcommand saying only what its arity, detail, and synopsis are is
/// written on one line with `;` separators, the way
/// `irules-http-header.tclspec` writes fourteen of them.
#[allow(clippy::too_many_lines)]
pub(super) fn subcommand_block(out: &mut Out, parent: &mut Ctx<'_>, sub: &Draft, keyword: &str) {
    let name = str_of(sub.get("name").unwrap_or(&Value::Null));
    let defaults = draft::default_subcommand_draft();
    let mut ctx = Ctx {
        defaults: &defaults,
        scope: tcl_spectcl::loader::hook_path(&parent.scope, name),
        owner: parent.owner.clone(),
        tables: parent.tables,
        availability: parent.availability,
        resolver_capabilities: parent.resolver_capabilities,
        vocabulary: parent.vocabulary,
    };
    let ctx = &mut ctx;

    let mut body = Out::at(out.indent + 1);
    let out_body = &mut body;

    arity_row(out_body, ctx, sub);
    arity_window_rows(out_body, ctx, sub);
    if ctx.set(sub, "detail") {
        match braced(str_of(&sub["detail"])) {
            Some(spelling) => out_body.line(&format!("detail {spelling}")),
            None => unwritable(out_body, "detail"),
        }
    }
    if ctx.set(sub, "synopsis") {
        match braced(str_of(&sub["synopsis"])) {
            Some(spelling) => out_body.line(&format!("synopsis {spelling}")),
            None => unwritable(out_body, "synopsis"),
        }
    }
    set_word(out_body, ctx, sub, "traits");
    set_word(out_body, ctx, sub, "surface");
    text(out_body, ctx, sub, "introduced_version");
    text(out_body, ctx, sub, "deprecated_version");
    text(out_body, ctx, sub, "retired_version");
    set_word(out_body, ctx, sub, "safe_on_uninit");
    expr_row(out_body, ctx, sub, "deprecation_fix", deprecation_fix_row);

    enum_word(out_body, ctx, sub, "return_type");
    if ctx.set(sub, "var_write_typing")
        && let Some(value) = sub["var_write_typing"].as_str()
    {
        match var_write_typing_word(value) {
            Some(spelling) => out_body.line(&format!("var_write_typing {spelling}")),
            None => todo(out_body, "var_write_typing"),
        }
    }
    expr_word(out_body, ctx, sub, "return_elements", return_elements_word);
    expr_word(
        out_body,
        ctx,
        sub,
        "var_elements_effect",
        var_elements_effect_word,
    );
    expr_word(
        out_body,
        ctx,
        sub,
        "representation_effect",
        representation_effect_word,
    );
    enum_word(out_body, ctx, sub, "inferred_storage_type");
    enum_word(out_body, ctx, sub, "body_kind");
    script_lookup_scope_row(out_body, ctx, sub);
    gap_todo(out_body, ctx, sub, "body_interpreter");
    gap_todo(out_body, ctx, sub, "body_execution");
    gap_todo(out_body, ctx, sub, "native_compilation");
    gap_todo(out_body, ctx, sub, "successful_handler");
    variable_receiver_list(out_body, ctx, sub);
    gap_todo(out_body, ctx, sub, "native_result");
    expr_word(
        out_body,
        ctx,
        sub,
        "byte_array_effect",
        byte_array_effect_word,
    );
    enum_word(out_body, ctx, sub, "pattern_type");
    enum_word(out_body, ctx, sub, "format_string_type");

    flag(out_body, ctx, sub, "mutator");
    flag(out_body, ctx, sub, "destructive");
    flag(out_body, ctx, sub, "returns_path");
    enum_word(out_body, ctx, sub, "source_index_bounds");
    enum_word(out_body, ctx, sub, "source_path_operation");
    enum_word(out_body, ctx, sub, "source_configuration_reference");
    flag(out_body, ctx, sub, "is_unescape");
    flag(out_body, ctx, sub, "loop_list_header");
    flag(out_body, ctx, sub, "creates_scope_alias");
    flag(out_body, ctx, sub, "arg_values_accept_prefix");
    count(out_body, ctx, sub, "body_arg_implicit_args");
    if ctx.set(sub, "defines_command_at") {
        out_body.line(&format!("defines_command_at {}", sub["defines_command_at"]));
    }
    if ctx.set(sub, "max_leading_option_words") {
        out_body.line(&format!(
            "max_leading_option_words {}",
            sub["max_leading_option_words"]
        ));
    }
    if ctx.set(sub, "min_abbrev") {
        out_body.line(&format!("min_abbrev {}", sub["min_abbrev"]));
    }
    enum_word(out_body, ctx, sub, "prefix_matching");
    count(out_body, ctx, sub, "option_prefix_words");
    count(out_body, ctx, sub, "reserved_trailing_words");
    text(out_body, ctx, sub, "cfg_rewrite_name");

    arg_row_statements(out_body, ctx, sub);
    callback_taint_input_table(out_body, ctx, sub);
    if ctx.set(sub, "repeated_args")
        && let Some(expr) = sub["repeated_args"].as_str()
    {
        match repeat_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out_body.row(&row, "");
                }
            }
            None => todo(out_body, "repeated_args"),
        }
    }

    clause_grammar_block(out_body, ctx, sub);
    native_hook(out_body, ctx, sub, "arg_role_resolver");
    native_hook(out_body, ctx, sub, "arg_role_count_resolver");
    native_hook(out_body, ctx, sub, "arg_role_layout_resolver");
    if ctx.resolver_capabilities {
        set_word(out_body, ctx, sub, "arg_role_resolver_roles");
    } else {
        vocabulary_todo(out_body, ctx, sub, "arg_role_resolver_roles", "2.1");
    }
    native_hook(out_body, ctx, sub, "command_prefix_resolver");
    native_hook(out_body, ctx, sub, "script_timing_resolver");
    native_hook(out_body, ctx, sub, "const_fold");
    native_hook(out_body, ctx, sub, "const_fold_versioned");
    semantics_block(out_body, ctx, sub);
    native_hook(out_body, ctx, sub, "literal_argument_validator");
    catalogue_hook(out_body, ctx, sub, "lowering_hook");
    catalogue_hook(out_body, ctx, sub, "codegen_hook");
    hook_window_rows(out_body, ctx, sub, "codegen_hook");
    catalogue_hook(out_body, ctx, sub, "inline_codegen_hook");
    hook_window_rows(out_body, ctx, sub, "inline_codegen_hook");
    catalogue_hook(out_body, ctx, sub, "analyser_hook");
    semantic_operation_row(out_body, ctx, sub);
    semantic_operation_window_rows(out_body, ctx, sub);
    gap_todo(out_body, ctx, sub, "completion");
    gap_todo(out_body, ctx, sub, "dispatch_dependencies");
    gap_todo(out_body, ctx, sub, "result_stability");
    refine_blocks(out_body, ctx, sub, "subcommand_forms");

    enum_word(out_body, ctx, sub, "command_table_effect");
    side_effect_rows(out_body, sub, ctx.availability);
    gap_todo(out_body, ctx, sub, "world_effects");
    gap_todo(out_body, ctx, sub, "state_transitions");

    taint_row(out_body, sub.get("taint_knowledge").unwrap_or(&Value::Null));
    text(out_body, ctx, sub, "taint_output_sink");
    set_word(out_body, ctx, sub, "taint_transform");
    enum_word(out_body, ctx, sub, "taint_transform_when");
    set_word(out_body, ctx, sub, "taint_double_encode_colour");
    enum_word(out_body, ctx, sub, "channel_configuration");
    if ctx.set(sub, "credential_arg") {
        out_body.line(&format!("credential_arg {}", sub["credential_arg"]));
    }
    text_list(out_body, ctx, sub, "sensitive_headers");

    option_block(out_body, ctx, sub);
    versioned_arg_value_rows_for(out_body, ctx, sub);
    for row in as_array(sub.get("sub_subcommands").unwrap_or(&Value::Null)) {
        if row
            .get("native_compilation")
            .is_some_and(|value| !value.is_null())
        {
            todo(out_body, "nested_native_compilation");
        }
        let mut words = vec!["sub_subcommand".to_owned(), name_word(str_of(&row["name"]))];
        let mut lost = false;
        let detail = str_of(&row["detail"]);
        if !detail.is_empty() {
            push_flag(&mut words, &mut lost, "-detail", braced(detail));
        }
        let synopsis = str_of(&row["synopsis"]);
        if !synopsis.is_empty() {
            push_flag(&mut words, &mut lost, "-synopsis", braced(synopsis));
        }
        if let Some(dialects) = row["surface"].as_array() {
            push_availability_flag(
                &mut words,
                &mut lost,
                &Value::Array(dialects.clone()),
                ctx.availability,
            );
        }
        if let Some(prefix) = row["option_prefix_words"]
            .as_u64()
            .filter(|prefix| *prefix > 0)
        {
            push_flag(
                &mut words,
                &mut lost,
                "-option_prefix_words",
                Some(prefix.to_string()),
            );
        }
        push_lifecycle_flags(&mut words, &mut lost, row);
        // A second-level operation with its own option table (`namespace
        // ensemble create` vs `configure`) takes a block body,
        // exactly as a subcommand does. An operation that declares *nothing*
        // about options — the JSON `null` — keeps the flag row and inherits;
        // an operation declaring an empty table gets an empty block, because
        // "no options here" and "ask the parent" are different claims and the
        // DSL has to be able to make both.
        let declared = row.get("options").filter(|v| !v.is_null());
        let mut ops = Out::at(out_body.indent + 1);
        for option in as_array(declared.unwrap_or(&Value::Null)).to_vec() {
            option_row(&mut ops, ctx, &option);
        }
        if lost || declared.is_none() {
            emit_row(out_body, "sub_subcommands", &words, lost, "-detail");
        } else if ops.text.is_empty() {
            out_body.line(&format!("{} {{}}", words.join(" ")));
        } else {
            out_body.line(&format!("{} {{", words.join(" ")));
            out_body.block(&ops);
            out_body.line("}");
        }
    }
    hover_block(out_body, ctx, sub);

    // A subcommand saying only its arity, detail, and synopsis goes on one
    // line — `;` separates statements exactly as a newline does.
    let one_liner = format!(
        "{keyword} {} {{ {} }}",
        name_word(name),
        body.statements.join(" ; ")
    );
    let simple = body.statements.len() <= 3
        && !body
            .statements
            .iter()
            .any(|line| line.ends_with('{') || line.contains('\n') || line.starts_with('#'));
    // Consecutive one-liners are a table and read as one; a block always gets
    // air around it. That is exactly how `irules-http-header.tclspec` lays its
    // fourteen subcommands out.
    let after_one_liner = out
        .statements
        .last()
        .is_some_and(|last| last.starts_with(&format!("{keyword} ")) && last.ends_with('}'));
    if simple && out.indent * 4 + one_liner.len() <= ONE_LINE_COLUMN {
        if !after_one_liner {
            out.gap();
        }
        out.line(&one_liner);
        out.losses.extend(body.losses);
        return;
    }

    out.gap();
    out.line(&format!("{keyword} {} {{", name_word(name)));
    out.block(&body);
    out.line("}");
}
