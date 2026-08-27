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

//! W001 / E002 / E003 for Tk widget *instance* dispatch (`.t instate …`,
//! `$w tag configure …`) — the receiver-typed sibling of `validity.rs`'s
//! ordinary registry-command checks and `var_command.rs`'s `TclOO`
//! `$obj method` checks (issue #927;
//! `docs/design/tk-widget-instance-typing.md`).
//!
//! Two-phase, mirroring [`super::var_command`]'s cross-function post-pass
//! (not `tk_checks.rs`'s in-file buffering): a candidate is *recorded*
//! during the main walk wherever the ordinary registry-command resolution
//! in [`super::validity::emit_w001_unknown_subcommand`] already gives up on
//! `cmd_name` (so this module changes no existing control flow, only adds a
//! branch at an existing abstention point), then *resolved* after the
//! whole file has been walked, in [`Analyser::flush_widget_dispatch_diagnostics`].
//! The two-phase split is required, not stylistic: a helper proc that
//! dispatches `.t instate …` may be *defined* — and so walked — textually
//! before the `ttk::treeview .t` call that creates `.t`, even though the
//! proc cannot run until after it (e.g. `proc setup {} { .t instate … };
//! ttk::treeview .t; setup`) — so `instance_classes` is not reliably
//! complete until the walk is done.
//!
//! ## Soundness
//!
//! `AnalysisResult::instance_classes` is whole-file and name-keyed, exactly
//! the "any var named x is treated as one everywhere" shape
//! `docs/design/tcloo-object-typing.md` calls unsound for diagnostics — with
//! one difference that makes it safe to use here: `Analyser::bind_registry_instance_class`
//! (`commands.rs`) is collision-aware, so a name bound to two *different*
//! classes anywhere in the file is dropped from the map entirely rather than
//! silently keeping whichever write happened last. A dropped/absent name
//! simply fails resolution below (silent abstention, never a wrong answer).
//! The `$var` case additionally inherits the registry-command arity check's
//! `{*}`-expansion abstention (recorded once, in [`WidgetDispatchSite::has_expand`]).

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token};
use tcl_registry::{CommandRegistry, SubCommand};

use super::super::state::Analyser;
use super::super::types::Severity;
use super::validity::arity_verdict;

/// `configure`/`cget` are universal on every Tk widget instance (baked into
/// Tk's C-level `Tk_ConfigureWidget`) but modelled nowhere in any widget's
/// `subcommands` — each widget's own `-option` table is what would drive
/// their *value* completion/arity, which no widget spec declares today.
/// Treating them as unconditionally known (not arity-checked) is the
/// conservative choice: `docs/design/tk-widget-instance-typing.md` chose
/// silence over guessing at the pair/single-option arity shape.
fn is_universal_widget_subcommand(word: &str) -> bool {
    !word.is_empty() && ("configure".starts_with(word) || "cget".starts_with(word))
}

/// One buffered `.w <subcommand> …` / `$w <subcommand> …` dispatch site
/// whose head the ordinary registry-command resolution could not resolve —
/// recorded so it can be re-checked once `instance_classes` is complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetDispatchSite {
    /// The receiver text as written: a bareword widget path (`.t`, no
    /// leading `$`) or a `$var` reference (`w`, `$`/`${}` already
    /// stripped) — paired with `is_dollar` to disambiguate.
    pub receiver: String,
    pub is_dollar: bool,
    /// The subcommand word (`args[0]` at the dispatch site).
    pub subcommand: String,
    pub subcommand_span: Span,
    /// Positional argument count *after* the subcommand word.
    pub argc_after_subcommand: usize,
    /// The words after the subcommand, their tokens, and their `{*}` flags —
    /// what the E-R14 option-relation check reads. Recorded here because the
    /// receiver's class is not known until the post-walk flush, by which time
    /// the source words are gone.
    pub args_after_subcommand: Vec<String>,
    pub arg_tokens_after_subcommand: Vec<Token>,
    pub arg_expand_after_subcommand: Vec<bool>,
    /// `true` when the subcommand word or any argument after it is
    /// `{*}`-expanded — the runtime count is then unknowable, so arity
    /// abstains entirely (the unknown-subcommand check still runs: a
    /// literal subcommand word is still a literal regardless of what
    /// follows it).
    pub has_expand: bool,
    /// Whole-command span (E002 "too few" / W001 anchor).
    pub cmd_span: Span,
}

