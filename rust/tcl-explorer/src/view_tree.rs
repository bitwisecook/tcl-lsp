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

//! Data-driven view models for the explorer TUI.
//!
//! Every analysis view is
//! an interactive tree of [`ViewNode`]s built from the *same* serialised
//! result the GUI consumes ([`crate::serialise::serialise_result`]). Each
//! node has a one-line `label`, a `detail` table revealed when highlighted,
//! and optional `children`. Keeping these builders next to the one
//! serialisation means the GUI and TUI show the same fields.
//!
//! `asm` / `wasm` deliberately stay on the text renderer — they
//! are specialised disassembly, navigated rather than expanded.

use serde_json::Value;

/// One expandable row: a summary `label`, a `detail` table, and children.
///
/// `Serialize` is for the differential test harness (compared against
/// `ViewNode`), not part of the explorer JSON contract.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct ViewNode {
    /// One-line summary shown in the tree.
    pub label: String,
    /// Key/value rows shown when the node is highlighted.
    pub detail: Vec<(String, String)>,
    /// Nested rows.
    pub children: Vec<ViewNode>,
    /// Optional style hint (a colour name).
    pub style: Option<String>,
}

impl ViewNode {
    fn leaf(label: impl Into<String>, detail: Vec<(String, String)>, style: Option<&str>) -> Self {
        Self {
            label: label.into(),
            detail,
            children: Vec::new(),
            style: style.map(str::to_owned),
        }
    }

    fn branch(
        label: impl Into<String>,
        detail: Vec<(String, String)>,
        children: Vec<ViewNode>,
        style: Option<&str>,
    ) -> Self {
        Self {
            label: label.into(),
            detail,
            children,
            style: style.map(str::to_owned),
        }
    }

    fn note(label: impl Into<String>, style: &str) -> Self {
        Self::leaf(label, Vec::new(), Some(style))
    }
}

// Scalar/JSON accessors.

fn s(v: &Value, key: &str) -> String {
    v.get(key).map(jstr).unwrap_or_default()
}

fn jstr(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Render a JSON scalar as a display string for label interpolation.
///
/// The only divergence from [`jstr`] is booleans: they render as
/// `True`/`False` (capitalised) where Rust's `bool` `Display` emits
/// `true`/`false`. The explorer's SSA-view const-branch label interpolates a
/// JSON bool, so it is capitalised here to stay byte-identical.
fn pystr(v: &Value) -> String {
    match v {
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        other => jstr(other),
    }
}

fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn det(k: &str, v: impl Into<String>) -> (String, String) {
    (k.to_owned(), v.into())
}

fn yn(v: &Value) -> &'static str {
    if v.as_bool().unwrap_or(false) {
        "yes"
    } else {
        "no"
    }
}

/// An interval endpoint: `null` is ±infinity.
fn fmt_bound(v: &Value, positive: bool) -> String {
    if v.is_null() {
        if positive { "+inf" } else { "-inf" }.to_owned()
    } else {
        jstr(v)
    }
}

/// A range dict → `line:col  (start…end)`.
fn rng(r: &Value) -> String {
    if !r.is_object() {
        return "?".to_owned();
    }
    let line = r["startLine"].as_i64().unwrap_or(0) + 1;
    let col = r["startCol"].as_i64().unwrap_or(0) + 1;
    let so = r["startOffset"].as_i64().unwrap_or(0);
    let eo = r["endOffset"].as_i64().unwrap_or(0);
    format!("{line}:{col}  ({so}…{eo})")
}

/// Proc summary flags.
fn flags(p: &Value) -> String {
    let mut f = Vec::new();
    if p["hasBarrier"].as_bool().unwrap_or(false) {
        f.push("barrier");
    }
    if p["hasUnknownCalls"].as_bool().unwrap_or(false) {
        f.push("unknown_calls");
    }
    if p["writesGlobal"].as_bool().unwrap_or(false) {
        f.push("writes_global");
    }
    if f.is_empty() {
        "—".to_owned()
    } else {
        f.join(", ")
    }
}

/// `{name#ver=lattice:type, …}` for a uses/defs map.
fn fmt_usedef(m: &Value) -> String {
    let Some(obj) = m.as_object() else {
        return "{}".to_owned();
    };
    let mut names: Vec<&String> = obj.keys().collect();
    names.sort();
    let parts: Vec<String> = names
        .iter()
        .map(|name| {
            let info = &obj[*name];
            if info.is_object() {
                let mut piece = format!("{name}#{}", jstr(&info["version"]));
                let lat = s(info, "lattice");
                if !lat.is_empty() {
                    piece.push('=');
                    piece.push_str(&lat);
                }
                let ty = s(info, "type");
                if !ty.is_empty() {
                    piece.push(':');
                    piece.push_str(&ty);
                }
                piece
            } else {
                format!("{name}#{}", jstr(info))
            }
        })
        .collect();
    format!("{{{}}}", parts.join(", "))
}

fn join_str_array(v: &Value) -> String {
    arr_to_strings(v).join(", ")
}

fn arr_to_strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().map(jstr).collect())
        .unwrap_or_default()
}

// --- IR ---

