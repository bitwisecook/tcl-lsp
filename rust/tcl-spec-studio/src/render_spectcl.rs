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

//! Render a draft as a `SpecTcl` `.tclspec` pack — the inverse of the loader.
//!
//! The loader ([`crate::spectcl`]) turns a pack into live `CommandSpec`s, which
//! the studio seeds drafts from. This module closes the loop: draft in,
//! `.tclspec` text out, so a shipped spec can be *exported* to a pack, a pack
//! can be round-tripped through the studio's editor, and the studio's DSL pane
//! has something to show. The syntax is the frozen one in
//! `docs/design/spec-dsl-examples/README.md`, and the twelve `*.tclspec` ports
//! beside it are the formatting exemplars: one-screen simple commands, row
//! statements rather than nested blocks, `\`-continued option rows with the
//! prose flag last.
//!
//! ## Three rules the renderer follows
//!
//! 1. **Only non-default fields are emitted.** Every key is compared against
//!    [`crate::draft::default_command_draft`] /
//!    [`crate::draft::default_subcommand_draft`], so a rendered pack says
//!    exactly what its author set and nothing else — the same promise
//!    [`crate::render_rs`] makes with `..CommandSpec::DEFAULT`.
//! 2. **Catalogue spellings are verbatim.** A draft already holds the Rust
//!    variant spelling for every enum and flag (`VarWrite`, `BYTE_COMPILED`,
//!    `tcl9.0`), which is exactly what the DSL wants, so no field is
//!    re-spelled on the way out.
//! 3. **Nothing is dropped silently.** A field the draft holds but the DSL
//!    cannot carry is emitted as a `# TODO(spectcl):` line naming the field,
//!    its documented spelling, and why the value did not survive — see
//!    [`GAPS`]. A field whose value *names engine code* is emitted in the
//!    documented `-native ID` form, which is not a loss: the loader installs
//!    its family's abstention either way, and the draft records both the same.
//!
//! ## What round-trips
//!
//! `tests/spectcl_roundtrip.rs` is the gate: every command of every browsable
//! dialect is drafted, rendered here, loaded back through
//! [`crate::spectcl::evaluate_pack`], and the two drafts are compared field by
//! field. Every field that legitimately differs is one of the [`GAPS`] below,
//! or a [`Loss`] this render reported for itself, and the test fails on
//! anything else.
//!
//! Two differences are neither, and are worth naming here because they are
//! properties of the *language* rather than of this renderer:
//!
//! - **`arg N -appends …` implies `-role CommandPrefix`.** So a spec that
//!   declares a command-prefix position with *no* role at that index cannot be
//!   said in `SpecTcl`: reloading always finds the implied role. 531 of the
//!   registry's command/dialect pairs are in that shape.
//! - **A quoted word is not verbatim.** The loader reads its words off the
//!   CST, where a braced word is every byte between the braces but a quoted
//!   one keeps its backslashes and has `$x` normalised to `${x}`. Prose with
//!   an unbalanced brace or a backslash-newline therefore has no faithful
//!   spelling at all, and is reported as a [`Loss`] rather than written down
//!   wrong — see [`word`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::Value;

use crate::draft;
use crate::draft::{Draft, SOURCE_DIALECT_KEY};

mod argument_rows;
mod definition_grammar;
mod effects;
mod gaps;
mod nested_blocks;
mod option_rows;
mod properties;
mod rust_expressions;
mod tcl_words;

use argument_rows::{
    arg_row_statements, arity_row, arity_window_rows, callback_taint_input_table,
    script_lookup_scope_row,
};
use definition_grammar::{
    clause_grammar_block, definition_body_block, hook_window_rows, manufacturer_rows,
    object_class_block, runtime_backing_row, semantic_operation_row,
    semantic_operation_window_rows,
};
use effects::{semantics_block, side_effect_rows, taint_row};
pub use gaps::{GAPS, Gap, GapKind, gap};
use nested_blocks::{refine_blocks, subcommand_block};
use option_rows::{option_block, push_values, versioned_arg_value_rows_for};
use properties::{
    catalogue_hook, count, enum_word, expr_row, expr_word, flag, gap_todo, index_list, native_hook,
    scalar, set_word, text, text_list, tk_geometry_row, todo, unwritable, variable_receiver_list,
    vocabulary_todo,
};
#[cfg(test)]
use rust_expressions::dialect_names;
#[cfg(test)]
use rust_expressions::option_conflict_rows;
use rust_expressions::{
    binds_handle_word, byte_array_effect_word, byte_array_payload_row, defines_symbol_row,
    deprecation_fix_row, oo_context_fact_rows, repeat_rows, representation_effect_word,
    return_elements_word, setter_constraint_rows, var_elements_effect_word, var_write_typing_word,
};
pub use tcl_words::header_word;
use tcl_words::{
    as_array, braced, emit_row, list_word, name_word, push_availability_flag, push_flag,
    push_lifecycle_flags, push_word, str_list_word, word,
};
#[cfg(test)]
use tcl_words::{availability_rows, available_flag};

