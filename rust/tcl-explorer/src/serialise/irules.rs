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

//! The iRules views: flow warnings, the cross-event connection scope and event order.

use super::{RetainedExplorerSource, retained_explorer_source};
use crate::ExplorerResult;
use crate::formatters::range_dict;
use serde_json::{Value, json};
use tcl_compiler::irules_checks::{
    find_conditional_collection_source_pairing_advice, find_hoistable_set_warnings,
    find_unnormalised_getter_warnings,
};
use tcl_lexer::{LineIndex, Span};
use tcl_spectcl::bundled::active_registry_for_dialect as registry_for_dialect;

/// Serialise the `irulesFlow` view: iRules flow / performance warnings —
/// `{code, message, range, severity}`, composing the five `irules_checks`
/// finders. Empty for non-iRules dialects (an empty list on the tcl corpus).
#[must_use]
pub fn serialise_irules_flow(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let warnings = irules_flow_warnings(result, &retained);

    let out: Vec<Value> = warnings
        .iter()
        .map(|report| match report {
            IrulesFlowReport::Check(w) => json!({
                "code": w.code.as_str(), "message": w.message,
                "range": range_dict(w.span, li, source), "severity": "warning",
            }),
            IrulesFlowReport::ConditionalSource {
                code,
                message,
                span,
            } => json!({
                "code": code.as_str(), "message": message,
                "range": range_dict(*span, li, source), "severity": "warning",
                "analysisKind": "conditional-source",
            }),
        })
        .collect();
    Value::Array(out)
}

pub(super) enum IrulesFlowReport {
    Check(tcl_compiler::irules_checks::IrulesCheckWarning),
    ConditionalSource {
        code: tcl_compiler::compiler_checks::DiagCode,
        message: String,
        span: Span,
    },
}

/// One actual-unit warning aggregation shared by the view and its count.
pub(super) fn irules_flow_warnings(
    result: &ExplorerResult,
    retained: &RetainedExplorerSource<'_>,
) -> Vec<IrulesFlowReport> {
    let registry = retained.registry;
    let dialect = retained.dialect;
    let cu = &result.unit;
    let mut warnings: Vec<_> = find_unnormalised_getter_warnings(
        cu,
        registry,
        dialect.map(tcl_dialect::DialectProfile::surface_query),
    )
    .into_iter()
    .map(IrulesFlowReport::Check)
    .collect();
    if let Some(advice) = find_conditional_collection_source_pairing_advice(
        cu,
        registry,
        dialect.map(tcl_dialect::DialectProfile::surface_query),
    ) {
        if let Some(flow) = advice.conditional_source_flow() {
            warnings.extend(flow.findings().iter().map(|finding| {
                IrulesFlowReport::ConditionalSource {
                    code: finding.code(),
                    message: finding.message().to_owned(),
                    span: finding.span(),
                }
            }));
        }
        warnings.extend(advice.pairings().iter().map(|pairing| {
            IrulesFlowReport::ConditionalSource {
                code: pairing.code(),
                message: pairing.message().to_owned(),
                span: pairing.span(),
            }
        }));
    }
    warnings.extend(
        find_hoistable_set_warnings(cu, dialect.map(tcl_dialect::DialectProfile::surface_query))
            .into_iter()
            .map(IrulesFlowReport::Check),
    );

    warnings
}

/// Serialise the retained cross-event connection scope, including each event
/// summary and the conservative cross-event/racy sets used by diagnostics.
#[must_use]
pub fn serialise_connection_scope(result: &ExplorerResult) -> Value {
    let Some(scope) = result.unit.connection_scope.as_ref() else {
        return json!({ "available": false, "summaries": [] });
    };
    let mut events: Vec<_> = scope.summaries.values().collect();
    events.sort_by(|a, b| a.event.cmp(&b.event));
    let summaries: Vec<Value> = events
        .iter()
        .map(|summary| {
            let mut defs: Vec<_> = summary.defs.iter().cloned().collect();
            defs.sort_unstable();
            let mut uses: Vec<_> = summary.uses_before_def.iter().cloned().collect();
            uses.sort_unstable();
            let mut unsets: Vec<_> = summary.unsets.iter().cloned().collect();
            unsets.sort_unstable();
            json!({ "event": summary.event, "defs": defs, "usesBeforeDef": uses, "unsets": unsets })
        })
        .collect();
    let sorted = |set: &std::collections::HashSet<String>| {
        let mut values: Vec<_> = set.iter().cloned().collect();
        values.sort_unstable();
        values
    };
    json!({
        "available": true,
        "summaries": summaries,
        "crossEventDefs": sorted(&scope.cross_event_defs),
        "crossEventImports": sorted(&scope.cross_event_imports),
        "racyStaticDefs": sorted(&scope.racy_static_defs),
    })
}