fn ir_nodes(nodes: &[Value]) -> Vec<ViewNode> {
    nodes
        .iter()
        .map(|n| {
            let detail = vec![
                det("kind", s(n, "kind")),
                det("summary", s(n, "summary")),
                det("range", rng(&n["range"])),
            ];
            let children = arr(n, "children")
                .iter()
                .map(|c| {
                    ViewNode::branch(
                        format!("{}:", s(c, "label")),
                        Vec::new(),
                        ir_nodes(arr(c, "body")),
                        Some("blue"),
                    )
                })
                .collect();
            ViewNode::branch(s(n, "summary"), detail, children, None)
        })
        .collect()
}

fn build_ir(d: &Value) -> Vec<ViewNode> {
    let ir = &d["ir"];
    let mut nodes = vec![ViewNode::branch(
        "top-level",
        Vec::new(),
        ir_nodes(arr(ir, "topLevel")),
        Some("cyan"),
    )];
    if let Some(procs) = ir["procedures"].as_object() {
        let mut names: Vec<&String> = procs.keys().collect();
        names.sort();
        for name in names {
            let proc = &procs[name];
            let params = arr_to_strings(&proc["params"]).join(" ");
            let header = format!("{name} {{{params}}}");
            nodes.push(ViewNode::branch(
                header,
                vec![det("range", rng(&proc["range"]))],
                ir_nodes(arr(proc, "body")),
                Some("cyan"),
            ));
        }
    }
    nodes
}

// --- CFG (pre-/post-SSA) ---

fn term_label(t: &Value) -> String {
    if !t.is_object() {
        return "term <none>".to_owned();
    }
    match s(t, "type").as_str() {
        "goto" => format!("term goto {}", s(t, "target")),
        "branch" => format!(
            "term branch {} → {}/{}",
            s(t, "condition"),
            s(t, "trueTarget"),
            s(t, "falseTarget")
        ),
        "return" => {
            let v = s(t, "value");
            if v.is_empty() {
                "term return".to_owned()
            } else {
                format!("term return {v}")
            }
        }
        _ => "term".to_owned(),
    }
}

