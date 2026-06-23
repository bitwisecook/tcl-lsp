//! iRule test framework — simulate BIG-IP TMM event processing.
//!
//! The TMM simulation itself is the Tcl orchestrator + shim + command mocks
//! under `tooling/irule_test/tcl/`, which run an iRule inside a Tcl interpreter
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
pub mod topology;

pub use embedded::EmbeddedLib;
pub use live::{LiveSession, SessionError};
pub use topology::{Topology, TopologyError};
