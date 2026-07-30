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

//! `irule_cfg_paths` — enumerate the control-flow paths through an iRule to
//! their terminal actions, annotated for test generation.
//!
//! Combines the `_extract_test_paths` / `_annotate_path` /
//! `_generate_path_questions` chain and the `irule_cfg_paths` MCP tool. It
//! walks the `{events, procedures}` flow tree from
//! [`tcl_diagram::diagram_data`], flattens every branch to a
//! terminal-action path, and annotates each path with a priority, the taint
//! warnings relevant to its action, and the user-facing questions an agent
//! should ask before writing assertions.
//!
//! Path matching keys on tainted-source substrings and command names — no
//! regular expressions — so this uses plain string containment rather than
//! pulling in a regex dependency.

use serde_json::{Map, Value, json};
use tcl_registry::registry_for_dialect;

const IRULES_DIALECT: &str = "f5-irules";

/// Actions that indicate security-sensitive paths (`_SECURITY_ACTIONS`).
const SECURITY_ACTIONS: &[&str] = &["reject", "drop", "discard", "HTTP::respond"];
/// Actions that indicate routing decisions (`_ROUTING_ACTIONS`).
const ROUTING_ACTIONS: &[&str] = &["pool", "node", "snat", "snatpool", "virtual"];
/// Taint codes that are security-critical (`_SECURITY_TAINT_CODES`).
const SECURITY_TAINT_CODES: &[&str] = &[
    "T100",
    "T101",
    "T102",
    "IRULE3001",
    "IRULE3002",
    "IRULE3003",
];
/// Tainted request-input references that elevate a path's priority.
const TAINTED_REFS: &[&str] = &[
    "HTTP::uri",
    "HTTP::path",
    "HTTP::host",
    "HTTP::query",
    "HTTP::header",
    "HTTP::cookie",
    "URI::query",
];

/// One condition on the way to a terminal action: either an `if` branch or a
/// `switch` arm. Carries only the fields the condition dicts expose.
enum Condition {
    If { condition: String, branch: Branch },
    Switch { subject: String, pattern: String },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Branch {
    True,
    Else,
}

impl Branch {
    fn as_str(self) -> &'static str {
        match self {
            Self::True => "true",
            Self::Else => "else",
        }
    }
}

impl Condition {
    /// The label fragment for this condition inside a `path_label`.
    fn label_part(&self) -> String {
        match self {
            Self::If { condition, branch } => format!("{condition} ({})", branch.as_str()),
            Self::Switch { subject, pattern } => format!("switch {subject} = {pattern}"),
        }
    }

    /// The text scanned for tainted-source references (concatenates the
    /// `condition` and `subject` fields; only one is ever non-empty per kind).
    fn taint_scan_text(&self) -> &str {
        match self {
            Self::If { condition, .. } => condition,
            Self::Switch { subject, .. } => subject,
        }
    }

    /// The human-readable summary fragment used when phrasing questions.
    fn summary_part(&self) -> String {
        match self {
            Self::If { condition, branch } => match branch {
                Branch::Else => "no prior conditions match".to_owned(),
                Branch::True => condition.clone(),
            },
            Self::Switch { subject, pattern } => {
                if pattern == "default" {
                    format!("{subject} doesn't match other cases")
                } else {
                    format!("{subject} matches '{pattern}'")
                }
            }
        }
    }

    /// Serialise to the wire dict shape the `conditions` list carries.
    fn to_json(&self) -> Value {
        match self {
            Self::If { condition, branch } => json!({
                "kind": "if",
                "condition": condition,
                "branch": branch.as_str(),
            }),
            Self::Switch { subject, pattern } => json!({
                "kind": "switch",
                "subject": subject,
                "pattern": pattern,
            }),
        }
    }

    /// Deep-copy (the walk holds conditions by reference; each collected path
    /// needs its own owned copy).
    fn clone_owned(&self) -> Self {
        match self {
            Self::If { condition, branch } => Self::If {
                condition: condition.clone(),
                branch: *branch,
            },
            Self::Switch { subject, pattern } => Self::Switch {
                subject: subject.clone(),
                pattern: pattern.clone(),
            },
        }
    }
}

/// A taint warning distilled to the fields the annotation logic reads.
struct Taint {
    code: String,
    sink_command: String,
    /// The full warning dict, re-emitted verbatim in `taint_warnings`.
    raw: Value,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    High,
    Normal,
    Low,
}

impl Priority {
    fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Normal => "normal",
            Self::Low => "low",
        }
    }
}

/// A terminal action reached by walking one branch of the flow tree.
struct Action {
    command: String,
    args: Vec<String>,
}

