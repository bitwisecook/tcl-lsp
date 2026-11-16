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

//! `namespace ensemble`: the ensemble's subcommands, `-map` targets and prefix setting.

use super::super::state::Analyser;
use super::qualify;
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use tcl_lexer::{Span, Token, TokenType};
use tcl_syntax::list::find_element;

impl Analyser {
    /// Handle `namespace ensemble create` — record the namespace as
    /// an ensemble so its tail names become valid commands, plus an
    /// explicit `-command name` override when present.
    ///
    /// The implicit form (`namespace eval ::ens { namespace ensemble
    /// create }`) dispatches through a command named after the enclosing
    /// namespace; `-command NAME` instead creates the ensemble's dispatch
    /// command under an arbitrary, possibly differently-namespaced, name.
    /// Without recording that name too, a call through it (`myEns
    /// subcmd …`) resolves to nothing the analyser knows — drawing a
    /// spurious W123 and abstaining from arity checking for the wrong
    /// reason (an unresolved name) rather than the right one (a
    /// dynamically-defined ensemble the analyser can't see the
    /// subcommand map of).
    ///
    /// `namespace ensemble create`'s options are registry data
    /// (`ENSEMBLE_CREATE_OPTIONS` in `tcl-registry`'s `namespace_`
    /// module), not a hardcoded name list — walking by each option's
    /// declared value arity (rather than a bare `opt == "-command"` scan
    /// over every word) is what keeps another option's *value* word
    /// (`-map`'s dict, `-subcommands`' list, …) from ever being misread as
    /// `-command`'s own flag or value.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceEnsemble`]
    /// (stamped on `namespace`'s `ensemble` subcommand); `args[0]` is
    /// still the subcommand word, and only the `create` form (checked
    /// on `args[1]`) mutates anything.
    pub fn handle_namespace_ensemble(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        head: Option<u32>,
    ) {
        if args.len() < 2 {
            return;
        }
        // `create` operates on the *current* namespace, with no explicit name
        // argument; `configure NAME ...` (the ensemble side of the
        // `namespace ensemble configure -map` gap) names an *existing* ensemble
        // command explicitly, resolved like any other command reference.
        // Both share the same `-command`/`-map`/`-subcommands` option set.
        // registry-axis-ok: subcommands — namespace ensemble create and configure; until the resolved invocation's selected subcommand is read
        let is_create = args[1] == "create";
        // registry-axis-ok: subcommands — namespace ensemble create and configure; until the resolved invocation's selected subcommand is read
        let is_configure = args[1] == "configure";
        if !is_create && !is_configure {
            return;
        }
        // `namespace ensemble create` names the ensemble after the namespace
        // the command *resolves in*, not the lexically enclosing one: run
        // inside the body of `proc ::tk::foo {} {...}` declared at top level,
        // the current namespace is `::tk`, so the ensemble is `::tk` — the
        // purely lexical namespace walk skips proc scopes and homes it
        // to `::` instead, losing every `<ns> sub` call site.
        let ns = self.command_resolution_namespace(scope_path);
        let ns_prefix = ns.clone();

        let (opts, opt_tokens, configure_target) = if is_create {
            if !ns.is_empty() && ns != "::" {
                self.ensemble_namespaces.insert(ns.clone());
            }
            (&args[2..], arg_tokens.get(2..).unwrap_or(&[]), None)
        } else {
            let Some(name) = args.get(2) else { return };
            if crate::naming::is_dynamic_word(name) {
                return;
            }
            let target = self.resolve_command_qualified_name(name, scope_path);
            (
                args.get(3..).unwrap_or(&[]),
                arg_tokens.get(3..).unwrap_or(&[]),
                Some(target),
            )
        };

        // The ensemble's own resolved command name:
        // `-command NAME`, when present, *replaces* the default naming
        // entirely (tclsh 8.6.14-verified: `namespace ensemble create
        // -command ::alt` creates only `::alt`, never the enclosing
        // namespace's own name too) — so scan for it before recording any
        // `-map`, rather than defaulting eagerly the way `ensemble_namespaces`
        // (a broader, over-inclusive "valid command name" recovery set) does.
        let explicit_command = is_create
            .then(|| {
                opts.iter()
                    .enumerate()
                    // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
                    .find_map(|(i, o)| (o == "-command").then(|| opts.get(i + 1)).flatten())
            })
            .flatten()
            .filter(|v| !v.is_empty() && !crate::naming::is_dynamic_word(v))
            .map(|v| qualify(&ns_prefix, v));
        let ensemble_key = configure_target
            .or(explicit_command)
            .or_else(|| (is_create && !ns.is_empty() && ns != "::").then(|| ns.clone()));

        let option_specs: Vec<&tcl_registry::hover::OptionSpec> = self
            .registry
            .as_deref()
            .and_then(|r| r.get("namespace"))
            .and_then(|spec| spec.subcommand("ensemble").map(|sub| (spec, sub)))
            .map_or_default(|(spec, sub)| {
                self.analysis_context()
                    .context()
                    .available_sub_option_specs(spec, sub)
            });

        let mut i = 0usize;
        while i < opts.len() {
            let Some(spec) = option_specs.iter().find(|o| o.matches(opts[i].as_str())) else {
                i += 1;
                continue;
            };
            let value = opts.get(i + 1);
            let value_tok = opt_tokens.get(i + 1).copied();
            match spec.name {
                // `-command NAME` names the ensemble command — its namespace
                // is recorded so `<ns> sub` calls resolve.  A dynamic value
                // (`$var` / `[cmd]`) can't be resolved statically.
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
                "-command" => {
                    if let Some(value) = value
                        && !value.is_empty()
                        && !value.starts_with('$')
                        && !value.starts_with('[')
                    {
                        self.ensemble_namespaces.insert(qualify(&ns_prefix, value));
                    }
                }
                // `-map {sub target sub target …}` — every *target* (an
                // odd-indexed element) is a command the ensemble dispatches to,
                // recorded so it is reached by references / definition / rename
                // — and, keyed by the ensemble's own resolved command name, so
                // the W129 safe-interpreter gate can resolve a call through
                // this redirect to the target.
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
                "-map" => {
                    if let (Some(value), Some(tok)) = (value, value_tok)
                        && let Some((text, text_tok)) = self.ensemble_list_literal(value, tok)
                    {
                        self.record_ensemble_map_targets(
                            &text,
                            text_tok,
                            scope_path,
                            ensemble_key.as_deref(),
                            head,
                        );
                    }
                }
                // `-subcommands {a b c}` — each subcommand `a` dispatches to
                // the command `<ns>::a` in the ensemble's namespace.
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
                "-subcommands" => {
                    if let (Some(value), Some(tok)) = (value, value_tok)
                        && let Some((text, text_tok)) = self.ensemble_list_literal(value, tok)
                    {
                        self.record_ensemble_subcommands(
                            &text,
                            text_tok,
                            &ns_prefix,
                            ensemble_key.as_deref(),
                            head,
                        );
                    }
                }
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
                "-prefixes" => self.record_ensemble_prefixes(value, ensemble_key.as_deref()),
                _ => {}
            }
            i += 1 + spec.value_word_count(opts, i);
        }
    }

