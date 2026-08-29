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

//! BPF-Tcl dialect command specifications.
//!
//! A purpose-built dialect for the BPF-Tcl framework, spanning every layer:
//! the low-level typed eBPF verbs (scalars, packet access, maps, verdicts) and
//! the F5-inspired `when <EVENT> priority N { … }` event layer. (A profile-based
//! top layer is a planned addition that will register here too.)
//!
//! Note: the `when` spec deliberately carries **no** lowering hook, so the
//! BPF-Tcl front-end re-lowers each handler body itself (a separate event space
//! from F5 iRules) rather than going through the `::when::` lowering.

// Layer 1 — typed scalars.
mod seti32;
mod setint;
mod setu32;
// Layer 1 — packet / context access.
mod load16;
mod load32;
mod load8;
mod pktlen;
mod setbuf;
// Layer 2 — maps.
mod map;
mod map_get;
mod map_has;
mod map_set;
// Layer 2 — verdicts.
mod accept;
mod drop;
mod next;
mod pass;
mod tx;
// Control flow.
mod loop_;
// Layer 3 — the event framework.
mod when;
// Layer 4 — the profile-based top layer (protocol facet).
mod field;
mod profile;
// Layer 4 — the profile-based top layer (template/macro facet).
mod template;
mod use_;
// Layer 4 — the profile-based top layer (capability/policy facet).
mod allow;
mod deny;
// Layer 4 — the profile-based top layer (attach/deployment facet).
mod attach;

use crate::spec::CommandSpec;

/// Return all BPF-Tcl command specifications.
#[must_use]
pub fn bpf_command_specs() -> Vec<CommandSpec> {
    vec![
        setint::spec(),
        seti32::spec(),
        setu32::spec(),
        setbuf::spec(),
        load8::spec(),
        load16::spec(),
        load32::spec(),
        pktlen::spec(),
        map::spec(),
        map_get::spec(),
        map_has::spec(),
        map_set::spec(),
        accept::spec(),
        drop::spec(),
        next::spec(),
        pass::spec(),
        tx::spec(),
        loop_::spec(),
        when::spec(),
        profile::spec(),
        field::spec(),
        template::spec(),
        use_::spec(),
        allow::spec(),
        deny::spec(),
        attach::spec(),
    ]
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SpecSurface};
    use super::*;

    #[test]
    fn all_bpf_commands_present_and_tagged() {
        let specs = bpf_command_specs();
        assert_eq!(specs.len(), 26);
        let names: Vec<&str> = specs.iter().map(|s| s.name).collect();
        for n in [
            "setint", "seti32", "setu32", "setbuf", "load8", "load16", "load32", "pktlen", "map",
            "map_get", "map_has", "map_set", "accept", "drop", "next", "pass", "tx", "loop",
            "when", "profile", "field", "template", "use", "allow", "deny", "attach",
        ] {
            assert!(names.contains(&n), "missing `{n}`");
        }
        assert!(specs.iter().all(|s| s.surface == Some(SpecSurface::BPF)));
    }

    #[test]
    fn every_bpf_command_carries_a_typed_lowering_descriptor() {
        // Issue #1202: the registry is the source of truth for BPF lowering —
        // a BPF-dialect spec without a `bpf_op` descriptor cannot be lowered
        // and must never exist.
        for spec in bpf_command_specs() {
            assert!(
                spec.bpf_op.is_some(),
                "BPF command `{}` has no bpf_op descriptor",
                spec.name
            );
        }
    }

    #[test]
    fn when_has_no_lowering_hook() {
        // The BPF `when` must stay a generic call the front-end re-lowers — it
        // must NOT carry the F5 `LoweringHookId::When`.
        let specs = bpf_command_specs();
        let when = specs.iter().find(|s| s.name == "when").expect("when spec");
        assert!(when.lowering_hook.is_none());
    }

    #[test]
    fn bpf_verbs_known_in_any_dialect() {
        let reg = crate::registry::CommandRegistry::build_default();
        assert!(reg.known_in_any_dialect("setint"));
        assert!(reg.known_in_any_dialect("map"));
        assert!(reg.known_in_any_dialect("pktlen"));
    }
}
