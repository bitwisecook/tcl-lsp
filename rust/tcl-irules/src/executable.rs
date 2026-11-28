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

//! Presentation projections of the shared conditional iRules source context.

#[cfg(feature = "test-instrumentation")]
use std::cell::Cell;
use std::collections::{HashSet, VecDeque};
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventEmissionCertainty;

/// Presentation-only source-candidate fields. Command/argument/event strings
/// grant no identity, runtime effects, entered frame or worker reachability.
/// Semantic consumers use `OriginalIrulesSourceContext`'s sealed words instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesExecutableCommand {
    /// Actual independently retained source command extent.
    pub span: tcl_lexer::Span,
    /// Selected source-schema label, without a live handler grant.
    pub command: String,
    /// Original literal values or written unknown operands for display.
    pub args: Vec<String>,
    /// Source variable-token labels, without cell or frame authority.
    pub variable_names: Vec<String>,
    /// Actual event-body source label, without event reachability.
    pub event: Option<String>,
}

#[cfg(feature = "test-instrumentation")]
thread_local! {
    static EXECUTABLE_CLOSURE_BUILDS: Cell<usize> = const { Cell::new(0) };
}
/// Reset this thread's source-closure capture count.
#[cfg(feature = "test-instrumentation")]
#[doc(hidden)]
pub fn reset_executable_closure_builds_for_tests() {
    EXECUTABLE_CLOSURE_BUILDS.with(|builds| builds.set(0));
}
/// Number of whole source-context captures at this compatibility ingress.
#[cfg(feature = "test-instrumentation")]
#[doc(hidden)]
#[must_use]
pub fn executable_closure_builds_for_tests() -> usize {
    EXECUTABLE_CLOSURE_BUILDS.with(Cell::get)
}

pub(crate) fn record_source_capture() {
    #[cfg(feature = "test-instrumentation")]
    EXECUTABLE_CLOSURE_BUILDS.with(|builds| builds.set(builds.get() + 1));
}

/// Conditional event-rooted source candidates. Applicability and procedure
/// binding remain unproved; this compatibility return is presentation only.
#[must_use]
pub fn irules_executable_commands(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    crate::OriginalIrulesSourceContext::capture(source, registry)
        .map_or_else(Vec::new, |context| context.presentation_commands(source))
}

/// Source candidates associated with an actual retained event-body descriptor.
/// An event label is a readonly filter, not worker or handler entry authority.
#[must_use]
pub fn irules_event_executable_closure(
    source: &str,
    event: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    once_per_span(
        irules_executable_commands(source, registry)
            .into_iter()
            .filter(|command| {
                command
                    .event
                    .as_deref()
                    .is_some_and(|actual| actual.eq_ignore_ascii_case(event))
            }),
    )
}

/// A closure lists a command once, however many of its handlers reach it.
fn once_per_span(
    commands: impl Iterator<Item = IrulesExecutableCommand>,
) -> Vec<IrulesExecutableCommand> {
    let mut seen = HashSet::new();
    commands
        .filter(|command| seen.insert(command.span))
        .collect()
}

/// Possible authored event-emission relation in the current source schema.
/// The Registry certainty describes the selected schema, without execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesEventEmissionEdge {
    /// Actual emitting source command extent.
    pub span: tcl_lexer::Span,
    /// Source event-body descriptor label.
    pub from_event: String,
    /// Actual selected source-schema label.
    pub command: String,
    /// Possible target event from the authored Registry schema.
    pub to_event: &'static str,
    /// Schema certainty, without observed execution.
    pub certainty: EventEmissionCertainty,
}

fn source_emission_edges(
    context: &crate::OriginalIrulesSourceContext,
) -> Vec<IrulesEventEmissionEdge> {
    let actual = context.context_registry();
    let mut out = Vec::new();
    for command in context.commands() {
        let Some(arguments) = command
            .words()
            .arguments()
            .iter()
            .map(|word| std::str::from_utf8(word.literal_bytes()?).ok())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let emission = command
            .words()
            .with_source_schema(actual, |selected| {
                if !selected
                    .argument_count_for_arity()
                    .is_some_and(|count| selected.semantics.arity.accepts(count))
                {
                    return None;
                }
                actual
                    .commands()
                    .get_exact(selected.canonical_command)?
                    .event_emission_for_args(&arguments)
            })
            .flatten();
        let Some(emission) = emission else {
            continue;
        };
        for to_event in emission.events {
            out.push(IrulesEventEmissionEdge {
                span: command.span(),
                from_event: command.event_source().event().to_owned(),
                command: command.words().command().to_owned(),
                to_event,
                certainty: emission.certainty,
            });
        }
    }
    out
}

/// Current conditional event-emission source relations, through authentic argv.
#[must_use]
pub fn irules_event_emission_edges(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesEventEmissionEdge> {
    crate::OriginalIrulesSourceContext::capture(source, registry)
        .map_or_else(Vec::new, |context| source_emission_edges(&context))
}

/// Conditional source paths widened by actual authored event-emission schema.
/// This union supplies neither an entered event nor complete runtime coverage.
#[must_use]
pub fn irules_event_reachable_closure(
    source: &str,
    event: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    let Some(context) = crate::OriginalIrulesSourceContext::capture(source, registry) else {
        return Vec::new();
    };
    let edges = source_emission_edges(&context);
    let mut pending = VecDeque::from([event.to_owned()]);
    let mut visited = HashSet::new();
    while let Some(event) = pending.pop_front() {
        if !visited.insert(event.to_ascii_uppercase()) {
            continue;
        }
        pending.extend(
            edges
                .iter()
                .filter(|edge| edge.from_event.eq_ignore_ascii_case(&event))
                .map(|edge| edge.to_event.to_owned()),
        );
    }
    once_per_span(
        context
            .presentation_commands(source)
            .into_iter()
            .filter(|command| {
                command
                    .event
                    .as_deref()
                    .is_some_and(|event| visited.contains(&event.to_ascii_uppercase()))
            }),
    )
}
