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

//! iRule event structural facts: category grouping and enabling profiles.
//!
//! For each iRule event this records two facts — the protocol *categories* it
//! belongs to (`HTTP`, `SSL`, `TCP`, …) and the profile *types* that enable it
//! (`HTTP`, `CLIENTSSL`, `ACCESS`, …). It is deliberately facts-only: event
//! semantics and prose live in [`crate::events`] and
//! [`crate::event_descriptions`].
//!
//! The enabling-profile facts tie the event registry to the profile graph: the
//! inverse view ([`events_for_profile`]) answers "which events does attaching
//! profile P make available", and profile names match
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
    /// Profile *types* that enable the event, uppercase to match
    /// [`crate::profiles::ProfileSpec::name`] (e.g. `["HTTP"]`).
    pub profiles: &'static [&'static str],
}

/// The facts recorded for `event`, or `None` when the event is unknown.
#[must_use]
pub fn event_facts(event: &str) -> Option<&'static EventFacts> {
    EVENT_FACTS.iter().find(|e| e.event == event)
}

/// Events enabled by profile type `profile` (case-insensitive), sorted. The
/// profile-graph inverse of [`EventFacts::profiles`].
#[must_use]
pub fn events_for_profile(profile: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = EVENT_FACTS
        .iter()
        .filter(|e| e.profiles.iter().any(|p| p.eq_ignore_ascii_case(profile)))
        .map(|e| e.event)
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
        assert!(f.profiles.contains(&"HTTP"));
        assert!(event_facts("__NO_SUCH_EVENT__").is_none());
    }

    #[test]
    fn inverse_views_tie_to_the_profile_graph() {
        // The HTTP profile enables HTTP_REQUEST (case-insensitive lookup).
        let http_events = events_for_profile("http");
        assert!(http_events.contains(&"HTTP_REQUEST"));
        // Category grouping works.
        let ssl_events = events_in_category("SSL");
        assert!(!ssl_events.is_empty());
        // Unknown profile/category → empty.
        assert!(events_for_profile("__nope__").is_empty());
        assert!(events_in_category("__nope__").is_empty());
    }
}