/// The DSL **vocabulary** version a rendered pack declares — the word after
/// the pack name in `speclib <pack> <version> { … }`.
///
/// The rule is "declare the newest vocabulary the *body* actually uses": a
/// header naming an older vocabulary than the body needs is exactly the
/// inconsistency the loader's per-site notice reports, so pinning this to a
/// stale literal would make the renderer produce that notice on its own
/// output.
///
/// Tracking [`tcl_spectcl::NEWEST_VOCABULARY_VERSION`] directly is correct
/// because [`availability_rows`] writes every dialect set the algebra can
/// carry as `available` / `-available` — the 2.0 word — at every scope the
/// loader reads it: command, subcommand, option, form, side effect, option
/// conflict, and second-level operation. A rendered body is therefore 2.0
/// wherever a draft says anything about availability at all, which is
/// nearly every command in the registry; pinning the header older would
/// reproduce the loader's per-site notice on an `available` row under an
/// older declaration.
///
/// The other two 2.0 additions are **pack-level blocks a draft cannot
/// hold**: a draft is one command's model, so nothing in it renders as an
/// `environment` or a `dialect` block. This is harmless: a draft that says
/// nothing about availability renders a 1.x body under a 2.0 header, and
/// over-declaring produces no notice anywhere (the loader only reports a
/// *site newer than* the declaration) — a pack the renderer just wrote has
/// no legacy rows left untranslated for U1 to fail on, it has fields, not
/// rows.
pub const DSL_VERSION: &str = tcl_spectcl::NEWEST_VOCABULARY_VERSION;

const AVAILABILITY_VERSION: &str = "2.0";
const RESOLVER_CAPABILITIES_VERSION: &str = "2.1";
const POSITIONAL_ARITY_VERSION: &str = "2.1";
const SCRIPT_LOOKUP_SCOPE_VERSION: &str = "2.0";

/// Column the renderer tries to keep rows inside before continuing a row with
/// a `\`, matching the ports' own wrapping.
const WRAP_COLUMN: usize = 92;

/// Column the one-line `subcommand NAME { … }` form is allowed to reach.
///
/// Wider than [`WRAP_COLUMN`] because that is what the ports do:
/// `irules-http-header.tclspec` writes fourteen subcommands as one line each,
/// the longest of them 143 columns, and breaking those into five-line blocks
/// would cost the file its one-screen shape for nothing.
const ONE_LINE_COLUMN: usize = 144;

// The output buffer

/// An indented line buffer with a one-slot blank-line separator.
///
/// **Only the first physical line of a statement is indented.** A braced word
/// carrying prose may run over many lines, and the loader takes every byte
/// between its braces verbatim — so indenting a continuation line would change
/// the value, and the equivalence gate compares those strings byte for byte.
/// That is also why a block body is rendered *at its final indent* rather than
/// rendered flat and shifted afterwards.
#[derive(Debug, Default)]
struct Out {
    text: String,
    indent: usize,
    /// A blank line is owed before the next line, unless nothing follows.
    pending_blank: bool,
    /// Every statement written, unindented — what the one-line subcommand
    /// form is decided from.
    statements: Vec<String>,
    /// Every field this buffer could not carry, in the order they arose.
    losses: Vec<Loss>,
}

/// One field a render could not carry, and why.
///
/// Returned by [`render_pack_reporting`] so a caller — the studio's DSL pane,
/// or the round-trip gate — can say what a pack does *not* say without having
/// to re-derive it from the `TODO` comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    /// The draft / schema key.
    pub key: String,
    /// One sentence naming the cause.
    pub reason: String,
}

/// Losses attributed to one command in a multi-command pack render.
///
/// The ordinary reporting entry point flattens these for UI callers. Exhaustive
/// consumers can retain the command boundary while evaluating the rendered pack
/// once instead of starting one evaluator per command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLosses {
    /// The rendered command name.
    pub command: String,
    /// Fields from that command which the renderer could not carry.
    pub losses: Vec<Loss>,
}

impl Out {
    /// A buffer whose statements start at `indent` levels of four spaces.
    fn at(indent: usize) -> Self {
        Self {
            indent,
            ..Self::default()
        }
    }

