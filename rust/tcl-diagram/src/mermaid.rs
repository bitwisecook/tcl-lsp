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

//! Deterministic Mermaid `flowchart` serialiser for [`crate::diagram_data`].
//!
//! Turns the IR-derived `{events, procedures}` flow tree into a Mermaid diagram
//! string — no LLM, no network — so the BIG-IP report (and the CLI / MCP) can
//! show an iRule's control flow offline.

use serde_json::Value;
use tcl_registry::CommandRegistry;

use crate::data::diagram_data_with_config;

const MAX_NODES: usize = 600;

/// An inbound edge: the source node id and the label to put on the arrow.
type Edge = (String, String);

struct Builder {
    out: String,
    n: usize,
    nodes: usize,
}

impl Builder {
    fn id(&mut self) -> String {
        let id = format!("n{}", self.n);
        self.n += 1;
        id
    }
    fn full(&self) -> bool {
        self.nodes >= MAX_NODES
    }
    /// Emit a node `id<open>"label"<close>` and connect every inbound edge.
    fn node(&mut self, incoming: &[Edge], open: &str, label: &str, close: &str) -> String {
        let id = self.id();
        self.nodes += 1;
        self.out.push_str("  ");
        self.out.push_str(&id);
        self.out.push_str(open);
        self.out.push('"');
        self.out.push_str(&esc(label));
        self.out.push('"');
        self.out.push_str(close);
        self.out.push('\n');
        for (from, elabel) in incoming {
            self.connect(from, &id, elabel);
        }
        id
    }
    fn connect(&mut self, from: &str, to: &str, label: &str) {
        self.out.push_str("  ");
        self.out.push_str(from);
        if label.is_empty() {
            self.out.push_str(" --> ");
        } else {
            self.out.push_str(" -->|\"");
            self.out.push_str(&esc(label));
            self.out.push_str("\"| ");
        }
        self.out.push_str(to);
        self.out.push('\n');
    }

    /// Render a sequence of flow nodes; return the fall-through edges.
    fn seq(&mut self, flow: &[Value], incoming: Vec<Edge>) -> Vec<Edge> {
        let mut edges = incoming;
        for node in flow {
            if self.full() {
                break;
            }
            edges = self.render(node, edges);
        }
        edges
    }

    fn render(&mut self, node: &Value, incoming: Vec<Edge>) -> Vec<Edge> {
        let kind = node.get("kind").and_then(Value::as_str).unwrap_or("");
        let label = node.get("label").and_then(Value::as_str).unwrap_or("");
        match kind {
            "action" => {
                let cmd = node.get("command").and_then(Value::as_str).unwrap_or("");
                let id = self.node(&incoming, "[", label, "]");
                if is_terminal(cmd) {
                    Vec::new() // control leaves the event
                } else {
                    vec![(id, String::new())]
                }
            }
            "assign" => {
                let var = node.get("var").and_then(Value::as_str).unwrap_or("");
                let val = node.get("value").and_then(Value::as_str).unwrap_or("");
                let id = self.node(&incoming, "[", &format!("set {var} = {val}"), "]");
                vec![(id, String::new())]
            }
            "return" => {
                let id = self.node(
                    &incoming,
                    "([",
                    if label.is_empty() { "return" } else { label },
                    "])",
                );
                let _ = id;
                Vec::new()
            }
            "proc_call" => {
                let id = self.node(&incoming, "[[", label, "]]");
                vec![(id, String::new())]
            }
            "truncated" => {
                let id = self.node(
                    &incoming,
                    "[",
                    if label.is_empty() { "…" } else { label },
                    "]",
                );
                vec![(id, String::new())]
            }
            "loop" => {
                let head = self.node(
                    &incoming,
                    "{{",
                    if label.is_empty() { "loop" } else { label },
                    "}}",
                );
                let body = node
                    .get("body")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let tails = self.seq(&body, vec![(head.clone(), "each".into())]);
                for (from, _) in &tails {
                    self.connect(from, &head, "repeat");
                }
                vec![(head, "done".into())]
            }
            "catch" => {
                let head = self.node(&incoming, "[", "catch", "]");
                let body = node
                    .get("body")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                self.seq(&body, vec![(head, String::new())])
            }
            "if" => self.render_if(node, incoming),
            "switch" => self.render_switch(node, &incoming),
            _ => {
                let text = if label.is_empty() { kind } else { label };
                let id = self.node(&incoming, "[", text, "]");
                vec![(id, String::new())]
            }
        }
    }

