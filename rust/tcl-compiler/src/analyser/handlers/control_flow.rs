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

//! Commands that run a body script: `foreach`, `switch`, `catch` and `uplevel`.

use super::super::state::Analyser;
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;
use tcl_lexer::{Token, TokenType};

impl Analyser {
    /// Handle `uplevel #0 { body }`: the script runs in the global
    /// frame, so the body's locals belong to a global-rooted scope
    /// rather than the enclosing proc.  Open an [`ScopeKind::Uplevel`]
    /// child scope (nested under the current scope, but tagged so
    /// completion / definition treat it as a global frame and ignore the
    /// proc's locals) and analyse the body there.
    ///
    /// Returns `true` for every single-braced-body `uplevel` form; a
    /// multi-word / non-literal script is left to the generic recursion (the
    /// W301 injection check already flags that shape).
    ///
    /// The frame the body runs in depends on the level word: `#0` is the global
    /// frame; `N` / `#N` / an implicit level is a *caller* frame that is
    /// statically unknown.  Either way the body's locals do **not** belong to
    /// the enclosing proc, so both open an [`ScopeKind::Uplevel`] child scope
    /// (tagged with the level word so variable resolution can tell global-frame
    /// from unknown-frame: `#0` resolves outward to the global namespace, a
    /// non-`#0` level abstains — the true frame can't be named).  Without the
    /// child scope the body's variables merged into the enclosing proc's
    /// locals, silently unifying two variables in different frames.
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Uplevel`];
    /// the level word is a shape check, not a command name.
    pub fn handle_uplevel_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        // Two literal-body shapes: `uplevel LEVEL {body}` (level word + single
        // braced body) and `uplevel {body}` (implicit level 1).  A `#0` level
        // records the global frame; anything else records an unknown caller
        // frame.  Multi-word scripts (`args.len() > 2`) fall through.
        let (level_word, body_tok, body_text) = match args.len() {
            2 => {
                let Some(bt) = arg_tokens.get(1).copied() else {
                    return false;
                };
                (args[0].clone(), bt, args[1].clone())
            }
            1 => {
                let Some(bt) = arg_tokens.first().copied() else {
                    return false;
                };
                ("1".to_owned(), bt, args[0].clone())
            }
            _ => return false,
        };
        // A literal `{…}` block, or a script *built* with `list` — the latter
        // is not dynamic (`uplevel #0 [list upvar #0 A B]` runs exactly one
        // deterministic command, tclsh-identical to the braced spelling), so
        // it opens the same frame scope.  Any other shape falls
        // through to the generic recursion.
        if body_tok.kind != TokenType::Str
            && !self.registry.as_deref().is_some_and(|r| {
                crate::script_arg::list_quoted_script_command(r, body_tok, &body_text).is_some()
            })
        {
            return false;
        }

        let path = scope_path.to_vec();
        let child_idx = {
            let Some(parent) =
                super::super::scope::scope_at_mut(&mut self.result.global_scope, &path)
            else {
                return false;
            };
            // The scope name carries the level word so `lookup_var_in_scope_chain`
            // can distinguish the `#0` global frame (resolve outward) from a
            // non-`#0` unknown caller frame (abstain outward).
            let mut child = super::super::types::Scope::new(
                super::super::types::ScopeKind::Uplevel,
                level_word,
            );
            // Only a literal `{…}` block may claim the token bytes as the
            // frame's `body_span` — same rule as `namespace eval` above.  A
            // `[list …]` build substitutes its elements in the *calling*
            // frame before `uplevel` changes frames, so an offset-keyed
            // lookup on (say) `$file` in `uplevel #0 [list source [file join
            // $file]]` must keep resolving lexically to the proc's own
            // parameter, not stop at this frame.
            child.body_span = (body_tok.kind == TokenType::Str).then_some(body_tok.span);
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path;
        child_path.push(child_idx);
        self.analyse_body(&body_text, body_tok, &child_path);
        true
    }