    fn line(&mut self, text: &str) {
        if self.pending_blank && !self.text.is_empty() {
            self.text.push('\n');
        }
        self.pending_blank = false;
        if !text.is_empty() {
            for _ in 0..self.indent {
                self.text.push_str("    ");
            }
            self.text.push_str(text);
            self.statements.push(text.to_owned());
        }
        self.text.push('\n');
    }

    /// Ask for one blank line before whatever comes next.
    fn gap(&mut self) {
        if !self.text.is_empty() {
            self.pending_blank = true;
        }
    }

    /// Append an already-indented block, keeping its own statement list.
    fn block(&mut self, other: &Self) {
        if self.pending_blank && !self.text.is_empty() {
            self.text.push('\n');
        }
        self.pending_blank = false;
        self.text.push_str(&other.text);
        self.statements.extend(other.statements.iter().cloned());
        self.losses.extend(other.losses.iter().cloned());
    }

    fn indented<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
        self.indent += 1;
        let out = body(self);
        self.indent -= 1;
        out
    }

    /// A `# …` comment, wrapped to a readable width.
    fn comment(&mut self, text: &str) {
        let width = 78usize.saturating_sub(self.indent * 4);
        let mut line = String::new();
        for token in text.split_whitespace() {
            if !line.is_empty() && line.len() + 1 + token.len() > width {
                let done = std::mem::take(&mut line);
                self.line(&format!("# {done}"));
                line.push_str("  ");
            }
            if !line.is_empty() && !line.ends_with("  ") {
                line.push(' ');
            }
            line.push_str(token);
        }
        if !line.is_empty() {
            self.line(&format!("# {line}"));
        }
    }

    /// One statement written as `word word …`, continued with `\` when it runs
    /// past [`WRAP_COLUMN`].
    ///
    /// `break_before` names the flag a long row is split in front of — the
    /// ports put the prose flag on its own continuation line and everything
    /// else on the first.
    fn row(&mut self, words: &[String], break_before: &str) {
        let flat = words.join(" ");
        let split = if self.indent * 4 + flat.len() <= WRAP_COLUMN || break_before.is_empty() {
            None
        } else {
            words
                .iter()
                .position(|w| w == break_before)
                .filter(|at| *at > 0)
        };
        let Some(at) = split else {
            self.line(&flat);
            return;
        };
        self.line(&format!("{} \\", words[..at].join(" ")));
        self.indented(|out| out.line(&words[at..].join(" ")));
        // One logical statement, however many physical lines it took.
        self.statements.truncate(self.statements.len() - 2);
        self.statements.push(flat);
    }
}

// Shared `values` tables

/// The pack-level `values NAME { … }` tables a render needs.
///
/// An `arg`/`option` row can spell a plain value set inline (`-values {a b}`),
/// but a value carrying a `detail`, a Tcl-version floor, or a completion code
/// has nowhere to put them on the row — so those become a shared table,
/// exactly as `string.tclspec`'s `is-classes` and `return.tclspec`'s
/// `return-codes` do.
#[derive(Debug, Default)]
struct ValueTables {
    /// Shared value tables, in declaration order.
    tables: Vec<ValueTable>,
}

#[derive(Debug)]
struct ValueTable {
    name: String,
    values: Value,
    /// Top-level commands which refer to this table.
    owners: BTreeSet<String>,
}

