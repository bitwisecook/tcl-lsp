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
//! decide a geometry-manager conflict, so it accumulates per-parent usage and
//! is decided in the same flush.
//!
//! Two distinct questions are involved, and conflating them was issue #1188:
//!
//! - **Is Tk active?** — the authoritative, *exact* fact: the `tk` dialect, or
//!   a `package require Tk` that the registry's
//!   [`PackageRequire`](tcl_registry::hooks::AnalyserHookId::PackageRequire)
//!   hook recorded into `result.package_requires` during the walk.  This is
//!   [`Analyser::has_tk_require`] + `tk_dialect`, and it is the *only* gate on
//!   whether any TK diagnostic is emitted.
//! - **Is it worth accumulating?** — a pure performance precheck
//!   ([`tk_checks_could_apply`]), run before the walk to skip the per-command
//!   widget/geometry bookkeeping on a document that cannot possibly be Tk.
//!   It is a deliberately loose *necessary* condition (over-approximating is
//!   free: whatever it buffers is discarded by the flush unless the exact
//!   activation fact holds), so it can never change a diagnostic.
//!
//! The incremental per-item analysis path
//! ([`Analyser::analyse_per_item_with`]) falls back to full
//! [`Analyser::analyse`] once Tk is **exactly** known to be active, so Tk's
//! whole-file accumulator (which an isolated proc body cannot see) never
//! diverges from full analysis.  Before #1188 that fallback was driven by the
//! precheck instead — three independent substring searches for `package`,
//! `require`, and `Tk`, matching anywhere in the file including comments,
//! strings, and generated data — which forced a whole-file re-analysis on
//! every keystroke for 27.7% of the tcllib corpus's source lines, ~78% of them
//! false positives.
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
//! - **TK1001** (WARNING): geometry-container conflict — two distinct managers
//!   whose registry descriptors claim exclusive propagation ownership (Tk's
//!   `pack` and `grid`) use the same effective container in one interpreter.
//! - **TK1002** (WARNING): widget path references a parent that does not exist
//!   *in that interpreter's* hierarchy.
//! - **TK1003** (HINT): unknown option for a widget command (per-command, so
//!   interpreter-independent).

use tcl_core_types::DiagCode;
use tcl_lexer::Token;

use super::state::Analyser;
use super::types::Severity;

/// Per-parent geometry-manager usage accumulated during the walk so
/// TK1001 can be decided post-walk.
#[derive(Debug, Default)]
pub(super) struct TkGeometryUsage {
    /// The distinct registry-declared managers that claim exclusive geometry
    /// propagation ownership of this effective container.
    pub managers: std::collections::BTreeSet<String>,
    /// Each geometry call site `(manager, span)`, in document order, so a
    /// conflict reports on every offending call.
    pub sites: Vec<(String, tcl_lexer::Span)>,
}

/// The active geometry claim for one widget. Tk allows a content window to
/// switch managers; the previous manager's claim disappears at that point.
#[derive(Debug, Clone)]
pub(super) struct TkActiveGeometry {
    pub manager: String,
    pub container: String,
    pub span: tcl_lexer::Span,
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
    /// Current manager/container per content widget. This is deliberately
    /// temporal: historical calls cannot conflict after `forget`/`remove` or
    /// after the same widget switches manager.
    pub geometry_by_widget: std::collections::HashMap<String, TkActiveGeometry>,
    /// Geometry claims whose widget pathname may or may not exist or be
    /// managed after a control-flow arm.  These are kept path-scoped so an
    /// uncertain `.left` operation does not suppress a definite `.right`
    /// conflict.
    pub uncertain_geometry_widgets: std::collections::HashSet<String>,
    /// Effective containers whose manager ownership may differ by path.
    pub uncertain_geometry_containers: std::collections::HashSet<String>,
    /// Possible widget-to-container relationships for path-sensitive claims.
    /// The paths need not be lexically related because `-in` can redirect a
    /// widget into an unrelated container.
    pub uncertain_geometry_claims:
        std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// A dynamic pathname/container leaves no sound finite scope.  This is
    /// the only domain-wide widening; definite destroy/release can clear the
    /// path-scoped sets without touching unrelated containers.
    pub geometry_uncertain: bool,
}

