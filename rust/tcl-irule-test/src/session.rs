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

//! Session driver — bootstrap the TMM-sim orchestrator on the VM and fire
//! events.
//!
//! The simulation is the Tcl under `rust/tcl-irule-test/tcl/` (orchestrator +
//! TMM shim + command mocks); a session sources it on a [`tcl-vm`] interpreter,
//! applies a [`crate::topology`] setup (or hand-built `::orch::` calls), fires
//! events, and reads back the pool/node decisions, logs, and assertions.
//!
//! This module owns the VM-independent half: the **session contract and the
//! setup-script assembly** — topology generation and the orchestrator-bootstrap
//! script. The live VM round-trip (sourcing the orchestrator on a [`tcl-vm`]
//! interpreter and driving real events — `when` event dispatch, `HTTP::*`,
//! `pool`/`node`, `class match`, …) lives in [`crate::live::LiveSession`].
//!
//! [`SessionPlan::into_bootstrap`] is the exact script the live driver runs to
//! stand a session up.

pub use tcl_registry::f5::RuleIdentity;

use crate::topology::{Topology, TopologyError};

/// Source with optional explicitly supplied logical rule ownership.
#[derive(Debug, Clone)]
pub struct RuleSource {
    /// Configuration/session identity; `None` preserves the unnamed API.
    pub identity: Option<RuleIdentity>,
    /// Literal iRule source.
    pub source: String,
    /// Whether event handlers are attached to the simulated virtual server.
    pub register_events: bool,
}

impl RuleSource {
    /// Existing source-only convenience input, without invented identity.
    #[must_use]
    pub fn unnamed(source: impl Into<String>) -> Self {
        Self {
            identity: None,
            source: source.into(),
            register_events: true,
        }
    }

    /// An explicitly named rule attached to the simulated virtual server.
    #[must_use]
    pub fn named(identity: RuleIdentity, source: impl Into<String>) -> Self {
        Self {
            identity: Some(identity),
            source: source.into(),
            register_events: true,
        }
    }

    /// A named procedure library whose events are not attached.
    #[must_use]
    pub fn library(identity: RuleIdentity, source: impl Into<String>) -> Self {
        Self {
            identity: Some(identity),
            source: source.into(),
            register_events: false,
        }
    }

    /// Render the framework load command with the shared literal list encoder.
    #[must_use]
    pub fn load_command(&self) -> String {
        use tcl_syntax::list::list_element;
        self.identity.as_ref().map_or_else(
            || {
                if self.register_events {
                    format!("::orch::load_irule {}", list_element(&self.source))
                } else {
                    format!("::orch::load_rule {{}} {} 0", list_element(&self.source))
                }
            },
            |identity| {
                format!(
                    "::orch::load_rule {} {} {}",
                    list_element(identity.as_path()),
                    list_element(&self.source),
                    u8::from(self.register_events)
                )
            },
        )
    }
}

/// The transport / L7 profiles a session is configured with (the orchestrator
/// `-profiles` list).
#[derive(Debug, Clone, Default)]
pub struct Profiles(pub Vec<String>);

impl Profiles {
    /// Render as the orchestrator `-profiles {…}` argument.
    #[must_use]
    pub fn to_tcl(&self) -> String {
        self.0.join(" ")
    }
}

/// A fully-assembled session bootstrap: the orchestrator `source`s, the
/// `::orch::init`, the topology/`configure` setup, and the loaded iRules — i.e.
/// everything needed to stand the session up on the VM before firing events.
#[derive(Debug, Clone)]
pub struct SessionPlan {
    profiles: Profiles,
    setup: String,
    rules: Vec<RuleSource>,
}

impl SessionPlan {
    /// Build a plan from an explicit profile list and a topology setup fragment
    /// (e.g. from [`Topology::generate_tcl_setup`]).
    #[must_use]
    pub fn new(profiles: Profiles, setup: impl Into<String>) -> Self {
        Self {
            profiles,
            setup: setup.into(),
            rules: Vec::new(),
        }
    }

