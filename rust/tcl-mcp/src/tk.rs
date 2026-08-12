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

//! `tk_layout` — extract a Tk widget tree from source for preview rendering.
//!
//! A line-based heuristic scan (not a full Tcl parse) that builds a JSON widget
//! hierarchy with types, geometry-manager assignments, visual options, and
//! pack/grid conflict notes.

use std::collections::HashMap;

use serde_json::{Map, Value, json};

const GEOMETRY_COMMANDS: &[&str] = &["grid", "pack", "place"];

const WIDGET_COMMANDS: &[&str] = &[
    "button",
    "canvas",
    "checkbutton",
    "destroy",
    "entry",
    "frame",
    "label",
    "labelframe",
    "listbox",
    "menu",
    "menubutton",
    "message",
    "panedwindow",
    "radiobutton",
    "scale",
    "scrollbar",
    "spinbox",
    "text",
    "toplevel",
    "ttk::button",
    "ttk::checkbutton",
    "ttk::combobox",
    "ttk::entry",
    "ttk::frame",
    "ttk::label",
    "ttk::labelframe",
    "ttk::menubutton",
    "ttk::notebook",
    "ttk::panedwindow",
    "ttk::progressbar",
    "ttk::radiobutton",
    "ttk::scale",
    "ttk::scrollbar",
    "ttk::separator",
    "ttk::sizegrip",
    "ttk::spinbox",
    "ttk::treeview",
];

const VISUAL_OPTIONS: &[&str] = &[
    "-text",
    "-textvariable",
    "-width",
    "-height",
    "-relief",
    "-label",
    "-orient",
    "-state",
    "-show",
    "-wrap",
    "-from",
    "-to",
    "-values",
    "-selectmode",
];

const GEOMETRY_OPTIONS: &[&str] = &[
    "-row",
    "-column",
    "-rowspan",
    "-columnspan",
    "-sticky",
    "-side",
    "-fill",
    "-expand",
    "-anchor",
    "-padx",
    "-pady",
    "-ipadx",
    "-ipady",
    "-x",
    "-y",
    "-relx",
    "-rely",
    "-relwidth",
    "-relheight",
    "-in",
    "-before",
    "-after",
    "-weight",
    "-minsize",
];

/// Geometry-manager subcommands that push the widget path to `words[2]`.
const GEOMETRY_SUBCOMMANDS: &[&str] = &[
    "configure",
    "forget",
    "info",
    "propagate",
    "slaves",
    "columnconfigure",
    "rowconfigure",
    "remove",
    "anchor",
];

struct Widget {
    pathname: String,
    wtype: String,
    options: Vec<(String, String)>,
    geometry: Option<String>,
    geometry_options: Vec<(String, String)>,
    children: Vec<String>,
    title: Option<String>,
}

impl Widget {
    fn new(pathname: &str, wtype: &str) -> Self {
        Self {
            pathname: pathname.to_owned(),
            wtype: wtype.to_owned(),
            options: Vec::new(),
            geometry: None,
            geometry_options: Vec::new(),
            children: Vec::new(),
            title: None,
        }
    }
}

/// `true` when `p` is a widget path (`.` followed by at least one char).
fn is_widget_path(p: &str) -> bool {
    p.starts_with('.') && p.len() > 1
}

/// The parent widget path — everything before the last `.`, or `.` for a
/// top-level child; `""` for the root `.` itself.
fn parent_path(p: &str) -> String {
    if p == "." {
        return String::new();
    }
    match p.rfind('.') {
        Some(idx) if idx > 0 => p[..idx].to_owned(),
        _ => ".".to_owned(),
    }
}

/// Shorten a widget command to its display type (`ttk::x` → `ttk-x`).
fn short_type(cmd: &str) -> String {
    cmd.strip_prefix("ttk::")
        .map_or_else(|| cmd.to_owned(), |rest| format!("ttk-{rest}"))
}

/// Parse `-option value` pairs, first-seen order, last value wins (Tcl dict
/// semantics).
fn parse_options(words: &[&str]) -> Vec<(String, String)> {
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, String> = HashMap::new();
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        if word.starts_with('-') && i + 1 < words.len() {
            if !map.contains_key(word) {
                order.push(word.to_owned());
            }
            map.insert(word.to_owned(), words[i + 1].to_owned());
            i += 2;
        } else {
            i += 1;
        }
    }
    order
        .into_iter()
        .map(|k| {
            let v = map[&k].clone();
            (k, v)
        })
        .collect()
}

fn filter_options(opts: Vec<(String, String)>, allowed: &[&str]) -> Vec<(String, String)> {
    opts.into_iter()
        .filter(|(k, _)| allowed.contains(&k.as_str()))
        .collect()
}

