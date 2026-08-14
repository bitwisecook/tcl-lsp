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

mod dsl_highlight;

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

// ---------------------------------------------------------------------------
// The pack store — the DSL document is the model
// ---------------------------------------------------------------------------
//
// Every export here takes the `.tclspec` **source** and returns JSON, so the
// browser holds exactly one piece of state — the document — and the Rust side
// stays a pure function of it. That is what makes the DSL pane and the form
// two projections of one thing rather than two stores to reconcile.
//
// The one concession to a browser's reality is the cache below. Loading a pack
// leaks its specs by design (`Box::leak`: a loaded pack is meant to outlive the
// call, as it does in the server), so re-parsing on every keystroke would grow
// the page's heap without bound. Keeping the last store keyed on its source
// text means a burst of queries against an unchanged document parses once.

use std::cell::RefCell;

use tcl_spec_studio::store::{Builtins, PackStore, Resolution};

thread_local! {
    /// The last document loaded, so repeated queries against it are free.
    static STORE: RefCell<Option<PackStore>> = const { RefCell::new(None) };
}

/// Run `body` against `source` as a loaded store, reusing the cached parse
/// when the document has not changed.
fn with_store<T>(source: &str, body: impl FnOnce(&PackStore) -> T) -> T {
    STORE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_none_or(|store| store.source() != source) {
            *slot = Some(PackStore::from_source(source));
        }
        body(slot.as_ref().expect("just loaded"))
    })
}

/// Run `body` against a mutable store, publishing whatever document it leaves
/// behind back into the cache.
fn with_store_mut<T>(source: &str, body: impl FnOnce(&mut PackStore) -> T) -> (String, T) {
    STORE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_none_or(|store| store.source() != source) {
            *slot = Some(PackStore::from_source(source));
        }
        let store = slot.as_mut().expect("just loaded");
        let out = body(store);
        (store.source().to_owned(), out)
    })
}

/// A brand-new, empty pack document named `pack_name` — valid DSL from the
/// first byte, so the DSL pane never shows a blank page.
#[wasm_bindgen]
#[must_use]
pub fn pack_new(pack_name: &str) -> String {
    let name = if pack_name.trim().is_empty() {
        "mylib"
    } else {
        pack_name
    };
    to_string(&json!({ "source": PackStore::empty(name).source() }))
}

/// Load `source` into the store and report what it holds.
///
/// Returns the sidebar's whole world: the pack name and DSL version, one row
/// per declared command with its per-command state, every loader notice, and
/// every collision with the shipped registry.
#[wasm_bindgen]
#[must_use]
pub fn pack_load(source: &str, dialect: &str) -> String {
    let builtins = Builtins::for_dialect(dialect);
    with_store(source, |store| {
        to_string(&Resolution::new(builtins, store).store_view())
    })
}

/// The merged view of one command: which of the two definitions is live, and
/// both of them when the pack and the registry both speak.
///
/// `{"error": …}` when neither model knows the name.
#[wasm_bindgen]
#[must_use]
pub fn pack_command(source: &str, name: &str, dialect: &str) -> String {
    let builtins = Builtins::for_dialect(dialect);
    with_store(source, |store| {
        match Resolution::new(builtins, store).view(name) {
            Some(view) => to_string(&view),
            None => error(&format!(
                "`{name}` is in neither the pack nor the {dialect} registry"
            )),
        }
    })
}

/// Write `draft_json` back as the definition of `name`, returning the new
/// document.
///
/// `overrides` writes the declaration as `command NAME -override { … }` — the
/// pack's claim on a shipped name.
///
/// The reply says two things about how the document was reached. `writeback` is
/// `"spliced"` when one statement's bytes were replaced and every other byte
/// (comments included) left alone, or `"rerendered"` when the splice could not
/// be verified and the whole document was rebuilt. `dropped` names every
/// property the declaration used to state and no longer does — a hook body the
/// draft could not carry and the write could not reach — so the loss is
/// reported to the author instead of happening in silence.
#[wasm_bindgen]
#[must_use]
pub fn pack_set_command(source: &str, name: &str, draft_json: &str, overrides: bool) -> String {
    let draft = match parse_draft(draft_json) {
        Ok(draft) => draft,
        Err(message) => return error(&message),
    };
    let (next, write) = with_store_mut(source, |store| store.set_command(name, &draft, overrides));
    to_string(&json!({
        "source": next,
        "writeback": write.how.key(),
        "dropped": write.dropped,
        "name": name,
    }))
}