impl ValueTables {
    /// Register `values` under a name derived from `hint`, reusing an
    /// identical table when one is already declared.
    fn intern(&mut self, hint: &str, values: &Value, owner: &str) -> String {
        if let Some(table) = self.tables.iter_mut().find(|table| table.values == *values) {
            table.owners.insert(owner.to_owned());
            return table.name.clone();
        }
        let base = slug(hint);
        let mut name = base.clone();
        let mut n = 2;
        while self.tables.iter().any(|table| table.name == name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        self.tables.push(ValueTable {
            name: name.clone(),
            values: values.clone(),
            owners: BTreeSet::from([owner.to_owned()]),
        });
        name
    }

    fn render(&self, out: &mut Out) -> BTreeMap<String, Vec<Loss>> {
        let mut by_command: BTreeMap<String, Vec<Loss>> = BTreeMap::new();
        for table in &self.tables {
            let losses_before = out.losses.len();
            out.gap();
            out.line(&format!("values {} {{", table.name));
            out.indented(|out| {
                for value in as_array(&table.values) {
                    let Some(spelling) = word(str_of(&value["value"])) else {
                        unwritable(out, "arg_values");
                        continue;
                    };
                    let mut row = vec!["value".to_owned(), spelling];
                    if let Some(version) = value["min_tcl"].as_str() {
                        row.push("-min-tcl".to_owned());
                        row.push(tcl_version_word(version));
                    }
                    let mut lost = false;
                    push_lifecycle_flags(&mut row, &mut lost, value);
                    if lost {
                        unwritable(out, "arg_values");
                    }
                    if let Some(code) = value["code"].as_i64() {
                        row.push("-code".to_owned());
                        row.push(code.to_string());
                    }
                    let detail = str_of(&value["detail"]);
                    if !detail.is_empty() {
                        match braced(detail) {
                            Some(spelling) => {
                                row.push("-detail".to_owned());
                                row.push(spelling);
                            }
                            None => unwritable(out, "arg_values"),
                        }
                    }
                    out.row(&row, "-detail");
                }
            });
            out.line("}");
            let losses = &out.losses[losses_before..];
            for owner in &table.owners {
                by_command
                    .entry(owner.clone())
                    .or_default()
                    .extend_from_slice(losses);
            }
        }
        by_command
    }
}

/// A name safe to use as a pack-level declaration word.
fn slug(hint: &str) -> String {
    let mut out = String::with_capacity(hint.len());
    let mut last_dash = false;
    for c in hint.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_owned();
    if trimmed.is_empty() {
        "values".to_owned()
    } else {
        trimmed
    }
}

/// The catalogue spelling of a `min_tcl` floor as the DSL's own version word.
///
/// A draft holds the `TclVersion` variant (`V8_6`); the DSL takes `tcl8.6`,
/// which is also what the shipped table's own key is.
fn tcl_version_word(variant: &str) -> String {
    let digits: String = variant
        .trim_start_matches('V')
        .chars()
        .map(|c| if c == '_' { '.' } else { c })
        .collect();
    format!("tcl{digits}")
}

fn str_of(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

// Field-level emission

/// Everything one command or subcommand body needs while rendering.
struct Ctx<'a> {
    /// The default draft the body's fields are compared against.
    defaults: &'a Draft,
    /// The `-native` id prefix — `lsort`, or `string::is` for a subcommand.
    scope: String,
    /// The top-level command which owns any shared table interned here.
    owner: String,
    tables: &'a mut ValueTables,
    /// Whether the pack being rendered declares `SpecTcl` 2.0, and may
    /// therefore use the availability algebra.
    ///
    /// A renderer that emitted `available` into a pack declaring 1.x would
    /// produce exactly the header/body inconsistency the loader reports per
    /// site, so the *header* decides the spelling: a 2.0 pack — every
    /// pack the studio creates, since [`DSL_VERSION`] is the newest
    /// vocabulary — takes `available`, and a document that declares an older
    /// vocabulary keeps `dialects` and stays a valid pack of its own
    /// vocabulary. Moving such a document to 2.0 is `tcl spec upgrade`'s job,
    /// not a side effect of opening it in the studio.
    availability: bool,
    /// Whether the declared vocabulary can carry the complete role set of an
    /// argument-role resolver.
    resolver_capabilities: bool,
    /// The vocabulary the pack's `speclib` header declares.
    vocabulary: &'a str,
}

impl Ctx<'_> {
    /// Whether the declared vocabulary has the words `since` introduced.
    fn speaks(&self, since: &str) -> bool {
        !tcl_registry::version::compare(self.vocabulary, since).is_lt()
    }

    /// Whether `draft`'s `key` differs from the default a fresh spec has.
    fn set(&self, draft: &Draft, key: &str) -> bool {
        let value = draft.get(key);
        value.is_some() && value != self.defaults.get(key)
    }

    fn native(&self, field: &str) -> Vec<String> {
        vec![
            field.to_owned(),
            "-native".to_owned(),
            tcl_spectcl::loader::hook_path(&self.scope, field),
        ]
    }
}

