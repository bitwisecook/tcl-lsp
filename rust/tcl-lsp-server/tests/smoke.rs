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

//! Consolidated LSP smoke suite.
//!
//! Each `*_smoke` suite drives the real server over an in-memory duplex pipe
//! and proves one provider answers a client at all. They held one test apiece,
//! and every `tests/*.rs` is its own binary statically linking the whole
//! dependency graph — so eight files meant eight half-gigabyte link steps to
//! run eleven tests. Sharing one binary is the same arrangement
//! `tests/e2e.rs` already uses for the 58 end-to-end suites, and for the same
//! reason.
//!
//! The files live under `tests/smoke/` — a non-target subdirectory, so cargo
//! does not re-discover them as binaries of their own — and `#[path]` points at
//! them. The binary is named `smoke`, which is what keeps the whole tier
//! inside nextest's smoke filterset (`binary(/(^|_)smoke$/)` in
//! `.config/nextest.toml`) and inside a plain `cargo test smoke`.
//!
//! Each suite keeps its own copy of the JSON-RPC framing helpers, exactly as it
//! did as a standalone binary: they are separate modules, so nothing collides,
//! and a smoke test that inherited a shared harness would no longer be testing
//! the thing it claims to.

#[path = "smoke/completion_smoke.rs"]
mod completion_smoke;
#[path = "smoke/definition_smoke.rs"]
mod definition_smoke;
#[path = "smoke/diagnostics_delivery_smoke.rs"]
mod diagnostics_delivery_smoke;
#[path = "smoke/document_symbols_smoke.rs"]
mod document_symbols_smoke;
#[path = "smoke/folding_smoke.rs"]
mod folding_smoke;
#[path = "smoke/hover_smoke.rs"]
mod hover_smoke;
#[path = "smoke/inlay_hint_smoke.rs"]
mod inlay_hint_smoke;
#[path = "smoke/signature_help_smoke.rs"]
mod signature_help_smoke;