    /// Attach explicitly named or unnamed source inputs to the bootstrap.
    #[must_use]
    pub fn with_rules(mut self, rules: Vec<RuleSource>) -> Self {
        self.rules = rules;
        self
    }

    /// Build a plan for `vs_name` in `topology`, deriving the orchestrator setup
    /// from the parsed config.
    ///
    /// # Errors
    /// Propagates [`TopologyError`] when the virtual server is not found.
    pub fn from_topology(topology: &Topology, vs_name: &str) -> Result<Self, TopologyError> {
        let setup = topology.generate_tcl_setup(vs_name)?;
        Ok(Self::new(Profiles::default(), setup))
    }

    /// Assemble the Tcl bootstrap script a VM driver runs: source the
    /// orchestrator, initialise it, apply the configured profiles, then the
    /// topology/iRule setup. The orchestrator/shim Tcl is sourced from
    /// `lib_dir` (the directory holding `orchestrator.tcl`).
    #[must_use]
    pub fn into_bootstrap(&self, lib_dir: &str) -> String {
        use std::fmt::Write as _;
        let mut s = String::new();
        let _ = writeln!(s, "source [file join {{{lib_dir}}} orchestrator.tcl]");
        let _ = writeln!(s, "source [file join {{{lib_dir}}} scf_loader.tcl]");
        s.push_str("::orch::init\n");
        if !self.profiles.0.is_empty() {
            let _ = writeln!(
                s,
                "::orch::configure -profiles {{{}}}",
                self.profiles.to_tcl()
            );
        }
        s.push_str(&self.setup);
        for rule in &self.rules {
            let _ = writeln!(s, "\n{}", rule.load_command());
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_source_bootstrap_preserves_literal_identity_and_library_attachment() {
        let library = RuleSource::library(
            RuleIdentity::new("/Other/library").unwrap(),
            "proc helper {} {return ok}",
        );
        let named = RuleSource::named(
            RuleIdentity::new("/Common/rule").unwrap(),
            "when HTTP_REQUEST {call /Other/library::helper}",
        );
        let bootstrap = SessionPlan::new(Profiles::default(), "")
            .with_rules(vec![library, named])
            .into_bootstrap("/tmp/framework");
        assert!(
            bootstrap.contains("::orch::load_rule /Other/library {proc helper {} {return ok}} 0")
        );
        assert!(bootstrap.contains(
            "::orch::load_rule /Common/rule {when HTTP_REQUEST {call /Other/library::helper}} 1"
        ));
        assert_eq!(
            RuleSource::unnamed("when HTTP_REQUEST {pool a}").identity,
            None
        );
    }

    #[test]
    fn bootstrap_assembles_orchestrator_script() {
        let plan = SessionPlan::new(
            Profiles(vec!["TCP".into(), "HTTP".into()]),
            "::orch::add_pool p {1.2.3.4:80}\n",
        );
        let boot = plan.into_bootstrap("/opt/itest");
        assert!(boot.contains("source [file join {/opt/itest} orchestrator.tcl]"));
        assert!(boot.contains("::orch::init"));
        assert!(boot.contains("::orch::configure -profiles {TCP HTTP}"));
        assert!(boot.contains("::orch::add_pool p {1.2.3.4:80}"));
    }

    #[test]
    fn plan_from_topology() {
        let conf = "ltm virtual v { destination 10.0.0.1:80 profiles { http { } } }";
        let topo = Topology::from_source(conf);
        let plan = SessionPlan::from_topology(&topo, "v").expect("plan");
        let boot = plan.into_bootstrap("lib");
        assert!(boot.contains("::orch::configure -profiles {TCP HTTP}"));
        assert!(boot.contains("-local_addr 10.0.0.1 -local_port 80"));
    }
}