/// A fully annotated path from an event/proc to a terminal action.
struct PathInfo {
    event: String,
    conditions: Vec<Condition>,
    action: Action,
    path_label: String,
    priority: Priority,
    taint_warnings: Vec<Value>,
    questions: Vec<Value>,
}

impl PathInfo {
    fn to_json(&self) -> Value {
        json!({
            "event": self.event,
            "conditions": self.conditions.iter().map(Condition::to_json).collect::<Vec<_>>(),
            "action": { "command": self.action.command, "args": self.action.args },
            "path_label": self.path_label,
            "priority": self.priority.as_str(),
            "taint_warnings": self.taint_warnings,
            "questions": self.questions,
        })
    }
}

/// The annotated CFG paths as JSON values (each `{event, conditions, action,
/// path_label, priority, taint_warnings, questions}`), in priority order.
/// Shared with `generate_irule_test` (the `cfg_paths` field + per-path cases).
#[must_use]
pub(crate) fn cfg_paths_json(source: &str) -> Vec<Value> {
    extract_test_paths(source)
        .iter()
        .map(PathInfo::to_json)
        .collect()
}

/// MCP tool entry point: enumerate CFG paths for the `source` argument and
/// return the grouped/annotated wire shape.
#[must_use]
pub fn irule_cfg_paths(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let paths = extract_test_paths(source);

    // Group by event, preserving first-seen order (serde_json keeps insertion
    // order via the `preserve_order` feature).
    let mut by_event: Map<String, Value> = Map::new();
    for p in &paths {
        let entry = by_event
            .entry(p.event.clone())
            .or_insert_with(|| json!({ "count": 0, "paths": [] }));
        let obj = entry.as_object_mut().expect("event group object");
        let paths_arr = obj
            .get_mut("paths")
            .and_then(Value::as_array_mut)
            .expect("paths array");
        paths_arr.push(p.to_json());
        let count = paths_arr.len();
        obj.insert("count".to_owned(), json!(count));
    }

    // Flatten every path's questions, tagging each with its path label/priority.
    let mut all_questions: Vec<Value> = Vec::new();
    for p in &paths {
        for q in &p.questions {
            let mut merged = Map::new();
            merged.insert("path_label".to_owned(), json!(p.path_label));
            merged.insert("priority".to_owned(), json!(p.priority.as_str()));
            if let Some(fields) = q.as_object() {
                for (k, v) in fields {
                    merged.insert(k.clone(), v.clone());
                }
            }
            all_questions.push(Value::Object(merged));
        }
    }

    let high_priority = paths
        .iter()
        .filter(|p| p.priority == Priority::High)
        .count();

    json!({
        "total_paths": paths.len(),
        "high_priority_paths": high_priority,
        "paths_by_event": Value::Object(by_event),
        "questions": all_questions,
        "coverage_hint": "Each path represents a unique route through the iRule to a terminal \
             action.  To achieve full branch coverage, write at least one test per \
             path.  Pay special attention to paths with 'else' conditions — they \
             represent fallback/default behavior that is often under-tested.",
        "agentic_hint": "Present the 'questions' list to the user to gather expected values \
             before generating tests.  Questions are ordered by priority — ask \
             about high-priority (security) paths first.  Use the user's answers \
             to fill in assertion values instead of guessing from the source code.  \
             If the user provides custom input values, use those instead of the \
             suggested defaults derived from branch conditions.",
    })
}

/// Walk the flow tree and collect every terminal-action path, sorted by
/// priority (high → normal → low).
fn extract_test_paths(source: &str) -> Vec<PathInfo> {
    let registry = registry_for_dialect(IRULES_DIALECT);
    let data = tcl_diagram::diagram_data_for_dialect(source, registry, IRULES_DIALECT);
    if data.get("error").is_some() {
        return Vec::new();
    }

    let taints = collect_taints(source);
    let mut paths: Vec<PathInfo> = Vec::new();

    if let Some(events) = data.get("events").and_then(Value::as_array) {
        for event in events {
            let name = event.get("name").and_then(Value::as_str).unwrap_or("");
            walk_flow(flow_of(event), name, &mut Vec::new(), &taints, &mut paths);
        }
    }

    // Helper procs may carry actions too; label them `proc:<name>`.
    if let Some(procs) = data.get("procedures").and_then(Value::as_array) {
        for proc in procs {
            let name = proc.get("name").and_then(Value::as_str).unwrap_or("");
            let event = format!("proc:{name}");
            walk_flow(flow_of(proc), &event, &mut Vec::new(), &taints, &mut paths);
        }
    }

    // Stable sort by priority so high-priority paths come first.
    paths.sort_by_key(|p| p.priority);
    paths
}