    /// Handle `foreach varList1 list1 ?varList2 list2 ...? body` (and the
    /// `foreach_in_collection` dialect variant, whose spec carries the same
    /// hook).
    ///
    /// Defines every `varListN` the registry's roles name (its repeated
    /// `LoopVarList` layout covers an unlimited number of `varList`/`list`
    /// pairs, tclsh 8.6/9.0-verified) in the active scope, through the one
    /// binder [`Self::handle_var_binding_command`], then recurses into the
    /// body so vars defined inside the loop land in the enclosing scope. What
    /// stays here is the literal-iteration simulation, the page's residue.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Foreach`].
    pub fn handle_foreach_command(
        &mut self,
        _cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 {
            return false;
        }
        // A single-pair `foreach VAR {literal list} { rename ::$VAR ... ;
        // proc ::$VAR ... }` loop over a fully literal list (the real
        // `tk/library/accessibility.tcl` rename-away-and-reinstall idiom)
        // binds `VAR` to a *different* value each
        // iteration — a fact the single-value `const_strings` scope map
        // can't represent generally (`foreach` shares one flat cell for the
        // whole loop, confirmed against tclsh: no per-iteration scope to key
        // a value under). Bind `VAR` to the *first* literal element before
        // the one normal body walk below, so `rename`/`proc`'s own
        // constant-fold (unchanged) resolves that iteration correctly;
        // `simulate_remaining_foreach_iterations` afterwards narrowly
        // re-dispatches just `rename`/`proc` for every *additional* literal
        // element, since those are the only two commands go-to-definition/
        // references/rename need to see resolved here.
        let list_braced = arg_tokens
            .get(1)
            .is_some_and(|tok| tok.kind == TokenType::Str);
        let literal_binding = (args.len() == 3)
            .then(|| {
                Self::literal_foreach_binding(&args[0], &args[1], list_braced, self.word_rules())
            })
            .flatten();
        if let Some((var, elements)) = &literal_binding
            && let Some(first) = elements.first()
        {
            self.set_const_string(var, first.clone(), arg_tokens[1].span, scope_path);
        }
        // The body is always the last argument; recurse so vars
        // defined inside the loop land in the enclosing scope.
        if let (Some(body_text), Some(body_tok)) = (args.last(), arg_tokens.last().copied()) {
            self.control_flow_body_depth += 1;
            let bound = literal_binding.as_ref().and_then(|(var, elements)| {
                Some((var.clone(), elements.first()?.clone(), body_tok.span))
            });
            let pushed = bound.is_some();
            self.literal_foreach_bindings.extend(bound);
            self.analyse_body(body_text, body_tok, scope_path);
            if pushed {
                self.literal_foreach_bindings.pop();
            }
            if let Some((var, elements)) = &literal_binding {
                self.simulate_remaining_foreach_iterations(var, elements, body_tok, scope_path);
            }
            self.control_flow_body_depth -= 1;
        }
        true
    }

    /// Whether a `foreach`'s only var/list pair (`var_list_text`/
    /// `list_text`) is the narrow, fully-literal shape
    /// [`Self::simulate_remaining_foreach_iterations`] can simulate: a
    /// single plain identifier (not itself dynamic, not a multi-name list)
    /// bound to a Tcl-list-valued sequence of literal (non-dynamic)
    /// elements. Returns `(var, elements)`.
    ///
    /// `list_braced` says whether the value word was brace-quoted. Braces
    /// suppress substitution across the whole word, so *every* element of a
    /// braced list is literal however many `$`/`[` characters it holds — the
    /// element text arrives with the braces stripped, and scanning it alone
    /// made one odd element abstain from the whole simulation.
    /// tclsh-proof (8.6.16 / 9.0.4):
    ///   foreach n {aa {$b} cc} { puts $n }   ->  aa / $b / cc
    /// i.e. the loop runs three times over literal values, and no read of
    /// `b` happens at any point.
    pub(super) fn literal_foreach_binding(
        var_list_text: &str,
        list_text: &str,
        list_braced: bool,
        rules: tcl_syntax::word_rules::WordValueRules,
    ) -> Option<(String, Vec<String>)> {
        let vars = rules.split_list(var_list_text).ok()?;
        let [var] = vars.as_slice() else {
            return None;
        };
        if crate::naming::is_dynamic_word(var) {
            return None;
        }
        // Parse the value as a real Tcl list, not `split_whitespace`: a
        // braced element like `{bar baz}` in `foo {bar baz}` is a *single*
        // list element (`bar baz`), and whitespace-splitting would mis-slice
        // it into `{bar` + `baz}` and bind the loop var to those bogus
        // fragments — the re-dispatched `rename`/`proc` handlers would then
        // invent command facts for `{bar`/`baz}` and miss the real `bar baz`
        // iteration. A malformed list (unbalanced
        // brace/quote) is not a valid `foreach` value at all — real Tcl
        // errors on it — so `split_list`'s `Err` means abstain entirely.
        let elements: Vec<String> = rules
            .split_list(list_text)
            .ok()?
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect();
        if elements.is_empty()
            || elements
                .iter()
                .any(|e| tcl_syntax::naming::word_is_dynamic(e, list_braced))
        {
            return None;
        }
        Some((var.to_string(), elements))
    }