/// The `hover { … }` block.
fn hover_block(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    let null = Value::Null;
    let hover = if ctx.set(draft, "hover") {
        &draft["hover"]
    } else {
        &null
    };
    let examples = if ctx.set(draft, "documentation_examples") {
        as_array(&draft["documentation_examples"])
    } else {
        &[]
    };
    if hover.is_null() && examples.is_empty() {
        return;
    }
    out.gap();
    out.line("hover {");
    out.indented(|out| {
        let scalar_row = |out: &mut Out, key: &str, word_name: &str| {
            let text = str_of(&hover[key]);
            if text.is_empty() {
                return;
            }
            match braced(text) {
                Some(spelling) => out.line(&format!("{word_name} {spelling}")),
                None => unwritable(out, "hover"),
            }
        };
        scalar_row(out, "summary", "summary");
        for synopsis in as_array(&hover["synopsis"]) {
            match braced(str_of(synopsis)) {
                Some(spelling) => out.line(&format!("synopsis {spelling}")),
                None => unwritable(out, "hover"),
            }
        }
        // Three words are renamed from their Rust field names, and nothing
        // else in the DSL renames a key.
        scalar_row(out, "snippet", "description");
        scalar_row(out, "source", "source");
        // ONE `example` block, never one row per example: repeated rows join
        // with a single newline, which would flatten an `examples` string that
        // separates its examples with a blank line.
        scalar_row(out, "examples", "example");
        scalar_row(out, "return_value", "returns");
        for example in examples {
            match documentation_example_rows(example) {
                Some(rows) => {
                    out.line("example {");
                    out.indented(|out| {
                        for row in &rows {
                            out.line(row);
                        }
                    });
                    out.line("}");
                }
                None => unwritable(out, "documentation_examples"),
            }
        }
    });
    out.line("}");
}

/// The rows of one structured `example { code …; carrier …; note … }`.
fn documentation_example_rows(example: &Value) -> Option<Vec<String>> {
    let mut rows = vec![format!("code {}", braced(str_of(&example["code"]))?)];
    let carrier = &example["carrier"];
    if !carrier.is_null() {
        rows.push(format!(
            "carrier {} {}",
            carrier["line"].as_u64()?,
            braced(str_of(&carrier["needle"]))?
        ));
    }
    for note in as_array(&example["notes"]) {
        rows.push(format!(
            "note {} {} {}",
            note["line"].as_u64()?,
            braced(str_of(&note["needle"]))?,
            braced(str_of(&note["label"]))?
        ));
    }
    Some(rows)
}

// Command and subcommand bodies

