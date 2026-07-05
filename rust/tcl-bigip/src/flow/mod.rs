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

//! PCAP flow extraction and session pairing for `f5 explain-flow`.
//!
//! Walks a capture into
//! per-5-tuple [`Flow`]s ([`packets::extract_flows`]), then pairs them into
//! bidirectional [`Connection`]s and front/back [`Session`]s
//! ([`sessions::pair_sessions`]). The per-session *explanation* (which virtual
//! server matched, the iRule event chain, the policy trace) is built by the
//! driver in `f5-cli`, which consumes these types.

pub mod model;
pub mod packets;
pub mod sessions;
pub mod tshark;

pub use model::{Connection, Flow, FlowKey, Session};
pub use packets::extract_flows;
pub use sessions::{pair_connections, pair_sessions};
pub use tshark::{enrich_with_tshark, extract_flows_via_tshark, tshark_available};
