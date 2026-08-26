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

//! The browser transport for the Tcl language server.
//!
//! `tcl_lsp_server::Backend` wrapped in `LspService` is the entire protocol
//! core, and it is transport-free: the native binary drives it over stdio,
//! this crate drives it over `postMessage`. Nothing here decides anything
//! about Tcl — every handler, diagnostic, and provider a page sees is the same
//! code the native `tcl-lsp-server` binary runs, including the two protocol
//! shims in [`tcl_lsp_server::service`], which are applied here for the same
//! reasons `main.rs` applies them.
//!
//! # The protocol on the wire
//!
//! One LSP JSON-RPC message per `postMessage`, as a string, in both
//! directions — no envelope, no framing headers. That is exactly what
//! `monaco-languageclient`'s browser worker transport speaks, so a page can
//! point it at this worker with no adapter. Content-Length framing exists to
//! delimit messages in a byte stream; `postMessage` already delimits them.
//!
//! # Concurrency
//!
//! Each client→server request is dispatched and then *detached*: the driver
//! loop goes straight back to reading. It has to. During `initialized` the
//! server issues `workspace/configuration` and awaits the client's reply, so a
//! driver that awaited each handler to completion before reading again would
//! be waiting for a message it has stopped listening for. The reply arrives on
//! the same `send` path and is routed to the socket's response sink.
//!
//! # Files
//!
//! The worker's [`vfs::MemoryStore`] is where every *closed* file comes from —
//! the host fills it in with [`LspWorker::vfs_upsert`]. Open documents never
//! touch it; they arrive as `didOpen`/`didChange` like anywhere else.
//!
//! The whole-workspace paths read the same store, so a filled one gives a real
//! workspace session rather than an empty one: the folder scan indexes every
//! upserted file under a declared workspace folder, the package database is
//! built from upserted `pkgIndex.tcl` / `tclIndex` files, and `.tclspec` packs
//! the host registers with [`LspWorker::vfs_upsert_spec_pack`] load as the
//! bundled tier. See [`tcl_lsp_core::vfs`] for the seam itself.
//!
//! Ordering is the host's job and the protocol already says it: upsert before
//! `initialize`, because `initialized` is what loads the packs and runs the
//! scan. Files that appear later need no new message either — post them as an
//! ordinary `workspace/didChangeWatchedFiles`, exactly as an editor would for a
//! file that changed outside it.

use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::mpsc;
use futures::sink::SinkExt as _;
use tcl_lsp_server::service::{inject_type_hierarchy_provider, normalise_request_uris};
use tcl_lsp_server::vfs::MemoryStore;
use tcl_lsp_server::{Backend, vfs};
use tower::Service as _;
use tower_lsp_server::LspService;
use tower_lsp_server::jsonrpc::{Request, Response};
use wasm_bindgen::prelude::*;

/// One language-server session, driven over `postMessage`.
///
/// Construct one per worker. It owns the service, the client socket pumps, and
/// the in-memory file store, and it stays alive for as long as the worker
/// does.
#[wasm_bindgen]
pub struct LspWorker {
    /// Client→server messages, queued for the driver task.
    inbox: mpsc::UnboundedSender<String>,
    /// The closed-file store, kept so `vfs_*` can reach it after construction.
    store: Arc<MemoryStore>,
}

#[wasm_bindgen]
impl LspWorker {
    /// Start a session that posts every server→client message by calling
    /// `post` with one JSON-RPC string.
    ///
    /// `post` is called with `this` unbound and exactly one argument, so
    /// `self.postMessage` can be passed straight in from a worker.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(post: js_sys::Function) -> Self {
        console_error_panic_hook::set_once();

        let store = Arc::new(MemoryStore::new());
        let backend_store: Arc<dyn vfs::SourceStore> = store.clone();
        let (service, socket) =
            LspService::new(move |client| Backend::with_store(client, backend_store));
        let (requests, mut responses) = socket.split();

