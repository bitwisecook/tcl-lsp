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
//! ## Which interpreter's windows?
//!
//! TK1001 and TK1002 both ask a question about *runtime state* — "has this
//! parent been created?", "has this container already been claimed by another
//! geometry manager?" — and that state is per-interpreter, not per-file.  Tk's
//! `TkCreateMainWindow` (9.0.4 `generic/tkWindow.c`) allocates a fresh
//! `TkMainInfo` for **each** interpreter it initialises, with its own
//! widget-path `nameTable` seeded with its own `.` root; `Tk_NameToWindow` and
//! `TkSetGeometryContainer` both resolve through that per-application table.
//! So `interp create c; load {} Tk c; c eval { frame .top }` and a parent-side
//! `frame .top` are two unrelated windows.
//!
//! The accumulators are therefore keyed by **interpreter domain**
//! (`Analyser::tk_domains`) — the same synthetic
//! `@interp@<path>[#<epoch>]` identity the shared isolation helper
//! `isolate_interp_eval_body` already homes a child body's procs and variables
//! under.  Before issue #1141 they were one flat pair of file-wide fields, which
//! produced a false TK1001 (a parent-side `pack` "conflicting" with a child-side
//! `grid`) and a missed TK1002 (a parent created in one interpreter vouching for
//! a child widget in another).
//!
//! Diagnostic codes:
//!
//! - **TK1001** (WARNING): geometry-manager conflict — `pack` and `grid`
//!   used on the same parent *in the same interpreter* (a runtime error in Tk).
//! - **TK1002** (WARNING): widget path references a parent that does not exist
//!   *in that interpreter's* hierarchy.
//! - **TK1003** (HINT): unknown option for a widget command (per-command, so
//!   interpreter-independent).

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

/// One interpreter's Tk window hierarchy as the walk models it (issue
/// #1141) — the analyser-side mirror of C Tk's per-interpreter
/// `TkMainInfo`.
///
/// Every interpreter that loads Tk gets a fresh `TkMainInfo` from
/// `TkCreateMainWindow` (Tk 9.0.4 `generic/tkWindow.c`) with its own
/// widget-path `nameTable` and its own `.` root entry, and every widget
/// lookup (`Tk_NameToWindow`) and geometry-manager claim
/// (`TkSetGeometryContainer`) goes through that table.  So `.top` created
/// in `child eval {…}` and `.top` created in the parent script are two
/// unrelated windows, and neither TK1002's parent-existence question nor
/// TK1001's `pack`/`grid` conflict may be decided across the two.
#[derive(Debug, Default)]
pub(super) struct TkDomainState {
    /// `false` when the interpreter this domain models could not be named
    /// statically (an `interp eval $unknown {…}` body).  Such a domain may
    /// in truth *be* any other domain, including the main interpreter, so
    /// the TK1002 existence question widens across it in both directions
    /// rather than treating it as a distinct hierarchy.
    pub resolved: bool,
    /// Widget paths created so far in this interpreter, so a child's
    /// parent can be checked for existence (TK1002).
    pub created_widgets: std::collections::HashSet<String>,
    /// Per-parent geometry-manager usage in this interpreter, keyed by
    /// parent widget path, flushed post-walk so a `pack`/`grid` conflict
    /// can be decided (TK1001).
    pub geometry: std::collections::BTreeMap<String, TkGeometryUsage>,
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

        // Every widget/geometry fact belongs to the Tk hierarchy of the
        // interpreter this command runs in, never to a file-wide pool.
        let (domain, resolved) = self.current_tk_domain();

        if self.is_widget_command(cmd_name)
            && let Some(path) = args.first()
            && is_widget_path(path)
        {
            // TK1002: the parent widget must already exist *in this
            // hierarchy*.  The root `.` always exists (every `TkMainInfo`
            // seeds its `nameTable` with it), so it is never flagged.
            let parent = parent_widget_path(path);
            if !parent.is_empty()
                && parent != "."
                && !self.tk_parent_widget_exists(&domain, resolved, parent)
            {
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

            self.tk_domain_state(&domain, resolved)
                .created_widgets
                .insert(path.clone());

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
            let usage = self
                .tk_domain_state(&domain, resolved)
                .geometry
                .entry(parent)
                .or_default();
            usage.managers.insert(cmd_name.to_string());
            usage.sites.push((cmd_name.to_string(), span));
        }
    }

