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

//! F5 BIG-IP object model and config parser.
//!
//! The parser turns BIG-IP `.conf` / `.scf` text into a [`model`] of
//! typed objects via `parse_bigip_conf` -> `BigipConfig`. The
//! object/property *schema* is reused from `tcl_registry::bigip`.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod apl;
pub mod canonical;
pub mod cleanup;
pub mod conf_tokens;
pub mod convert;
pub mod error;
pub mod f5_trailer;
pub mod flow;
pub mod graph;
pub mod grep;
pub mod irule_context;
pub mod jsonfmt;
pub mod links;
pub mod lint;
pub mod model;
pub mod parser;
pub mod pcap_enrich;
pub mod pcap_remap;
pub mod pcapng;
pub mod policy_eval;
pub mod range;
pub mod redact;
pub mod refs;
pub mod rule_extract;
pub mod secrets;
pub mod stats;
pub mod tls;
pub mod tmsh_emit;
pub mod validator;
pub mod value;
pub mod wireshark_profile;

pub use error::BigipError;
pub use range::{Position, Range};
