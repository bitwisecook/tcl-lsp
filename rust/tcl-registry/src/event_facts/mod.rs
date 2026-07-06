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

//! iRule event structural facts: protocol category grouping.
//!
//! For each iRule event this records the protocol *categories* it belongs to
//! (`HTTP`, `SSL`, `TCP`, …). It is deliberately facts-only: event semantics
//! and prose live in [`crate::events`] and [`crate::event_descriptions`].
//!
//! Enabling-profile facts are **not** recorded here. Which profile *types* make
//! an event available is the hand-curated
//! [`crate::events::EventProps::implied_profiles`] — the single source of truth;
//! the inverse view ([`events_for_profile`]) reads that directly so this
//! schema-derived table cannot drift from it. Profile names match
//! [`crate::profiles::ProfileSpec::name`].

mod generated;

pub use generated::EVENT_FACTS;

/// Structural facts for one iRule event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventFacts {
    /// Event name (e.g. `"HTTP_REQUEST"`).
    pub event: &'static str,
    /// Protocol categories the event belongs to (e.g. `["HTTP", "SSL"]`).
    pub categories: &'static [&'static str],
}

/// The facts recorded for `event`, or `None` when the event is unknown.
#[must_use]
pub fn event_facts(event: &str) -> Option<&'static EventFacts> {
    EVENT_FACTS.iter().find(|e| e.event == event)
}

/// Events enabled by profile type `profile` (case-insensitive), sorted.
///
/// Derived from the authoritative
/// [`crate::events::EventProps::implied_profiles`] rather than a generated
/// table, so it stays in lockstep with the event registry (no drift, full
/// event coverage).
#[must_use]
pub fn events_for_profile(profile: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = crate::events::event_props_table()
        .into_iter()
        .filter(|(_, props)| {
            props
                .implied_profiles
                .iter()
                .any(|p| p.eq_ignore_ascii_case(profile))
        })
        .map(|(name, _)| name)
        .collect();
    out.sort_unstable();
    out
}

/// Events in protocol `category` (case-insensitive), sorted.
#[must_use]
pub fn events_in_category(category: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = EVENT_FACTS
        .iter()
        .filter(|e| e.categories.iter().any(|c| c.eq_ignore_ascii_case(category)))
        .map(|e| e.event)
        .collect();
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_broad_and_well_formed() {
        assert!(
            EVENT_FACTS.len() > 100,
            "expected a broad event set, got {}",
            EVENT_FACTS.len()
        );
        // Sorted by event name, unique.
        for w in EVENT_FACTS.windows(2) {
            assert!(w[0].event < w[1].event, "events sorted & unique");
        }
    }

    #[test]
    fn http_request_facts_resolve() {
        let f = event_facts("HTTP_REQUEST").expect("HTTP_REQUEST present");
        assert!(f.categories.contains(&"HTTP"));
        assert!(event_facts("__NO_SUCH_EVENT__").is_none());
    }

    #[test]
    fn inverse_views_tie_to_the_profile_graph() {
        // The HTTP profile enables HTTP_REQUEST (case-insensitive lookup).
        let http_events = events_for_profile("http");
        assert!(http_events.contains(&"HTTP_REQUEST"));
        // events_for_profile is sourced from events.rs implied_profiles, so it
        // reflects the full registry — e.g. FASTHTTP enables HTTP_RESPONSE,
        // which the old generated table dropped.
        assert!(events_for_profile("FASTHTTP").contains(&"HTTP_RESPONSE"));
        // Category grouping works.
        let ssl_events = events_in_category("SSL");
        assert!(!ssl_events.is_empty());
        // Unknown profile/category → empty.
        assert!(events_for_profile("__nope__").is_empty());
        assert!(events_in_category("__nope__").is_empty());
    }
}