/// Render the body of one `command NAME { … }` block.
#[allow(clippy::too_many_lines)]
fn command_body(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    // Identity and availability.
    set_word(out, ctx, draft, "surface");
    set_word(out, ctx, draft, "traits");
    arity_row(out, ctx, draft);
    arity_window_rows(out, ctx, draft);
    text(out, ctx, draft, "required_package");
    tk_geometry_row(out, ctx, draft);
    text(out, ctx, draft, "tcllib_package");
    text(out, ctx, draft, "implementation_namespace");
    text(out, ctx, draft, "introduced_version");
    text(out, ctx, draft, "deprecated_version");
    text(out, ctx, draft, "retired_version");
    flag(out, ctx, draft, "warn_missing_import");
    flag(out, ctx, draft, "is_namespace_exported");
    flag(out, ctx, draft, "unsafe_command");
    flag(out, ctx, draft, "arg_values_accept_prefix");
    text_list(out, ctx, draft, "excluded_events");
    set_word(out, ctx, draft, "safe_on_uninit");
    expr_row(out, ctx, draft, "deprecation_fix", deprecation_fix_row);

    // Types and shape.
    out.gap();
    enum_word(out, ctx, draft, "return_type");
    if ctx.set(draft, "var_write_typing")
        && let Some(value) = draft["var_write_typing"].as_str()
    {
        match var_write_typing_word(value) {
            Some(spelling) => out.line(&format!("var_write_typing {spelling}")),
            None => todo(out, "var_write_typing"),
        }
    }
    gap_todo(out, ctx, draft, "variable_write_min_args");
    expr_word(out, ctx, draft, "return_elements", return_elements_word);
    expr_word(
        out,
        ctx,
        draft,
        "var_elements_effect",
        var_elements_effect_word,
    );
    expr_word(
        out,
        ctx,
        draft,
        "representation_effect",
        representation_effect_word,
    );
    enum_word(out, ctx, draft, "inferred_storage_type");
    enum_word(out, ctx, draft, "body_kind");
    script_lookup_scope_row(out, ctx, draft);
    enum_word(out, ctx, draft, "irules_source_flow");
    gap_todo(out, ctx, draft, "body_interpreter");
    gap_todo(out, ctx, draft, "body_execution");
    gap_todo(out, ctx, draft, "procedure_definition");
    gap_todo(out, ctx, draft, "native_compilation");
    gap_todo(out, ctx, draft, "successful_handler");
    variable_receiver_list(out, ctx, draft);
    gap_todo(out, ctx, draft, "native_result");
    gap_todo(out, ctx, draft, "form_value_effects");
    expr_word(out, ctx, draft, "byte_array_effect", byte_array_effect_word);
    enum_word(out, ctx, draft, "pattern_type");
    gap_todo(out, ctx, draft, "pattern_arg_resolver");
    enum_word(out, ctx, draft, "format_string_type");
    expr_row(
        out,
        ctx,
        draft,
        "byte_array_payload",
        byte_array_payload_row,
    );

    arg_row_statements(out, ctx, draft);
    callback_taint_input_table(out, ctx, draft);
    if ctx.set(draft, "repeated_args")
        && let Some(expr) = draft["repeated_args"].as_str()
    {
        match repeat_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "repeated_args"),
        }
    }
    count(out, ctx, draft, "reserved_trailing_words");
    count(out, ctx, draft, "option_prefix_words");
    count(out, ctx, draft, "body_arg_implicit_args");
    if ctx.set(draft, "assigns_variable_at") {
        scalar(
            out,
            ctx,
            draft,
            "assigns_variable_at",
            &draft["assigns_variable_at"].to_string(),
        );
    }
    if ctx.set(draft, "creates_instance_at") {
        scalar(
            out,
            ctx,
            draft,
            "creates_instance_at",
            &draft["creates_instance_at"].to_string(),
        );
    }
    if ctx.set(draft, "defines_command_at") {
        scalar(
            out,
            ctx,
            draft,
            "defines_command_at",
            &draft["defines_command_at"].to_string(),
        );
    }
    expr_row(out, ctx, draft, "defines_symbol", defines_symbol_row);

    // Subcommand dispatch.
    flag(out, ctx, draft, "allow_unknown_subcommands");
    enum_word(out, ctx, draft, "prefix_matching");
    enum_word(out, ctx, draft, "default_form_first_word");
    text_list(out, ctx, draft, "self_receiver_words");

    // The clause grammar, then the hooks.
    clause_grammar_block(out, ctx, draft);
    out.gap();
    native_hook(out, ctx, draft, "arg_role_resolver");
    native_hook(out, ctx, draft, "arg_role_count_resolver");
    native_hook(out, ctx, draft, "arg_role_layout_resolver");
    if ctx.resolver_capabilities {
        set_word(out, ctx, draft, "arg_role_resolver_roles");
    } else {
        vocabulary_todo(out, ctx, draft, "arg_role_resolver_roles", "2.1");
    }
    native_hook(out, ctx, draft, "command_prefix_resolver");
    native_hook(out, ctx, draft, "script_timing_resolver");
    native_hook(out, ctx, draft, "clause_shape_check");
    native_hook(out, ctx, draft, "const_fold");
    native_hook(out, ctx, draft, "const_fold_versioned");
    semantics_block(out, ctx, draft);
    enum_word(out, ctx, draft, "source_index_bounds");
    native_hook(out, ctx, draft, "literal_argument_validator");
    native_hook(out, ctx, draft, "context_gate");
    catalogue_hook(out, ctx, draft, "lowering_hook");
    catalogue_hook(out, ctx, draft, "codegen_hook");
    hook_window_rows(out, ctx, draft, "codegen_hook");
    catalogue_hook(out, ctx, draft, "inline_codegen_hook");
    hook_window_rows(out, ctx, draft, "inline_codegen_hook");
    catalogue_hook(out, ctx, draft, "analyser_hook");
    catalogue_hook(out, ctx, draft, "return_type_hook");
    semantic_operation_row(out, ctx, draft);
    semantic_operation_window_rows(out, ctx, draft);
    gap_todo(out, ctx, draft, "bpf_op");
    gap_todo(out, ctx, draft, "completion");
    gap_todo(out, ctx, draft, "dispatch_dependencies");
    gap_todo(out, ctx, draft, "result_stability");

    // Effects.
    out.gap();
    enum_word(out, ctx, draft, "command_table_effect");
    side_effect_rows(out, draft, ctx.availability);
    gap_todo(out, ctx, draft, "frame_effect");
    gap_todo(out, ctx, draft, "world_effects");
    gap_todo(out, ctx, draft, "state_transitions");

    // Taint and security.
    out.gap();
    taint_row(out, draft.get("taint_knowledge").unwrap_or(&Value::Null));
    text(out, ctx, draft, "taint_output_sink");
    text_list(out, ctx, draft, "taint_output_sink_subcommands");
    text(out, ctx, draft, "taint_log_sink");
    index_list(out, ctx, draft, "taint_network_sink_args");
    index_list(out, ctx, draft, "taint_code_sink_args");
    enum_word(out, ctx, draft, "taint_numeric_coercion");
    text_list(out, ctx, draft, "taint_interp_eval_subcommands");
    set_word(out, ctx, draft, "taint_source");
    set_word(out, ctx, draft, "taint_transform");
    enum_word(out, ctx, draft, "taint_transform_when");
    set_word(out, ctx, draft, "taint_double_encode_colour");
    set_word(out, ctx, draft, "taint_sink_safe_colour");
    native_hook(out, ctx, draft, "taint_sink_gate");
    enum_word(out, ctx, draft, "channel_configuration");
    text_list(out, ctx, draft, "credential_options");
    text_list(out, ctx, draft, "sensitive_headers");
    if ctx.set(draft, "setter_constraints") {
        match setter_constraint_rows(&draft["setter_constraints"]) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "-message");
                }
            }
            None => todo(out, "setter_constraints"),
        }
    }

    // iRules.
    gap_todo(out, ctx, draft, "event_requires");
    gap_todo(out, ctx, draft, "event_requirement_forms");
    gap_todo(out, ctx, draft, "event_emits");
    gap_todo(out, ctx, draft, "event_emission_forms");
    gap_todo(out, ctx, draft, "data_collection");
    gap_todo(out, ctx, draft, "side_switch_target");
    gap_todo(out, ctx, draft, "event_handler_priority");
    gap_todo(out, ctx, draft, "irules_top_level_effect");

    // Translation.
    out.gap();
    if ctx.set(draft, "xc_translatable")
        && let Some(value) = draft["xc_translatable"].as_bool()
    {
        out.line(if value {
            "xc_translatable yes"
        } else {
            "xc_translatable no"
        });
    }
    text(out, ctx, draft, "deprecated_replacement");
    flag(out, ctx, draft, "deprecated_replacement_drop_in");
    enum_word(out, ctx, draft, "source_deprecation_advice");
    text(out, ctx, draft, "alias_of");
    runtime_backing_row(out, ctx, draft);

    // Descriptors.
    out.gap();
    definition_body_block(out, ctx, draft);
    manufacturer_rows(out, draft);
    gap_todo(out, ctx, draft, "case_list");
    object_class_block(out, ctx, draft);
    gap_todo(out, ctx, draft, "body_scope");
    if ctx.set(draft, "oo_context_facts")
        && let Some(expr) = draft["oo_context_facts"].as_str()
    {
        match oo_context_fact_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "oo_context_facts"),
        }
    }
    if ctx.set(draft, "binds_handle")
        && let Some(expr) = draft["binds_handle"].as_str()
    {
        match binds_handle_word(expr) {
            Some(spelling) => out.line(&format!("binds_handle {spelling}")),
            None => todo(out, "binds_handle"),
        }
    }
    // No property word in the vocabulary yet — see the `remote_method` entry in
    // `GAPS`.
    gap_todo(out, ctx, draft, "remote_method");

    // Options.
    option_block(out, ctx, draft);
    versioned_arg_value_rows_for(out, ctx, draft);

    // Documentation.
    if ctx.set(draft, "forms") {
        out.gap();
        for form in as_array(&draft["forms"]) {
            let mut row = vec!["form".to_owned(), str_of(&form["kind"]).to_owned()];
            let mut lost = false;
            push_word(&mut row, &mut lost, braced(str_of(&form["synopsis"])));
            if let Some(dialects) = form["surface"].as_array() {
                push_availability_flag(
                    &mut row,
                    &mut lost,
                    &Value::Array(dialects.clone()),
                    ctx.availability,
                );
            }
            push_lifecycle_flags(&mut row, &mut lost, form);
            emit_row(out, "forms", &row, lost, "");
        }
    }
    hover_block(out, ctx, draft);

    refine_blocks(out, ctx, draft, "command_forms");

    // Subcommands.
    for sub in as_array(draft.get("subcommands").unwrap_or(&Value::Null)) {
        let Some(body) = sub.as_object() else {
            continue;
        };
        subcommand_block(out, ctx, body, "subcommand");
    }
}