    /// Record a `namespace ensemble … -prefixes 0` configuration.
    ///
    /// `-prefixes 0` turns off this ensemble's `Tcl_GetIndexFromObj` prefix
    /// matching, so an abbreviated subcommand word there is a plain
    /// unknown-subcommand error rather than an abbreviation. Keyed by the
    /// ensemble's resolved command name so the abbreviation machinery (W145,
    /// the formatter's expansion) abstains on it.
    ///
    /// tclsh-proof (8.6.16): `namespace ensemble create -command ::e
    /// -subcommands {alpha} -prefixes 0` then `::e al` →
    /// `unknown subcommand "al": must be alpha`; without `-prefixes 0` the
    /// same call succeeds.
    fn record_ensemble_prefixes(&mut self, value: Option<&String>, ensemble_key: Option<&str>) {
        if let (Some(value), Some(key)) = (value, ensemble_key)
            && matches!(tcl_registry::abbrev::resolve_boolean(value), Some(false))
        {
            self.result.prefixless_ensembles.insert(key.to_owned());
        }
    }

    /// Resolve a `-map`/`-subcommands` option value to its statically-known
    /// list text + representative token, or `None` when nothing static can
    /// be extracted.
    ///
    /// A plain literal (`{sub target …}`) passes through unchanged — the
    /// pre-existing, common case, where `-command`'s own equivalent
    /// dynamic-value check (`starts_with('$')`/`starts_with('[')`) already
    /// guards a *whole*-value dynamic. `-map`/`-subcommands` additionally
    /// tolerate one dynamic *element* among literal ones today
    /// ([`Self::list_word_elements`]'s per-element `is_dynamic_word`), but a
    /// value that is itself one whole dynamic `[...]` substitution is not a
    /// list at all — naively word-splitting
    /// `[dict merge [namespace ensemble configure tk -map] {systray
    /// ::tk::systray}]` would misread fragments of the *expression*
    /// (`"tk"`, `"configure"`, …) as bogus subcommand/target pairs, which is
    /// worse than abstaining. Falls back to
    /// [`Self::dict_merge_literal_tail`] for the one dynamic shape real code
    /// needs; anything else abstains.
    fn ensemble_list_literal(&self, value: &str, value_tok: Token) -> Option<(String, Token)> {
        if !crate::naming::is_dynamic_word(value) {
            return Some((value.to_owned(), value_tok));
        }
        self.dict_merge_literal_tail(value_tok)
    }

