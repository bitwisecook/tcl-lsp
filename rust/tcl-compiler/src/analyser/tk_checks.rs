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

//! Analyser-level Tk-dialect checks.
//!
//! These run per command from
//! [`super::commands::Analyser::emit_dispatch_site_diagnostics`].  The
//! whole pass is gated on whether the file is Tk — *not* on a dialect
//! string alone — so the checks activate when either the analysis
//! dialect is `tk` (a `wish`-labelled document) **or** the file declares
//! `package require Tk` (so a plain `.tcl` script mapped to `tcl8.6` still
//! gets them).  Because the `package require` is a whole-file fact only
//! known after the walk, every TK diagnostic is buffered during the walk
//! and emitted post-walk by
//! [`super::state::Analyser::flush_tk_geometry_diagnostics`] from the shared
//! diagnostic-emission tail, gated on the resolved activation condition.
//! TK1001 additionally needs all of a parent's geometry calls before it can
//! decide a `pack`/`grid` conflict, so it accumulates per-parent usage and
//! is decided in the same flush.
//!
//! The per-command accumulation is itself gated on a cheap
//! [`tk_possibly_active`] precheck (`tk` dialect, or the source mentions
//! `package` / `require` / `Tk`).  The incremental per-item analysis
//! path ([`Analyser::analyse_per_item_with`]) falls back to full
//! [`Analyser::analyse`] when [`tk_possibly_active`] holds, so Tk's
//! whole-file accumulator (which an isolated proc body cannot see) never
//! diverges from full analysis.
//!
//! Diagnostic codes:
//!
//! - **TK1001** (WARNING): geometry-manager conflict — `pack` and `grid`
//!   used on the same parent (a runtime error in Tk).
//! - **TK1002** (WARNING): widget path references a non-existent parent.
//! - **TK1003** (HINT): unknown option for a widget command.

use tcl_core_types::DiagCode;
use tcl_lexer::Token;

use super::state::Analyser;
use super::types::{Diagnostic, Severity};

/// Per-parent geometry-manager usage accumulated during the walk so
/// TK1001 can be decided post-walk.
#[derive(Debug, Default)]
pub(super) struct TkGeometryUsage {
    /// The distinct geometry managers (`pack` / `grid` / `place`) seen
    /// for this parent.
    pub managers: std::collections::BTreeSet<String>,
    /// Each geometry call site `(manager, span)`, in document order, so a
    /// conflict reports on every offending call.
    pub sites: Vec<(String, tcl_lexer::Span)>,
}

/// Tk geometry-manager commands.
const GEOMETRY_COMMANDS: &[&str] = &["pack", "grid", "place"];

/// Cheap precheck for whether Tk diagnostics could apply to this document:
/// the analysis dialect is `tk`, or the source mentions `package` /
/// `require` / `Tk` (so a `package require Tk` — however the lexer splits
/// its words — is not missed).  Used both to gate the per-command
/// accumulation and to force the incremental per-item path back to full
/// analysis for Tk documents.
#[must_use]
pub(super) fn tk_possibly_active(source: &str, dialect: &str) -> bool {
    dialect == "tk"
        || (source.contains("package") && source.contains("require") && source.contains("Tk"))
}

/// Return `true` if `name` is a Tk geometry-manager command.
fn is_geometry_command(name: &str) -> bool {
    GEOMETRY_COMMANDS.contains(&name)
}

