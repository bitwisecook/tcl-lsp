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

//! Plain-text / ANSI rendering of the explorer views.
//!
//! A reuse-positive renderer over the shared `view_tree` [`ViewNode`] model
//! — the *same* forest the TUI navigates — so the CLI `--text` mode, the TUI,
//! and (via JSON) the GUI all present one model. Each view is drawn as a
//! box-drawing tree with the highlighted node's detail rows inlined beneath
//! it; colour follows the ANSI palette.
//!
//! The bytecode `asm` view stays a flat disassembly (its serialiser already
//! produces a ready-to-print `text` field), and degraded views (`wasm`)
//! render an "unavailable" note rather than disappearing — matching the GUI's
//! graceful-degradation behaviour.

use serde_json::Value;

use crate::view_tree::{ViewNode, build_view};
use crate::views::tree_view_ids;

const RESET: &str = "\x1b[0m";

// Box-drawing connectors.
const TREE_BRANCH: &str = "├── ";
const TREE_LAST: &str = "└── ";
const TREE_VBAR: &str = "│   ";
const TREE_GAP: &str = "    ";

/// Map a `ViewNode` style name to an ANSI SGR code.
fn ansi_code(style: &str) -> Option<&'static str> {
    Some(match style {
        "bold" => "\x1b[1m",
        "dim" => "\x1b[2m",
        "red" => "\x1b[31m",
        "green" => "\x1b[32m",
        // No 8-colour orange; the web palette's orange maps to yellow here.
        "yellow" | "orange" => "\x1b[33m",
        "blue" => "\x1b[34m",
        "magenta" => "\x1b[35m",
        "cyan" => "\x1b[36m",
        _ => return None,
    })
}

/// Wrap `text` in `style`'s ANSI codes when colour is enabled.
fn paint(text: &str, style: Option<&str>, use_colour: bool) -> String {
    if !use_colour {
        return text.to_owned();
    }
    match style.and_then(ansi_code) {
        Some(code) => format!("{code}{text}{RESET}"),
        None => text.to_owned(),
    }
}

/// Render the whole result as text, optionally filtered to the views whose id
/// contains one of the `show` terms (case-insensitive substring).
#[must_use]
pub fn render_all(data: &Value, show: &[String], use_colour: bool) -> String {
    let mut out = String::new();
    for view in tree_view_ids() {
        if !selected(view, show) {
            continue;
        }
        push_section(
            &mut out,
            view,
            &render_view(view, data, use_colour),
            use_colour,
        );
    }
    // The bytecode disassembly is a flat listing, not a navigable tree.
    if selected("asm", show) {
        push_section(&mut out, "asm", &render_asm(data, use_colour), use_colour);
    }
    if selected("wasm", show) {
        push_section(&mut out, "wasm", &render_wasm(data), use_colour);
    }
    if out.is_empty() {
        return "compiler explorer: no matching views\n".to_owned();
    }
    out
}

/// Whether `view` passes the `--show` filter.
fn selected(view: &str, show: &[String]) -> bool {
    show.is_empty()
        || show
            .iter()
            .any(|s| view.to_lowercase().contains(&s.to_lowercase()))
}

/// Append a titled section (bold header + body + blank line) to `out`.
fn push_section(out: &mut String, view: &str, body: &str, use_colour: bool) {
    out.push_str(&paint(&format!("=== {view} ==="), Some("bold"), use_colour));
    out.push('\n');
    out.push_str(body);
    out.push('\n');
}

/// Render a single tree view's [`ViewNode`] forest as a box-drawing tree.
#[must_use]
pub fn render_view(view: &str, data: &Value, use_colour: bool) -> String {
    let roots = build_view(view, data);
    if roots.is_empty() {
        return "  (no data)\n".to_owned();
    }
    let mut out = String::new();
    let last = roots.len() - 1;
    for (i, node) in roots.iter().enumerate() {
        render_node(node, "", i == last, use_colour, &mut out);
    }
    out
}