    fn render_if(&mut self, node: &Value, incoming: Vec<Edge>) -> Vec<Edge> {
        let branches = node
            .get("branches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut tails: Vec<Edge> = Vec::new();
        let mut pending = incoming; // the "no" flow entering the next test
        let mut saw_else = false;
        for br in &branches {
            if self.full() {
                break;
            }
            let cond = br.get("condition").and_then(Value::as_str).unwrap_or("");
            let body = br
                .get("body")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if cond.eq_ignore_ascii_case("else") {
                tails.extend(self.seq(&body, std::mem::take(&mut pending)));
                saw_else = true;
            } else {
                let d = self.node(&pending, "{", &format!("if {cond}"), "}");
                tails.extend(self.seq(&body, vec![(d.clone(), "yes".into())]));
                pending = vec![(d, "no".into())];
            }
        }
        if !saw_else {
            tails.extend(pending); // fall through from the last test's "no"
        }
        tails
    }

    fn render_switch(&mut self, node: &Value, incoming: &[Edge]) -> Vec<Edge> {
        let subject = node.get("subject").and_then(Value::as_str).unwrap_or("");
        let arms = node
            .get("arms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let d = self.node(incoming, "{", &format!("switch {subject}"), "}");
        let mut tails: Vec<Edge> = Vec::new();
        let mut has_default = false;
        for arm in &arms {
            if self.full() {
                break;
            }
            let pat = arm.get("pattern").and_then(Value::as_str).unwrap_or("");
            let body = arm
                .get("body")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if pat.eq_ignore_ascii_case("default") {
                has_default = true;
            }
            tails.extend(self.seq(&body, vec![(d.clone(), pat.to_string())]));
        }
        if !has_default {
            tails.push((d, "no match".into())); // fall through when nothing matches
        }
        tails
    }
}

/// Commands that end an event's control flow.
fn is_terminal(command: &str) -> bool {
    matches!(
        command.to_ascii_lowercase().as_str(),
        "return" | "drop" | "reject" | "discard"
    )
}

/// Escape a Mermaid label: quotes, and the operators / metacharacters Mermaid
/// mis-parses inside `"…"` labels.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.trim().chars() {
        match c {
            '"' => out.push('\''),
            '\n' | '\r' | '\t' => out.push(' '),
            _ => out.push(c),
        }
    }
    // Word forms for &&/||/! so Mermaid doesn't treat `&` as an HTML entity.
    let out = out.replace("&&", " and ").replace("||", " or ");
    let trimmed = out.trim();
    if trimmed.chars().count() > 60 {
        let cut: String = trimmed.chars().take(59).collect();
        format!("{cut}…")
    } else {
        trimmed.to_string()
    }
}

/// Build a Mermaid `flowchart` for an iRule source, or `""` when there is
/// nothing worth drawing (no events / procedures, or a parse failure).
#[must_use]
pub fn irule_flowchart_mermaid(source: &str, registry: &CommandRegistry) -> String {
    // Always the iRules dialect: lex with the f5-irules preset so `if {expr}{body}`
    // (`}{` valid in TMM) forms correct control-flow branches instead of the
    // stock-Tcl mis-segmentation.
    let data = diagram_data_with_config(
        source,
        registry,
        tcl_lexer::LexerConfig::for_dialect("f5-irules"),
    );
    let events = data
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let procs = data
        .get("procedures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if events.is_empty() && procs.is_empty() {
        return String::new();
    }

    let mut b = Builder {
        out: String::from("flowchart TD\n"),
        n: 0,
        nodes: 0,
    };
    for ev in &events {
        if b.full() {
            break;
        }
        let name = ev.get("name").and_then(Value::as_str).unwrap_or("event");
        let ev_id = b.node(&[], "([", name, "])");
        b.out.push_str("  class ");
        b.out.push_str(&ev_id);
        b.out.push_str(" evNode\n");
        let flow = ev
            .get("flow")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let _ = b.seq(&flow, vec![(ev_id, String::new())]);
    }
    for pr in &procs {
        if b.full() {
            break;
        }
        let name = pr.get("name").and_then(Value::as_str).unwrap_or("proc");
        let pr_id = b.node(&[], "[[", &format!("proc {name}"), "]]");
        let body = pr
            .get("body")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let _ = b.seq(&body, vec![(pr_id, String::new())]);
    }
    b.out
        .push_str("  classDef evNode fill:#dbeafe,stroke:#2563eb,color:#0b3a86;\n");
    b.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flowchart_handles_irule_brace_chain() {
        // `if {expr}{body}` — `}{` valid in TMM. `irule_flowchart_mermaid` must
        // lex with the f5-irules preset, so both branches (and their `pool`
        // actions) appear. Under stock-Tcl lexing the `}{` derails the parse and
        // the flowchart comes out empty.
        let src = "when HTTP_REQUEST {\n  if { [HTTP::uri] eq \"/a\" }{\n    pool a_pool\n  } else {\n    pool b_pool\n  }\n}";
        let registry = tcl_registry::registry_for_dialect("f5-irules");
        let out = irule_flowchart_mermaid(src, registry);
        assert!(out.contains("a_pool"), "missing then-branch pool:\n{out}");
        assert!(out.contains("b_pool"), "missing else-branch pool:\n{out}");
    }
}