    /// Recognise the `[dict merge EXISTING {literal}]`-shaped value the real
    /// `tk/library/systray.tcl` (and `print.tcl`, `fileicon.tcl`,
    /// `accessibility.tcl`) idiom uses to splice new entries onto a
    /// pre-existing ensemble's `-map`/`-subcommands` without a literal
    /// `{...}` value of its own: `namespace ensemble
    /// configure tk -map [dict merge [namespace ensemble configure tk -map]
    /// {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]`.
    /// `EXISTING` (whatever it evaluates to — typically a self-referential
    /// query of the ensemble's own current map) is left unknown, but the
    /// spliced literal tail is a statically known fact regardless of what
    /// `EXISTING` is.
    ///
    /// Deliberately narrow: exactly `dict merge ARG {literal}` (2 dict-merge operands, the
    /// second a literal word) — does not recognise `dict set`/`dict
    /// replace`/`concat`/a list-building helper proc, or a `dict merge`
    /// with more than 2 operands. A documented scope boundary, not an
    /// oversight (no attested real-world instance of those forms).
    fn dict_merge_literal_tail(&self, value_tok: Token) -> Option<(String, Token)> {
        if value_tok.kind != TokenType::Cmd {
            return None;
        }
        let config = self.file_lexer_config();
        let sm = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let descended = descend_token(&sm, value_tok, config);
        let segs = segments_from_tree(descended.tree(), &sm);
        let [seg] = segs.as_slice() else { return None };
        // registry-axis-ok: subcommands — a dict merge value shape; until the resolved invocation's selected subcommand is read
        if seg.texts.len() != 4 || seg.texts[0] != "dict" || seg.texts[1] != "merge" {
            return None;
        }
        let tail = seg.texts[3].clone();
        if crate::naming::is_dynamic_word(&tail) {
            return None;
        }
        seg.argv.get(3).map(|tok| (tail, *tok))
    }

    /// The `(element, span)` pairs of a list word's *top-level* Tcl-list
    /// elements — proper brace/quote-aware splitting
    /// ([`find_element`]), not naive whitespace splitting, so a braced
    /// multi-word element (`{source b.tcl}`, the shape a `-map` *target*
    /// commonly takes — see [`Self::record_ensemble_map_targets`]) comes
    /// back as one element instead of being shredded into stray fragments
    /// that no longer line up in pairs (a naive `split_whitespace` turns
    /// `-map {go {source b.tcl}}` into `["go", "{source", "b.tcl}"]`, an
    /// unmatched three-way split that silently drops the pairing entirely).
    /// Each element's span is
    /// located inside the token's content (`content_offset` skips the
    /// opening delimiter). A malformed trailing element (unmatched
    /// brace/quote, typically mid-edit) simply stops the scan early,
    /// matching this codebase's established lenient-list-parsing
    /// convention (`tcl_syntax::list::split_list_lenient`) rather than
    /// discarding everything already parsed. Shared by the ensemble
    /// `-map` / `-subcommands` extraction; a dynamic element is left for
    /// the caller to skip.
    fn list_word_elements(list_text: &str, tok: Token) -> Vec<(String, Span)> {
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let mut out = Vec::new();
        let mut pos = 0usize;
        while let Ok(Some(el)) = find_element(list_text, pos) {
            if let Some(text) = list_text.get(el.value.clone()) {
                let start = content_start + u32::try_from(el.value.start).unwrap_or(0);
                let end = content_start + u32::try_from(el.value.end).unwrap_or(0);
                out.push((text.to_string(), Span::new(start, end)));
            }
            pos = el.next;
        }
        out
    }

