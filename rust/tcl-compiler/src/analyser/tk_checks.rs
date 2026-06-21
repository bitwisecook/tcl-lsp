//! Analyser-level Tk-dialect checks — Rust port of `analyser/checks/tk.py`.
//!
//! These run per command from
//! [`super::commands::Analyser::emit_dispatch_site_diagnostics`] and are
//! all gated on the `tk` dialect.  TK1001 is a whole-document check
//! (a `pack`/`grid` conflict can only be decided once every geometry
//! call in the parent has been seen), so it accumulates per-parent
//! usage across the walk and is flushed post-walk by
//! [`super::state::Analyser::flush_tk_geometry_diagnostics`] from the
//! shared diagnostic-emission tail.
//!
//! Diagnostic codes ported here:
//!
//! - **TK1001** (WARNING): geometry-manager conflict — `pack` and `grid`
//!   used on the same parent (a runtime error in Tk).
//! - **TK1002** (WARNING): widget path references a non-existent parent.
//! - **TK1003** (HINT): unknown option for a widget command.

use tcl_lexer::Token;

use super::state::Analyser;
use super::types::{Diagnostic, Severity};

/// Per-parent geometry-manager usage accumulated during the walk so
/// TK1001 can be decided post-walk.  Mirrors the `geometry_by_parent` /
/// `geometry_ranges` pair in `analyser/checks/tk.py`.
#[derive(Debug, Default)]
pub(super) struct TkGeometryUsage {
    /// The distinct geometry managers (`pack` / `grid` / `place`) seen
    /// for this parent.
    pub managers: std::collections::BTreeSet<String>,
    /// Each geometry call site `(manager, span)`, in document order, so a
    /// conflict reports on every offending call exactly like Python.
    pub sites: Vec<(String, tcl_lexer::Span)>,
}

/// Tk widget-creation commands (`button`, `label`, … and their `ttk::`
/// forms).  Mirrors `WIDGET_COMMANDS` in `dialects/tk/dialect/common.py`.
const WIDGET_COMMANDS: &[&str] = &[
    "button",
    "label",
    "entry",
    "text",
    "frame",
    "canvas",
    "listbox",
    "scrollbar",
    "menu",
    "menubutton",
    "toplevel",
    "message",
    "scale",
    "spinbox",
    "checkbutton",
    "radiobutton",
    "labelframe",
    "panedwindow",
    "destroy",
    "ttk::button",
    "ttk::label",
    "ttk::entry",
    "ttk::frame",
    "ttk::checkbutton",
    "ttk::radiobutton",
    "ttk::scrollbar",
    "ttk::spinbox",
    "ttk::scale",
    "ttk::panedwindow",
    "ttk::notebook",
    "ttk::treeview",
    "ttk::combobox",
    "ttk::progressbar",
    "ttk::separator",
    "ttk::labelframe",
    "ttk::menubutton",
    "ttk::sizegrip",
];

/// Tk geometry-manager commands.  Mirrors `GEOMETRY_COMMANDS`.
const GEOMETRY_COMMANDS: &[&str] = &["pack", "grid", "place"];

/// Return `true` if `name` is a Tk widget-creation command.
fn is_widget_command(name: &str) -> bool {
    WIDGET_COMMANDS.contains(&name)
}

/// Return `true` if `name` is a Tk geometry-manager command.
fn is_geometry_command(name: &str) -> bool {
    GEOMETRY_COMMANDS.contains(&name)
}

