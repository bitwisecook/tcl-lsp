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

//! Registry-declared command-to-event emission (issue #1708).
//!
//! Every claim here is F5-documented behaviour, cited on the registry spec:
//! `TCP::notify` (<https://clouddocs.f5.com/api/irules/TCP__notify.html>) and
//! `NAME::lookup` (<https://clouddocs.f5.com/api/irules/NAME__lookup.html>).
//! There is no tclsh analogue — iRules events are a BIG-IP surface.

use tcl_dialect::model::{Family, SurfaceLayer};
use tcl_irules::{irules_event_emission_edges, irules_event_reachable_closure};
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventEmissionCertainty;

fn registry() -> CommandRegistry {
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    reg
}

/// Whether the closure of `event` reaches a command spelled `command`.
fn reaches(source: &str, event: &str, command: &str) -> bool {
    irules_event_reachable_closure(source, event, &registry())
        .iter()
        .any(|c| c.command == command)
}

fn edge_targets(source: &str) -> Vec<&'static str> {
    let mut targets: Vec<&'static str> = irules_event_emission_edges(source, &registry())
        .into_iter()
        .map(|edge| edge.to_event)
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

const NOTIFY_REQUEST: &str = r#"
when CLIENT_ACCEPTED {
    TCP::notify request
}
when USER_REQUEST {
    log local0. "reached the user request handler"
}
"#;

const NOTIFY_RESPONSE: &str = r#"
when CLIENT_ACCEPTED {
    TCP::notify response
}
when USER_RESPONSE {
    log local0. "reached the user response handler"
}
"#;

const NOTIFY_EOM: &str = r#"
when CLIENT_ACCEPTED {
    TCP::notify eom
}
when USER_REQUEST {
    log local0. "must not be reachable through eom"
}
when USER_RESPONSE {
    log local0. "must not be reachable through eom"
}
"#;

/// `TCP::notify request` reaches a `USER_REQUEST` handler.
#[test]
fn a_literal_notify_request_reaches_the_user_request_handler() {
    assert_eq!(edge_targets(NOTIFY_REQUEST), vec!["USER_REQUEST"]);
    assert!(
        reaches(NOTIFY_REQUEST, "CLIENT_ACCEPTED", "log"),
        "the USER_REQUEST handler's body must join the closure",
    );
}

/// And `response` reaches `USER_RESPONSE`, not `USER_REQUEST`.
#[test]
fn a_literal_notify_response_reaches_the_user_response_handler() {
    assert_eq!(edge_targets(NOTIFY_RESPONSE), vec!["USER_RESPONSE"]);
}

/// `eom` only marks a message boundary — it reaches neither user event.
///
/// The form declares an empty emission rather than leaving the subcommand
/// undescribed, so it cannot inherit `request`/`response`'s edge. That is the
/// mistake a consumer-side `TCP::notify` name match would make.
#[test]
fn notify_eom_reaches_no_user_event() {
    assert!(
        edge_targets(NOTIFY_EOM).is_empty(),
        "eom raises nothing: {:?}",
        edge_targets(NOTIFY_EOM),
    );
    assert!(
        !reaches(NOTIFY_EOM, "CLIENT_ACCEPTED", "log"),
        "neither user handler may join the closure",
    );
}

/// `NAME::lookup` reaches `NAME_RESOLVED` and the helper called from it.
///
/// Asynchronously: the handler runs when the DNS answer arrives, so it is
/// reachable but is not a continuation of the lookup. The edge records that;
/// the closure records only that it may run.
#[test]
fn a_name_lookup_reaches_the_resolved_handler_and_its_helpers() {
    let source = r#"
proc handle_resolution { } {
    log local0. "helper called from the resolved handler"
}
when CLIENT_ACCEPTED {
    NAME::lookup "example.com"
}
when NAME_RESOLVED {
    call handle_resolution
}
"#;
    let edges = irules_event_emission_edges(source, &registry());
    assert_eq!(edges.len(), 1, "{edges:?}");
    assert_eq!(edges[0].to_event, "NAME_RESOLVED");
    assert_eq!(edges[0].from_event, "CLIENT_ACCEPTED");
    assert_eq!(edges[0].certainty, EventEmissionCertainty::Asynchronous);

    let closure = irules_event_reachable_closure(source, "CLIENT_ACCEPTED", &registry());
    assert!(
        closure.iter().any(|c| c.command == "call"),
        "the resolved handler joins the closure: {closure:?}",
    );
    assert!(
        closure.iter().any(|c| c.command == "log"),
        "and so does the procedure it calls: {closure:?}",
    );
}