    /// The head word `(text, span)` of a command-prefix string — Tcl-list
    /// parses `text` and returns just its first element, the command
    /// actually invoked once the prefix's trailing words and the caller's
    /// own arguments are appended (mirrors
    /// `signature_scan::command_prefix::extract_prefix_head`'s
    /// braced-multi-word case, applied to a string with no lexer token of
    /// its own — a `-map` target sits *inside* another list element, not
    /// as a distinct token). `base_start` is `text`'s own absolute start
    /// offset in the source, so the returned span locates the head word
    /// there, not merely within `text`. `None` for an empty or malformed
    /// (unmatched brace/quote) prefix.
    fn command_prefix_head(text: &str, base_start: u32) -> Option<(String, Span)> {
        let head_el = find_element(text, 0).ok().flatten()?;
        let head = text.get(head_el.value.clone())?;
        if head.is_empty() {
            return None;
        }
        let start = base_start + u32::try_from(head_el.value.start).ok()?;
        let end = base_start + u32::try_from(head_el.value.end).ok()?;
        Some((head.to_string(), Span::new(start, end)))
    }

    /// Record every `-map` target (the odd elements of the `sub target …`
    /// list) as a command reference resolved in the caller's namespace, and
    /// — when `ensemble_key` is `Some` (the ensemble's own resolved
    /// qualified command name, computed by [`Self::handle_namespace_ensemble`])
    /// — also record each `sub -> target` pair into
    /// `self.ensemble_command_maps`, so the W129 safe-interpreter gate can
    /// resolve a call reaching a hidden command only through this ensemble's
    /// redirect (`None` when the ensemble's own name
    /// couldn't be resolved statically, matching every other guard in this
    /// handler), *and* — when the paired subcommand word is also static —
    /// file the same `sub -> target` fact under `ensemble_key` in
    /// [`AnalysisResult::ensemble_subcommand_targets`] so `definition`/
    /// `hover`/`references` can resolve a `<ensemble> <subcommand>` call site
    /// — a distinct field from `ensemble_command_maps`
    /// above, serving navigation rather than the safe-interpreter gate.
    ///
    /// The navigation entry is tagged
    /// [`EnsembleSubcommandProvenance::Map`](crate::signature_scan::types::EnsembleSubcommandProvenance::Map):
    /// a `-map` key is an arbitrary name, so a consumer that
    /// *rewrites* the subcommand word — rename — must leave it alone, unlike
    /// the `-subcommands` sibling below whose entry is the target's own tail.
    ///
    /// A `-map` target is a command name **or a command prefix** in real
    /// Tcl (tclsh 8.6.14-verified: `-map {go {string length}}` dispatches
    /// `myens go hello` to `string length hello`) — the command actually
    /// invoked is the prefix's *head*; the rest are baked-in arguments, not
    /// part of the command's identity. [`Self::command_prefix_head`]
    /// extracts just that head (recording the whole multi-word target text
    /// verbatim, or worse, splitting it on whitespace before pairing it with
    /// its subcommand at all, means
    /// a target like `{source b.tcl}` never matches the registry's bare
    /// `source` and W129 stays silently missed for this valid indirection
    /// shape) — the reference, the map entry, and the navigation entry all
    /// use only the head.
    ///
    /// Every `-map` value **replaces** the ensemble's entire subcommand
    /// table in real Tcl, whether given at `create` or a later
    /// `configure` (tclsh 8.6.14-verified: `configure myens -map {ok
    /// puts}` after `create ... -map {bad source ok puts}` turns `myens
    /// bad` into an "unknown or ambiguous subcommand" error, not a
    /// leftover redirect to `source`).  Merging new pairs into the existing
    /// cached map instead of replacing it would leave a subcommand a later
    /// `-map` dropped still resolving to its stale target, a false-positive
    /// risk. The cached
    /// map for `ensemble_key` is cleared before any of its new pairs are
    /// inserted.
    ///
    /// The reporting map supports navigation only. A selector token does not
    /// own the mapped target's original source/value producer and cannot issue
    /// an interpreter visibility subject or suppress invocation effects.
    fn record_ensemble_map_targets(
        &mut self,
        list_text: &str,
        tok: Token,
        scope_path: &[usize],
        ensemble_key: Option<&str>,
        command_head: Option<u32>,
    ) {
        if let Some(key) = ensemble_key {
            self.ensemble_command_maps
                .entry(key.to_string())
                .or_default()
                .clear();
        }
        for pair in Self::list_word_elements(list_text, tok).chunks(2) {
            let [(sub, _), (target, span)] = pair else {
                continue;
            };
            if crate::naming::is_dynamic_word(target) {
                continue;
            }
            let Some((head, head_span)) = Self::command_prefix_head(target, span.start()) else {
                continue;
            };
            if let Some(key) = ensemble_key {
                self.ensemble_command_maps
                    .entry(key.to_string())
                    .or_default()
                    .insert(sub.clone(), head.clone());
            }
            let resolved = self.resolve_command_qualified_name(&head, scope_path);
            if let Some(key) = ensemble_key
                && !crate::naming::is_dynamic_word(sub)
            {
                self.result
                    .ensemble_subcommand_targets
                    .entry(key.to_owned())
                    .or_default()
                    .insert(
                        sub.clone(),
                        super::super::types::EnsembleSubcommandTarget {
                            target: resolved.clone(),
                            provenance:
                                crate::signature_scan::types::EnsembleSubcommandProvenance::Map,
                        },
                    );
                self.ensemble_record_offsets
                    .insert(key.to_owned(), tok.span.start());
            }
            self.push_command_reference_with_policy(
                head,
                head_span,
                resolved,
                None,
                false,
                Self::ensemble_target_lookup(command_head, tok),
            );
        }
    }

