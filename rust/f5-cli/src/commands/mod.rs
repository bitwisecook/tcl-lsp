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

//! Verb handlers for the `f5-query` CLI.
//!
//! The handlers here need only file I/O plus the `tcl-bigip` parser.

pub mod cleanup;
pub mod convert;
pub mod diff;
pub mod emit;
pub mod enrich_pcapng;
pub mod enrich_wireshark;
pub mod explain;
pub mod explain_flow;
pub mod extract;
pub mod fetch;
pub mod graph;
pub mod grep;
pub mod irule;
pub mod merge;
pub mod pcap_remap;
pub mod pull;
pub mod push;
pub mod query;
pub mod redact;
pub mod registry_dump;
pub mod remote;
pub mod rename;
pub mod scf;
pub mod secrets;
pub mod split;
pub mod stats;
pub mod tmsh;
pub mod unredact;
pub mod validate;