/// Recursively render `node` under `prefix`, with its detail rows inlined.
fn render_node(node: &ViewNode, prefix: &str, is_last: bool, use_colour: bool, out: &mut String) {
    let connector = if is_last { TREE_LAST } else { TREE_BRANCH };
    out.push_str(prefix);
    out.push_str(connector);
    out.push_str(&paint(&node.label, node.style.as_deref(), use_colour));
    out.push('\n');

    let child_prefix = format!("{prefix}{}", if is_last { TREE_GAP } else { TREE_VBAR });
    for (key, value) in &node.detail {
        let row = format!("· {key}: {value}");
        out.push_str(&child_prefix);
        out.push_str(&paint(&row, Some("dim"), use_colour));
        out.push('\n');
    }
    let last = node.children.len().saturating_sub(1);
    for (i, child) in node.children.iter().enumerate() {
        render_node(child, &child_prefix, i == last, use_colour, out);
    }
}

/// Render the bytecode disassembly from each `asm` function's flat `text`.
fn render_asm(data: &Value, use_colour: bool) -> String {
    let Some(functions) = data.get("asm").and_then(Value::as_array) else {
        return "  (no bytecode)\n".to_owned();
    };
    if functions.is_empty() {
        return "  (no bytecode)\n".to_owned();
    }
    let mut out = String::new();
    for func in functions {
        if let Some(name) = func.get("name").and_then(Value::as_str) {
            out.push_str(&paint(name, Some("cyan"), use_colour));
            out.push('\n');
        }
        if let Some(text) = func.get("text").and_then(Value::as_str) {
            for line in text.lines() {
                out.push_str("  ");
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

/// Render the WASM view: print the module's WAT (carried as the `text` of the
/// synthetic `(module)` entry), then a one-line header per emitted function.
fn render_wasm(data: &Value) -> String {
    use std::fmt::Write as _;
    let Some(Value::Array(entries)) = data.get("wasm") else {
        return "  unavailable\n".to_owned();
    };
    if entries.is_empty() {
        return "  (no WASM functions)\n".to_owned();
    }
    let mut out = String::new();
    for entry in entries {
        if entry.get("kind").and_then(Value::as_str) == Some("module") {
            if let Some(text) = entry.get("text").and_then(Value::as_str) {
                out.push_str(text);
                if !text.ends_with('\n') {
                    out.push('\n');
                }
            }
        } else {
            let name = entry.get("name").and_then(Value::as_str).unwrap_or("?");
            let n = entry.get("instrCount").and_then(Value::as_u64).unwrap_or(0);
            let _ = writeln!(out, "  fn {name}: {n} instr");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_pipeline, serialise_result};

    fn data(src: &str) -> Value {
        serialise_result(&run_pipeline(src, "tcl8.6"))
    }

    #[test]
    fn render_view_draws_a_tree() {
        let d = data("set x 1\nset y 2");
        let text = render_view("ir", &d, false);
        assert!(text.contains("└── ") || text.contains("├── "));
        assert!(!text.contains('\x1b'), "no colour when disabled");
    }

    #[test]
    fn render_all_includes_headers_and_asm() {
        let d = data("interp create child");
        let text = render_all(&d, &[], false);
        assert!(text.contains("=== ir ==="));
        assert!(text.contains("=== worldSsa ==="));
        assert!(text.contains("=== asm ==="));
        assert!(text.contains("=== wasm ==="));
    }

    #[test]
    fn show_filter_limits_views() {
        let d = data("set x 1");
        let text = render_all(&d, &["ir".to_owned()], false);
        assert!(text.contains("=== ir ==="));
        assert!(!text.contains("=== cfg ==="));
    }

    #[test]
    fn colour_emits_ansi() {
        let d = data("set x 1");
        let text = render_all(&d, &["ir".to_owned()], true);
        assert!(text.contains('\x1b'), "colour should emit ANSI codes");
    }

    #[test]
    fn wasm_renders_wat_module() {
        // The eval-fallback emitter produces a real WASM module; the view shows
        // its WAT (a `(module …)` with at least the `$::top` function).
        let d = data("set x 1\nputs $x");
        let wat = render_wasm(&d);
        assert!(wat.contains("(module"), "{wat}");
        assert!(wat.contains("$::top"), "{wat}");
    }
}