    /// The interpreter domain the walk is currently inside, as
    /// `(domain, resolved)`: the `@interp@…` identity of the innermost
    /// `interp eval` / `NAME eval` body on the walk stack, or `("", true)`
    /// for the main interpreter.
    ///
    /// This is the *same* synthetic-key mechanism that homes a child body's
    /// procs and variables under `@interp@<path>[#<epoch>]`
    /// (`isolate_interp_eval_body` in `super::handlers`) — deliberately not a
    /// second, parallel notion of "which interpreter is this", so a domain
    /// identity can never drift between the scope tree and the Tk state.
    /// Folding in the deletion epoch also means
    /// `interp delete c; interp create c` really does end the old
    /// hierarchy: the recreated child starts with an empty `nameTable`
    /// (tclsh 9.0.4-verified for the analogous command table).
    fn current_tk_domain(&self) -> (String, bool) {
        self.interp_path_stack
            .last()
            .map_or_else(|| (String::new(), true), |f| (f.domain.clone(), f.resolved))
    }

    /// The accumulator for `domain`, created on first use.
    fn tk_domain_state(&mut self, domain: &str, resolved: bool) -> &mut TkDomainState {
        let state = self.tk_domains.entry(domain.to_string()).or_default();
        state.resolved = resolved;
        state
    }