/// The `flow` list of an event/proc dict, or an empty slice when absent.
fn flow_of(node: &Value) -> &[Value] {
    node.get("flow")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// The child flow list under `key`, or an empty slice when absent.
fn child_body<'a>(node: &'a Value, key: &str) -> &'a [Value] {
    node.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Recursively walk flow nodes, collecting paths to `action` terminals.
/// `conditions` is the branch chain built so far; it is pushed/popped in place.
fn walk_flow(
    nodes: &[Value],
    event: &str,
    conditions: &mut Vec<Condition>,
    taints: &[Taint],
    out: &mut Vec<PathInfo>,
) {
    for node in nodes {
        match node.get("kind").and_then(Value::as_str) {
            Some("action") => {
                let command = node.get("command").and_then(Value::as_str).unwrap_or("");
                let args = string_array(node.get("args"));
                out.push(build_path(event, conditions, command, args, taints));
            }
            Some("if") => {
                let branches = node.get("branches").and_then(Value::as_array);
                for branch in branches.into_iter().flatten() {
                    let cond = branch
                        .get("condition")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let branch_kind = if cond == "else" {
                        Branch::Else
                    } else {
                        Branch::True
                    };
                    conditions.push(Condition::If {
                        condition: cond.to_owned(),
                        branch: branch_kind,
                    });
                    walk_flow(child_body(branch, "body"), event, conditions, taints, out);
                    conditions.pop();
                }
            }
            Some("switch") => {
                let subject = node.get("subject").and_then(Value::as_str).unwrap_or("");
                let arms = node.get("arms").and_then(Value::as_array);
                for arm in arms.into_iter().flatten() {
                    let pattern = arm.get("pattern").and_then(Value::as_str).unwrap_or("");
                    conditions.push(Condition::Switch {
                        subject: subject.to_owned(),
                        pattern: pattern.to_owned(),
                    });
                    walk_flow(child_body(arm, "body"), event, conditions, taints, out);
                    conditions.pop();
                }
            }
            // Loops / catch / try: descend into the body without adding a
            // condition.
            Some("loop" | "catch" | "try") => {
                walk_flow(child_body(node, "body"), event, conditions, taints, out);
            }
            _ => {}
        }
    }
}

/// Collect a JSON array of strings into a `Vec<String>` (empty when absent).
fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Build one path from an action terminal, computing its label, priority,
/// relevant taint warnings, and questions (`_annotate_path`).
fn build_path(
    event: &str,
    conditions: &[Condition],
    command: &str,
    args: Vec<String>,
    taints: &[Taint],
) -> PathInfo {
    let path_label = build_label(event, conditions, command, &args);

    // Base priority from the action command.
    let mut priority = if SECURITY_ACTIONS.contains(&command) {
        Priority::High
    } else if ROUTING_ACTIONS.contains(&command) {
        Priority::Normal
    } else {
        Priority::Low
    };

    // Elevate a low-priority path to normal when a condition references a
    // tainted request-input source.
    if priority == Priority::Low
        && conditions
            .iter()
            .any(|c| TAINTED_REFS.iter().any(|r| c.taint_scan_text().contains(r)))
    {
        priority = Priority::Normal;
    }

    // Taint warnings relevant to this action's sink, plus a security escalation
    // if any warning carries a security-critical code.
    let mut relevant_taints: Vec<Value> = Vec::new();
    for t in taints {
        let sink = t.sink_command.as_str();
        if sink == command || (command == "pool" && (sink == "pool" || sink == "node")) {
            relevant_taints.push(t.raw.clone());
        }
        if SECURITY_TAINT_CODES.contains(&t.code.as_str()) {
            priority = Priority::High;
        }
    }

    let action = Action {
        command: command.to_owned(),
        args,
    };
    let questions = generate_questions(event, conditions, &action, &relevant_taints);

    PathInfo {
        event: event.to_owned(),
        conditions: conditions.iter().map(Condition::clone_owned).collect(),
        action,
        path_label,
        priority,
        taint_warnings: relevant_taints,
        questions,
    }
}

/// The human-readable `path_label`: `event → cond (branch) → command args`.
fn build_label(event: &str, conditions: &[Condition], command: &str, args: &[String]) -> String {
    let mut parts: Vec<String> = vec![event.to_owned()];
    for c in conditions {
        parts.push(c.label_part());
    }
    let action_text = format!("{command} {}", args.join(" "));
    parts.push(action_text.trim().to_owned());
    parts.join(" → ")
}