/// Serialise the `eventOrder` view: iRules `when EVENT [priority N] { body }`
/// handlers in canonical firing order.
///
/// Reuses the segmenter to find `when` commands (the body must be a braced
/// block) and the reuse-positive `EventRegistry::{order_events,
/// event_multiplicity}` for the canonical ordering / multiplicity class. Empty
/// for ordinary Tcl (no `when` blocks), so it is strictly gateable against the
/// Tcl corpus; the `when`-handler behaviour is pinned by a Rust unit test.
#[must_use]
pub fn serialise_event_order(source: &str, line_index: &LineIndex, dialect: &str) -> Value {
    let command_registry = registry_for_dialect(dialect);
    let identities = tcl_compiler::realm::document_realm_bindings(
        source,
        crate::environment::analyser_profile_for_dialect(dialect),
        &command_registry,
    );
    let handlers = tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver(
        source,
        &command_registry,
        &identities,
    );
    serialise_event_handlers(source, line_index, handlers)
}

/// Event-order advice from the actual retained Module inputs. Missing or
/// mismatched source ownership returns null, independently of an empty inventory.
#[must_use]
pub fn serialise_event_order_for_result(result: &ExplorerResult, line_index: &LineIndex) -> Value {
    let Some(retained) = retained_explorer_source(result, &result.source) else {
        return Value::Null;
    };
    let handlers =
        tcl_registry::events::top_level_when_handlers_with_source_config_and_head_resolver(
            &result.source,
            retained.registry,
            retained.config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
            retained.realm.as_ref(),
        );
    serialise_event_handlers(&result.source, line_index, handlers)
}

fn serialise_event_handlers(
    source: &str,
    line_index: &LineIndex,
    handlers: Vec<tcl_syntax::event_handler::EventHandler>,
) -> Value {
    use std::collections::HashMap;

    use tcl_registry::events::EventRegistry;

    /// One discovered `when` handler.
    struct Handler {
        priority: i64,
        /// File-order index among matched handlers (tie-breaker).
        idx: usize,
        /// The `EVENT` token's span (for the navigable range).
        span: Span,
    }

    let mut per_event: HashMap<String, Vec<Handler>> = HashMap::new();
    for (matched, handler) in handlers.into_iter().enumerate() {
        let event = handler.event;
        let span = handler.event_span;
        let priority = i64::from(handler.effective_priority);
        per_event.entry(event).or_default().push(Handler {
            priority,
            idx: matched,
            span,
        });
    }

    // Each event's handlers fire lowest-priority first, file order breaking ties.
    for handlers in per_event.values_mut() {
        handlers.sort_by_key(|h| (h.priority, h.idx));
    }

    let registry = EventRegistry::build();
    let keys: Vec<String> = per_event.keys().cloned().collect();
    let mut entries = Vec::new();
    for event in registry.order_events(&keys) {
        let multiplicity = registry.event_multiplicity(&event);
        let handlers = &per_event[&event];
        let mut prev_priority: Option<i64> = None;
        let mut offset = 0;
        for handler in handlers {
            if prev_priority == Some(handler.priority) {
                offset += 1;
            } else {
                offset = 0;
                prev_priority = Some(handler.priority);
            }
            entries.push(json!({
                "event": event,
                "base_priority": handler.priority,
                "priority_offset": offset,
                "multiplicity": multiplicity,
                "range": range_dict(handler.span, line_index, source),
            }));
        }
    }
    Value::Array(entries)
}