// One flat walk building the nested CFG view (functions -> blocks -> items);
// the per-level nesting is clearer inline than threaded through helpers.
#[allow(clippy::too_many_lines)]
fn build_cfg(funcs: &[Value], post: bool) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for f in funcs {
        let mut blocks = Vec::new();
        for b in arr(f, "blocks") {
            let mut items: Vec<ViewNode> = Vec::new();
            if post {
                for p in arr(b, "phis") {
                    let inc = p["incoming"]
                        .as_object()
                        .map(|o| {
                            o.iter()
                                .map(|(k, v)| format!("{k}:{}", jstr(v)))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    items.push(ViewNode::leaf(
                        format!("phi {}#{} ← {inc}", s(p, "name"), s(p, "version")),
                        vec![
                            det("type", {
                                let t = s(p, "type");
                                if t.is_empty() { "?".to_owned() } else { t }
                            }),
                            det("incoming", inc),
                        ],
                        Some("magenta"),
                    ));
                }
            }
            for st in arr(b, "statements") {
                let mut detail = vec![
                    det("summary", s(st, "summary")),
                    det("range", rng(&st["range"])),
                ];
                if post {
                    detail.push(det("uses", fmt_usedef(&st["uses"])));
                    detail.push(det("defs", fmt_usedef(&st["defs"])));
                }
                items.push(ViewNode::leaf(s(st, "summary"), detail, None));
            }
            let term = &b["terminator"];
            items.push(ViewNode::leaf(
                term_label(term),
                vec![det("range", rng(&term["range"]))],
                Some("blue"),
            ));
            let mut tags = String::new();
            if b["isEntry"].as_bool().unwrap_or(false) {
                tags.push_str(" [entry]");
            }
            if b["isUnreachable"].as_bool().unwrap_or(false) {
                tags.push_str(" [unreachable]");
            }
            blocks.push(ViewNode::branch(
                format!("block {}{tags}", s(b, "name")),
                Vec::new(),
                items,
                Some("bold"),
            ));
        }
        if post && f["analysis"].is_object() {
            let a = &f["analysis"];
            let mut asub = Vec::new();
            for br in arr(a, "constantBranches") {
                asub.push(ViewNode::leaf(
                    format!(
                        "const branch {}: always {}",
                        s(br, "block"),
                        pystr(&br["value"])
                    ),
                    vec![
                        det("condition", s(br, "condition")),
                        det("take", s(br, "takenTarget")),
                    ],
                    Some("blue"),
                ));
            }
            for ds in arr(a, "deadStores") {
                asub.push(ViewNode::leaf(
                    format!(
                        "dead store {} {}#{}",
                        s(ds, "block"),
                        s(ds, "variable"),
                        s(ds, "version")
                    ),
                    vec![
                        det("block", s(ds, "block")),
                        det("stmt index", s(ds, "stmtIndex")),
                    ],
                    Some("yellow"),
                ));
            }
            let unreachable = arr(a, "unreachableBlocks");
            if !unreachable.is_empty() {
                let joined = join_str_array(&a["unreachableBlocks"]);
                asub.push(ViewNode::leaf(
                    format!("unreachable: {joined}"),
                    vec![det("blocks", joined)],
                    Some("magenta"),
                ));
            }
            if let Some(types) = a["inferredTypes"].as_object() {
                for (name, ty) in types {
                    asub.push(ViewNode::leaf(
                        format!("{name}: {}", jstr(ty)),
                        vec![det("value", name.clone()), det("type", jstr(ty))],
                        Some("green"),
                    ));
                }
            }
            if !asub.is_empty() {
                blocks.push(ViewNode::branch("analysis", Vec::new(), asub, Some("bold")));
            }
        }
        out.push(ViewNode::branch(
            format!(
                "function {} (entry={}, {} blocks)",
                s(f, "name"),
                s(f, "entry"),
                s(f, "blockCount")
            ),
            Vec::new(),
            blocks,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no functions)", "dim")]
    } else {
        out
    }
}

// --- Per-function tables ---

/// Build per-function nodes from a list, skipping functions with no
/// entries, with a fallback note. Shared by loops/types/intervals/rendered.
fn per_function<F>(
    funcs: &[Value],
    entries_key: &str,
    empty_note: &str,
    mut row: F,
) -> Vec<ViewNode>
where
    F: FnMut(&Value) -> ViewNode,
{
    let mut out = Vec::new();
    for f in funcs {
        let entries = arr(f, entries_key);
        if entries.is_empty() {
            continue;
        }
        let rows: Vec<ViewNode> = entries.iter().map(&mut row).collect();
        out.push(ViewNode::branch(
            format!("function {}", s(f, "name")),
            Vec::new(),
            rows,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note(empty_note, "dim")]
    } else {
        out
    }
}

fn build_loops(d: &Value) -> Vec<ViewNode> {
    per_function(arr(d, "loops"), "loops", "(no natural loops)", |lp| {
        ViewNode::leaf(
            format!(
                "header {} (depth {}, {} blocks)",
                s(lp, "header"),
                s(lp, "depth"),
                s(lp, "blockCount")
            ),
            vec![
                det("header block", s(lp, "header")),
                det("nesting depth", s(lp, "depth")),
                det("blocks", join_str_array(&lp["blocks"])),
                det("latches", {
                    let l = join_str_array(&lp["latches"]);
                    if l.is_empty() { "—".to_owned() } else { l }
                }),
            ],
            Some("blue"),
        )
    })
}

fn build_types(d: &Value) -> Vec<ViewNode> {
    per_function(arr(d, "types"), "entries", "(no inferred types)", |e| {
        let var = s(e, "variable");
        let label = if var.starts_with('(') {
            format!("{var}: {}", s(e, "type"))
        } else {
            format!("{var}#{}: {}", s(e, "version"), s(e, "type"))
        };
        ViewNode::leaf(
            label,
            vec![
                det("variable", var),
                det("version", s(e, "version")),
                det("kind", s(e, "kind")),
                det("type", s(e, "type")),
            ],
            None,
        )
    })
}

fn build_intervals(d: &Value) -> Vec<ViewNode> {
    per_function(arr(d, "intervals"), "entries", "(no bounded ranges)", |e| {
        let lo = fmt_bound(&e["lo"], false);
        let hi = fmt_bound(&e["hi"], true);
        ViewNode::leaf(
            format!("{}#{}: [{lo}, {hi}]", s(e, "variable"), s(e, "version")),
            vec![
                det("variable", s(e, "variable")),
                det("version", s(e, "version")),
                det("lower bound", lo),
                det("upper bound", hi),
            ],
            None,
        )
    })
}

fn build_bounds(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for f in arr(d, "bounds") {
        let findings = arr(f, "findings");
        let divzero = arr(f, "divzero");
        if findings.is_empty() && divzero.is_empty() {
            continue;
        }
        let mut items = Vec::new();
        for b in findings {
            let lo = fmt_bound(&b["lo"], false);
            let hi = fmt_bound(&b["hi"], true);
            items.push(ViewNode::leaf(
                format!(
                    "{} {} ${} in [{lo}, {hi}] vs length {}",
                    s(b, "code"),
                    s(b, "command"),
                    s(b, "indexVar"),
                    s(b, "length")
                ),
                vec![
                    det("code", s(b, "code")),
                    det("command", s(b, "command")),
                    det("index var", format!("${}", s(b, "indexVar"))),
                    det("index range", format!("[{lo}, {hi}]")),
                    det("container length", s(b, "length")),
                    det("reason", s(b, "reason")),
                ],
                Some("yellow"),
            ));
        }
        for dz in divzero {
            items.push(ViewNode::leaf(
                format!("{} '{}' divisor provably 0", s(dz, "code"), s(dz, "op")),
                vec![
                    det("code", s(dz, "code")),
                    det("operator", s(dz, "op")),
                    det("reason", "divisor interval is exactly [0, 0]"),
                ],
                Some("yellow"),
            ));
        }
        out.push(ViewNode::branch(
            format!("function {}", s(f, "name")),
            Vec::new(),
            items,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note(
            "(no provable out-of-range / divide-by-zero)",
            "dim",
        )]
    } else {
        out
    }
}

fn build_rendered(d: &Value) -> Vec<ViewNode> {
    per_function(
        arr(d, "renderedProperties"),
        "entries",
        "(no rendered-property flags)",
        |e| {
            ViewNode::leaf(
                format!("{}#{}", s(e, "variable"), s(e, "version")),
                vec![
                    det("variable", s(e, "variable")),
                    det("version", s(e, "version")),
                    det("may", {
                        let m = join_str_array(&e["may"]);
                        if m.is_empty() { "—".to_owned() } else { m }
                    }),
                    det("must", {
                        let m = join_str_array(&e["must"]);
                        if m.is_empty() { "—".to_owned() } else { m }
                    }),
                ],
                None,
            )
        },
    )
}

// --- Data flow / interprocedural ---

fn build_dataflow(d: &Value) -> Vec<ViewNode> {
    let df = &d["dataflow"];
    if !df.is_object() || arr(df, "functions").is_empty() {
        return vec![ViewNode::note("(no data-flow information)", "dim")];
    }
    let mut out = Vec::new();
    for f in arr(df, "functions") {
        let mut nodes = Vec::new();
        for a in arr(f, "aliases") {
            nodes.push(ViewNode::leaf(
                format!("alias {} ↔ {}", s(a, "localName"), s(a, "targetName")),
                vec![
                    det("local", s(a, "localName")),
                    det("target", s(a, "targetName")),
                    det("reason", s(a, "reason")),
                ],
                Some("orange"),
            ));
        }
        for n in arr(f, "nodes") {
            let dead = if n["isDead"].as_bool().unwrap_or(false) {
                " (DEAD)"
            } else {
                ""
            };
            let label = format!(
                "{}#{} [{} in {}]{dead}",
                s(n, "name"),
                s(n, "version"),
                s(n, "defKind"),
                s(n, "block")
            );
            let mut detail = vec![
                det("ssa value", format!("{}#{}", s(n, "name"), s(n, "version"))),
                det("def kind", s(n, "defKind")),
                det("block", s(n, "block")),
                det("dead", yn(&n["isDead"])),
            ];
            if !n["useCount"].is_null() {
                detail.push(det("uses", s(n, "useCount")));
            }
            let lat = s(n, "lattice");
            if !lat.is_empty() {
                detail.push(det("lattice", lat));
            }
            let ti = s(n, "typeInfo");
            if !ti.is_empty() {
                detail.push(det("type", ti));
            }
            nodes.push(ViewNode::leaf(
                label,
                detail,
                Some(if n["isDead"].as_bool().unwrap_or(false) {
                    "yellow"
                } else {
                    "green"
                }),
            ));
        }
        let sm = &f["summary"];
        out.push(ViewNode::branch(
            format!(
                "function {} ({} defs, {} uses)",
                s(f, "name"),
                sm["totalDefs"].as_i64().unwrap_or(0),
                sm["totalUses"].as_i64().unwrap_or(0)
            ),
            Vec::new(),
            nodes,
            Some("cyan"),
        ));
    }
    out
}

fn build_interproc(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for p in arr(d, "interprocedural") {
        if s(p, "kind") == "method" {
            let mut detail = vec![
                det("method kind", {
                    let m = s(p, "methodKind");
                    if m.is_empty() { "method".to_owned() } else { m }
                }),
                det("pure", yn(&p["pure"])),
                det("calls", {
                    let c = join_str_array(&p["calls"]);
                    if c.is_empty() { "—".to_owned() } else { c }
                }),
            ];
            let wiv = arr(p, "writesInstanceVars");
            if !wiv.is_empty() {
                detail.push(det(
                    "writes instance vars",
                    join_str_array(&p["writesInstanceVars"]),
                ));
            }
            detail.push(det("flags", flags(p)));
            out.push(ViewNode::leaf(
                format!("{} (method)", s(p, "name")),
                detail,
                Some("cyan"),
            ));
        } else {
            let mut detail = vec![
                det("arity", s(p, "arity")),
                det("pure", yn(&p["pure"])),
                det("foldable", yn(&p["foldable"])),
                det("return shape", s(p, "returnShape")),
                det("calls", {
                    let c = join_str_array(&p["calls"]);
                    if c.is_empty() { "—".to_owned() } else { c }
                }),
            ];
            // The caller-uniform-literal seed SCCP ran this procedure under.
            // Shown only when there is one, so the common unseeded procedure
            // reads no differently than before.
            let seeds = join_str_array(&p["paramConstants"]);
            if !seeds.is_empty() {
                detail.push(det("param constants", seeds));
            }
            detail.push(det("flags", flags(p)));
            out.push(ViewNode::leaf(
                format!("{} arity={}", s(p, "name"), s(p, "arity")),
                detail,
                Some("cyan"),
            ));
        }
    }
    if out.is_empty() {
        vec![ViewNode::note("(no procedures)", "dim")]
    } else {
        out
    }
}

/// Why the interprocedural constant seed fired (or declined) for each
/// procedure: the registry-declared unit boundaries the file crosses, whether
/// the host supplied a cross-file view, and the merged call-site evidence
/// per callee (issue #977).
fn build_unit_scope(d: &Value) -> Vec<ViewNode> {
    let scope = &d["unitScope"];
    let boundaries = join_str_array(&scope["boundaries"]);
    let mut out = vec![ViewNode::leaf(
        "unit".to_owned(),
        vec![
            det(
                "registry boundaries",
                if boundaries.is_empty() {
                    "none".to_owned()
                } else {
                    boundaries
                },
            ),
            det("cross-file view", yn(&scope["hasCrossFileEvidence"])),
            det("seeding", s(scope, "seeding")),
        ],
        Some("cyan"),
    )];
    for callee in arr(scope, "callees") {
        let detail = arr(callee, "positions")
            .iter()
            .map(|p| det("arg", format!("{}: {}", s(p, "index"), s(p, "verdict"))))
            .chain(std::iter::once(det(
                "argument counts",
                join_str_array(&callee["argCounts"]),
            )))
            .collect();
        out.push(ViewNode::leaf(s(callee, "name"), detail, Some("green")));
    }
    if out.len() == 1 {
        out.push(ViewNode::note("(no resolvable call sites)", "dim"));
    }
    out
}

// --- Optimiser / GVN / shimmer / taint / iRules / callouts ---

fn build_opt(d: &Value) -> Vec<ViewNode> {
    let out: Vec<ViewNode> = arr(d, "optimisations")
        .iter()
        .map(|o| {
            ViewNode::leaf(
                format!(
                    "{} {} → {}",
                    s(o, "code"),
                    s(o, "message"),
                    s(o, "replacement")
                ),
                vec![
                    det("code", s(o, "code")),
                    det("message", s(o, "message")),
                    det("replacement", s(o, "replacement")),
                    det("range", rng(&o["range"])),
                ],
                Some("green"),
            )
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no optimiser rewrites)", "dim")]
    } else {
        out
    }
}

fn build_structural_index(d: &Value) -> Vec<ViewNode> {
    let si = &d["structuralIndex"];
    if !si.is_object() {
        return vec![ViewNode::note("(structural index unavailable)", "dim")];
    }
    let mut roots = Vec::new();
    let complete = si["scriptComplete"].as_bool().unwrap_or(false);
    roots.push(ViewNode::leaf(
        format!("script complete: {complete}"),
        Vec::new(),
        Some(if complete { "green" } else { "yellow" }),
    ));

    let boundaries = arr(si, "commandBoundaries");
    let cb_children: Vec<ViewNode> = boundaries
        .iter()
        .map(|b| {
            ViewNode::leaf(
                format!(
                    "offset {} ({}:{})",
                    s(b, "offset"),
                    b["line"].as_i64().unwrap_or(0) + 1,
                    b["col"].as_i64().unwrap_or(0) + 1
                ),
                Vec::new(),
                None,
            )
        })
        .collect();
    roots.push(ViewNode::branch(
        format!("command boundaries ({})", boundaries.len()),
        Vec::new(),
        cb_children,
        Some("cyan"),
    ));

    for (key, label) in [("brackets", "brackets [ ]"), ("braces", "braces { }")] {
        let group = &si[key];
        let inert = arr(group, "inertSpans");
        let children: Vec<ViewNode> = inert
            .iter()
            .map(|span| {
                let terminated = span["terminated"].as_bool().unwrap_or(false);
                ViewNode::leaf(
                    format!(
                        "inert {}…{} {}",
                        s(span, "start"),
                        s(span, "end"),
                        if terminated {
                            "(terminated)"
                        } else {
                            "(unterminated)"
                        }
                    ),
                    vec![det("text", s(span, "text"))],
                    Some(if terminated { "dim" } else { "red" }),
                )
            })
            .collect();
        roots.push(ViewNode::branch(
            format!(
                "{label}: {} unterminated, {} inert span(s)",
                s(group, "unterminated"),
                inert.len()
            ),
            vec![det("structural events", s(group, "structuralEvents"))],
            children,
            None,
        ));
    }
    roots
}

fn build_source_map(d: &Value) -> Vec<ViewNode> {
    let sm = &d["sourceMap"];
    if !sm.is_object() {
        return vec![ViewNode::note("(source map unavailable)", "dim")];
    }
    let mut roots = vec![ViewNode::leaf(
        format!(
            "{} bytes, {} line(s)",
            s(sm, "byteLength"),
            s(sm, "lineCount")
        ),
        Vec::new(),
        Some("cyan"),
    )];
    for line in arr(sm, "lines") {
        let n = line["line"].as_i64().unwrap_or(0) + 1;
        roots.push(ViewNode::leaf(
            format!("{n}: {}", s(line, "text")),
            vec![
                det(
                    "byte range",
                    format!(
                        "{}…{} ({} bytes)",
                        s(line, "start"),
                        s(line, "end"),
                        s(line, "length")
                    ),
                ),
                det("line", n.to_string()),
            ],
            None,
        ));
    }
    roots
}

fn build_optimiser_passes(d: &Value) -> Vec<ViewNode> {
    let out: Vec<ViewNode> = arr(d, "optimiserPasses")
        .iter()
        .map(|p| {
            let opts: Vec<ViewNode> = arr(p, "optimisations")
                .iter()
                .map(|o| {
                    ViewNode::leaf(
                        format!(
                            "{} {} → {}",
                            s(o, "code"),
                            s(o, "message"),
                            s(o, "replacement")
                        ),
                        vec![
                            det("code", s(o, "code")),
                            det("message", s(o, "message")),
                            det("replacement", s(o, "replacement")),
                            det("range", rng(&o["range"])),
                        ],
                        Some("green"),
                    )
                })
                .collect();
            let count = s(p, "count");
            ViewNode::branch(
                format!("{} ({count})", s(p, "label")),
                vec![det("pass", s(p, "id")), det("rewrites", count)],
                opts,
                Some(if arr(p, "optimisations").is_empty() {
                    "dim"
                } else {
                    "green"
                }),
            )
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(optimiser pipeline unavailable)", "dim")]
    } else {
        out
    }
}

fn build_gvn(d: &Value) -> Vec<ViewNode> {
    let out: Vec<ViewNode> = arr(d, "gvn")
        .iter()
        .map(|w| {
            ViewNode::leaf(
                format!("{} {}", s(w, "code"), s(w, "expression")),
                vec![
                    det("code", s(w, "code")),
                    det("message", s(w, "message")),
                    det("expression", s(w, "expression")),
                    det("first seen", rng(&w["firstRange"])),
                    det("range", rng(&w["range"])),
                ],
                Some("green"),
            )
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no redundant computations)", "dim")]
    } else {
        out
    }
}

fn build_shimmer(d: &Value) -> Vec<ViewNode> {
    let out: Vec<ViewNode> = arr(d, "shimmer")
        .iter()
        .map(|w| {
            ViewNode::leaf(
                format!("{} {}", s(w, "code"), s(w, "message")),
                vec![
                    det("code", s(w, "code")),
                    det("severity", {
                        let sv = s(w, "severity");
                        if sv.is_empty() {
                            "warning".to_owned()
                        } else {
                            sv
                        }
                    }),
                    det("message", s(w, "message")),
                    det("range", rng(&w["range"])),
                ],
                Some("yellow"),
            )
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no shimmer warnings)", "dim")]
    } else {
        out
    }
}

fn build_taint(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for w in arr(d, "taintWarnings") {
        let mut detail = vec![
            det("code", s(w, "code")),
            det("severity", {
                let sv = s(w, "severity");
                if sv.is_empty() {
                    "warning".to_owned()
                } else {
                    sv
                }
            }),
            det("message", s(w, "message")),
        ];
        let var = s(w, "variable");
        if !var.is_empty() {
            detail.push(det("variable", var));
        }
        let sink = s(w, "sinkCommand");
        if !sink.is_empty() {
            detail.push(det("sink command", sink));
        }
        detail.push(det("range", rng(&w["range"])));
        out.push(ViewNode::leaf(
            format!("{} {}", s(w, "code"), s(w, "message")),
            detail,
            Some("red"),
        ));
    }
    for f in arr(d, "taintTracking") {
        let ents: Vec<ViewNode> = arr(f, "entries")
            .iter()
            .map(|e| {
                ViewNode::leaf(
                    format!(
                        "{}#{}: {}",
                        s(e, "variable"),
                        s(e, "version"),
                        s(e, "taint")
                    ),
                    vec![
                        det("variable", s(e, "variable")),
                        det("version", s(e, "version")),
                        det("taint", s(e, "taint")),
                    ],
                    None,
                )
            })
            .collect();
        out.push(ViewNode::branch(
            format!("tracking {}", s(f, "name")),
            Vec::new(),
            ents,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no tainted data flows)", "dim")]
    } else {
        out
    }
}

fn build_irules(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for e in arr(d, "eventOrder") {
        let eff =
            e["base_priority"].as_i64().unwrap_or(0) + e["priority_offset"].as_i64().unwrap_or(0);
        out.push(ViewNode::leaf(
            format!("{} eff={eff}", s(e, "event")),
            vec![
                det("event", s(e, "event")),
                det("effective priority", eff.to_string()),
                det("base", s(e, "base_priority")),
                det("offset", s(e, "priority_offset")),
                det("multiplicity", {
                    let m = s(e, "multiplicity");
                    if m.is_empty() { "once".to_owned() } else { m }
                }),
                det("range", rng(&e["range"])),
            ],
            Some("blue"),
        ));
    }
    for w in arr(d, "irulesFlow") {
        out.push(ViewNode::leaf(
            format!("{} {}", s(w, "code"), s(w, "message")),
            vec![
                det("code", s(w, "code")),
                det("message", s(w, "message")),
                det("range", rng(&w["range"])),
            ],
            Some("yellow"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no iRules events)", "dim")]
    } else {
        out
    }
}

fn build_event_order(d: &Value) -> Vec<ViewNode> {
    let mut events: Vec<&Value> = arr(d, "eventOrder").iter().collect();
    events.sort_by(|a, b| {
        let ea =
            a["base_priority"].as_i64().unwrap_or(0) + a["priority_offset"].as_i64().unwrap_or(0);
        let eb =
            b["base_priority"].as_i64().unwrap_or(0) + b["priority_offset"].as_i64().unwrap_or(0);
        eb.cmp(&ea).then_with(|| s(a, "event").cmp(&s(b, "event")))
    });
    let out: Vec<ViewNode> = events
        .iter()
        .map(|e| {
            let eff = e["base_priority"].as_i64().unwrap_or(0)
                + e["priority_offset"].as_i64().unwrap_or(0);
            let m = s(e, "multiplicity");
            let mult = if !m.is_empty() && m != "once" {
                format!(" [{m}]")
            } else {
                String::new()
            };
            ViewNode::leaf(
                format!("{} eff={eff}{mult}", s(e, "event")),
                vec![
                    det("event", s(e, "event")),
                    det("effective priority", eff.to_string()),
                    det("base priority", s(e, "base_priority")),
                    det("priority offset", s(e, "priority_offset")),
                    det(
                        "multiplicity",
                        if m.is_empty() { "once".to_owned() } else { m },
                    ),
                    det("range", rng(&e["range"])),
                ],
                Some("blue"),
            )
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no event-order data)", "dim")]
    } else {
        out
    }
}

fn build_callouts(d: &Value) -> Vec<ViewNode> {
    let out: Vec<ViewNode> = arr(d, "annotations")
        .iter()
        .map(|a| {
            ViewNode::leaf(
                format!("{}: {}", s(a, "kind"), s(a, "label")),
                vec![
                    det("kind", s(a, "kind")),
                    det("severity", s(a, "severity")),
                    det("label", s(a, "label")),
                    det("range", rng(&a["range"])),
                ],
                None,
            )
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no salient annotations)", "dim")]
    } else {
        out
    }
}

// --- World SSA ---

fn world_location_label(location: &Value) -> String {
    let domain = s(location, "domain");
    match s(location, "kind").as_str() {
        "scoped" => format!("{domain}: {}", jstr(&location["subject"])),
        "any" => "all mutable world state".to_owned(),
        other => format!("{domain} ({other})"),
    }
}

fn world_site_label(site: &Value) -> String {
    match s(site, "kind").as_str() {
        "cfg" => format!("block {} {}", s(site, "block"), jstr(&site["position"])),
        "edge" => format!(
            "edge {} → {} ({})",
            s(site, "predecessor"),
            s(site, "successor"),
            jstr(&site["origin"])
        ),
        "node" => format!("node {}", jstr(&site["path"])),
        other => other.to_owned(),
    }
}

#[allow(clippy::too_many_lines)] // One tree is intentionally assembled in view order.
fn build_world_ssa(d: &Value) -> Vec<ViewNode> {
    arr(d, "worldSsa")
        .iter()
        .map(|function| {
            let availability = &function["availability"];
            let mut children = Vec::new();
            let locations: Vec<ViewNode> = arr(function, "locations")
                .iter()
                .map(|location| {
                    ViewNode::leaf(
                        world_location_label(location),
                        vec![
                            det("kind", s(location, "kind")),
                            det("domain", s(location, "domain")),
                            det("interpreter", jstr(&location["interpreter"])),
                            det("namespace", jstr(&location["namespace"])),
                            det("subject", jstr(&location["subject"])),
                        ],
                        None,
                    )
                })
                .collect();
            children.push(ViewNode::branch(
                format!("locations ({})", locations.len()),
                Vec::new(),
                locations,
                Some("cyan"),
            ));

            let operations: Vec<ViewNode> = arr(function, "operations")
                .iter()
                .map(|operation| {
                    let mut detail = vec![
                        det("kind", s(operation, "kind")),
                        det("version", s(operation, "version")),
                        det("site", world_site_label(&operation["site"])),
                        det("location", jstr(&operation["location"])),
                    ];
                    if operation["kind"] == "use" {
                        detail.push(det("reaching version", s(operation, "reachingVersion")));
                    }
                    if operation["kind"] == "phi" {
                        detail.push(det("includes initial", yn(&operation["includesInitial"])));
                    }
                    let incoming = arr(operation, "incoming")
                        .iter()
                        .map(|incoming| {
                            ViewNode::leaf(
                                format!(
                                    "block {}: v{}",
                                    s(incoming, "block"),
                                    s(incoming, "version")
                                ),
                                Vec::new(),
                                Some("dim"),
                            )
                        })
                        .collect();
                    ViewNode::branch(
                        format!(
                            "{} {} v{} @ {}",
                            s(operation, "kind"),
                            world_location_label(&operation["location"]),
                            s(operation, "version"),
                            world_site_label(&operation["site"])
                        ),
                        detail,
                        incoming,
                        match s(operation, "kind").as_str() {
                            "clobber" => Some("yellow"),
                            "phi" => Some("magenta"),
                            _ => None,
                        },
                    )
                })
                .collect();
            children.push(ViewNode::branch(
                format!("operations ({})", operations.len()),
                Vec::new(),
                operations,
                None,
            ));

            let invocations: Vec<ViewNode> = arr(function, "invocations")
                .iter()
                .map(|invoke| {
                    let transitions: Vec<ViewNode> = arr(invoke, "transitions")
                        .iter()
                        .map(|transition| {
                            let intents = arr(transition, "intents")
                                .iter()
                                .map(|intent| {
                                    ViewNode::leaf(
                                        format!(
                                            "{} {}",
                                            s(intent, "kind"),
                                            world_location_label(&intent["location"])
                                        ),
                                        vec![
                                            det("commit", s(intent, "commit")),
                                            det("location", jstr(&intent["location"])),
                                        ],
                                        None,
                                    )
                                })
                                .collect();
                            ViewNode::branch(
                                format!("{} ({})", s(transition, "kind"), s(transition, "commit")),
                                vec![det("abrupt transfer", s(transition, "abruptTransfer"))],
                                intents,
                                Some("green"),
                            )
                        })
                        .collect();
                    let proof = &invoke["proof"];
                    ViewNode::branch(
                        format!("{} invocation", s(invoke, "resolution")),
                        vec![
                            det("command", s(invoke, "command")),
                            det("completion", s(invoke, "completion")),
                            det("transition knowledge", s(invoke, "transitionKnowledge")),
                            det("result stability", jstr(&proof["resultStability"])),
                            det(
                                "dispatch dependencies",
                                jstr(&proof["dispatchDependencies"]),
                            ),
                            det("abstention", s(proof, "abstention")),
                        ],
                        transitions,
                        if s(invoke, "resolution") == "unresolved" {
                            Some("yellow")
                        } else {
                            None
                        },
                    )
                })
                .collect();
            children.push(ViewNode::branch(
                format!("invocation proof ({})", invocations.len()),
                Vec::new(),
                invocations,
                Some("blue"),
            ));

            ViewNode::branch(
                format!(
                    "function {} ({})",
                    s(function, "name"),
                    s(availability, "kind")
                ),
                vec![
                    det("availability", s(availability, "kind")),
                    det("executable IR", yn(&availability["hasExecutableIr"])),
                    det("reason", s(availability, "reasonKind")),
                ],
                children,
                if s(availability, "kind") == "available" {
                    None
                } else {
                    Some("yellow")
                },
            )
        })
        .collect()
}

/// Build the [`ViewNode`] forest for `view` from serialised `data`.
/// An unknown view id yields an empty forest.
#[must_use]
pub fn build_view(view: &str, data: &Value) -> Vec<ViewNode> {
    match view {
        "ir" => build_ir(data),
        "cfg" => build_cfg(arr(data, "cfgPreSsa"), false),
        "ssa" => build_cfg(arr(data, "cfgPostSsa"), true),
        "worldSsa" => build_world_ssa(data),
        "loops" => build_loops(data),
        "types" => build_types(data),
        "intervals" => build_intervals(data),
        "bounds" => build_bounds(data),
        "dataflow" => build_dataflow(data),
        "interproc" => build_interproc(data),
        "unitScope" => build_unit_scope(data),
        "rendered" => build_rendered(data),
        "structuralIndex" => build_structural_index(data),
        "sourceMap" => build_source_map(data),
        "opt" => build_opt(data),
        "optimiserPasses" => build_optimiser_passes(data),
        "gvn" => build_gvn(data),
        "shimmer" => build_shimmer(data),
        "taint" => build_taint(data),
        "irules" => build_irules(data),
        "eventOrder" => build_event_order(data),
        "callouts" => build_callouts(data),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_pipeline, serialise_result};

    fn data(src: &str) -> Value {
        serialise_result(&run_pipeline(src, "tcl8.6"))
    }

    #[test]
    fn tree_views_are_a_subset_of_view_meta() {
        // Tree views are declared in the canonical descriptor table. A tree
        // id without a builder would otherwise silently render as no data.
        let meta_ids: std::collections::HashSet<&str> =
            crate::views::VIEW_META.iter().map(|view| view.id).collect();
        for id in crate::views::tree_view_ids() {
            assert!(
                meta_ids.contains(id),
                "tree-view id {id:?} is not a known view descriptor id",
            );
        }
    }

    #[test]
    fn ir_view_has_top_level_and_proc_nodes() {
        let d = data("proc f {x} { return $x }\nf 1");
        let nodes = build_view("ir", &d);
        assert_eq!(nodes[0].label, "top-level");
        assert!(nodes.iter().any(|n| n.label.starts_with("::f")));
    }

    #[test]
    fn cfg_view_lists_blocks_with_entry_tag() {
        let d = data("if {$x} { puts hi }");
        let nodes = build_view("cfg", &d);
        let func = &nodes[0];
        assert!(func.label.starts_with("function ::top"));
        assert!(func.children.iter().any(|b| b.label.contains("[entry]")));
    }

    #[test]
    fn world_ssa_view_explains_availability_and_transition_policy() {
        let d = data("interp create child\ninterp hide child puts\n");
        let nodes = build_view("worldSsa", &d);
        assert!(nodes[0].label.contains("function ::top"));
        assert!(
            nodes[0]
                .children
                .iter()
                .any(|node| node.label.starts_with("operations"))
        );
        assert!(
            nodes[0]
                .children
                .iter()
                .any(|node| node.label.starts_with("invocation proof"))
        );
    }

    #[test]
    fn loops_view_reports_a_loop() {
        let d = data("for {set i 0} {$i < 3} {incr i} { puts $i }");
        let nodes = build_view("loops", &d);
        // function ::top → one loop header node.
        assert!(
            nodes[0]
                .children
                .iter()
                .any(|n| n.label.starts_with("header"))
        );
    }

    #[test]
    fn unknown_view_is_empty() {
        assert!(build_view("does-not-exist", &data("set x 1")).is_empty());
    }

    #[test]
    fn empty_analysis_views_have_a_dim_note() {
        let d = data("set x 1");
        let taint = build_view("taint", &d);
        assert_eq!(taint.len(), 1);
        assert_eq!(taint[0].style.as_deref(), Some("dim"));
    }
}