        // Server→client traffic: diagnostics, log messages, and the server's
        // own requests (`workspace/configuration`, `client/registerCapability`).
        // Their replies come back through `send` and are routed to `responses`
        // below.
        let outbound_post = post.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut requests = requests;
            while let Some(request) = requests.next().await {
                post_message(&outbound_post, &request);
            }
        });

        let (inbox, mut queue) = mpsc::unbounded::<String>();
        wasm_bindgen_futures::spawn_local(async move {
            let mut service = service;
            while let Some(text) = queue.next().await {
                match classify(&text) {
                    Some(Incoming::Request(request)) => {
                        // `poll_ready` is `Pending` for as long as an
                        // `initialize` is in flight, which is the transport's
                        // way of holding everything else behind it.
                        if futures::future::poll_fn(|cx| service.poll_ready(cx))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        let call = service.call(normalise_request_uris(*request));
                        let reply_post = post.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Ok(Some(response)) = call.await {
                                post_message(
                                    &reply_post,
                                    &inject_type_hierarchy_provider(response),
                                );
                            }
                        });
                    }
                    Some(Incoming::Response(response)) => {
                        if responses.send(response).await.is_err() {
                            break;
                        }
                    }
                    None => log_bad_message(&text),
                }
            }
        });

        Self { inbox, store }
    }

    /// Hand one client→server JSON-RPC message to the server.
    ///
    /// The message is queued and processed in arrival order; this returns
    /// immediately. A request's reply, and any server-initiated traffic it
    /// causes, arrive through the `post` callback.
    pub fn send(&self, json: &str) {
        if self.inbox.unbounded_send(json.to_owned()).is_err() {
            log_bad_message("the language server session has ended");
        }
    }

    /// Register (or replace) the contents of a file the editor does not have
    /// open, so cross-file navigation and `source` resolution can reach it.
    ///
    /// `uri` must be a `file:` URI. A URI whose path cannot be recovered is
    /// ignored — there is nothing the server could key it under.
    pub fn vfs_upsert(&self, uri: &str, text: &str) {
        if let Some(path) = file_path_of(uri) {
            self.store.upsert_uri(uri, path, text.as_bytes().to_vec());
        }
    }

    /// Forget a file previously registered with [`Self::vfs_upsert`].
    ///
    /// Returns whether anything was stored under that URI.
    pub fn vfs_delete(&self, uri: &str) -> bool {
        self.store.remove_uri(uri)
    }

    /// Register a `.tclspec` pack for the bundled tier.
    ///
    /// `name` is a file name, optionally with directories
    /// (`vendor.tclspec`, `eda/xilinx.tclspec`); it is placed under
    /// [`vfs::VIRTUAL_PACK_MOUNT`], which is where discovery looks when there
    /// is no executable to sit beside. Separate from [`Self::vfs_upsert`]
    /// because that one keys on a `file:` URI, and the mount deliberately is
    /// not one a URI can spell.
    ///
    /// Call it before `initialize`: `initialized` loads the pack set once, and
    /// a pack registered afterwards reaches the session only on the next
    /// reload (a `didChangeConfiguration`, or a
    /// `workspace/didChangeWatchedFiles` naming a `.tclspec`).
    pub fn vfs_upsert_spec_pack(&self, name: &str, text: &str) {
        let path = std::path::Path::new(vfs::VIRTUAL_PACK_MOUNT).join(name);
        self.store.upsert(path, text.as_bytes().to_vec());
    }

    /// The store path prefix [`Self::vfs_upsert_spec_pack`] writes under, for a
    /// host that would rather build the paths itself.
    #[must_use]
    pub fn spec_pack_mount() -> String {
        vfs::VIRTUAL_PACK_MOUNT.to_owned()
    }

    /// How many closed files the worker currently holds.
    #[must_use]
    pub fn vfs_len(&self) -> usize {
        self.store.len()
    }
}

/// What a client→server message turned out to be.
enum Incoming {
    /// A request or notification for the server to handle.
    Request(Box<Request>),
    /// The client's reply to a request the *server* made.
    Response(Response),
}

/// Decide whether a message is for the service or for the client socket.
///
/// A JSON-RPC message carrying `method` is a request or a notification and
/// belongs to the service; one carrying `result` or `error` instead is a reply
/// to something the server asked, and belongs to the socket's response sink.
/// Deserialising into `Request` first and falling back would misfile a
/// malformed request as a response, so the discriminator is checked directly.
fn classify(text: &str) -> Option<Incoming> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    if value.get("method").is_some() {
        serde_json::from_value(value)
            .ok()
            .map(|request| Incoming::Request(Box::new(request)))
    } else {
        serde_json::from_value(value).ok().map(Incoming::Response)
    }
}

/// Serialise one server→client message and hand it to the host callback.
fn post_message<T: serde::Serialize>(post: &js_sys::Function, message: &T) {
    let Ok(text) = serde_json::to_string(message) else {
        return;
    };
    // A host whose callback throws is a host bug, not a server one: report it
    // and keep the session alive rather than unwinding through the pump.
    if let Err(err) = post.call1(&JsValue::NULL, &JsValue::from_str(&text)) {
        web_log(&format!("tcl-lsp: the postMessage callback threw: {err:?}"));
    }
}

/// The filesystem path a `file:` URI names, if it names one.
///
/// Deliberately the same conversion the server itself uses (`Uri::to_file_path`
/// on the parsed URI), so a path the host registers is spelled the way the
/// server will look it up.
fn file_path_of(uri: &str) -> Option<std::path::PathBuf> {
    use std::str::FromStr as _;
    let parsed = tower_lsp_server::ls_types::Uri::from_str(uri).ok()?;
    Some(parsed.to_file_path()?.into_owned())
}

/// Report a message the worker could not make sense of.
fn log_bad_message(detail: &str) {
    web_log(&format!(
        "tcl-lsp: dropped an unparseable message: {detail}"
    ));
}

/// Write one line to the host console.
fn web_log(message: &str) {
    web_sys_console::error(&JsValue::from_str(message));
}

/// The one `console` binding this crate needs, hand-written rather than
/// pulling in `web-sys` for a single method.
mod web_sys_console {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    unsafe extern "C" {
        /// `console.error(message)`.
        #[wasm_bindgen(js_namespace = console, js_name = error)]
        pub fn error(message: &JsValue);
    }
}
