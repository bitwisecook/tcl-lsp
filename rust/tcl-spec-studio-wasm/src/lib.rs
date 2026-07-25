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

//! Rust → WASM facade for the command-registry spec studio.
//!
//! Every export takes and returns a JSON string, so the browser side needs no
//! generated types beyond the wasm-bindgen glue. The real work lives in
//! `tcl-spec-studio`; this file only marshals.
//!
//! The whole registry, the compiler's analyser, and both renderers are linked
//! into this module, so the studio page is genuinely self-contained: it browses
//! the live command registry, infers signatures from imported Tcl, and renders
//! `.rs` and stub output without a single network request.
//!
//! Errors are returned as a JSON `{"error": "…"}` object rather than thrown, so
//! the caller's `JSON.parse` always succeeds.

use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

/// Install a panic hook that routes Rust panics to `console.error` (panics
/// abort under wasm, so this is the only way to see them).
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

fn to_string(value: &Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|e| format!(r#"{{"error":"serialise failed: {e}"}}"#))
}

fn error(message: &str) -> String {
    to_string(&json!({ "error": message }))
}

/// Parse a draft argument into the object the studio works with.
fn parse_draft(draft_json: &str) -> Result<tcl_spec_studio::draft::Draft, String> {
    let value: Value =
        serde_json::from_str(draft_json).map_err(|e| format!("draft is not valid JSON: {e}"))?;
    match value {
        Value::Object(map) => Ok(map),
        _ => Err("draft must be a JSON object".to_owned()),
    }
}

/// The full field schema: group order, catalogues, and both field tables.
///
/// The front-end builds its entire form from this, so a new `CommandSpec`
/// field appears in the UI without any JavaScript change.
#[wasm_bindgen]
#[must_use]
pub fn schema() -> String {
    to_string(&tcl_spec_studio::schema::to_json())
}

/// The dialects the studio can browse, as `[{name, label}]`.
#[wasm_bindgen]
#[must_use]
pub fn dialects() -> String {
    to_string(&tcl_spec_studio::dialects())
}

/// An index of every command in `dialect` — name, summary, synopsis, and
/// subcommand / option counts.
#[wasm_bindgen]
#[must_use]
pub fn command_index(dialect: &str) -> String {
    to_string(&tcl_spec_studio::command_index(dialect))
}

/// A draft seeded from the live registry's spec for `name` under `dialect`.
#[wasm_bindgen]
#[must_use]
pub fn load_command(name: &str, dialect: &str) -> String {
    match tcl_spec_studio::load_command(name, dialect) {
        Some(draft) => to_string(&draft),
        None => error(&format!(
            "`{name}` is not a command in the {dialect} registry"
        )),
    }
}

/// A draft of a brand-new command, every field at its `CommandSpec::DEFAULT`.
#[wasm_bindgen]
#[must_use]
pub fn new_command() -> String {
    to_string(&Value::Object(
        tcl_spec_studio::draft::default_command_draft(),
    ))
}

/// A draft of a brand-new subcommand, every field at its
/// `SubCommand::DEFAULT`.
#[wasm_bindgen]
#[must_use]
pub fn new_subcommand() -> String {
    to_string(&Value::Object(
        tcl_spec_studio::draft::default_subcommand_draft(),
    ))
}

/// Render `draft_json` as a registry `.rs` source file, copyright banner
/// included.
///
/// Returns `{"path": "…", "source": "…"}` — the suggested repository path for
/// the file alongside its contents.
#[wasm_bindgen]
#[must_use]
pub fn render_rs(draft_json: &str, pack: &str) -> String {
    let draft = match parse_draft(draft_json) {
        Ok(draft) => draft,
        Err(message) => return error(&message),
    };
    let name = draft
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let source = tcl_spec_studio::render_rs::render(&draft);
    to_string(&json!({
        "path": tcl_spec_studio::render_rs::suggested_path(&name, pack),
        "source": source,
    }))
}

/// Render one or more drafts as a Tcl dialect stub.
///
/// `drafts_json` is a JSON array of drafts. `mode` is `"inline"` for a
/// `# tcl-lsp: stubs-begin` block or `"file"` for a standalone
/// `<dialect>.tcl.stubs`. Returns `{"path": "…", "source": "…"}`.
#[wasm_bindgen]
#[must_use]
pub fn render_stub(drafts_json: &str, mode: &str, dialect: &str) -> String {
    let value: Value = match serde_json::from_str(drafts_json) {
        Ok(value) => value,
        Err(e) => return error(&format!("drafts are not valid JSON: {e}")),
    };
    let Some(items) = value.as_array() else {
        return error("drafts must be a JSON array");
    };
    let mut drafts = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Object(map) => drafts.push(map.clone()),
            _ => return error("each draft must be a JSON object"),
        }
    }
    let mode = match mode {
        "file" => tcl_spec_studio::render_stub::Mode::File,
        _ => tcl_spec_studio::render_stub::Mode::Inline,
    };
    let source = tcl_spec_studio::render_stub::render(&drafts, mode, dialect);
    let path = match mode {
        tcl_spec_studio::render_stub::Mode::File => format!("{dialect}.tcl.stubs"),
        tcl_spec_studio::render_stub::Mode::Inline => "stubs.tcl".to_owned(),
    };
    to_string(&json!({ "path": path, "source": source }))
}

/// Import a Tcl package and infer a draft spec per procedure it defines.
///
/// `files_json` is `[{"name": "pkg.tcl", "text": "…"}]`. Returns the
/// [`Import`](tcl_spec_studio::infer::Import) JSON: the package name and
/// version from `package provide`, one draft per procedure with the evidence
/// behind each guess, and a warning per file that yielded nothing.
#[wasm_bindgen]
#[must_use]
pub fn import_package(files_json: &str, dialect: &str) -> String {
    let value: Value = match serde_json::from_str(files_json) {
        Ok(value) => value,
        Err(e) => return error(&format!("files are not valid JSON: {e}")),
    };
    let Some(items) = value.as_array() else {
        return error("files must be a JSON array");
    };
    let files: Vec<tcl_spec_studio::infer::SourceFile> = items
        .iter()
        .map(|item| tcl_spec_studio::infer::SourceFile {
            name: item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("untitled.tcl")
                .to_owned(),
            text: item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        })
        .collect();
    if files.is_empty() {
        return error("no files to import");
    }
    to_string(&tcl_spec_studio::infer::import_package(&files, dialect).to_json())
}