    /// For each literal element *after* the first (the first iteration is
    /// already covered by [`Self::handle_foreach_command`]'s own
    /// pre-binding + the loop's one normal body walk), narrowly re-dispatch
    /// just the body's own **named-definition installers** with `var`
    /// temporarily rebound to that element — the commands whose registry
    /// spec carries [`tcl_registry::Traits::INSTALLS_NAMED_DEFINITION`],
    /// because those (and only those) name their target with a word whose
    /// substituted value changes per iteration, so each element installs a
    /// *different* definition (`proc` / `rename`, and `oo::define`'s class
    /// target). Which commands those are is
    /// registry data, never a name list here — a newly-stamped spec joins
    /// the simulation with no edit to this walker.
    ///
    /// Every *other* command in the body keeps the single evaluation the
    /// normal walk above already gave it — deliberately narrow: this does
    /// not generally re-walk
    /// the body per iteration (which would duplicate diagnostics/scope
    /// entries for everything else).
    ///
    /// Cost is `O(elements × body-commands)` with no fixpoint: the element
    /// list is a bounded literal and each element re-dispatches only the
    /// already-segmented installer commands.
    ///
    /// Leaves `var` bound to the *last* element afterwards — the same
    /// value real Tcl leaves the loop variable holding once `foreach`
    /// completes.
    pub(super) fn simulate_remaining_foreach_iterations(
        &mut self,
        var: &str,
        elements: &[String],
        body_tok: Token,
        scope_path: &[usize],
    ) {
        use tcl_registry::hooks::AnalyserHookId as Hook;

        if elements.len() < 2 || body_tok.kind != TokenType::Str {
            return;
        }
        // `body_tok`'s span must address real text in `self.source` — a
        // direct unit-level `handle_foreach_command` call (bypassing the
        // real tokeniser) can pass a synthetic span that doesn't, the same
        // out-of-bounds guard `Self::cmd_fragments` already needs for the
        // identical reason.
        let start = body_tok.span.start() as usize;
        let end = body_tok.span.end() as usize;
        if start >= self.source.len() || end > self.source.len() || start >= end {
            return;
        }
        let config = self.file_lexer_config();
        let segs: Vec<SegmentedCommand> = {
            let sm = Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let descended = descend_token(&sm, body_tok, config);
            segments_from_tree(descended.tree(), &sm)
        };
        // Which of the body's commands install a per-iteration name is a
        // fixed property of the segmented body, so classify once (registry
        // lookup per body command) rather than once per element.
        let installers: Vec<(SegmentedCommand, Hook)> = segs
            .iter()
            .filter_map(|seg| {
                let resolved = self.resolve_analyser_hook_call(seg.name(), seg.args())?;
                resolved
                    .traits
                    .contains(tcl_registry::Traits::INSTALLS_NAMED_DEFINITION)
                    .then(|| (seg.clone(), resolved.hook))
            })
            .collect();
        if installers.is_empty() {
            return;
        }
        for (index, element) in elements.iter().enumerate().skip(1) {
            let _reporting_iteration = self
                .finite_foreach_names
                .advance(var, body_tok, scope_path, index, element);
            self.set_const_string(var, element.clone(), body_tok.span, scope_path);
            self.literal_foreach_bindings
                .push((var.to_owned(), element.clone(), body_tok.span));
            for (seg, hook) in &installers {
                let args = seg.args();
                let arg_tokens = seg.arg_tokens();
                let arg_single = seg.arg_single_token();
                match hook {
                    Hook::Proc => {
                        self.handle_proc_command_at(
                            seg.name(),
                            args,
                            arg_tokens,
                            arg_single,
                            scope_path,
                            seg.argv[0],
                        );
                    }
                    Hook::Rename => {
                        self.handle_rename(
                            args,
                            arg_tokens,
                            arg_single,
                            scope_path,
                            seg.span.start(),
                        );
                    }
                    Hook::OoDefine => {
                        self.handle_oo_define_command(
                            seg.name(),
                            args,
                            arg_tokens,
                            arg_single,
                            scope_path,
                        );
                    }
                    Hook::OoObjdefine => {
                        if let Some(cmd_tok) = seg.argv.first().copied() {
                            self.handle_oo_objdefine_at(
                                cmd_tok, args, arg_tokens, arg_single, scope_path,
                            );
                        }
                    }
                    // Unreachable for a spec carrying the trait —
                    // `installer_hook_is_redispatched` pins that every one
                    // of them lands on an arm above, so a newly-stamped
                    // spec cannot silently promise a re-dispatch this match
                    // never delivers.
                    _ => {}
                }
            }
            self.literal_foreach_bindings.pop();
        }
    }