// Entry points

/// Render one draft as a complete single-command `.tclspec` pack.
///
/// The pack takes its name from the dialect the draft was seeded from
/// ([`SOURCE_DIALECT_KEY`]) when it has one, which is what the ports do —
/// `speclib tcl 1.2`, `speclib f5-irules 1.2` — the newest vocabulary the
/// loader reads, per [`DSL_VERSION`].
#[must_use]
pub fn render(draft: &Draft) -> String {
    let pack = draft
        .get(SOURCE_DIALECT_KEY)
        .and_then(Value::as_str)
        .unwrap_or("pack")
        .to_owned();
    render_pack(std::slice::from_ref(draft), &pack)
}

/// Render a pack of drafts as one `.tclspec` file.
#[must_use]
pub fn render_pack(drafts: &[Draft], pack_name: &str) -> String {
    render_pack_with_version(drafts, pack_name, DSL_VERSION)
}

/// Render a pack of drafts with its declared `SpecTcl` vocabulary version.
///
/// A newly-created or exported pack uses [`DSL_VERSION`], while a document
/// loaded into the studio must retain the version its own `speclib` header
/// declared when a canonical re-render or write-back fallback rebuilds it.
#[must_use]
pub fn render_pack_with_version(drafts: &[Draft], pack_name: &str, dsl_version: &str) -> String {
    render_pack_reporting_with_version(drafts, pack_name, dsl_version).0
}

