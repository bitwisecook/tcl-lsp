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

//! `SpecTcl` spec packs, end to end (`docs/design/registry/spec-packs.md`).
//!
//! The claim under test is the whole point of the feature: **a user with a
//! private Tcl library drops a `.tclspec` in their workspace and their own
//! commands start behaving like shipped ones** — no Rust toolchain, no server
//! rebuild. So these tests drive the real binary over JSON-RPC against a real
//! directory, and assert on what the editor would show: hover text, the
//! absence of an unknown-command diagnostic, and load notices squiggled on the
//! pack file itself.

use crate::common::helpers::{decode_semantic_tokens, hover_text};
use crate::common::{Lsp, scaled_timeout};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[path = "spec_packs/environment_lifecycle.rs"]
mod environment_lifecycle;

#[path = "spec_packs/bundled_packs.rs"]
mod bundled_packs;
#[path = "spec_packs/claimed_extensions.rs"]
mod claimed_extensions;
#[path = "spec_packs/hook_bodies.rs"]
mod hook_bodies;
#[path = "spec_packs/pack_commands.rs"]
mod pack_commands;
#[path = "spec_packs/pack_documents.rs"]
mod pack_documents;
#[path = "spec_packs/reload.rs"]
mod reload;

/// A pack declaring one command with enough detail to be recognisable in
/// hover: a summary, a synopsis, and a real arity.
const MYLIB_PACK: &str = r"
speclib mylib 1 {
    command mylib::with_var {
        arity 2..3
        arg 0 -role VarWrite
        arg 1 -role Body
        hover {
            summary  {Run a script with a caller variable bound.}
            synopsis {mylib::with_var varName script ?mode?}
            returns  {The script's result.}
        }
    }
}
";

/// A private on-disk workspace for one test.
///
/// The name is deliberately plain ASCII: these paths become `file://` URIs on
/// both sides of the protocol, and the server percent-encodes what
/// `Uri::from_file_path` says to. A directory named after `ThreadId(17)` is
/// then two different strings depending on who built the URI, which is a
/// property of the test fixture and not of anything worth testing.
fn workspace(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-spec-packs-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("workspace root");
    // macOS exposes its temporary directory through `/var`, a symlink to
    // `/private/var`. Pack discovery canonicalises paths before publishing
    // diagnostics, so keep the client-side fixture URI in that same spelling.
    std::fs::canonicalize(root).expect("canonical workspace root")
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, body).expect("write");
}

fn file_uri(path: &Path) -> String {
    tcl_lsp_server::canonical_file_uri(path)
        .expect("pack fixture path has a representable file URI")
        .to_string()
}

/// Tell the server a pack file moved, exactly as a client's file watcher would.
fn notify_pack_changed(lsp: &mut Lsp, path: &Path, kind: i64) {
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(path), "type": kind }] }),
    );
}

const CREATED: i64 = 1;
const CHANGED: i64 = 2;
const DELETED: i64 = 3;

/// The packs the server reports as loaded, from `tcl-lsp.getEffectiveConfig`.
///
/// The settle signal these tests wait on. W123 is deliberately *not* it: the
/// analyser skips the unknown-command check for any name containing `::`
/// ("qualified — defer to per-namespace logic, conservative skip"), so a
/// namespaced pack command never had a W123 to suppress in the first place.
/// What a pack demonstrably changes for a qualified name is what the server
/// *knows* about it — hover, signature, arity — so that is what is asserted,
/// with this as the barrier.
fn loaded_packs(lsp: &mut Lsp) -> Vec<Value> {
    lsp.effective_config("")
        .get("spec_packs_loaded")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// The names of every pack the server reports as loaded.
///
/// A workspace pack is not the only thing in the list: the server also
/// discovers the **bundled** tier (the loadables tcl-lsp ships), so a test
/// about *this* workspace's pack must name it rather than count.
fn pack_names(lsp: &mut Lsp) -> Vec<String> {
    loaded_packs(lsp)
        .iter()
        .filter_map(|pack| pack.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

/// Block until the server reports a loaded pack called `name` — the settle
/// barrier for a workspace pack, since the bundled tier makes "any pack at
/// all" true from the first reload.
fn await_pack_named(lsp: &mut Lsp, name: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !pack_names(lsp).iter().any(|loaded| loaded == name) {
        assert!(
            std::time::Instant::now() < deadline,
            "the `{name}` pack was never reported as loaded; got {:?}",
            pack_names(lsp)
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