/// A computed subcommand or an unknown command invents no edge.
#[test]
fn dynamic_and_unknown_forms_produce_no_guessed_edge() {
    let dynamic = r#"
when CLIENT_ACCEPTED {
    set which request
    TCP::notify $which
}
when USER_REQUEST {
    log local0. "not reachable from a computed subcommand"
}
"#;
    assert!(
        edge_targets(dynamic).is_empty(),
        "a computed subcommand matches no literal form: {:?}",
        edge_targets(dynamic),
    );

    let unknown = r#"
when CLIENT_ACCEPTED {
    NOTAREALCOMMAND::notify request
}
when USER_REQUEST {
    log local0. "not reachable from an unknown command"
}
"#;
    assert!(
        edge_targets(unknown).is_empty(),
        "an unresolved head declares nothing: {:?}",
        edge_targets(unknown),
    );
}

/// The certainty rides on the edge, so a consumer can tell a possible
/// emission from a definite one without re-deriving it.
#[test]
fn a_possible_emission_is_not_reported_as_definite() {
    let edges = irules_event_emission_edges(NOTIFY_REQUEST, &registry());
    assert_eq!(edges.len(), 1, "{edges:?}");
    assert_eq!(
        edges[0].certainty,
        EventEmissionCertainty::Possible,
        "an mblb message-boundary context may consume the notify instead, and \
         nothing at the call site distinguishes the two",
    );
}

/// A call the runtime rejects outright raises nothing.
///
/// The form matcher sees only the `request` prefix, so without an argument
/// count check `TCP::notify request extra` would put an unreachable
/// `USER_REQUEST` handler into every consumer's reachability set.
#[test]
fn an_arity_invalid_call_raises_no_edge() {
    let invalid = r#"
when CLIENT_ACCEPTED {
    TCP::notify request extra
}
when USER_REQUEST {
    log local0. "not reachable from a call that cannot run"
}
"#;
    assert!(
        edge_targets(invalid).is_empty(),
        "the surplus argument breaks TCP::notify's arity: {:?}",
        edge_targets(invalid),
    );
    assert!(
        !reaches(invalid, "CLIENT_ACCEPTED", "log"),
        "and the handler it would have raised stays out of the closure",
    );
    assert_eq!(
        edge_targets(NOTIFY_REQUEST),
        vec!["USER_REQUEST"],
        "the same call at a valid arity still raises the event",
    );
}

/// A procedure emits on behalf of every event that calls it.
///
/// The whole-file executable inventory reaches each procedure once, so the
/// retained event would otherwise be whichever handler appears first in the
/// source — a silent loss of one of the two real edges.
#[test]
fn a_procedure_called_from_two_events_emits_under_both() {
    let source = r#"
proc helper { } {
    NAME::lookup "example.com"
}
when CLIENT_ACCEPTED {
    call helper
}
when HTTP_REQUEST {
    call helper
}
when NAME_RESOLVED {
    log local0. "resolved"
}
"#;
    let mut provenance: Vec<(String, &'static str)> =
        irules_event_emission_edges(source, &registry())
            .into_iter()
            .map(|edge| (edge.from_event, edge.to_event))
            .collect();
    provenance.sort_unstable();
    assert_eq!(
        provenance,
        vec![
            ("CLIENT_ACCEPTED".to_owned(), "NAME_RESOLVED"),
            ("HTTP_REQUEST".to_owned(), "NAME_RESOLVED"),
        ],
        "both callers own an edge to the resolved handler",
    );
}