    /// Whether `parent` has been created in the hierarchy `domain` names.
    ///
    /// Widened conservatively for domains whose interpreter could not be
    /// named statically: if *either* side of the comparison is unresolved,
    /// the two might be the same interpreter, so a widget created there
    /// counts as possibly present here and TK1002 abstains.  A warning's
    /// false positive is the expensive direction, and an unknowable
    /// `interp eval $handle {…}` is exactly where the analyser has no
    /// grounds to insist a parent is missing.
    fn tk_parent_widget_exists(&self, domain: &str, resolved: bool, parent: &str) -> bool {
        self.tk_domains.iter().any(|(key, state)| {
            (key == domain || !resolved || !state.resolved)
                && state.created_widgets.contains(parent)
        })
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
        let domains = std::mem::take(&mut self.tk_domains);
        let pending = std::mem::take(&mut self.tk_pending_diags);

        if !self.tk_dialect && !self.has_tk_require() {
            return;
        }

        self.result.diagnostics.extend(pending);
        // Each interpreter's containers are decided separately: a `pack` in
        // the parent script and a `grid` in a child's eval body claim two
        // different `TkWindow`s that merely share a path string, so they can
        // never conflict.
        for state in domains.into_values() {
            for (parent, usage) in state.geometry {
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

    /// Issue #1141 — the analyser's Tk widget/geometry state is keyed by
    /// interpreter domain, mirroring C Tk's per-interpreter `TkMainInfo`
    /// (`TkCreateMainWindow`, 9.0.4 `generic/tkWindow.c`: a fresh
    /// `nameTable` and a fresh `.` root per interpreter).  The interpreter
    /// isolation these tests lean on is tclsh 9.0.4-verified (a command —
    /// and a widget path *is* a command — created inside `child eval {…}`
    /// never appears in the parent's `info commands`); the Tk-specific half
    /// is read from the C sources, since Tk cannot be run headless here.
    mod interp_domains {
        use super::{codes, has};

        /// FP (the bug): a parent-side `grid` and a child-side `pack` on the
        /// same *path* are two different containers, so no conflict.
        #[test]
        fn tk1001_fp_does_not_fire_across_isolated_interps() {
            let src = "interp create child\n\
                       child eval { frame .top; pack .top.a }\n\
                       frame .top\n\
                       grid .top.b\n";
            assert!(
                !has(src, "tk", "TK1001"),
                "cross-interpreter geometry must not conflict: {:?}",
                codes(src, "tk")
            );
        }

        /// TP control: the same conflict entirely in the parent still fires.
        #[test]
        fn tk1001_tp_same_domain_conflict_still_fires() {
            let src = "interp create child\n\
                       child eval { frame .other }\n\
                       frame .top\n\
                       pack .top.a\n\
                       grid .top.b\n";
            assert!(has(src, "tk", "TK1001"));
        }

        /// FN (the bug's other half): a conflict genuinely inside one child
        /// interpreter must fire — it was previously decided against a pool
        /// that mixed in the parent's calls.
        #[test]
        fn tk1001_fn_conflict_inside_one_child_body_fires() {
            let src = "interp create child\n\
                       child eval { frame .top; pack .top.a; grid .top.b }\n";
            assert!(has(src, "tk", "TK1001"));
        }

        /// Two evals into the *same* live interpreter accumulate into one
        /// hierarchy, as in C — so a conflict split across them fires.
        #[test]
        fn tk1001_tp_conflict_split_across_two_evals_into_one_interp() {
            let src = "interp create child\n\
                       child eval { frame .top; pack .top.a }\n\
                       child eval { grid .top.b }\n";
            assert!(has(src, "tk", "TK1001"));
        }

        /// TN: no conflict anywhere, in either domain.
        #[test]
        fn tk1001_tn_pack_only_in_both_domains() {
            let src = "interp create child\n\
                       child eval { frame .top; pack .top.a }\n\
                       frame .top\n\
                       pack .top.b\n";
            assert!(!has(src, "tk", "TK1001"));
        }

        /// FN (the bug): `.top` exists only in the child, so the parent's
        /// `.top.inner` really does have no parent.
        #[test]
        fn tk1002_fn_parent_created_only_in_child_still_missing_here() {
            let src = "interp create child\n\
                       child eval { frame .top }\n\
                       frame .top.inner\n";
            assert!(
                has(src, "tk", "TK1002"),
                "parent in another interpreter must not vouch: {:?}",
                codes(src, "tk")
            );
        }

        /// The mirror image: `.top` exists only in the parent, so the
        /// child's `.top.inner` has no parent either.
        #[test]
        fn tk1002_fn_parent_created_only_in_parent_is_missing_in_child() {
            let src = "interp create child\n\
                       frame .top\n\
                       child eval { frame .top.inner }\n";
            assert!(has(src, "tk", "TK1002"));
        }

        /// TN: parent and child created in the same domain — silent.
        #[test]
        fn tk1002_tn_same_domain_parent_is_quiet() {
            let src = "interp create child\n\
                       child eval { frame .top; frame .top.inner }\n";
            assert!(!has(src, "tk", "TK1002"));
        }

        /// TN: the hierarchy accumulates across separate evals into one
        /// live interpreter.
        #[test]
        fn tk1002_tn_parent_from_an_earlier_eval_into_the_same_interp() {
            let src = "interp create child\n\
                       child eval { frame .top }\n\
                       child eval { frame .top.inner }\n";
            assert!(!has(src, "tk", "TK1002"));
        }

        /// The handle form (`child eval`) and the literal form
        /// (`interp eval child`) name the same domain — tclsh 9.0.4-verified
        /// that both reach one command table.
        #[test]
        fn handle_and_literal_eval_forms_share_one_hierarchy() {
            let src = "interp create child\n\
                       interp eval child { frame .top }\n\
                       child eval { frame .top.inner }\n";
            assert!(!has(src, "tk", "TK1002"));
        }

        /// An empty path targets the *current* interpreter (tclsh
        /// 9.0.4-verified), so it shares the caller's hierarchy in both
        /// directions.
        #[test]
        fn tn_empty_interp_path_stays_in_the_current_hierarchy() {
            let parent_first = "frame .top\ninterp eval {} { frame .top.inner }\n";
            assert!(!has(parent_first, "tk", "TK1002"));
            let conflict = "frame .top\npack .top.a\ninterp eval {} { grid .top.b }\n";
            assert!(has(conflict, "tk", "TK1001"));
        }

        /// A nested child (`interp create t` inside `s`'s body creates the
        /// path `s t` — tclsh 9.0.4-verified) is a third, distinct
        /// hierarchy.
        #[test]
        fn nested_interpreter_paths_are_distinct_hierarchies() {
            let src = "interp create s\n\
                       s eval { interp create t\n t eval { frame .top } }\n\
                       frame .top.inner\n";
            assert!(has(src, "tk", "TK1002"));
        }

        /// `interp delete` ends the domain: a recreated path starts with an
        /// empty window table (tclsh 9.0.4-verified for the analogous
        /// command table), so the old `.top` no longer vouches.
        #[test]
        fn deleted_and_recreated_interpreter_starts_a_fresh_hierarchy() {
            let src = "interp create c\n\
                       c eval { frame .top }\n\
                       interp delete c\n\
                       interp create c\n\
                       c eval { frame .top.inner }\n";
            assert!(has(src, "tk", "TK1002"));
        }

        /// A safe child is still its own hierarchy — and a conflict inside
        /// it is still a conflict.
        #[test]
        fn safe_child_is_its_own_hierarchy() {
            let isolated = "interp create -safe c\n\
                            c eval { frame .top; pack .top.a }\n\
                            frame .top\n\
                            grid .top.b\n";
            assert!(!has(isolated, "tk", "TK1001"));
            let inside = "interp create -safe c\n\
                          c eval { frame .top; pack .top.a; grid .top.b }\n";
            assert!(has(inside, "tk", "TK1001"));
        }

        /// A `$handle` bound by `set h [interp create]` resolves to a real
        /// domain, so it isolates exactly like the literal spelling.
        #[test]
        fn tracked_handle_binding_resolves_to_its_own_domain() {
            let src = "set h [interp create sandbox]\n\
                       $h eval { frame .top }\n\
                       frame .top.inner\n";
            assert!(has(src, "tk", "TK1002"));
        }

        /// Unknowable target — the domain widens rather than asserting a
        /// missing parent: `$i` could name the very interpreter the parent
        /// script runs in.
        #[test]
        fn unresolved_interp_path_widens_rather_than_flagging() {
            let from_parent = "frame .top\n\
                              proc run {i} { interp eval $i { frame .top.inner } }\n";
            assert!(
                !has(from_parent, "tk", "TK1002"),
                "unknowable target must abstain: {:?}",
                codes(from_parent, "tk")
            );
            let to_parent = "proc run {i} { interp eval $i { frame .top } }\n\
                             frame .top.inner\n";
            assert!(!has(to_parent, "tk", "TK1002"));
        }

        /// Widening is not blanket silence: with no `.top` created anywhere
        /// in the file, the unknowable body's `.top.inner` is still flagged.
        #[test]
        fn unresolved_interp_path_still_flags_a_parent_created_nowhere() {
            let src = "proc run {i} { interp eval $i { frame .top.inner } }\n";
            assert!(has(src, "tk", "TK1002"));
        }

        /// The geometry side never merges an unknowable domain into another:
        /// a `pack` there and a `grid` in the parent are not a conflict
        /// (avoiding a warning we cannot justify).
        #[test]
        fn unresolved_interp_path_does_not_conflict_with_the_parent() {
            let src = "frame .top\n\
                       grid .top.b\n\
                       proc run {i} { interp eval $i { pack .top.a } }\n";
            assert!(!has(src, "tk", "TK1001"));
        }

        /// Domain keying does not depend on the `tk` dialect: a plain
        /// `.tcl` file with `package require Tk` gets the same isolation.
        #[test]
        fn domain_keying_applies_under_package_require_tk_too() {
            let src = "package require Tk\n\
                       interp create child\n\
                       child eval { frame .top; pack .top.a }\n\
                       frame .top\n\
                       grid .top.b\n";
            assert!(!has(src, "tcl8.6", "TK1001"));
            let fires = "package require Tk\n\
                         interp create child\n\
                         child eval { frame .top }\n\
                         frame .top.inner\n";
            assert!(has(fires, "tcl8.6", "TK1002"));
        }
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
