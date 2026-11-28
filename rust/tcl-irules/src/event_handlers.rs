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

//! Registry-driven extraction of iRules event handlers and effective priorities.

use tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::OriginalIrulesSourceContext;

/// Conditional authored iRules handler and literal priority report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesEventHandler {
    /// Literal event written in the handler declaration.
    pub event: String,
    /// Literal inherited or per-handler priority.
    pub priority: u16,
}

/// Capture original hosted source before projecting handler priority advice.
#[must_use]
pub fn extract_irules_event_handlers(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesEventHandler> {
    OriginalIrulesSourceContext::capture(source, registry).map_or_else(Vec::new, |context| {
        extract_original_irules_event_handlers(source, &context)
    })
}

/// Project top-level handler reports from the same complete original source.
/// Dynamic setters and unavailable source selection withhold inherited priority;
/// a later literal setter restores it. Inline priority affects only its handler.
#[must_use]
pub fn extract_original_irules_event_handlers(
    source: &str,
    context: &OriginalIrulesSourceContext,
) -> Vec<IrulesEventHandler> {
    let mut inherited_priority = None;
    let mut inherited_unknown = false;
    let mut handlers = Vec::new();
    for (_, words) in context.top_level_source_vectors(source) {
        let Some(words) = words else {
            inherited_unknown = true;
            continue;
        };
        let Some((setter, policy, roles)) =
            words.with_source_schema(context.context_registry(), |schema| {
                (
                    schema
                        .semantics
                        .traits
                        .contains(Traits::SETS_EVENT_PRIORITY),
                    schema.authored_event_handler_priority(),
                    schema.arg_roles(),
                )
            })
        else {
            inherited_unknown = true;
            continue;
        };
        if setter {
            inherited_priority = literal_u16_arg(words, 0);
            inherited_unknown = inherited_priority.is_none();
            continue;
        }
        let Some(policy) = policy else {
            continue;
        };
        let Some(event) = literal_arg(words, 0) else {
            continue;
        };
        let Some(roles) = roles else {
            continue;
        };
        let body_index = roles
            .iter()
            .find_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
            .unwrap_or(words.arguments().len());
        let explicit = (1..body_index)
            .find(|&index| literal_arg(words, index) == Some(policy.keyword))
            .map(|index| literal_u16_arg(words, index + 1));
        let priority = explicit.unwrap_or_else(|| {
            (!inherited_unknown).then(|| inherited_priority.unwrap_or(policy.default_priority))
        });
        if let Some(priority) = priority {
            handlers.push(IrulesEventHandler {
                event: event.to_owned(),
                priority,
            });
        }
    }
    handlers
}

fn literal_u16_arg(words: &OriginalRegistryWords, index: usize) -> Option<u16> {
    literal_arg(words, index)?.parse().ok()
}

fn literal_arg(words: &OriginalRegistryWords, index: usize) -> Option<&str> {
    words.original_argument_word(index)?;
    std::str::from_utf8(words.arguments().get(index)?.literal_bytes()?).ok()
}
