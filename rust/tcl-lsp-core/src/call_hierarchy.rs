//! Call-hierarchy provider — Rust port of
//! `lsp/features/call_hierarchy.py`.
//!
//! Three entry points:
//!
//! * [`prepare`] — resolves the proc at the cursor into a
//!   single [`CallHierarchyItem`].
//! * [`incoming_calls`] — every call site in the document that
//!   targets the given proc, grouped by the enclosing proc.
//! * [`outgoing_calls`] — every call site inside the given
//!   proc's body, grouped by the called proc.
//!
//! Edge enumeration mirrors Python's per-proc call-site
//! computation by walking `analysis.command_invocations` and
//! intersecting their byte spans with each proc's body span.
//! No workspace-wide call-graph index is required — every
//! computation stays in-document.

use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One hierarchy item — proc identification plus its name and
/// definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyItem {
    /// Proc name (qualified).
    pub name: String,
    /// Detail (e.g. parameter list summary).
    pub detail: Option<String>,
    /// Range of the entire definition.
    pub range: LspRange,
    /// Range of just the name token.
    pub selection_range: LspRange,
}

/// Resolve a "prepare call hierarchy" request to a single
/// item — the proc whose name is at the cursor.
#[must_use]
pub fn prepare(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<CallHierarchyItem> {
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            return vec![item_for_proc(proc_def, qname, &line_index)];
        }
    }
    Vec::new()
}

/// Build a [`CallHierarchyItem`] for a given proc definition.
fn item_for_proc(proc_def: &ProcDef, qname: &str, line_index: &LineIndex) -> CallHierarchyItem {
    let name_range = span_to_range(line_index, proc_def.name_span);
    let body_range = span_to_range(line_index, proc_def.body_span);
    let detail = if proc_def.params.is_empty() {
        None
    } else {
        Some(format!("({} params)", proc_def.params.len()))
    };
    let full_range = LspRange {
        start_line: name_range.start_line,
        start_character: name_range.start_character,
        end_line: body_range.end_line,
        end_character: body_range.end_character,
    };
    CallHierarchyItem {
        name: qname.to_owned(),
        detail,
        range: full_range,
        selection_range: name_range,
    }
}

/// `true` when `inv_span` lies inside `proc_body_span`.  Used
/// to bucket each call site into its enclosing proc.
fn span_contains(outer: tcl_lexer::Span, inner: tcl_lexer::Span) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

/// Find the proc whose body span contains `inv_span` — i.e.
/// the proc the call site sits inside.  Returns `None` for
/// top-level call sites.
fn enclosing_proc<'a>(
    analysis: &'a AnalysisResult,
    inv_span: tcl_lexer::Span,
) -> Option<(&'a str, &'a ProcDef)> {
    let mut best: Option<(&'a str, &'a ProcDef)> = None;
    for (qname, proc_def) in &analysis.all_procs {
        if !span_contains(proc_def.body_span, inv_span) {
            continue;
        }
        // Pick the smallest enclosing body so nested procs
        // bucket calls into the inner proc rather than the
        // outer.
        let body_len = proc_def.body_span.end() - proc_def.body_span.start();
        let pick = match best {
            None => true,
            Some((_, prev)) => {
                let prev_len = prev.body_span.end() - prev.body_span.start();
                body_len < prev_len
            }
        };
        if pick {
            best = Some((qname.as_str(), proc_def));
        }
    }
    best
}

/// `true` when `inv` (a command invocation) targets `proc_def`.
/// Mirrors the matching logic in [`crate::references`] /
/// [`crate::rename`].
fn invocation_targets(
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    proc_def: &ProcDef,
    qname: &str,
) -> bool {
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname);
    inv.name == proc_def.name
        || inv.name == proc_def.qualified_name
        || inv.name == qname_no_prefix
        || inv
            .resolved_qualified_name
            .as_deref()
            .is_some_and(|r| r == proc_def.qualified_name)
}

/// One incoming-call entry: the caller proc plus the spans at
/// which it calls the target.  Mirrors
/// `lsprotocol.types.CallHierarchyIncomingCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingCall {
    /// The proc that contains the call sites.
    pub from: CallHierarchyItem,
    /// Spans at which `from` calls the target proc.
    pub from_ranges: Vec<LspRange>,
}

/// One outgoing-call entry: the called proc plus the spans
/// from which it's called.  Mirrors
/// `lsprotocol.types.CallHierarchyOutgoingCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingCall {
    /// The proc being called.
    pub to: CallHierarchyItem,
    /// Spans at which the source proc calls `to`.
    pub from_ranges: Vec<LspRange>,
}