impl Analyser {
    /// Record a widget-dispatch candidate — called from
    /// [`super::validity::emit_w001_unknown_subcommand`] at the point it
    /// would otherwise silently abstain because `cmd_name` isn't a
    /// registered command. Cheap pre-filter: only a shape that could
    /// possibly be a tracked instance receiver (a `.`-prefixed bareword —
    /// [`super::super::tk_checks::is_widget_path`] — or any `$var`) is worth
    /// buffering; anything else is either a genuine unknown-command typo
    /// (W123's job) or cannot resolve regardless.
    pub(in crate::analyser) fn record_widget_dispatch_candidate(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        arg_tokens: &[Token],
        arg_expand_in: &[bool],
    ) {
        let (receiver, is_dollar) = if let Some(rest) = cmd_name.strip_prefix('$') {
            (
                rest.strip_prefix('{')
                    .and_then(|r| r.strip_suffix('}'))
                    .unwrap_or(rest),
                true,
            )
        } else if super::super::tk_checks::is_widget_path(cmd_name) {
            (cmd_name, false)
        } else {
            return;
        };
        let Some(subcommand) = args.first() else {
            return;
        };
        let Some(subcommand_tok) = arg_tokens.first() else {
            return;
        };
        // `arg_expand_in` is parallel to the full argv (head at index 0);
        // the subcommand is index 1, its trailing args start at index 2.
        let has_expand = arg_expand_in.get(1..).is_some_and(|e| e.iter().any(|&x| x));
        self.widget_dispatch_sites.push(WidgetDispatchSite {
            receiver: receiver.to_string(),
            is_dollar,
            subcommand: subcommand.clone(),
            subcommand_span: subcommand_tok.span,
            argc_after_subcommand: args.len().saturating_sub(1),
            args_after_subcommand: args.get(1..).unwrap_or(&[]).to_vec(),
            arg_tokens_after_subcommand: arg_tokens.get(1..).unwrap_or(&[]).to_vec(),
            // `arg_expand_in` is parallel to the full argv (head at index 0),
            // so the words after the subcommand start at index 2.
            arg_expand_after_subcommand: arg_expand_in.get(2..).unwrap_or(&[]).to_vec(),
            has_expand,
            cmd_span: cmd_tok.span,
        });
    }

    /// **W147 / W152 on an object-instance method** (E-R14).
    ///
    /// The instance-dispatch path is where `struct::tree`'s
    /// `walk -order in -type bfs` lives, and until E-R14 it had no
    /// option-relation consumer at all — the P5 note in
    /// `tcl-registry/src/commands/tcllib/data_structures.rs` recorded exactly
    /// that gap. The evaluation is the same shared, native one the ordinary
    /// command path uses ([`super::validity::option_relation_diagnostics`]);
    /// only the walk that finds the words differs, because a method's words
    /// start after the method name.
    fn emit_instance_option_relations(
        &mut self,
        class: &str,
        site: &WidgetDispatchSite,
        sub: &'static SubCommand,
    ) {
        if sub.option_relations.is_empty() && sub.constraints.is_none() {
            return;
        }
        let source = self.source.clone();
        let (options, positionals, complete) = super::validity::scan_invocation_words(
            &sub.option_specs(None, None),
            sub.option_placement,
            &site.args_after_subcommand,
            &site.arg_tokens_after_subcommand,
            &site.arg_expand_after_subcommand,
            &source,
            site.cmd_span,
        );
        let display_name = format!("{class} {}", site.subcommand);
        let relations: Vec<&'static tcl_registry::OptionRelation> =
            sub.option_relations.iter().collect();
        for (_, diagnostic) in super::validity::option_relation_diagnostics(
            &display_name,
            &relations,
            sub.constraints,
            &options,
            &positionals,
            complete,
            site.cmd_span,
        ) {
            self.result.diagnostics.push(diagnostic);
        }
    }