/// Return `true` if `path` matches Tcl/Tk widget-path syntax — a leading
/// `.`, then a letter/underscore, then letters / digits / `_` / `.`
/// (`^\.[a-zA-Z_][a-zA-Z0-9_.]*$`); note the bare root `.` does *not*
/// match (it has no first component).
///
/// `pub(crate)`: also the single source of truth for
/// `signature_scan::command_prefix`'s command-prefix-head guard — a widget
/// path is a dynamically-bound Tk window command, never a resolvable
/// user proc, so a callback prefix whose head is one (`-yscrollcommand
/// {.sb set}`) must not be treated as a checkable command reference.
pub(crate) fn is_widget_path(path: &str) -> bool {
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

/// Return the parent widget path for `widget_path`: `.` has no parent
/// (`""`); a single-component path (`.foo`) has the root `.` as parent;
/// otherwise strip the final `.component`.
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
    /// Return `true` if `name` is a Tk widget-creation command — driven by
    /// the registry (`creates_instance_at` + `required_package == "Tk"`),
    /// not a hand-maintained name list: the previous hardcoded
    /// `WIDGET_COMMANDS` had already drifted from the registry (it named
    /// `ttk::scrollbar` / `ttk::labelframe`, neither of which has ever had a
    /// registered spec on this branch), which is exactly the class of bug a
    /// second source of truth invites (issue #927;
    /// `docs/design/tk-widget-instance-typing.md`).
    fn is_widget_command(&self, name: &str) -> bool {
        self.registry.is_some_and(|r| {
            r.get(name).is_some_and(|s| {
                s.creates_instance_at.is_some() && s.required_package == Some("Tk")
            })
        })
    }

    /// Per-command Tk dispatch.  Tracks widget creation and geometry-manager
    /// usage, *buffering* TK1002 / TK1003 (and recording geometry usage for
    /// the post-walk TK1001 flush) rather than emitting inline — the
    /// activation condition (`tk` dialect or a `package require Tk`) is a
    /// whole-file fact only resolved post-walk.  A no-op unless
    /// [`tk_possibly_active`] held for this document.
    pub(super) fn emit_tk_checks(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        if !self.tk_possibly_active {
            return;
        }

        if self.is_widget_command(cmd_name)
            && let Some(path) = args.first()
            && is_widget_path(path)
        {
            self.tk_created_widgets.insert(path.clone());

            // TK1002: the parent widget must already exist.  The root `.`
            // always exists, so it is never flagged.
            let parent = parent_widget_path(path);
            if !parent.is_empty() && parent != "." && !self.tk_created_widgets.contains(parent) {
                self.tk_pending_diags.push(Diagnostic {
                    code: DiagCode::Tk1002,
                    span: cmd_tok.span,
                    message: format!(
                        "Widget path '{path}' references non-existent parent '{parent}'."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }

            // TK1003: unknown option for the widget command.
            self.emit_tk1003_unknown_options(cmd_name, args, arg_tokens, cmd_tok);
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

    /// TK1003 — buffer `-option` words that the widget command does not
    /// declare.  The check is silent when the command has no registry spec
    /// (so unknown widgets never false positive).
    ///
    /// Widget-creation options are alternating `-option value` pairs
    /// (`Tk_ConfigureWidget`), so the scan walks pairs: an option word is
    /// validated and its *value* word skipped — a value that itself starts
    /// with `-` (`-padx -2`) is data, never an unknown option.  A unique
    /// prefix of a declared option is accepted, matching Tk's
    /// abbreviation rule; an ambiguous prefix abstains.  A non-`-` word in
    /// option position ends the scan (Tk itself errors there).  Each
    /// finding anchors on the offending option word and carries a "did you
    /// mean…?" replace fix drawn from the declared option set.
    fn emit_tk1003_unknown_options(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        let Some(registry) = self.registry else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };
        let known: Vec<&str> = spec.switch_names(None);
        let mut findings: Vec<(String, tcl_lexer::Span)> = Vec::new();
        let mut i = 1;
        while i < args.len() {
            let arg = &args[i];
            if !arg.starts_with('-') || arg == "-" || arg.starts_with("--") {
                // Non-option word in option position — Tk stops parsing
                // options here, so anything after is not ours to judge.
                break;
            }
            // Dynamic option word (`-$style`, `-[pick]`) — unknowable.
            if arg.contains('$') || arg.contains('[') {
                i += 2;
                continue;
            }
            let exact = known.contains(&arg.as_str());
            let prefix_hits = known.iter().filter(|k| k.starts_with(arg.as_str())).count();
            if !exact && prefix_hits == 0 {
                let span = arg_tokens.get(i).map_or(cmd_tok.span, |t| t.span);
                findings.push((arg.clone(), span));
            }
            // Skip the option's value word.
            i += 2;
        }
        for (arg, span) in findings {
            let suggestions = crate::text::suggest_similar(
                &arg,
                known.iter().copied(),
                1,
                crate::text::scaled_max_distance(&arg),
            );
            let mut message = format!("Unknown option '{arg}' for {cmd_name}.");
            let mut fixes: Vec<super::types::CodeFix> = Vec::new();
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, " Did you mean '{best}'?");
                fixes.push(super::types::CodeFix {
                    span,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
            self.tk_pending_diags.push(Diagnostic {
                code: DiagCode::Tk1003,
                span,
                message,
                severity: Severity::Hint,
                fixes,
            });
        }
    }

    /// Whether the file declared `package require Tk`.
    fn has_tk_require(&self) -> bool {
        self.result
            .package_requires
            .iter()
            .any(|pr| pr.name == "Tk")
    }

    /// Post-walk flush of all TK diagnostics.  Emits the buffered TK1002 /
    /// TK1003 and decides TK1001 (a parent mixing `pack` and `grid` is a Tk
    /// runtime error, reported on every offending geometry call) — but only
    /// when the document is actually Tk: the `tk` dialect, or a detected
    /// `package require Tk`.  Clears the accumulated state either way so a
    /// reused [`Analyser`] starts clean.
    pub(super) fn flush_tk_geometry_diagnostics(&mut self) {
        let geometry = std::mem::take(&mut self.tk_geometry);
        let pending = std::mem::take(&mut self.tk_pending_diags);
        self.tk_created_widgets.clear();

        if !self.tk_dialect && !self.has_tk_require() {
            return;
        }

        self.result.diagnostics.extend(pending);
        for (parent, usage) in geometry {
            if usage.managers.contains("pack") && usage.managers.contains("grid") {
                for (_manager, span) in usage.sites {
                    self.result.diagnostics.push(Diagnostic {
                        code: DiagCode::Tk1001,
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
    use tcl_core_types::DiagCode;

    fn codes(source: &str, dialect: &str) -> Vec<(String, String)> {
        let mut a = Analyser::new();
        let res = a.analyse(source, dialect);
        res.diagnostics
            .iter()
            .filter(|d| d.code.as_str().starts_with("TK"))
            .map(|d| (d.code.to_string(), d.message.clone()))
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
    fn no_tk_checks_in_plain_tcl_without_tk_require() {
        // Plain Tcl with no `package require Tk` must stay silent even with
        // Tk-shaped commands.
        let src = "frame .outer.inner";
        assert!(!has(src, "tcl8.6", "TK1002"));
    }

    #[test]
    fn tk_checks_fire_on_plain_tcl_with_package_require_tk() {
        // A `.tcl` file mapped to `tcl8.6` that declares `package require Tk`
        // must still get the checks.
        let src = "package require Tk\nframe .outer.inner";
        assert!(has(src, "tcl8.6", "TK1002"));
    }

    /// Registry-driven `is_widget_command` (issue #927) must cover every
    /// registered widget constructor, not just the handful the old
    /// hardcoded list happened to name — proven here with a `ttk::`
    /// constructor the old list also named, and a plain-Tk one it did too,
    /// so this is a coverage check on the *mechanism*, not just a
    /// re-assertion of the same case `tk1002_fires_for_missing_parent` uses.
    #[test]
    fn tk1002_fires_for_ttk_and_listbox_constructors() {
        assert!(has("ttk::treeview .outer.inner", "tk", "TK1002"));
        assert!(has("listbox .outer.inner", "tk", "TK1002"));
    }

    /// A command the registry does not recognise at all must never be
    /// treated as a widget constructor — this is what made the old
    /// hardcoded `WIDGET_COMMANDS` list's drift (`ttk::scrollbar` /
    /// `ttk::labelframe`, neither ever a registered spec on this branch)
    /// silently harmless rather than a live false-positive risk once new
    /// checks start trusting `is_widget_command`; the registry-driven
    /// version simply cannot drift the same way.
    #[test]
    fn unknown_command_is_never_treated_as_a_widget_constructor() {
        assert!(!has("totallyMadeUpCommand .outer.inner", "tk", "TK1002"));
    }

    #[test]
    fn package_require_tk_substring_alone_does_not_activate() {
        // The cheap substring precheck can match, but without a real
        // `package require Tk` the post-walk gate stays closed.
        let src = "set msg \"package require Tk in a comment\"\nframe .outer.inner";
        assert!(!has(src, "tcl8.6", "TK1002"));
    }

    #[test]
    fn per_item_matches_full_for_tk_with_proc() {
        // The incremental per-item path must agree with full analysis for a
        // Tk document whose widgets / geometry live inside a proc (the
        // whole-file accumulator an isolated body cannot see).
        let src = "package require Tk\n\
                   frame .top\n\
                   proc build {} {\n\
                   pack .top.a\n\
                   grid .top.b\n\
                   }\n";
        let full = Analyser::new().analyse(src, "tcl8.6");
        let per_item = Analyser::new().analyse_per_item(src, "tcl8.6");
        let codes_of = |r: &super::super::types::AnalysisResult| {
            let mut v: Vec<(String, u32)> = r
                .diagnostics
                .iter()
                .map(|d| (d.code.to_string(), d.span.start()))
                .collect();
            v.sort();
            v
        };
        assert_eq!(codes_of(&full), codes_of(&per_item));
        // And the conflict really is reported (proc-body geometry flushed).
        assert!(full.diagnostics.iter().any(|d| d.code == DiagCode::Tk1001));
    }
}
