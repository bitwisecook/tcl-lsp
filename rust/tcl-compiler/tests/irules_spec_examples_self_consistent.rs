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

//! Registry self-consistency for the IRULE1001 event-validity diagnostic.
//!
//! The diagnostic is deliberately a generic registry consumer: it reads a
//! command's `event_requires` descriptor and the event matrix. That means a
//! data error can create a false positive for every use of a command. The
//! registry's documented examples are a useful, independent assertion of the
//! advertised legal context. This test checks every iRules spec whose example
//! establishes an event context and attributes only diagnostics emitted for the
//! command that owns that example.
//!
//! This is a data-quality gate, not a proof of F5 command availability. An
//! example can be incomplete or absent, and it cannot establish every legal
//! event. The authoritative source remains the command documentation linked by
//! each registry spec.

use tcl_dialect::model::{SurfaceQuery, Family};
use tcl_compiler::analyser::Analyser;
use tcl_core_types::{DiagCode, Severity};
use tcl_dialect::DialectSet;
use tcl_registry::model::ingress::static_context_for;
use tcl_registry::{events::EventRegistry, profiles::ProfileRegistry};
use tcl_dialect::model::{SpecSurface};

/// Every IRULE1001 diagnostic raised against `command` in `source`.
fn invalidity_diagnostics_for(source: &str, command: &str) -> Vec<String> {
    let command_prefix = format!("'{command}'");
    let mut analyser = Analyser::new();
    analyser
        .analyse(source, "f5-irules")
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::Irule1001)
        // A Hint means that a command is legal in this event, but the source
        // did not declare a profile. It is deliberately not a contradiction:
        // a short documentation example normally omits virtual-server setup.
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .map(|diagnostic| diagnostic.message)
        .filter(|message| message.starts_with(&command_prefix))
        .collect()
}

#[test]
fn irules_documented_examples_do_not_violate_their_own_event_contract() {
    let registry = static_context_for("f5-irules").commands();
    let mut names: Vec<&str> = registry.command_names().collect();
    names.sort_unstable();
    names.dedup();

    let mut checked = 0usize;
    let mut offenders = Vec::new();
    for name in names {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        if !spec.supports_dialect(Some(SurfaceQuery::any_release(Family::F5Irules))) {
            continue;
        }
        let Some(example) = spec.hover.as_ref().map(|hover| hover.examples) else {
            continue;
        };
        // IRULE1001 is event-scoped. Examples without a `when` block say
        // nothing about that contract, and are intentionally outside this
        // invariant.
        if !example.contains("when ") {
            continue;
        }
        checked += 1;
        offenders.extend(
            invalidity_diagnostics_for(example, name)
                .into_iter()
                .map(|message| format!("  {name}: {message}")),
        );
    }

    assert!(
        checked > 100,
        "expected a broad iRules example set, but checked only {checked} specs"
    );
    assert!(
        offenders.is_empty(),
        "{} iRules registry example(s) violate their command's own event \
         contract. Correct the registry requirement data or the documented \
         example; do not weaken the generic IRULE1001 consumer:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

#[test]
fn irules_declared_additional_events_are_legal_in_the_registry_matrix() {
    let registry = static_context_for("f5-irules").commands();
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let mut names: Vec<&str> = registry.command_names().collect();
    names.sort_unstable();
    names.dedup();

    let mut offenders = Vec::new();
    for name in names {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        if let Some(requirements) = spec.event_requires.as_ref() {
            for event in requirements.also_in {
                if !registry.is_irules_command_legal_in_event(name, event, &events, &profiles) {
                    offenders.push(format!("  {name}: declared also_in {event}"));
                }
            }
        }
        for form in spec.event_requirement_forms {
            for event in form.only_in {
                if !registry.is_irules_call_legal_in_event(
                    name,
                    form.argument_prefix,
                    event,
                    &events,
                    &profiles,
                ) {
                    offenders.push(format!(
                        "  {name} {}: declared only_in {event}",
                        form.argument_prefix.join(" ")
                    ));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "an iRules command's `event_requires.also_in` data must be accepted by \
         the same registry legality matrix IRULE1001 consumes:\n{}",
        offenders.join("\n")
    );
}