    /// Post-walk: resolve every buffered [`WidgetDispatchSite`] against the
    /// (now-complete) `instance_classes`/`created_instance_commands` and
    /// emit W001 / E002 / E003. A receiver that never resolves to a widget
    /// class (untracked, ambiguous, or a `TclOO`/tcllib registry class
    /// instead — those are `var_command.rs`'s job) is silently dropped, not
    /// diagnosed: this pass only *adds* checks for widget instance
    /// dispatch, it never re-litigates a call some other emitter already
    /// covers or intentionally abstains on.
    pub(in crate::analyser) fn flush_widget_dispatch_diagnostics(
        &mut self,
        registry: &CommandRegistry,
    ) {
        let sites = std::mem::take(&mut self.widget_dispatch_sites);
        for site in sites {
            let is_dollar = site.is_dollar;
            let Some(class) = self
                .result
                .instance_classes
                .get(&site.receiver)
                .filter(|_| {
                    is_dollar
                        || self
                            .result
                            .created_instance_commands
                            .contains(&site.receiver)
                })
            else {
                continue;
            };
            // Self-referential widgets have no `ObjectClassSpec` distinct
            // from their own `CommandSpec` — `registry.instance_method`
            // already resolves this (`object_class` → same `subcommands`
            // slice), so a class that resolves via the registry at all but
            // isn't a widget (a tcllib factory, `superclasses` non-empty,
            // …) is handled identically; nothing here is Tk-specific beyond
            // the receiver-tracking that fed `class`.
            let Some(spec) = registry.get(class) else {
                continue;
            };
            if is_universal_widget_subcommand(&site.subcommand) {
                continue;
            }
            // A class whose methods live on an `ObjectClassSpec` (every
            // tcllib factory: `struct::tree`, `struct::graph`, …) declares
            // them there, not in the creator command's own `subcommands`.
            // Reading only the latter reported every such method as unknown
            // — a false positive this fallback removes, and the reason the
            // option-relation check below could not reach an instance method
            // at all.
            let object_class = registry.object_class(class);
            let resolved = spec
                .subcommands
                .iter()
                .find(|s: &&SubCommand| s.name == site.subcommand)
                .or_else(|| {
                    registry
                        .instance_methods(class)
                        .into_iter()
                        .find(|s| s.name == site.subcommand)
                });
            let Some(sub) = resolved else {
                if object_class.is_some_and(|class| class.allow_unknown_methods) {
                    continue;
                }
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W001,
                        site.subcommand_span,
                        format!(
                            "Unknown subcommand '{}' for widget '{class}'",
                            site.subcommand
                        ),
                        Severity::Warning,
                    ));
                continue;
            };
            if site.has_expand {
                continue;
            }
            let class = class.clone();
            self.emit_instance_option_relations(&class, &site, sub);
            if let Some(diag) = arity_verdict(
                &format!("{class} {}", site.subcommand),
                sub.arity,
                site.argc_after_subcommand,
                false,
                site.cmd_span,
                None,
                sub.primary_synopsis(),
            ) {
                self.result.diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;

    fn codes(src: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn has(src: &str, code: &str) -> bool {
        codes(src).iter().any(|(c, _)| c == code)
    }

    #[test]
    fn w001_fires_for_unknown_widget_subcommand_bareword() {
        let src = "ttk::treeview .t\n.t bogus\n";
        assert!(has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn w001_fires_for_unknown_widget_subcommand_var() {
        let src = "set lb [listbox .l]\n$lb bogus\n";
        assert!(has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn w001_silent_for_known_widget_subcommand() {
        let src = "ttk::treeview .t\n.t instate {selected} {}\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn w001_silent_for_configure_and_cget_though_unmodelled() {
        // `configure`/`cget` are universal on every widget but appear in no
        // widget's `subcommands` table — must never be flagged.
        let src = "ttk::treeview .t\n.t configure -show tree\n.t cget -show\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn e002_fires_for_widget_subcommand_arity() {
        // `curselection` takes no further arguments.
        let src = "set lb [listbox .l]\n$lb curselection extra\n";
        assert!(has(src, "E003"), "{:?}", codes(src));
    }

    #[test]
    fn e002_fires_for_too_few_widget_subcommand_args() {
        // `move` requires exactly 3 args (item parent index).
        let src = "ttk::treeview .t\n.t move onlyone\n";
        assert!(has(src, "E002"), "{:?}", codes(src));
    }

    #[test]
    fn abstains_when_receiver_is_ambiguous_across_procs() {
        // `.t` is a treeview in one proc and a listbox in another —
        // `bind_registry_instance_class` drops the name as ambiguous, so
        // neither dispatch is diagnosed (never a confident wrong answer).
        let src = "proc a {} {\n    ttk::treeview .t\n    .t bogus\n}\nproc b {} {\n    listbox .t\n    .t alsobogus\n}\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn resolves_when_widget_created_after_the_proc_that_uses_it_is_defined() {
        // The proc dispatching `.t` is *defined* (and so walked) before the
        // `ttk::treeview .t` call that creates it — only valid because the
        // proc isn't actually *called* until after — proving the post-walk
        // two-phase design, not a single top-to-bottom pass, is what makes
        // this resolve.
        let src = "proc setup {} {\n    .t bogus\n}\nttk::treeview .t\nsetup\n";
        assert!(has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn abstains_for_untracked_bareword() {
        // `.q` was never created by anything — must not be confused with an
        // ordinary unknown-command (that's W123's job, not this module's).
        let src = ".q bogus\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn abstains_for_expanded_dispatch() {
        let src = "ttk::treeview .t\nset args {selected}\n.t instate {*}$args bogus_extra_bogus\n";
        // Whatever this resolves to, the `{*}`-expanded tail's true count
        // is unknowable — must not fire a false arity diagnostic.
        assert!(!has(src, "E002") && !has(src, "E003"), "{:?}", codes(src));
    }

    #[test]
    fn tcloo_dispatch_is_unaffected() {
        // A plain TclOO `$obj method` dispatch must keep going through
        // `var_command.rs`'s W308, not this module (registry.get(class)
        // finds no CommandSpec for a user class, so this module silently
        // no-ops for it).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d meow\n";
        assert!(has(src, "W308"), "{:?}", codes(src));
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }
}