/// The package name whose presence activates the TK checks.  The single
/// source of truth for both halves of the gate: the `package require` name
/// [`Analyser::has_tk_require`] looks for, and the `CommandSpec`
/// [`required_package`](tcl_registry::CommandSpec::required_package) that
/// makes a registry command a Tk widget command
/// ([`Analyser::is_widget_command`]).
pub(super) const TK_PACKAGE: &str = "Tk";

/// Cheap **necessary** condition for the per-command Tk accumulation to be
/// worth running at all — *not* the activation decision (see the module docs).
///
/// Activation requires either the `tk` environment or a statically-resolvable
/// `package require Tk`, and the latter cannot exist in a source that never
/// contains the literal package name.  So this is sound: it never returns
/// `false` for a document that goes on to activate.  It over-approximates
/// freely — a `Tk` inside a comment, a string, or generated data trips it —
/// which costs only a registry lookup per command, because
/// [`Analyser::flush_tk_geometry_diagnostics`] discards everything the walk
/// buffered unless the exact activation fact holds.
#[must_use]
pub(super) fn tk_checks_could_apply(source: &str, tk_environment: bool) -> bool {
    tk_environment || source.contains(TK_PACKAGE)
}

/// Compiler-facing wrapper around the registry-owned Tk widget-path grammar.
/// The bare root `.` is not a widget command and therefore does not match.
///
/// `pub(crate)`: also the single source of truth for
/// `signature_scan::command_prefix`'s command-prefix-head guard — a widget
/// path is a dynamically-bound Tk window command, never a resolvable
/// user proc, so a callback prefix whose head is one (`-yscrollcommand
/// {.sb set}`) must not be treated as a checkable command reference.
pub(crate) fn is_widget_path(path: &str) -> bool {
    !path.contains(['$', '[', ']']) && tcl_registry::tk_geometry::is_widget_path(path)
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

fn widget_is_within(candidate: &str, ancestor: &str) -> bool {
    tcl_registry::tk_geometry::widget_path_is_within(candidate, ancestor)
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
        self.registry.as_deref().is_some_and(|r| {
            r.get(name).is_some_and(|s| {
                s.creates_instance_at.is_some() && s.required_package == Some(TK_PACKAGE)
            })
        })
    }

    /// Return the registry-declared semantics for a Tk geometry manager.
    fn geometry_spec(
        &self,
        name: &str,
    ) -> Option<tcl_registry::tk_geometry::TkGeometryManagerSpec> {
        self.registry.as_deref()?.get(name)?.tk_geometry
    }

    /// Per-command Tk dispatch.  Tracks widget creation and geometry-manager
    /// usage, *buffering* TK1002 / TK1003 (and recording geometry usage for
    /// the post-walk TK1001 flush) rather than emitting inline — the
    /// activation condition (`tk` dialect or a `package require Tk`) is a
    /// whole-file fact only resolved post-walk.  A no-op unless
    /// [`tk_possibly_active`] held for this document.
    // One registry-dispatched transaction updates the shared widget,
    // geometry, and option-use state. Splitting it would make those facts
    // order-dependent across helpers.
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_tk_checks(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        if !self.tk_accumulation_enabled {
            return;
        }

        // Every widget/geometry fact belongs to the Tk hierarchy of the
        // interpreter this command runs in, never to a file-wide pool.
        let (domain, resolved) = self.current_tk_domain();
        let path_uncertain = self.control_flow_body_depth > 0 || self.conditional_depth > 0;

        if self.is_widget_command(cmd_name) {
            if let Some(path) = args.first().filter(|path| is_widget_path(path)) {
                if path_uncertain {
                    self.mark_tk_geometry_paths_uncertain(&domain, resolved, Some(path), None);
                }
            } else {
                // A dynamic constructor can establish any widget path, so no
                // finite scope is sound here, whether or not it is branched.
                self.mark_tk_geometry_domain_uncertain(&domain, resolved);
            }
        }

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
                self.tk_pending_diags
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::Tk1002,
                        cmd_tok.span,
                        format!("Widget path '{path}' references non-existent parent '{parent}'."),
                        Severity::Warning,
                    ));
            }

            self.tk_domain_state(&domain, resolved)
                .created_widgets
                .insert(path.clone());

            // TK1003: unknown option for the widget command.
            self.emit_tk1003_unknown_options(cmd_name, args, arg_tokens, cmd_tok);
        }

        // Registry-declared Tk teardown ends both the pathname lifetime and
        // any geometry claim held by that widget. Keep this generic: the
        // analyser does not name `destroy`.
        let tears_down_tk_widget = self
            .registry
            .as_deref()
            .and_then(|registry| registry.get(cmd_name))
            .is_some_and(|spec| {
                spec.required_package == Some(TK_PACKAGE)
                    && spec
                        .traits
                        .contains(tcl_registry::Traits::FIRE_AND_FORGET_TEARDOWN)
            });
        if tears_down_tk_widget {
            let paths: Vec<String> = args
                .iter()
                .filter(|path| *path == "." || is_widget_path(path))
                .cloned()
                .collect();
            let has_dynamic_path = args
                .iter()
                .any(|path| *path != "." && !is_widget_path(path));
            if has_dynamic_path {
                self.mark_tk_geometry_domain_uncertain(&domain, resolved);
            } else if path_uncertain {
                if paths.is_empty() {
                    self.mark_tk_geometry_domain_uncertain(&domain, resolved);
                } else {
                    for path in paths {
                        let affected: Vec<(String, String)> = self
                            .tk_domain_state(&domain, resolved)
                            .geometry_by_widget
                            .iter()
                            .filter(|(widget, _)| widget_is_within(widget, &path))
                            .map(|(widget, active)| (widget.clone(), active.container.clone()))
                            .collect();
                        if affected.is_empty() {
                            self.mark_tk_geometry_paths_uncertain(
                                &domain,
                                resolved,
                                Some(&path),
                                None,
                            );
                        }
                        for (widget, container) in affected {
                            self.mark_tk_geometry_paths_uncertain(
                                &domain,
                                resolved,
                                Some(&widget),
                                Some(&container),
                            );
                        }
                    }
                }
                return;
            } else {
                for path in paths {
                    let descendants: Vec<String> = {
                        let state = self.tk_domain_state(&domain, resolved);
                        state
                            .created_widgets
                            .iter()
                            .filter(|candidate| widget_is_within(candidate, &path))
                            .cloned()
                            .collect()
                    };
                    for descendant in descendants {
                        self.tk_domain_state(&domain, resolved)
                            .created_widgets
                            .remove(&descendant);
                        self.release_tk_geometry(&domain, resolved, &descendant);
                    }
                    self.clear_tk_geometry_uncertainty_for_path(&domain, resolved, &path);
                }
            }
            return;
        }

        // Track the active geometry state in source order. Tk permits one
        // content window to switch managers, and forget/remove releases its
        // old claim; only simultaneously active siblings can conflict.
        if let Some(geometry) = self.geometry_spec(cmd_name) {
            let Some(spec) = self
                .registry
                .as_deref()
                .and_then(|registry| registry.get(cmd_name))
            else {
                return;
            };

            let resolved_subcommand = args.first().and_then(|word| spec.resolve_subcommand(word));
            if let Some(subcommand) = resolved_subcommand
                && geometry.release_subcommands.contains(&subcommand.name)
            {
                if geometry.container_policy
                    != tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive
                {
                    return;
                }
                let release_paths: Vec<String> = args
                    .iter()
                    .skip(1)
                    .filter(|path| is_widget_path(path))
                    .cloned()
                    .collect();
                let has_dynamic_release = args.iter().skip(1).any(|path| !is_widget_path(path));
                if has_dynamic_release {
                    // A computed pathname may name any currently managed
                    // widget, so even a neighbouring literal release cannot
                    // keep the remaining container ownership exact.
                    self.mark_tk_geometry_domain_uncertain(&domain, resolved);
                    return;
                }
                for widget_path in release_paths {
                    let container = self
                        .tk_domains
                        .get(&domain)
                        .and_then(|state| {
                            state
                                .geometry_by_widget
                                .get(&widget_path)
                                .map(|active| active.container.clone())
                        })
                        .unwrap_or_else(|| parent_widget_path(&widget_path).to_owned());
                    if path_uncertain {
                        self.mark_tk_geometry_paths_uncertain(
                            &domain,
                            resolved,
                            Some(&widget_path),
                            Some(&container),
                        );
                    } else {
                        self.release_tk_geometry(&domain, resolved, &widget_path);
                        self.clear_tk_geometry_uncertainty_for_release(
                            &domain,
                            resolved,
                            &widget_path,
                            &container,
                        );
                    }
                }
                return;
            }

            let (target_start, options) =
                if geometry.direct_form && args.first().is_some_and(|path| is_widget_path(path)) {
                    (0, spec.options)
                } else if let Some(subcommand) = resolved_subcommand
                    && geometry.placement_subcommand == Some(subcommand.name)
                {
                    (1, subcommand.options)
                } else {
                    return;
                };
            let option_start = args
                .iter()
                .skip(target_start)
                .position(|word| tcl_registry::spec::resolve_option_prefix(options, word).is_some())
                .map_or(args.len(), |index| index + target_start);
            // Walk option/value groups, rather than searching every word:
            // an option value may itself look like `-in`. Tk processes
            // duplicate options in order, so the final `-in` wins.
            let explicit_container = geometry.container_option.and_then(|container_option| {
                let mut found = None;
                let mut index = option_start;
                while index < args.len() {
                    if args[index] == "--" {
                        break;
                    }
                    let Some(option) =
                        tcl_registry::spec::resolve_option_prefix(options, &args[index])
                    else {
                        index += 1;
                        continue;
                    };
                    let values = option.value_indices(args, index);
                    if option.name == container_option {
                        found = values.last().and_then(|value| args.get(*value));
                    }
                    index += 1 + values.len();
                }
                found
            });
            let has_dynamic_target = args
                .iter()
                .skip(target_start)
                .take(option_start.saturating_sub(target_start))
                .any(|path| !is_widget_path(path));
            let targets: Vec<(String, tcl_lexer::Span)> = args
                .iter()
                .skip(target_start)
                .take(option_start.saturating_sub(target_start))
                .enumerate()
                .filter(|(_, path)| is_widget_path(path))
                .map(|(index, path)| {
                    (
                        path.clone(),
                        arg_tokens
                            .get(index + target_start)
                            .map_or(cmd_tok.span, |token| token.span),
                    )
                })
                .collect();
            if geometry.container_policy
                == tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive
                && (has_dynamic_target
                    || targets.is_empty()
                    || explicit_container
                        .is_some_and(|container| container != "." && !is_widget_path(container)))
            {
                self.mark_tk_geometry_domain_uncertain(&domain, resolved);
                return;
            }
            for (widget_path, span) in targets {
                let parent = match explicit_container {
                    Some(container) if container == "." || is_widget_path(container) => {
                        (*container).clone()
                    }
                    Some(_) => continue,
                    None => parent_widget_path(&widget_path).to_string(),
                };
                if path_uncertain {
                    if geometry.container_policy
                        == tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive
                    {
                        self.mark_tk_geometry_paths_uncertain(
                            &domain,
                            resolved,
                            Some(&widget_path),
                            Some(&parent),
                        );
                    }
                    continue;
                }
                if self.tk_geometry_path_uncertain(&domain, resolved, &widget_path)
                    || self.tk_geometry_path_uncertain(&domain, resolved, &parent)
                {
                    continue;
                }
                self.release_tk_geometry(&domain, resolved, &widget_path);
                if geometry.container_policy
                    == tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive
                    && self.claim_tk_geometry(
                        &domain,
                        resolved,
                        &widget_path,
                        cmd_name,
                        &parent,
                        span,
                    )
                {
                    // A Tk geometry command stops at the first failed claim;
                    // later targets in the same command are not processed.
                    break;
                }
            }
        }
    }

    /// Mark only the widget/container that a branch-selected operation can
    /// affect.  A source-order walk has no branch join, but it can still keep
    /// unrelated containers precise.
    fn mark_tk_geometry_paths_uncertain(
        &mut self,
        domain: &str,
        resolved: bool,
        widget_path: Option<&str>,
        container: Option<&str>,
    ) {
        let state = self.tk_domain_state(domain, resolved);
        if let Some(path) = widget_path {
            state.uncertain_geometry_widgets.insert(path.to_owned());
        }
        if let Some(container) = container {
            state
                .uncertain_geometry_containers
                .insert(container.to_owned());
            if let Some(path) = widget_path {
                state
                    .uncertain_geometry_claims
                    .entry(path.to_owned())
                    .or_default()
                    .insert(container.to_owned());
            }
        }
    }

    fn mark_tk_geometry_domain_uncertain(&mut self, domain: &str, resolved: bool) {
        self.tk_domain_state(domain, resolved).geometry_uncertain = true;
    }

    fn tk_geometry_path_uncertain(&self, domain: &str, resolved: bool, path: &str) -> bool {
        let Some(state) = self.tk_domains.get(domain) else {
            return false;
        };
        if state.resolved != resolved {
            return true;
        }
        state.geometry_uncertain
            || state
                .uncertain_geometry_containers
                .iter()
                .any(|candidate| candidate == path)
            || state
                .uncertain_geometry_widgets
                .iter()
                .any(|candidate| widget_is_within(path, candidate))
    }

    fn clear_tk_geometry_uncertainty_for_release(
        &mut self,
        domain: &str,
        resolved: bool,
        widget_path: &str,
        _container: &str,
    ) {
        let state = self.tk_domain_state(domain, resolved);
        state.uncertain_geometry_widgets.remove(widget_path);
        state.uncertain_geometry_claims.remove(widget_path);
        Self::rebuild_uncertain_geometry_containers(state);
    }

    fn clear_tk_geometry_uncertainty_for_path(&mut self, domain: &str, resolved: bool, path: &str) {
        let state = self.tk_domain_state(domain, resolved);
        if path == "." {
            state.uncertain_geometry_widgets.clear();
            state.uncertain_geometry_containers.clear();
            state.uncertain_geometry_claims.clear();
            state.geometry_uncertain = false;
            return;
        }
        state
            .uncertain_geometry_widgets
            .retain(|candidate| !widget_is_within(candidate, path));
        state
            .uncertain_geometry_claims
            .retain(|candidate, _| !widget_is_within(candidate, path));
        Self::rebuild_uncertain_geometry_containers(state);
    }

    fn rebuild_uncertain_geometry_containers(state: &mut TkDomainState) {
        state.uncertain_geometry_containers.clear();
        for containers in state.uncertain_geometry_claims.values() {
            state
                .uncertain_geometry_containers
                .extend(containers.iter().cloned());
        }
    }

    fn release_tk_geometry(&mut self, domain: &str, resolved: bool, widget_path: &str) {
        let state = self.tk_domain_state(domain, resolved);
        let Some(active) = state.geometry_by_widget.remove(widget_path) else {
            return;
        };
        let Some(usage) = state.geometry.get_mut(&active.container) else {
            return;
        };
        usage.sites.retain(|(_, span)| *span != active.span);
        if !usage
            .sites
            .iter()
            .any(|(manager, _)| manager == &active.manager)
        {
            usage.managers.remove(&active.manager);
        }
        if usage.sites.is_empty() {
            state.geometry.remove(&active.container);
        }
    }

    /// Claim `container` for `manager`; returns true when Tk would reject the
    /// claim because another exclusive manager still has active content.
    fn claim_tk_geometry(
        &mut self,
        domain: &str,
        resolved: bool,
        widget_path: &str,
        manager: &str,
        container: &str,
        span: tcl_lexer::Span,
    ) -> bool {
        if self.tk_geometry_path_uncertain(domain, resolved, widget_path)
            || self.tk_geometry_path_uncertain(domain, resolved, container)
        {
            return false;
        }
        let conflict = self
            .tk_domain_state(domain, resolved)
            .geometry
            .get(container)
            .and_then(|usage| {
                usage
                    .sites
                    .iter()
                    .find(|(active, _)| active != manager)
                    .map(|(active, _)| active.clone())
            });
        if let Some(active) = conflict {
            self.tk_pending_diags
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::Tk1001,
                    span,
                    format!(
                        "Geometry manager conflict: cannot use '{manager}' in parent \
                         '{container}' while '{active}' still manages content there."
                    ),
                    Severity::Warning,
                ));
            return true;
        }

        let state = self.tk_domain_state(domain, resolved);
        let usage = state.geometry.entry(container.to_owned()).or_default();
        usage.managers.insert(manager.to_owned());
        usage.sites.push((manager.to_owned(), span));
        state.geometry_by_widget.insert(
            widget_path.to_owned(),
            TkActiveGeometry {
                manager: manager.to_owned(),
                container: container.to_owned(),
                span,
            },
        );
        false
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
        let Some(registry) = self.registry.as_deref() else {
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
                    // TK1003: an edit-distance guess at the intended option.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
            self.tk_pending_diags.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::Tk1003,
                    span,
                    message,
                    Severity::Hint,
                )
                .with_fixes(fixes),
            );
        }
    }

    /// Whether the walk recorded a statically-resolvable `package require Tk`.
    ///
    /// The exact activation fact, and the *only* input (besides the `tk`
    /// dialect) that decides whether a TK diagnostic is emitted.  It reads
    /// `result.package_requires`, which is populated generically by
    /// [`Analyser::handle_package_require`](super::state::Analyser) under the
    /// registry's [`PackageRequire`](tcl_registry::hooks::AnalyserHookId::PackageRequire)
    /// hook — so a `-exact` flag, a trailing version constraint, line
    /// continuations, a `package require` nested in a `namespace eval` / `if` /
    /// proc body, and comments, strings, and regexes that merely *mention* the
    /// words are all handled by the ordinary command walk rather than by a
    /// bespoke scanner here.
    ///
    /// Two documented limits, both inherited from the walk rather than added
    /// here, so per-item and full analysis still agree exactly:
    ///
    /// - a **dynamic** name (`package require [set p]`) is recorded verbatim
    ///   and cannot match; likewise a `package` reached after a `rename` or
    ///   hidden in a safe interpreter is not a resolvable `package require`;
    /// - `::package require Tk` does not match either, because
    ///   `resolve_analyser_hook_call` deliberately refuses a `::`-qualified
    ///   spelling of a bareword global command (pinned by issue #923), so no
    ///   `package_requires` entry is recorded for it at all — a pre-existing
    ///   false negative of the whole-file walk, not one this gate introduced.
    ///
    /// `pub(super)`: [`Analyser::analyse_per_item_with`](super::state::Analyser)
    /// consults the same fact post-walk to decide whether the incremental path
    /// must hand off to a full analysis (issue #1188).
    pub(super) fn has_tk_require(&self) -> bool {
        self.result
            .package_requires
            .iter()
            .any(|pr| pr.name == TK_PACKAGE)
    }

    /// Post-walk flush of all TK diagnostics.  Emits the buffered TK1002 /
    /// TK1003 and publishes TK1001 conflicts recorded by the temporal geometry
    /// transfer — but only
    /// when the document is actually Tk: the `tk` dialect, or a detected
    /// `package require Tk`.  Clears the accumulated state either way so a
    /// reused [`Analyser`] starts clean.
    pub(super) fn flush_tk_geometry_diagnostics(&mut self) {
        let _domains = std::mem::take(&mut self.tk_domains);
        let pending = std::mem::take(&mut self.tk_pending_diags);

        if !self.tk_dialect && !self.has_tk_require() {
            return;
        }

        self.result.diagnostics.extend(pending);
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
    fn tk_widget_paths_share_the_registry_grammar() {
        assert!(!has(
            "frame .main-pane\nframe .main-pane.child",
            "tk",
            "TK1002"
        ));
        assert!(!super::is_widget_path("."));
        assert!(!super::is_widget_path(".bad..child"));
        assert!(!super::is_widget_path(".dynamic$tail"));
        assert!(!super::is_widget_path(".dynamic[set tail]"));
        assert!(super::is_widget_path(".main-pane.child"));
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
    fn tk1001_abstains_across_mutually_exclusive_placements() {
        // The source-order walk must not claim that both managers are active:
        // only one arm executes for any one run of the script.
        let src = "frame .top\nif {$use_pack} {\n    pack .top.a\n} else {\n    grid .top.b\n}";
        assert!(!has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn tk1001_abstains_after_a_path_dependent_placement() {
        // A later placement cannot be proved to conflict when the earlier
        // manager was installed only on one control-flow path.
        let src = "frame .top\nif {$use_pack} {\n    pack .top.a\n}\ngrid .top.b";
        assert!(!has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn tk1001_keeps_unrelated_containers_definite_after_path_uncertainty() {
        let src = "frame .left\nframe .right\nif {$use_pack} {\n    pack .left.a\n}\npack .right.a\ngrid .right.b";
        assert!(has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn root_container_uncertainty_does_not_widen_to_nested_containers() {
        let src =
            "if {$use_pack} {\n    pack .root_child\n}\nframe .left\npack .left.a\ngrid .left.b";
        assert!(has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn conditional_constructor_does_not_poison_unrelated_geometry() {
        let src = "frame .left\nframe .right\nif {$make_child} {\n    frame .conditional\n}\npack .right.a\ngrid .right.b";
        assert!(has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn conditional_teardown_marks_active_geometry_container_uncertain() {
        let src = "frame .top\nframe .top.a\npack .top.a\nif {$remove} {\n    destroy .top.a\n}\ngrid .top.b";
        assert!(!has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn unrelated_release_does_not_clear_conditional_teardown_container() {
        let src = "frame .top\nframe .top.a\npack .top.a\nif {$remove} {\n    destroy .top.a\n}\npack forget .unrelated\ngrid .top.b";
        assert!(!has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn conditional_release_uses_explicit_in_container() {
        let src = "frame .holder\nframe .a\nframe .b\npack .a -in .holder\nif {$release} {\n    pack forget .a\n}\ngrid .b -in .holder";
        assert!(!has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn releasing_one_uncertain_in_claim_keeps_shared_container_uncertain() {
        let src = "frame .holder\nframe .a\nframe .b\nif {$place_a} {\n    pack .a -in .holder\n}\nif {$place_b} {\n    pack .b -in .holder\n}\npack forget .a\ngrid .c -in .holder";
        assert!(!has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn conditional_independent_place_does_not_poison_exclusive_geometry() {
        let src = "frame .left\nframe .right\nif {$place_it} {\n    place .left.a\n}\npack .right.a\ngrid .right.b";
        assert!(has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn definite_release_clears_path_scoped_geometry_uncertainty() {
        let src = "frame .left\nif {$use_pack} {\n    pack .left.a\n}\npack forget .left.a\npack .left.b\ngrid .left.c";
        assert!(has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn definite_destroy_and_recreate_clears_path_scoped_uncertainty() {
        let src = "frame .left\nif {$remove} {\n    destroy .left\n}\ndestroy .left\nframe .left\npack .left.a\ngrid .left.b";
        assert!(has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn tk1001_still_fires_for_definite_straight_line_placements() {
        let src = "frame .top\nif {$condition} { set noop 1 }\npack .top.a\ngrid .top.b";
        assert!(has(src, "tk", "TK1001"), "{:?}", codes(src, "tk"));
    }

    #[test]
    fn tk1001_quiet_for_pack_only() {
        let src = "frame .top\npack .top.a\npack .top.b";
        assert!(!has(src, "tk", "TK1001"));
    }

    #[test]
    fn tk1001_tracks_active_placements_not_command_history() {
        // Tk unmanages a content window from its old manager before handing
        // it to the new one; with no other packed sibling, this is legal.
        assert!(!has(
            "frame .top\nframe .top.a\npack .top.a\ngrid .top.a",
            "tk",
            "TK1001"
        ));
        // Explicit release likewise frees the container claim.
        assert!(!has(
            "frame .top\nframe .top.a\nframe .top.b\npack .top.a\npack forget .top.a\ngrid .top.b",
            "tk",
            "TK1001"
        ));
        // A query names a widget but never places it.
        assert!(!has(
            "frame .top\nframe .top.a\nframe .top.b\npack .top.a\ngrid info .top.b",
            "tk",
            "TK1001"
        ));
        // Switching one of two packed siblings leaves the other's pack claim
        // active, so grid is rejected.
        assert!(has(
            "frame .top\nframe .top.a\nframe .top.b\npack .top.a .top.b\ngrid .top.a",
            "tk",
            "TK1001"
        ));

        // Destroying the root tears down the entire Tk window hierarchy and
        // every active geometry claim. The root's spelling must not be
        // treated as the ordinary prefix `..`.
        assert!(!has(
            "frame .top\nframe .top.a\npack .top.a\ndestroy .\nframe .top\nframe .top.b\ngrid .top.b",
            "tk",
            "TK1001"
        ));
    }

    #[test]
    fn tk1001_uses_registry_container_policy_and_qualified_spellings() {
        // `place` manages content but does not call TkSetGeometryContainer, so
        // it can coexist with grid/pack. Qualified exclusive managers still
        // resolve to the same registry descriptors.
        assert!(!has(
            "frame .top\nplace .top.a\ngrid .top.b",
            "tk",
            "TK1001"
        ));
        assert!(has(
            "frame .top\n::pack .top.a\n::grid .top.b",
            "tk",
            "TK1001"
        ));
        // A non-manager command on the same parent is still not a conflict.
        assert!(!has(
            "frame .top\npack .top.a\nraise .top.b",
            "tk",
            "TK1001"
        ));
    }

    #[test]
    fn tk1001_uses_the_effective_in_container_and_every_direct_target() {
        let source = "frame .left\nframe .right\nframe .a\nframe .b\nframe .c\npack .a .b -in .left\ngrid .c -in .right";
        assert!(!has(source, "tk", "TK1001"));
        let conflict =
            "frame .holder\nframe .a\nframe .b\npack .a -in .holder\ngrid .b -in .holder";
        assert!(has(conflict, "tk", "TK1001"));
    }

    #[test]
    fn tk1001_abstains_when_a_mixed_release_can_remove_another_claim() {
        let source = "frame .left\nframe .right\nframe .left.a\nframe .right.a\nframe .right.b\npack .left.a\npack .right.a\npack forget .left.a $which\ngrid .right.b";
        assert!(!has(source, "tk", "TK1001"), "{:?}", codes(source, "tk"));
    }

    #[test]
    fn tk1001_abstains_when_a_mixed_placement_can_move_another_claim() {
        let source = "frame .left\nframe .right\nframe .left.a\nframe .right.a\nframe .right.b\npack .right.a\npack .left.a $which -in .left\ngrid .right.b";
        assert!(!has(source, "tk", "TK1001"), "{:?}", codes(source, "tk"));
    }

    #[test]
    fn tk1001_uses_the_last_duplicate_container_option() {
        let source = "frame .left\nframe .right\nframe .a\nframe .b\npack .a -in .left -in .right\ngrid .b -in .right";
        assert!(has(source, "tk", "TK1001"), "{:?}", codes(source, "tk"));
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