/// [`render_pack`], plus every field the render could not carry.
///
/// Each [`Loss`] corresponds to one `# TODO(spectcl):` line in the text, so a
/// caller can list what the pack does not say without re-reading its comments.
#[must_use]
pub fn render_pack_reporting(drafts: &[Draft], pack_name: &str) -> (String, Vec<Loss>) {
    render_pack_reporting_with_version(drafts, pack_name, DSL_VERSION)
}

/// [`render_pack_reporting`], retaining which command produced each loss.
///
/// This is useful for exhaustive round-trip checks which render a whole pack
/// at once but still need per-command loss accounting.
#[must_use]
pub fn render_pack_reporting_by_command(
    drafts: &[Draft],
    pack_name: &str,
) -> (String, Vec<CommandLosses>) {
    let (text, _, losses) = render_pack_reporting_impl(drafts, pack_name, DSL_VERSION);
    (text, losses)
}

/// [`render_pack_reporting`] with an explicit `speclib` vocabulary version.
///
/// Kept separate from the default entry point so exports continue to target
/// the newest vocabulary, while an already-authored document round-trips its
/// header byte-for-byte in meaning.
#[must_use]
pub fn render_pack_reporting_with_version(
    drafts: &[Draft],
    pack_name: &str,
    dsl_version: &str,
) -> (String, Vec<Loss>) {
    let (text, losses, _) = render_pack_reporting_impl(drafts, pack_name, dsl_version);
    (text, losses)
}

fn render_pack_reporting_impl(
    drafts: &[Draft],
    pack_name: &str,
    dsl_version: &str,
) -> (String, Vec<Loss>, Vec<CommandLosses>) {
    let mut tables = ValueTables::default();
    let mut bodies: Vec<(String, Out)> = Vec::new();
    // The header decides the vocabulary the body may use, never the other way
    // round: `available` is a 2.0 word, so a pack declaring less keeps the
    // legacy `dialects` spelling and stays internally consistent.
    let availability = !tcl_registry::version::compare(dsl_version, AVAILABILITY_VERSION).is_lt();
    let resolver_capabilities =
        !tcl_registry::version::compare(dsl_version, RESOLVER_CAPABILITIES_VERSION).is_lt();

    for draft in drafts {
        let name = str_of(draft.get("name").unwrap_or(&Value::Null)).to_owned();
        let defaults = draft::default_command_draft();
        // Rendered at its final indent: a braced word's continuation lines are
        // part of the value and must never be shifted afterwards.
        let mut body = Out::at(1);
        {
            let mut ctx = Ctx {
                defaults: &defaults,
                scope: name.clone(),
                owner: name.clone(),
                tables: &mut tables,
                availability,
                resolver_capabilities,
                vocabulary: dsl_version,
            };
            command_body(&mut body, &mut ctx, draft);
        }
        bodies.push((name, body));
    }

    let mut out = Out::default();
    out.comment(
        "Rendered from the tcl-lsp command registry by the spec studio's SpecTcl \
         renderer (rust/tcl-spec-studio/src/render_spectcl.rs).",
    );
    out.comment(
        "The syntax is docs/design/spec-dsl-examples/README.md. Every `TODO(spectcl)` \
         line below names a field the source spec sets that a pack cannot yet say.",
    );
    out.line("");
    out.line(&format!(
        "speclib {} {dsl_version} {{",
        name_word(pack_name)
    ));
    // The ports do not indent a pack's own declarations — a `.tclspec` is one
    // long file of `command` blocks, and a second level of indentation buys
    // nothing.
    let mut table_losses = tables.render(&mut out);
    for (name, body) in &bodies {
        out.gap();
        out.line(&format!("command {} {{", name_word(name)));
        out.block(body);
        out.line("}");
    }
    out.gap();
    out.line("}");
    let losses_by_command = bodies
        .into_iter()
        .map(|(command, body)| {
            let mut losses = body.losses;
            losses.extend(table_losses.remove(&command).unwrap_or_default());
            CommandLosses { command, losses }
        })
        .collect();
    (out.text, out.losses, losses_by_command)
}

#[cfg(test)]
mod tests;
