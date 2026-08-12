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

//! iRule test framework — simulate BIG-IP TMM event processing.
//!
//! The TMM simulation itself is the Tcl orchestrator + shim + command mocks
//! under `rust/tcl-irule-test/tcl/`, which run an iRule inside a Tcl interpreter
//! on [`tcl-vm`](https://docs.rs). This crate provides the Rust-side glue:
//!
//! * [`topology`] — turn a parsed BIG-IP config ([`tcl_bigip`]) into the
//!   `::orch::` setup commands that configure the orchestrator for a virtual
//!   server (profiles, VIP, pools, data-groups, attached iRules). Usable today.
//! * [`session`] — the driver that bootstraps the orchestrator Tcl on the VM
//!   and fires events. Running events end-to-end is gated on the VM growing the
//!   iRule command surface the orchestrator relies on (see the module docs).
//!
//! ```no_run
//! use tcl_irule_test::topology::Topology;
//!
//! let topo = Topology::from_source(std::fs::read_to_string("bigip.conf")?.as_str());
//! let setup = topo.generate_tcl_setup("/Common/my_vs")?;
//! // `setup` is a Tcl fragment of `::orch::configure` / `::orch::add_pool` / …
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![forbid(unsafe_code)]

pub mod embedded;
pub mod live;
pub mod session;
pub mod sim;
pub mod topology;

pub use embedded::EmbeddedLib;
pub use live::{LiveSession, SessionError};
pub use sim::{SimOutcome, SimRequest, simulate_irule};
pub use topology::{Topology, TopologyError};