/// Return `true` if `path` matches Tcl/Tk widget-path syntax — a leading
/// `.`, then a letter/underscore, then letters / digits / `_` / `.`.
/// Mirrors `WIDGET_PATH_RE` in `dialects/tk/dialect/common.py`
/// (`^\.[a-zA-Z_][a-zA-Z0-9_.]*$`); note the bare root `.` does *not*
/// match (it has no first component).
fn is_widget_path(path: &str) -> bool {
    let mut chars = path.chars();
    if chars.next() != Some('.') {
        return false;
    }
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Return the parent widget path for `widget_path`.  Mirrors
/// `parent_widget_path`: `.` has no parent (`""`); a single-component
/// path (`.foo`) has the root `.` as parent; otherwise strip the final
/// `.component`.
fn parent_widget_path(widget_path: &str) -> &str {
    if widget_path == "." {
        return "";
    }
    match widget_path.rfind('.') {
        Some(idx) if idx > 0 => &widget_path[..idx],
        _ => ".",
    }
}

impl Analyser {
    /// Per-command Tk-dialect dispatch.  Tracks widget creation and
    /// geometry-manager usage, emitting TK1002 / TK1003 inline and
    /// recording geometry usage for the post-walk TK1001 flush.  A no-op
    /// outside the `tk` dialect.
    pub(super) fn emit_tk_checks(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        if self.dialect != "tk" {
            return;
        }

        if is_widget_command(cmd_name)
            && let Some(path) = args.first()
            && is_widget_path(path)
        {
            self.tk_created_widgets.insert(path.clone());

            // TK1002: the parent widget must already exist.  The root `.`
            // always exists, so it is never flagged.
            let parent = parent_widget_path(path);
            if !parent.is_empty() && parent != "." && !self.tk_created_widgets.contains(parent) {
                self.result.diagnostics.push(Diagnostic {
                    code: "TK1002".to_string(),
                    span: cmd_tok.span,
                    message: format!(
                        "Widget path '{path}' references non-existent parent '{parent}'."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }

            // TK1003: unknown option for the widget command.
            self.emit_tk1003_unknown_options(cmd_name, args, cmd_tok);
        }

        // Track geometry-manager usage for the post-walk TK1001 check.
        if is_geometry_command(cmd_name)
            && let Some(widget_path) = args.first()
            && is_widget_path(widget_path)
        {
            let parent = parent_widget_path(widget_path).to_string();
            let span = arg_tokens.first().map_or(cmd_tok.span, |t| t.span);
            let usage = self.tk_geometry.entry(parent).or_default();
            usage.managers.insert(cmd_name.to_string());
            usage.sites.push((cmd_name.to_string(), span));
        }
    }

    /// TK1003 — flag `-option` arguments that the widget command does
    /// not declare.  Mirrors the option scan in `check_tk_diagnostics`:
    /// a lone `-` / `--` is skipped, and the check is silent when the
    /// command has no registry spec (so unknown widgets never false
    /// positive).
    fn emit_tk1003_unknown_options(&mut self, cmd_name: &str, args: &[String], cmd_tok: Token) {
        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };
        let known: std::collections::HashSet<&str> = spec.switch_names(None).into_iter().collect();
        let mut unknown: Vec<String> = Vec::new();
        for arg in &args[1..] {
            if arg.starts_with('-')
                && !arg.starts_with("--")
                && arg.len() > 1
                && !known.contains(arg.as_str())
            {
                unknown.push(arg.clone());
            }
        }
        for arg in unknown {
            self.result.diagnostics.push(Diagnostic {
                code: "TK1003".to_string(),
                span: cmd_tok.span,
                message: format!("Unknown option '{arg}' for {cmd_name}."),
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// TK1001 — post-walk flush: a parent that mixes `pack` and `grid`
    /// is a Tk runtime error, reported on every geometry call for that
    /// parent.  Mirrors the final loop in `check_tk_diagnostics`.  Clears
    /// the accumulated state so a reused [`Analyser`] starts clean.
    pub(super) fn flush_tk_geometry_diagnostics(&mut self) {
        let geometry = std::mem::take(&mut self.tk_geometry);
        self.tk_created_widgets.clear();
        if self.dialect != "tk" {
            return;
        }
        for (parent, usage) in geometry {
            if usage.managers.contains("pack") && usage.managers.contains("grid") {
                for (_manager, span) in usage.sites {
                    self.result.diagnostics.push(Diagnostic {
                        code: "TK1001".to_string(),
                        span,
                        message: format!(
                            "Geometry manager conflict: cannot mix 'pack' and 'grid' \
                             in the same parent '{parent}'."
                        ),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::Analyser;

    fn codes(source: &str, dialect: &str) -> Vec<(String, String)> {
        let mut a = Analyser::new();
        let res = a.analyse(source, dialect);
        res.diagnostics
            .iter()
            .filter(|d| d.code.starts_with("TK"))
            .map(|d| (d.code.clone(), d.message.clone()))
            .collect()
    }

    fn has(source: &str, dialect: &str, code: &str) -> bool {
        codes(source, dialect).iter().any(|(c, _)| c == code)
    }

    #[test]
    fn tk1002_fires_for_missing_parent() {
        // `.outer` was never created, so `.outer.inner` has no parent.
        assert!(has("frame .outer.inner", "tk", "TK1002"));
    }

    #[test]
    fn tk1002_quiet_when_parent_created() {
        let src = "frame .outer\nframe .outer.inner";
        assert!(!has(src, "tk", "TK1002"));
    }

    #[test]
    fn tk1002_quiet_for_root_child() {
        // The root `.` always exists, so `.top` is fine.
        assert!(!has("frame .top", "tk", "TK1002"));
    }

    #[test]
    fn tk1001_fires_for_pack_grid_conflict() {
        let src = "frame .top\npack .top.a\ngrid .top.b";
        assert!(has(src, "tk", "TK1001"));
    }

    #[test]
    fn tk1001_quiet_for_pack_only() {
        let src = "frame .top\npack .top.a\npack .top.b";
        assert!(!has(src, "tk", "TK1001"));
    }

    #[test]
    fn no_tk_checks_outside_tk_dialect() {
        // The same conflict in plain Tcl must stay silent.
        let src = "frame .outer.inner";
        assert!(!has(src, "tcl", "TK1002"));
    }
}