    /// Handle `switch ?options? string ?pattern body? ...`.
    ///
    /// Arity checking lives in `compiler_checks::arity_checks` via
    /// the IR; this handler walks each arm body so locals defined
    /// inside an arm land in the enclosing scope.
    ///
    /// Switch has two forms:
    ///
    /// 1. ``switch ?options? string pattern body ?pattern body? ...``
    ///    — pattern and body args alternate inline.
    /// 2. ``switch ?options? string {pattern body ?pattern body? ...}``
    ///    — pattern/body pairs live inside a single braced
    ///    block.  See [`crate::segmenter::flatten_clause_list_elements_with_config`]
    ///    for how that braced form is split.
    ///
    /// Bodies that are literally ``-`` are fall-through markers
    /// (the next arm's body fires) and are skipped — recursing
    /// into the literal ``-`` would produce a useless command.
    ///
    /// `-regexp` pattern recording: each non-`default`
    /// pattern arm is recorded as a `RegexPattern` with
    /// ``command = "switch"`` when its token is a literal
    /// (`Esc` / `Str`) or a `Var` token whose original substitution resolves
    /// through checked Logical declaration and value provenance.  Command-substitution patterns
    /// are skipped (runtime-computed values can't be statically
    /// resolved).
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Switch`].
    pub fn handle_switch_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        let Some(registry) = self.registry.as_deref() else {
            return true;
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some((case, invocation)) = registry.case_invocation(
            cmd_name,
            &arg_refs,
            Some(self.analysis_context().context().authoring_query()),
        ) else {
            // Preserve malformed-switch handling while allowing valid
            // subjectless Expect clause lists through the registry resolver.
            return args.len() >= 2;
        };
        let invocation_is_regexp = invocation.mode == tcl_registry::spec::CaseMatchMode::Regexp;
        if let Some(i) = invocation.clause_list_index {
            // Form 2 — single braced body containing all pairs.
            let body_text = args[i].clone();
            let Some(body_tok) = arg_tokens.get(i).copied() else {
                return true;
            };
            let clauses = crate::segmenter::flatten_case_list_clauses(
                &self.source,
                &body_text,
                body_tok,
                &case,
                self.lexer_config(),
            );
            let shape = tcl_syntax::case_list::CaseListShape {
                clause_flags: case.clause_flags,
                clause_value_flags: case.clause_value_flags,
            };
            let clause_regexp: Vec<bool> =
                tcl_syntax::case_list::split_case_list(&body_text, &shape)
                    .iter()
                    .map(|clause| {
                        clause.flags.iter().any(|flag| {
                            body_text
                                .get(flag.start..flag.end)
                                .and_then(|word| shape.resolve_flag(word))
                                == case.clause_regex_flag
                        })
                    })
                    .collect();
            let clause_count = clauses.len();
            for (clause_index, ((pat_text, pat_tok), (body_text, body_tok))) in
                clauses.iter().enumerate()
            {
                if invocation_is_regexp || clause_regexp.get(clause_index) == Some(&true) {
                    self.record_switch_regexp_pattern(
                        cmd_name,
                        &case,
                        pat_text,
                        *pat_tok,
                        (clause_index, clause_count),
                        scope_path,
                    );
                }
                if case.fallthrough_body != Some(body_text.as_str()) {
                    self.analyse_control_flow_body(body_text, *body_tok, scope_path);
                }
            }
        } else if let Some(i) = invocation.inline_clause_start {
            // Form 1 — pattern/body pairs inline in args/arg_tokens.
            let Some(clauses) = case.inline_clauses(&arg_refs, i) else {
                return true;
            };
            let clause_count = clauses.len();
            for (clause_index, clause) in clauses.iter().enumerate() {
                let pattern_index = clause.pattern_index;
                let body_index = clause.body_index;
                if (invocation_is_regexp
                    || clause.mode == tcl_registry::spec::CaseMatchMode::Regexp)
                    && let Some(pat_tok) = arg_tokens.get(pattern_index).copied()
                {
                    self.record_switch_regexp_pattern(
                        cmd_name,
                        &case,
                        &args[pattern_index],
                        pat_tok,
                        (clause_index, clause_count),
                        scope_path,
                    );
                }
                let Some(body_index) = body_index else {
                    continue;
                };
                let body_text = &args[body_index];
                if let Some(body_tok) = arg_tokens.get(body_index).copied()
                    && case.fallthrough_body != Some(body_text.as_str())
                {
                    self.analyse_control_flow_body(body_text, body_tok, scope_path);
                }
            }
        }
        true
    }

    /// Record one ``switch -regexp`` arm's pattern.  Skipped for
    /// the ``default`` keyword (Tcl's catch-all).  Literal tokens
    /// are recorded verbatim; `Var` tokens are resolved via the
    /// `const_strings` map (the ``regex-vars`` propagation);
    /// `Cmd` substitutions are skipped (can't statically
    /// resolve).
    fn record_switch_regexp_pattern(
        &mut self,
        command: &str,
        case: &tcl_registry::spec::CaseListSpec,
        pattern: &str,
        tok: Token,
        clause_position: (usize, usize),
        scope_path: &[usize],
    ) {
        let (clause_index, clause_count) = clause_position;
        if case.is_keyword_pattern(pattern, clause_index, clause_count) {
            return;
        }
        self.record_regex_pattern_token(pattern, tok, command, scope_path);
    }

    /// Handle `catch SCRIPT ?RESULTVAR? ?OPTIONSVAR?`.
    ///
    /// Defines the optional `RESULTVAR` and `OPTIONSVAR` bindings
    /// (they receive values when the body throws / completes) and
    /// bumps `conditional_depth` for the duration of the body.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Catch`].
    pub fn handle_catch_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        // The script body (args[0]) is evaluated by `catch`, so walk it like
        // every other body-bearing command — otherwise the per-command
        // syntactic checks (W100 unbraced `expr`, W104, W304, …) never reach
        // inside a `catch { … }`, under-reporting. `analyse_body` no-ops on a
        // dynamic body (`catch $cmd`).
        //
        // The body is a *guarded probe*: `catch { package require Foo }` is the
        // idiomatic optional-dependency check, so facts recorded inside it
        // (package requirements, …) must be marked conditional. Bump
        // `conditional_depth` for the walk, matching `if`/`try` bodies (and this
        // handler's own contract — see the doc comment above).
        if let Some(body_tok) = arg_tokens.first().copied() {
            self.conditional_depth += 1;
            self.analyse_body(&args[0], body_tok, scope_path);
            self.conditional_depth -= 1;
        }
        // The result-var / options-var positions come from the registry's
        // `ArgRole::VarWrite` rows on the `catch` spec, matching the nested
        // `[catch …]` path in `dispatch_nested_segment`. The literal spec
        // name is sound here: `AnalyserHookId::Catch` dispatch already
        // resolved the head (qualified spellings included) to this spec.
        if let Some(registry) = self.registry.as_deref() {
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            for i in registry.arg_indices_for_role(
                "catch",
                &arg_strs,
                tcl_registry::arg_role::ArgRole::VarWrite,
            ) {
                if let (Some(name), Some(tok)) = (args.get(i), arg_tokens.get(i)) {
                    let name = name.clone();
                    self.define_var(&name, *tok, scope_path, false, None);
                }
            }
        }
        true
    }
}
