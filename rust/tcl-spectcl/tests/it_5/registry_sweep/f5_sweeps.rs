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

//! iRules event and command legality, and taint-source colours.

use super::*;

// SWEEP 3 — iRules event ⇄ command cross-product accessors over the corpus.

/// Drive `event_info`, `valid_irules_commands_for_event`,
/// `is_irules_command_legal_in_event`, and `irules_events_for_command` across
/// EVERY known event, asserting the legality listing and the O(1) check agree,
/// and that `event_info` fields are internally consistent.
///
/// f5-dialect: iRules events / profiles are F5 concepts, not tclsh.
#[test]
fn sweep_event_command_legality_consistent() {
    let (reg, ()) = (registry_for_dialect("f5-irules"), ());
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();

    let all_events = events.all_event_names();
    assert!(
        all_events.len() > 50,
        "too few events: {}",
        all_events.len()
    );

    for event in &all_events {
        // Version-aware "known": an event retired before the default target
        // release (the classic pre-10.0 XML parser set) is correctly reported
        // unknown *there*, so it is checked at a release inside its own
        // lifetime instead. Everything else uses the permissive `None`.
        let lifecycle = events.event_lifecycle(event).expect("listed event");
        let target = lifecycle.retired.and(lifecycle.introduced);
        let listed = reg.valid_irules_commands_for_event(event, &events, &profiles, target);
        // Listing is sorted + unique (the accessor sorts).
        let mut sorted = listed.clone();
        sorted.sort_unstable();
        assert_eq!(listed, sorted, "{event}: valid-command listing not sorted");

        // Cross-check: a representative slice of the listing must pass the O(1)
        // legality check. (Full cross-product is large; the invariant is
        // per-command, so a sample is sufficient and keeps the test fast.)
        for cmd in listed.iter().take(25) {
            assert!(
                reg.is_irules_command_legal_in_event(cmd, event, &events, &profiles),
                "{cmd} listed valid in {event} but is_legal disagrees"
            );
        }

        // event_info agrees with the listing on known-ness and count.
        let info = reg.event_info(event, &events, &profiles, target);
        assert!(info.known, "{event}: listed event reports known=false");
        assert_eq!(
            info.event,
            event.to_uppercase(),
            "{event}: event_info name not upper-cased"
        );
        assert_eq!(
            info.valid_command_count(),
            listed.len(),
            "{event}: event_info count disagrees with listing"
        );
        // side label is one of the documented vocabulary.
        assert!(
            [
                "client-side",
                "server-side",
                "client-side and server-side",
                "global"
            ]
            .contains(&info.side),
            "{event}: unexpected side label {:?}",
            info.side
        );
    }

    // Inverse listing for a handful of representative commands: every event it
    // lists must independently pass the legality check.
    for cmd in ["HTTP::header", "TCP::collect", "SSL::cert", "when", "log"] {
        let legal = reg.irules_events_for_command(cmd, &events, &profiles);
        let mut sorted = legal.clone();
        sorted.sort_unstable();
        assert_eq!(legal, sorted, "{cmd}: events_for_command not sorted");
        for ev in &legal {
            assert!(
                reg.is_irules_command_legal_in_event(cmd, ev, &events, &profiles),
                "{cmd}: events_for_command lists {ev} but is_legal disagrees"
            );
        }
    }

    // An unknown event is illegal for everything and yields an empty listing.
    assert_eq!(
        reg.valid_irules_commands_for_event("__FAKE__", &events, &profiles, None)
            .len(),
        0
    );
    let unknown = reg.event_info("__fake__", &events, &profiles, None);
    assert!(!unknown.known);
    assert_eq!(unknown.side, "unknown");
}

// SWEEP 4 — taint-source / colour metadata across the source corpus.

/// Every command the registry reports as a taint source resolves to a spec
/// whose `taint_source` colour is set; the colour includes TAINTED (a source is
/// attacker-controlled by definition). Exercises the `TAINT_SOURCE_INDEX` and the
/// `taint_source` query helper across the whole index.
///
/// registry-metadata / f5-dialect: taint colours are registry security data.
#[test]
fn sweep_taint_source_colours() {
    // Build a registry that has the iRules pack loaded so the F5 getters'
    // specs are resolvable for the cross-check.
    let reg = registry_for_dialect("f5-irules");

    let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
    let mut sources_seen = 0usize;
    for name in &names {
        if let Some(colour) = reg.taint_source(name) {
            sources_seen += 1;
            // A taint source's colour is non-empty and marks the value tainted.
            assert!(
                !colour.is_empty(),
                "{name}: taint_source returned an empty colour set"
            );
            assert!(
                colour.contains(TaintColour::TAINTED),
                "{name}: taint source colour lacks TAINTED: {colour:?}"
            );
        }
    }
    assert!(
        sources_seen > 0,
        "no taint sources found in the f5-irules registry"
    );

    // Spot-check a couple of well-known colour-carrying sources (f5-dialect):
    // HTTP::path / HTTP::uri are path-prefixed sources.
    for src in ["HTTP::path", "HTTP::uri"] {
        if let Some(c) = reg.taint_source(src) {
            assert!(
                c.contains(TaintColour::TAINTED),
                "{src}: expected TAINTED in colour {c:?}"
            );
        }
    }
}