/// Split on whitespace runs with at most `n` splits: the remainder after the
/// `n`th split stays intact.
fn splitn_ws(s: &str, n: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = s.trim_start();
    while out.len() < n {
        match rest.find(char::is_whitespace) {
            Some(idx) => {
                out.push(&rest[..idx]);
                rest = rest[idx..].trim_start();
            }
            None => break,
        }
    }
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

fn options_json(opts: &[(String, String)]) -> Value {
    let mut m = Map::new();
    for (k, v) in opts {
        m.insert(k.clone(), json!(v));
    }
    Value::Object(m)
}

fn widget_json(path: &str, widgets: &HashMap<String, Widget>) -> Value {
    let Some(w) = widgets.get(path) else {
        return Value::Null;
    };
    let mut m = Map::new();
    m.insert("pathname".to_owned(), json!(w.pathname));
    m.insert("type".to_owned(), json!(w.wtype));
    m.insert("options".to_owned(), options_json(&w.options));
    m.insert(
        "geometry".to_owned(),
        w.geometry.clone().map_or(Value::Null, Value::String),
    );
    m.insert(
        "geometry_options".to_owned(),
        options_json(&w.geometry_options),
    );
    m.insert(
        "children".to_owned(),
        Value::Array(w.children.iter().map(|c| widget_json(c, widgets)).collect()),
    );
    if let Some(title) = &w.title {
        m.insert("title".to_owned(), json!(title));
    }
    Value::Object(m)
}

/// `tk_layout` handler — `{root, widget_count, geometry_conflicts}`.
pub fn tk_layout(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let lines: Vec<&str> = source.lines().collect();

    let mut widgets: HashMap<String, Widget> = HashMap::new();
    widgets.insert(".".to_owned(), Widget::new(".", "toplevel"));

    // Root window title — last `wm title .` wins.
    let mut title = "Tk".to_owned();
    for line in &lines {
        let stripped = line.trim();
        if stripped.starts_with("wm title .") {
            let parts = splitn_ws(stripped, 3);
            if parts.len() >= 4 {
                parts[3]
                    .trim_matches('"')
                    .trim_matches('{')
                    .trim_matches('}')
                    .clone_into(&mut title);
            }
        }
    }
    if let Some(root) = widgets.get_mut(".") {
        root.title = Some(title);
    }

    // Per-parent geometry managers, in first-seen order, for conflict detection.
    let mut geom_parents: Vec<(String, Vec<String>)> = Vec::new();

    for line in &lines {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = stripped.split_whitespace().collect();
        let Some(&cmd) = words.first() else { continue };

        // Widget creation.
        if WIDGET_COMMANDS.contains(&cmd) && words.len() >= 2 {
            let path = words[1];
            if is_widget_path(path) {
                let mut widget = Widget::new(path, &short_type(cmd));
                widget.options = filter_options(parse_options(&words[2..]), VISUAL_OPTIONS);
                widgets.insert(path.to_owned(), widget);
                let parent = parent_path(path);
                if let Some(parent_w) = widgets.get_mut(&parent) {
                    parent_w.children.push(path.to_owned());
                }
            }
        }

        // Geometry-manager commands.
        if GEOMETRY_COMMANDS.contains(&cmd) && words.len() >= 2 {
            let mut target_idx = 1;
            if words.len() >= 3 && GEOMETRY_SUBCOMMANDS.contains(&words[1]) {
                target_idx = 2;
            }
            if target_idx >= words.len() {
                continue;
            }
            let path = words[target_idx];
            if !is_widget_path(path) {
                continue;
            }
            if let Some(w) = widgets.get_mut(path) {
                w.geometry = Some(cmd.to_owned());
                w.geometry_options =
                    filter_options(parse_options(&words[target_idx + 1..]), GEOMETRY_OPTIONS);
            }
            let parent = parent_path(path);
            match geom_parents.iter_mut().find(|(p, _)| p == &parent) {
                Some((_, mgrs)) => {
                    if !mgrs.iter().any(|m| m == cmd) {
                        mgrs.push(cmd.to_owned());
                    }
                }
                None => geom_parents.push((parent, vec![cmd.to_owned()])),
            }
        }
    }

    let conflicts: Vec<Value> = geom_parents
        .iter()
        .filter(|(_, mgrs)| mgrs.iter().any(|m| m == "pack") && mgrs.iter().any(|m| m == "grid"))
        .map(|(parent, _)| {
            json!(format!(
                "Cannot mix 'pack' and 'grid' in parent '{parent}'."
            ))
        })
        .collect();

    json!({
        "root": widget_json(".", &widgets),
        "widget_count": widgets.len(),
        "geometry_conflicts": conflicts,
    })
}