/// Enumerate incoming calls for the proc identified by
/// `item.name`.  Walks every command invocation in
/// `analysis.command_invocations` that targets the proc and
/// groups by enclosing proc.  Top-level call sites are bucketed
/// under a synthetic `<top-level>` caller.
///
/// Returns an empty `Vec` when `item.name` doesn't match any
/// proc in `analysis.all_procs`.
#[must_use]
pub fn incoming_calls(
    source: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<IncomingCall> {
    let line_index = LineIndex::new(source);
    let Some((target_qname, target_proc)) =
        analysis.all_procs.iter().find(|(qn, _)| **qn == item.name)
    else {
        return Vec::new();
    };
    // (caller-qname → (caller-item, ranges)) keyed by qname so
    // multiple call sites from one caller group together.
    let mut by_caller: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for inv in &analysis.command_invocations {
        if !invocation_targets(inv, target_proc, target_qname) {
            continue;
        }
        // Skip the proc's own declaration site (the name
        // span sits inside the `proc` invocation's range and
        // would otherwise self-link).
        if span_contains(target_proc.name_span, inv.range)
            || span_contains(inv.range, target_proc.name_span)
        {
            continue;
        }
        let inv_range = span_to_range(&line_index, inv.range);
        let caller_key = enclosing_proc(analysis, inv.range)
            .map_or_else(|| "<top-level>".to_owned(), |(qn, _)| qn.to_owned());
        let entry = by_caller.entry(caller_key.clone()).or_insert_with(|| {
            let caller_item = if caller_key == "<top-level>" {
                CallHierarchyItem {
                    name: caller_key.clone(),
                    detail: None,
                    range: LspRange {
                        start_line: 0,
                        start_character: 0,
                        end_line: 0,
                        end_character: 0,
                    },
                    selection_range: LspRange {
                        start_line: 0,
                        start_character: 0,
                        end_line: 0,
                        end_character: 0,
                    },
                }
            } else {
                let proc = &analysis.all_procs[&caller_key];
                item_for_proc(proc, &caller_key, &line_index)
            };
            (caller_item, Vec::new())
        });
        entry.1.push(inv_range);
    }
    by_caller
        .into_values()
        .map(|(from, from_ranges)| IncomingCall { from, from_ranges })
        .collect()
}

/// Enumerate outgoing calls from the proc identified by
/// `item.name`.  Walks every command invocation whose span
/// sits inside the proc's body span, then resolves each to its
/// target proc (when the invocation names a user proc).  Calls
/// to built-in commands are dropped (they have no
/// [`CallHierarchyItem`] to point to).  Multiple call sites to
/// the same target group together.
#[must_use]
pub fn outgoing_calls(
    source: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<OutgoingCall> {
    let line_index = LineIndex::new(source);
    let Some((_, source_proc)) = analysis.all_procs.iter().find(|(qn, _)| **qn == item.name) else {
        return Vec::new();
    };
    // Map target qname → (target item, list of ranges).
    let mut by_target: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for inv in &analysis.command_invocations {
        if !span_contains(source_proc.body_span, inv.range) {
            continue;
        }
        // Find the user-proc this invocation targets, if any.
        for (qname, proc_def) in &analysis.all_procs {
            if invocation_targets(inv, proc_def, qname) {
                let inv_range = span_to_range(&line_index, inv.range);
                let entry = by_target
                    .entry(qname.clone())
                    .or_insert_with(|| (item_for_proc(proc_def, qname, &line_index), Vec::new()));
                entry.1.push(inv_range);
                break;
            }
        }
    }
    by_target
        .into_values()
        .map(|(to, from_ranges)| OutgoingCall { to, from_ranges })
        .collect()
}

fn span_to_range(line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at(span.start());
    let end = line_index.position_at(span.end());
    LspRange {
        start_line: start.line,
        start_character: start.character,
        end_line: end.line,
        end_character: end.character,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn prepare_resolves_proc_at_cursor() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "::greet");
    }

    #[test]
    fn prepare_returns_empty_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(prepare(src, 0, 6, &analysis).is_empty());
    }

    // -- S-call-hierarchy-rich: incoming + outgoing calls -----------

    #[test]
    fn incoming_calls_from_other_procs() {
        // `caller` calls `target`; `target` is being asked
        // for its incoming calls.
        let src = "proc target {} {}\nproc caller {} { target }\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(src, target, &analysis);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "::caller");
        assert_eq!(incoming[0].from_ranges.len(), 1);
    }

    #[test]
    fn incoming_calls_group_by_caller() {
        // Two call sites from the same caller bucket together
        // into a single IncomingCall entry.
        let src = "proc target {} {}\nproc caller {} { target\n target }\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(src, target, &analysis);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from_ranges.len(), 2);
    }

    #[test]
    fn incoming_calls_top_level_bucketed_as_top_level() {
        let src = "proc target {} {}\ntarget\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(src, target, &analysis);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "<top-level>");
    }

    #[test]
    fn outgoing_calls_from_proc_body() {
        // `caller` calls `target` and `other`; outgoing-calls
        // for `caller` should include both.
        let src = "proc target {} {}\nproc other {} {}\nproc caller {} { target\n other }\n";
        let analysis = analyse(src);
        let items = prepare(src, 2, 6, &analysis);
        assert_eq!(items[0].name, "::caller");
        let outgoing = outgoing_calls(src, &items[0], &analysis);
        let target_names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert!(target_names.contains(&"::target"), "{outgoing:?}");
        assert!(target_names.contains(&"::other"), "{outgoing:?}");
    }

    #[test]
    fn outgoing_calls_skip_builtins() {
        // `caller` calls `puts` (built-in) and `target`
        // (user proc).  Only `target` should appear in the
        // outgoing list.
        let src = "proc target {} {}\nproc caller {} { puts hi\n target }\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 6, &analysis);
        assert_eq!(items[0].name, "::caller");
        let outgoing = outgoing_calls(src, &items[0], &analysis);
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["::target"], "{outgoing:?}");
    }

    #[test]
    fn outgoing_calls_empty_for_unknown_proc() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let bogus = CallHierarchyItem {
            name: "::not_a_real_proc".to_string(),
            detail: None,
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            selection_range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
        };
        assert!(outgoing_calls(src, &bogus, &analysis).is_empty());
    }
}