/// Drop `name` from the document, returning the new document.
#[wasm_bindgen]
#[must_use]
pub fn pack_remove_command(source: &str, name: &str) -> String {
    let (next, removed) = with_store_mut(source, |store| store.remove_command(name));
    if removed {
        to_string(&json!({ "source": next, "removed": name }))
    } else {
        error(&format!("the pack does not declare `{name}`"))
    }
}

/// Re-render the whole document from its derived drafts — the pack's canonical
/// form.
///
/// This is the one operation that deliberately discards the author's layout,
/// so it is an explicit action rather than something a form edit does behind
/// their back.
#[wasm_bindgen]
#[must_use]
pub fn pack_render(source: &str) -> String {
    with_store(source, |store| {
        to_string(&json!({ "source": store.canonical() }))
    })
}

/// The structured validation report for `source` — the studio's twin of
/// `tcl spec check` and the MCP server's `spectcl_check`.
#[wasm_bindgen]
#[must_use]
pub fn pack_validate(source: &str, dialect: &str) -> String {
    let builtins = Builtins::for_dialect(dialect);
    with_store(source, |store| {
        to_string(&Resolution::new(builtins, store).validate())
    })
}

// ---------------------------------------------------------------------------
// The Test tab — the pack, installed, under the real analyser
// ---------------------------------------------------------------------------

/// Analyse `sample` with `source`'s pack installed over the `dialect`
/// registry.
///
/// Returns what an editor would show and how it got there: the analyser's
/// diagnostics, the sample split into render chunks, and one token entry per
/// word carrying its command, that command's origin (pack / shipped /
/// shadowed), the roles the registry gives it, and **which spec property
/// produced it**.
///
/// The pack rides in through `tcl_spectcl::install::registry_with_packs` and
/// `Analyser::with_pack_overlay` — the same two seams the language server uses
/// — so a diagnostic here is the diagnostic a user's editor would raise.
#[wasm_bindgen]
#[must_use]
pub fn pack_test_analyse(source: &str, sample: &str, dialect: &str) -> String {
    let builtins = Builtins::for_dialect(dialect);
    with_store(source, |store| {
        let merged = Resolution::new(builtins, store);
        to_string(&tcl_spec_studio::sample::Bench::new(&merged).analyse(sample))
    })
}

/// The deep-inspection view of the word covering byte `offset` of `sample`.
///
/// The resolved spec and where it came from, the argument's role with the
/// declaration that produced it, the subcommand it selected if any, the
/// diagnostics that land on it, and whether the pack declares the command (so
/// a surface can offer to open it in the form).
#[wasm_bindgen]
#[must_use]
pub fn pack_test_inspect(source: &str, sample: &str, dialect: &str, offset: usize) -> String {
    let builtins = Builtins::for_dialect(dialect);
    with_store(source, |store| {
        let merged = Resolution::new(builtins, store);
        match tcl_spec_studio::sample::Bench::new(&merged).inspect(sample, offset) {
            Some(view) => to_string(&view),
            None => error("no word at that position"),
        }
    })
}

// ---------------------------------------------------------------------------
// The Pack DSL tab — client-side highlight and hover for the DSL itself
// ---------------------------------------------------------------------------

/// Classified byte-span tokens for `.tclspec` `source` — what the Pack DSL
/// editor's overlay paints. `[{start, end, class, text}, …]`, non-overlapping
/// and in source order.
///
/// Classes: `comment`, `command-word` (a statement head — `command`,
/// `arity`, `option`, `hover`, …), `property-flag` (a `-flag` word),
/// `value` (an ordinary bare word), `number`, `variable` (`$name`), and
/// `punctuation` (`;` and `{*}`). See `dsl_highlight` for how a block's
/// contents are told apart as further DSL statements versus a flat word
/// list versus foreign/prose content.
#[wasm_bindgen]
#[must_use]
pub fn dsl_highlight(source: &str) -> String {
    to_string(&dsl_highlight::highlight_json(source))
}

/// The hover for the DSL word covering byte `offset` of `source`.
///
/// `{"found": true, "title": …, "body": …}` when `offset` lands on a
/// recognised statement head or property flag (backed by
/// `tcl_spec_studio::schema` / `help`, a catalogue variant, or the DSL's own
/// grammar keywords); `{"found": false, ...}` otherwise — including when the
/// word under the cursor is ordinary pack-author data (a value, a number, a
/// variable) rather than DSL vocabulary.
#[wasm_bindgen]
#[must_use]
pub fn dsl_hover(source: &str, offset: usize) -> String {
    to_string(&dsl_highlight::hover_json(source, offset))
}