    /// A target word of a configuring command with a known head is looked up
    /// at the ensemble's dispatches; without one it stays deferred.
    fn ensemble_target_lookup(
        head: Option<u32>,
        container: Token,
    ) -> crate::signature_scan::types::SignatureCommandLookup {
        head.map_or(
            crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
            |invocation_offset| {
                crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget {
                    invocation_offset,
                    container_start: container.span.start(),
                    container_end: container.span.end(),
                }
            },
        )
    }

    /// Record each `-subcommands` name as a reference to the command
    /// `<ns>::<name>` the ensemble maps it to, and file the same
    /// `subcommand → resolved target` fact under `ensemble_key` in
    /// [`AnalysisResult::ensemble_subcommand_targets`] — the `-subcommands`
    /// sibling of [`Self::record_ensemble_map_targets`] (same
    /// one-directional-only gap, same shape).
    ///
    /// Tagged
    /// [`EnsembleSubcommandProvenance::Subcommands`](crate::signature_scan::types::EnsembleSubcommandProvenance::Subcommands):
    /// here the subcommand word *is* the target's tail — the
    /// ensemble derives `<ns>::<name>` from it — so renaming the target must
    /// rewrite the entry and the dispatch word with it.
    fn record_ensemble_subcommands(
        &mut self,
        list_text: &str,
        tok: Token,
        ns_prefix: &str,
        ensemble_key: Option<&str>,
        command_head: Option<u32>,
    ) {
        for (elem, span) in Self::list_word_elements(list_text, tok) {
            if crate::naming::is_dynamic_word(&elem) {
                continue;
            }
            let resolved = qualify(ns_prefix, &elem);
            if let Some(key) = ensemble_key {
                self.result
                    .ensemble_subcommand_targets
                    .entry(key.to_owned())
                    .or_default()
                    .insert(
                        elem.clone(),
                        super::super::types::EnsembleSubcommandTarget {
                            target: resolved.clone(),
                            provenance: crate::signature_scan::types::
                                EnsembleSubcommandProvenance::Subcommands,
                        },
                    );
                self.ensemble_record_offsets
                    .insert(key.to_owned(), tok.span.start());
            }
            self.push_command_reference_with_policy(
                elem,
                span,
                resolved,
                None,
                false,
                Self::ensemble_target_lookup(command_head, tok),
            );
        }
    }
}
