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

//! The wire: capability negotiation, bounded client requests and lifts into LSP types.

use std::future::Future;
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::Position;

mod capabilities;
mod lifts;
mod requests;

#[cfg(test)]
pub(super) use capabilities::rename_file_operation_options;
#[cfg(test)]
pub(super) use capabilities::semantic_tokens_capability;
pub(super) use capabilities::{
    build_server_capabilities, client_lacks_utf16_support, client_supports_message_action_items,
    client_supports_pull_diagnostics, client_supports_relative_watch_patterns,
    client_supports_show_document, negotiate_position_encoding,
};
#[cfg(test)]
pub(super) use lifts::lift_symbol_kind;
pub(super) use lifts::{
    lift_bigip_range, lift_completion_item, lift_document_symbol, lift_folding_range, lift_hover,
    lift_lsp_range, lift_signature_help, lift_span, lsp_severity, materialise_selection_range,
};

/// How long to wait for the editor to answer a *server-to-client* request
/// before giving up on it.
///
/// The transport keeps stdin routing independent of handler progress (see
/// [`transport_liveness`](crate::transport_liveness)), so this timeout is
/// defence in depth rather than the session's deadlock breaker for stdin
/// itself. It bounds every server-to-client request issued through
/// [`bounded_client_request`] (which is every one of them but notifications —
/// see that function's doc comment for why bounding all of them matters for
/// process exit, not just stdin liveness).
pub(super) const CLIENT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Bound a server-to-client *request* (never a notification — those do not
/// await a reply) with [`CLIENT_REQUEST_TIMEOUT`], mapping a timed-out
/// request onto the same `jsonrpc::Error` shape a declined one already
/// produces, so every existing `Err(..)`/`let _ = ..` call site is unchanged.
///
/// `tower-lsp-server` 0.23 resolves a server-to-client request by firing a
/// `oneshot` held in its pending-response table (see the dependency's
/// `service::client::pending`) when the matching response arrives on stdin.
/// If the client is gone — the VS Code extension host exited without ever
/// sending `shutdown`/`exit`, or exited right after sending them but before a
/// reply this call happened to be waiting on landed — nothing ever fires that
/// `oneshot`, and the `.await` here would hang forever. `Server::serve` does
/// not return while *any* handler future is still pending, `exit`/stdin-EOF
/// notwithstanding, so one unbounded await like that is enough on its own to
/// keep the whole process alive indefinitely after the session is over
/// (issue #2021) — the exit watchdog in `main.rs` is a backstop for exactly
/// this and the analogous long-running-scan case, but bounding the awaits
/// themselves is what lets the *normal* `serve`-returns exit path win most of
/// the time instead of falling through to the watchdog's grace period.
pub(super) async fn bounded_client_request<T>(
    request: impl Future<Output = jsonrpc::Result<T>>,
) -> jsonrpc::Result<T> {
    bounded_client_request_with_timeout(CLIENT_REQUEST_TIMEOUT, request).await
}

/// [`bounded_client_request`]'s body, parameterised on the timeout so tests
/// can exercise the elapsed-deadline arm in milliseconds rather than the
/// production 10s.
pub(super) async fn bounded_client_request_with_timeout<T>(
    timeout: std::time::Duration,
    request: impl Future<Output = jsonrpc::Result<T>>,
) -> jsonrpc::Result<T> {
    match crate::rt::timeout(timeout, request).await {
        Ok(result) => result,
        Err(_elapsed) => Err(jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!(
                "tcl-lsp: the editor did not answer a server-to-client request within {}s",
                timeout.as_secs()
            )
            .into(),
            data: None,
        }),
    }
}

/// The protocol's reading of a position whose character lies beyond its
/// line: the end of that line. A line beyond the document stays as sent.
pub(super) fn position_within_line(source: &str, position: Position) -> Position {
    let index = tcl_lexer::LineIndex::new_lsp(source);
    if position.line as usize >= index.line_count() {
        return position;
    }
    let end = index.position_at_utf16(
        index.offset_at_utf16(position.line, tcl_lexer::Utf16Col::new(u32::MAX), source),
        source,
    );
    if end.line == position.line && position.character > end.character.get() {
        Position::new(position.line, end.character.get())
    } else {
        position
    }
}

/// Byte offset of the LSP `(line, character)` cursor in `source`.
/// `character` is a UTF-16 code-unit offset. `None` when the line is
/// out of range.
pub(super) fn line_col_to_byte_offset(source: &str, line: u32, col: u32) -> Option<usize> {
    line_col_to_byte_offset_indexed(&tcl_lexer::LineIndex::new_lsp(source), source, line, col)
}

/// [`line_col_to_byte_offset`], but resolved through a caller-supplied
/// `index` instead of rebuilding one — for a caller that already holds the
/// document's persisted [`tcl_lexer::LineIndex`] (`DocumentState::line_index`),
/// so a per-request lookup doesn't pay for a whole-document rescan.
pub(super) fn line_col_to_byte_offset_indexed(
    index: &tcl_lexer::LineIndex,
    source: &str,
    line: u32,
    col: u32,
) -> Option<usize> {
    if line as usize >= index.line_count() {
        return None;
    }
    Some(index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(col), source) as usize)
}

/// The `workspace/foldingRange/refresh` server→client request (LSP 3.18).
///
/// `ls-types` 0.0.6 predates this method, so it is declared locally to be sent
/// via [`Client::send_request`].  Params and result are both `()` per the spec.
/// Asks the client to re-request folding ranges for all editors.  Sent after a
/// config change flips `features.folding` (the client otherwise keeps its
/// cached ranges until the next document edit) and once from `initialized`,
/// so a tab restored before the provider went live recomputes its folding —
/// and, in `VS Code`, its sticky-scroll model with it.
pub(super) enum FoldingRangeRefreshRequest {}

impl tower_lsp_server::ls_types::request::Request for FoldingRangeRefreshRequest {
    type Params = ();
    type Result = ();
    const METHOD: &'static str = "workspace/foldingRange/refresh";
}