/// Generate the user-facing questions for one path
/// (`_generate_path_questions`). `relevant_taints` are the warnings already
/// attached to the path.
fn generate_questions(
    event: &str,
    conditions: &[Condition],
    action: &Action,
    relevant_taints: &[Value],
) -> Vec<Value> {
    let cmd = action.command.as_str();
    let cond_summary = build_condition_summary(conditions);
    let mut questions: Vec<Value> = Vec::new();
    // The primary action question, the fallback-branch question, and the taint
    // sanitisation question are independent; each contributes at most one entry.
    questions.extend(command_question(event, cmd, &action.args, &cond_summary));
    questions.extend(fallback_question(conditions));
    questions.extend(taint_question(relevant_taints, cmd));
    questions
}

/// A `{question, field, suggested}` object.
fn question(text: &str, field: &str, suggested: &Value) -> Value {
    json!({ "question": text, "field": field, "suggested": suggested })
}

/// The primary question for a path's terminal action, keyed by its command.
fn command_question(event: &str, cmd: &str, args: &[String], cond_summary: &str) -> Option<Value> {
    // "When <summary>, …" phrasing, falling back to "unconditionally".
    let when = |fallback: &str| -> String {
        if cond_summary.is_empty() {
            fallback.to_owned()
        } else {
            cond_summary.to_owned()
        }
    };
    match cmd {
        "pool" if !args.is_empty() => {
            let pool = &args[0];
            let text = if cond_summary.is_empty() {
                format!("Should all requests in {event} go to pool '{pool}'?")
            } else {
                format!("When {cond_summary}, should the request go to pool '{pool}'?")
            };
            Some(question(&text, "expected_pool", &json!(pool)))
        }
        "reject" => {
            let text = if cond_summary.is_empty() {
                format!("Should all connections in {event} be rejected?")
            } else {
                format!("When {cond_summary}, should the connection be rejected?")
            };
            Some(question(&text, "expected_reject", &json!("yes")))
        }
        "HTTP::redirect" => {
            let target = args.first().map_or("?", String::as_str);
            Some(question(
                &format!(
                    "When {}, what URL should the redirect go to?",
                    when("unconditionally")
                ),
                "expected_redirect",
                &json!(target),
            ))
        }
        "HTTP::respond" => {
            let status = args.first().map_or("200", String::as_str);
            Some(question(
                &format!(
                    "When {}, what HTTP status should be returned?",
                    when("unconditionally")
                ),
                "expected_status",
                &json!(status),
            ))
        }
        "node" if !args.is_empty() => {
            let node = &args[0];
            Some(question(
                &format!(
                    "When {}, should traffic go to node '{node}'?",
                    when("unconditionally")
                ),
                "expected_node",
                &json!(node),
            ))
        }
        _ => None,
    }
}

/// For the first `else` / `default` branch, ask about the fallback input.
fn fallback_question(conditions: &[Condition]) -> Option<Value> {
    conditions.iter().find_map(|c| match c {
        Condition::If {
            branch: Branch::Else,
            ..
        } => Some(question(
            "What input value should trigger the 'else' / fallback path?",
            "fallback_input",
            &Value::Null,
        )),
        Condition::Switch { subject, pattern } if pattern == "default" => Some(question(
            &format!("What input value should trigger the 'default' case of switch on {subject}?"),
            "default_input",
            &Value::Null,
        )),
        _ => None,
    })
}

/// For tainted paths, ask about sanitisation of the first relevant warning.
fn taint_question(relevant_taints: &[Value], cmd: &str) -> Option<Value> {
    let first = relevant_taints.first()?;
    let variable = first.get("variable").and_then(Value::as_str).unwrap_or("?");
    let sink = first
        .get("sink_command")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(cmd);
    Some(question(
        &format!(
            "The variable '{variable}' carries user input into '{sink}'. \
             Is sanitization needed here?"
        ),
        "needs_sanitization",
        &json!("yes"),
    ))
}

/// A human-readable summary of a path's conditions
/// (`_build_condition_summary`), joined with `" and "`.
fn build_condition_summary(conditions: &[Condition]) -> String {
    conditions
        .iter()
        .map(Condition::summary_part)
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Run taint analysis and distil each warning to the fields the annotation
/// logic reads (`_get_taint_warnings`), which reads the `taint_warnings` list
/// from the dataflow graph.
fn collect_taints(source: &str) -> Vec<Taint> {
    let registry = registry_for_dialect(IRULES_DIALECT);
    let graph = tcl_lsp_core::graphs::dataflow_graph(source, registry, IRULES_DIALECT);
    graph
        .get("taint_warnings")
        .and_then(Value::as_array)
        .map(|warnings| {
            warnings
                .iter()
                .map(|w| Taint {
                    code: w
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned(),
                    sink_command: w
                        .get("sink_command")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned(),
                    raw: w.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}
