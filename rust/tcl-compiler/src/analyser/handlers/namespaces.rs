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

//! `namespace eval`, `path`, `import`, `export`, `forget` and `unknown`.

use super::super::state::Analyser;
use tcl_dialect::model::{SpecSurface, surface_admits};
use tcl_lexer::{Token, TokenType};

impl Analyser {
    fn record_resolved_namespace_reference(
        &mut self,
        resolved_dynamic: Option<&str>,
        token: Option<&Token>,
        scope_path: &[usize],
        invocation_offset: Option<u32>,
    ) {
        if let (Some(resolved), Some(tok)) = (resolved_dynamic, token)
            && !self
                .result
                .namespace_refs
                .iter()
                .any(|reference| reference.span == tok.span)
        {
            // The general namespace role collector already retains genuine
            // computed inputs. Keep this presentation-only legacy record when
            // it has no independently retained input at the same source word.
            let here = self.command_resolution_namespace(scope_path);
            self.result.namespace_refs.push(
                super::super::types::NamespaceRef::from_original(
                    self.declaration_namespace_scope(scope_path),
                    self.declaration_name_policy(),
                    resolved,
                    tok.span,
                    u16::from(tok.content_offset),
                    true,
                    crate::naming::qualify(&here, resolved),
                )
                .with_original_input(match invocation_offset {
                    Some(offset) => self
                        .head_identities
                        .original_written_name_input_at_invocation_span(
                            &tcl_lexer::SourceImage::document(&self.source),
                            self.file_lexer_config(),
                            offset,
                            tok.span,
                        ),
                    None => self
                        .head_identities
                        .original_written_name_input_at_span(tok.span, self.file_lexer_config()),
                }),
            );
        }
    }

    /// Handle `namespace eval`: opens a new namespace scope and
    /// schedules its body for analysis.
    ///
    /// Returns `true` when the command was handled.
    ///
    /// Creates the child namespace scope so downstream handlers see
    /// qualified names resolve through it, then recurses into the
    /// body.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceEval`] (stamped
    /// on `namespace`'s `eval` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_eval_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        self.handle_namespace_eval_command_at_invocation(
            args, arg_tokens, arg_single, scope_path, None,
        )
    }

    /// Source-only namespace geometry from the actual command point, when
    /// dispatch retains one. Direct compatibility calls keep their ordinary
    /// span-based lookup without inventing an invocation offset.
    pub(in crate::analyser) fn handle_namespace_eval_command_at_invocation(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        invocation_offset: Option<u32>,
    ) -> bool {
        // Implementation contract: naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        if args.len() < 2 {
            return false;
        }
        let ns_name = args[1].clone();
        // **W314** — an all-colon segment (`namespace eval :`) creates a
        // namespace no absolute path can spell.
        if let Some(ns_tok) = arg_tokens.get(1) {
            self.emit_w314_unaddressable_namespace(&ns_name, ns_tok.span);
        }
        // The scope's `body_span` is what offset-keyed lookups resolve
        // *against* the namespace frame, so it may only cover text that runs
        // in that frame.  A literal `{…}` block does.  A `[…]` substitution
        // does **not**: Tcl evaluates it in the *calling* frame and hands
        // `namespace eval` the resulting value, so `namespace eval ::
        // [list source [file join $::tk_library $file.tcl]]` (Tk's
        // `::tk::SourceLibFile`) reads the proc's own `$file` parameter
        // before the namespace is entered at all.  Claiming those bytes for
        // the namespace made the scope-chain lookup stop there and answer
        // nothing for that read.
        let body_tok = arg_tokens.get(2).copied();
        let body_span = body_tok
            .filter(|t| t.kind == TokenType::Str)
            .map(|t| t.span);
        let body_text = args.get(2).cloned();
        // `namespace eval $ns [list namespace unknown $handler]` — the
        // list-wrapped installer idiom `analyse_body`'s literal-`{...}`
        // -only gate below never sees.
        if let Some(tok) = body_tok {
            self.detect_list_wrapped_namespace_unknown(tok);
        }

        // A dynamic target (`namespace eval $name { … }`, the irc.tcl
        // per-connection idiom) can't be resolved to a real namespace path —
        // and, unlike a literal one, two lexically unrelated occurrences
        // that happen to write the *same* variable name (an unremarkable
        // choice for this exact idiom) must never collapse into one scope:
        // each occurrence gets its own synthetic, per-call-site domain,
        // keyed by this argument token's own source offset (unique per
        // occurrence, deterministic, and — like `@interp@` — unrepresentable
        // in real Tcl, so it can never collide with a literal namespace of
        // the same written text). Mirrors `interp eval`'s dynamic-path
        // handling a few hooks below, which is conservatively isolated the
        // same way rather than merged by raw text.
        //
        // A computed target whose value is **constant-dominated** is not
        // dynamic in any way that matters: `set ns ::app; namespace eval $ns
        // { … }` creates `::app` on every run, so the block's procs really do
        // home to `::app::…` and the block really is a declaring site for
        // `::app`. That case is settled through the same identity-resolution
        // helper the command head, `source`, `rename`, and
        // `oo::define`'s target word already use — one lattice answers "what
        // does this word name" everywhere, and its dominance requirement keeps
        // a branch-conditional binding out.
        let resolved_dynamic = crate::naming::is_dynamic_word(&ns_name)
            .then(|| {
                self.resolve_dynamic_word(
                    &ns_name,
                    arg_tokens.get(1).copied(),
                    arg_single.get(1).copied().unwrap_or(false),
                    scope_path,
                )
            })
            .flatten()
            .filter(|resolved| !resolved.is_empty() && !crate::naming::is_dynamic_word(resolved));
        self.record_resolved_namespace_reference(
            resolved_dynamic.as_deref(),
            arg_tokens.get(1),
            scope_path,
            invocation_offset,
        );
        let scope_name = match resolved_dynamic {
            Some(resolved) => resolved,
            None if crate::naming::is_dynamic_word(&ns_name) => match arg_tokens.get(1) {
                Some(tok) => self.mint_synthetic_offset_name("@dynns@", tok.span.start()),
                None => ns_name.clone(),
            },
            None => ns_name,
        };

        let naming_scope = arg_tokens
            .get(1)
            .and_then(|token| {
                let input = match invocation_offset {
                    Some(offset) => {
                        self.original_variable_operand_input_at_invocation(offset, token.span)
                    }
                    None => self.original_variable_operand_input(token.span),
                }?;
                self.declaration_namespace_scope(scope_path)?
                    .child_from_input(&input)
            })
            .or_else(|| {
                self.original_interp_namespace_source_scope(
                    invocation_offset?,
                    arg_tokens,
                    scope_path,
                )
            });
        let path = scope_path.to_vec();
        let child_scope_idx = {
            let mut child = super::super::types::Scope::new(
                super::super::types::ScopeKind::Namespace,
                scope_name,
            );
            child.body_span = body_span;
            child.naming_scope = naming_scope;
            child.original_interpreter_source_domain =
                super::super::scope::scope_at(&self.result.global_scope, &path)
                    .and_then(|scope| scope.original_interpreter_source_domain.clone());
            // The written `NAME` word, so the outline can point its
            // `selectionRange` at the name rather than the whole body.
            // Recorded even for a dynamic `$ns` target: the
            // word is still where the user would want the cursor.
            child.name_span = arg_tokens.get(1).map(|t| t.span);
            let Some(parent) =
                super::super::scope::scope_at_mut(&mut self.result.global_scope, &path)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path;
        child_path.push(child_scope_idx);

        // Body recursion lets procs and classes declared inside
        // ``namespace eval`` register with the correct namespace
        // prefix.  Words past the body join into the script exactly as
        // `eval`'s do, so a multi-word call is analysed as the whole
        // concatenation or not at all — never as its first
        // word alone, which invented an E002 on the trailing words'
        // arguments and lost every write they performed.
        if args.len() == 3
            && let (Some(name), Some(body)) = (arg_tokens.get(1), body_tok)
        {
            self.record_original_vendor_variable_body(
                name.span,
                body.span,
                None,
                crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Namespace,
            );
        }
        if args.len() > 3 {
            self.analyse_namespace_eval_tail(args, arg_tokens, &child_path);
        } else if let (Some(text), Some(tok)) = (body_text, body_tok) {
            self.analyse_body(&text, tok, &child_path);
        }
        true
    }

    /// Analyse the script a multi-word `namespace eval` / `namespace inscope`
    /// evaluates, in the namespace scope `child_path` already opened for it.
    ///
    /// The two subcommands share this hook but not their tail semantics, and
    /// the registry records the split:
    ///
    /// - `namespace eval ns arg ?arg …?` carries
    ///   [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`] alone — the tail
    ///   space-joins into the script (`namespace eval ::n set l2 hello` sets
    ///   `::n::l2`, tclsh8.6.14/9.0.4-confirmed), so a fully-static tail is
    ///   joined and walked.
    /// - `namespace inscope ns script ?arg …?` adds
    ///   [`tcl_registry::Traits::SCRIPT_APPENDS_LIST_ARGS`] — the tail is
    ///   appended as *list elements*, so `namespace inscope :: {puts} {a b}`
    ///   prints `a b` where `namespace eval :: {puts} {a b}` errors. A join
    ///   would be simply wrong, and reconstructing the list quoting is beyond
    ///   what the analyser models, so any trailing word means the call is
    ///   consumed without walking.
    ///
    /// A dynamic word takes the same consume-without-walk route as
    /// [`Self::handle_interp_eval_command`]'s multi-word arm: substitution
    /// runs before concatenation, so the real script is unknowable.
    fn analyse_namespace_eval_tail(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        child_path: &[usize],
    ) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let list_append = registry
            .get("namespace")
            .and_then(|spec| spec.subcommand(&args[0]))
            .is_some_and(|sub| {
                sub.traits
                    .contains(tcl_registry::Traits::SCRIPT_APPENDS_LIST_ARGS)
            });
        if list_append {
            return;
        }
        let (Some(words), Some(tokens)) = (args.get(2..), arg_tokens.get(2..)) else {
            return;
        };
        let Some(first_tok) = tokens.first() else {
            return;
        };
        let Some((script, span)) =
            super::super::utils::concat_script_window(words, tokens, &self.source)
        else {
            // The tail cannot be walked (a dynamic word). A braced first word
            // is still a literal script prefix — concatenation appends after
            // it — and it is the namespace's whole visible body in the common
            // mangled-document case (an unbalanced brace inside the body word
            // drags trailing text into extra words). Walk it rather than
            // discarding every proc and variable the namespace declares.
            if first_tok.kind == TokenType::Str {
                self.analyse_body(&words[0], *first_tok, child_path);
            }
            return;
        };
        let anchor = Token::new(TokenType::Str, span);
        self.analyse_body(&script, anchor, child_path);
    }

    /// Handle `namespace path {…}`: record the current namespace's
    /// command-resolution search path for the post-walk settlement
    /// ([`Self::finalise_invocation_resolutions`]).
    ///
    /// Only the two-word set form
    /// with a *literal* list is recorded — the one-word query form mutates
    /// nothing, and a dynamic list (`$var` / `[cmd]`) is statically
    /// unknowable, so it keeps the conservative empty path. Each
    /// declaration replaces the namespace's whole path, as in C Tcl, so
    /// the lexically-last one wins (settlement is call-time / whole-file,
    /// like the rest of the resolution model).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespacePath`] (stamped
    /// on `namespace`'s `path` subcommand); `args[0]` is still the
    /// subcommand word.
    ///
    /// The argument is a *list* of namespace names inside one word, so each
    /// element is additionally recorded as a
    /// [`NamespaceRef`](super::super::types::NamespaceRef) at its own source span.
    /// Whole-word `ArgRole`s cannot express that — the
    /// word as a whole is not one namespace name — so the split happens here,
    /// where the semantics are already modelled, using the shared Tcl list
    /// grammar rather than a second scanner. That is also what makes a braced
    /// element (`namespace path {{my ns} ::b}`) come out right, which the
    /// previous `split_whitespace` did not.
    pub fn handle_namespace_path_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() != 2 {
            return;
        }
        if surface_admits(
            SpecSurface::TCL85_PLUS,
            Some(&self.analysis_context().context().authoring_query()),
        ) && let Some(token) = arg_tokens.get(1)
        {
            let config = self.file_lexer_config();
            let original = self
                .head_identities
                .original_written_name_input_at_span(token.span, config)
                .or_else(|| {
                    self.declaration_name_policy()
                        .and_then(|policy| {
                            self.head_identities.original_source_name_at_span(
                                token.span,
                                config,
                                self.word_rules(),
                                policy,
                            )
                        })
                        .map(|occurrence| {
                            crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                                occurrence.name_input().clone(),
                            )
                        })
                });
            if let Some(parent) = original {
                let here = self.command_resolution_namespace(scope_path);
                if let Some(entries) = self
                    .record_original_namespace_path_element_refs(&parent, token.span, scope_path)
                {
                    self.namespace_paths.insert(here, entries);
                } else {
                    // Byte names with no UTF-8 report cannot leave an older
                    // String path active. The original reference inventory is
                    // independent of this compatibility map.
                    self.namespace_paths.remove(&here);
                }
                return;
            }
            if self.declaration_name_policy().is_some() {
                self.result.namespace_path_computed.push(token.span);
                return;
            }
        }
        // The word arrives with its braces already stripped, so a text-only
        // dynamism scan mistakes `namespace path {$ns ::a}` for a computed
        // path and abstains from the whole command. Braces suppress
        // substitution, so the token kind is the authority.
        // tclsh-proof: tclsh8.6.14 —
        //   set nn {::$ns}; namespace eval $nn {}; namespace eval a {}
        //   namespace eval n { namespace path {::$ns ::a} }
        //   namespace eval n { namespace path }   ->  {::$ns} ::a
        // i.e. the entry is a namespace literally *named* `::$ns`; without
        // that namespace existing the same line errors with
        // `namespace "$ns" not found in "::n"` — never a variable read.
        let braced = arg_tokens
            .get(1)
            .is_some_and(|tok| tok.kind == TokenType::Str);
        if tcl_syntax::naming::word_is_dynamic(&args[1], braced) {
            // Record the abstention rather than merely staying silent: a
            // consumer that has to be exhaustive about namespace occurrences
            // (the namespace rename tier) cannot otherwise tell this document
            // from one that declares no path at all.
            if let Some(tok) = arg_tokens.get(1) {
                self.result.namespace_path_computed.push(tok.span);
            }
            return;
        }
        // The `namespace path` command-resolution tier is an 8.5 addition
        // (`NamespacePathCmd`, tclNamesp.c); 8.4 has no path tier, so a bare
        // call there never reaches a path namespace.  Recording the path under
        // a pre-8.5 dialect would make command resolution / definition / hover
        // falsely settle a call onto a path entry the runtime never consults —
        // so skip it, matching the `namespace path` subcommand's own dialect
        // gate (which already flags the command W002 there).
        if !surface_admits(
            SpecSurface::TCL85_PLUS,
            Some(&self.analysis_context().context().authoring_query()),
        ) {
            return;
        }
        let ns = self.command_resolution_namespace(scope_path);
        let Ok(elements) = self.word_rules().split_list(&args[1]) else {
            // A malformed list (an unbalanced brace) is not a path Tcl would
            // accept either; record nothing rather than half of one.
            return;
        };
        let entries: Vec<String> = elements.iter().map(ToString::to_string).collect();
        self.record_namespace_path_element_refs(
            &args[1],
            arg_tokens.get(1),
            braced,
            &ns,
            scope_path,
        );
        self.namespace_paths.insert(ns, entries);
    }

    fn record_original_namespace_path_element_refs(
        &mut self,
        parent: &crate::signature_scan::scope::SignatureSourceNameInput,
        parent_span: tcl_lexer::Span,
        scope_path: &[usize],
    ) -> Option<Vec<String>> {
        let Some(children) = parent.original_list_elements_with_source_spans() else {
            self.result.namespace_path_computed.push(parent_span);
            return None;
        };
        let context = self.declaration_namespace_scope(scope_path);
        let mut reports = Some(Vec::with_capacity(children.len()));
        for (input, extent) in children {
            let report = std::str::from_utf8(input.bytes()).ok();
            if let Some(value) = report {
                if let Some(reports) = &mut reports {
                    reports.push(value.to_owned());
                }
            } else {
                reports = None;
            }
            let span = extent.unwrap_or(parent_span);
            self.result.namespace_refs.push(
                super::super::types::NamespaceRef::from_original(
                    context.clone(),
                    Some(input.policy()),
                    report.unwrap_or(""),
                    span,
                    0,
                    false,
                    String::new(),
                )
                .with_original_input(Some(input)),
            );
        }
        reports
    }

    /// Record one [`NamespaceRef`](super::super::types::NamespaceRef) per element of
    /// a `namespace path {…}` list, at the element's own source span.
    ///
    /// Non-declaring: a path entry *refers* to a namespace, it does not
    /// create one (tclsh 8.6.16 / 9.0.4: `namespace path ::nope` fails
    /// `namespace "::nope" not found`, so the name must already exist).
    /// Rooting follows the ordinary relative rule — a relative entry is
    /// current-namespace-relative only, which is what
    /// `command_resolution_candidates` already assumes.
    ///
    /// Silently does nothing when the word is not a braced/bare literal the
    /// span arithmetic can trust (`token` absent, or the element offsets fall
    /// outside it).
    ///
    /// `word_braced` says whether the *whole* path word was brace-quoted. If
    /// it was, no element substitutes however many `$`/`[` characters it
    /// holds, so the per-element dynamism scan must be suppressed exactly as
    /// the whole-word one already is — otherwise `namespace
    /// path {::$ns ::a}` records `::$ns` as a path entry but not as a
    /// reference, and the same function disagrees with itself.
    fn record_namespace_path_element_refs(
        &mut self,
        raw: &str,
        token: Option<&Token>,
        word_braced: bool,
        here: &str,
        scope_path: &[usize],
    ) {
        let Some(token) = token else { return };
        let Ok(decoded) = self.word_rules().split_list(raw) else {
            return;
        };
        let mut decoded = decoded.into_iter();
        let base = token.span.start() + u32::from(token.content_offset);
        let original_parent = self
            .head_identities
            .original_written_name_input_at_span(token.span, self.file_lexer_config());
        let mut scan = 0usize;
        let mut ordinal = 0usize;
        while let Ok(Some(el)) = tcl_syntax::list::find_element(raw, scan) {
            let Some(text) = decoded.next() else { break };
            let original_input = original_parent
                .as_ref()
                .and_then(|parent| parent.original_list_element(ordinal));
            ordinal += 1;
            let (start, end) = (el.value.start, el.value.end);
            if el.next <= scan {
                break;
            }
            scan = el.next;
            if text.is_empty() || tcl_syntax::naming::word_is_dynamic(&text, word_braced) {
                continue;
            }
            let Ok(start_u32) = u32::try_from(start) else {
                continue;
            };
            let Ok(end_u32) = u32::try_from(end) else {
                continue;
            };
            self.result.namespace_refs.push(
                super::super::types::NamespaceRef::from_original(
                    self.declaration_namespace_scope(scope_path),
                    self.declaration_name_policy(),
                    &text,
                    tcl_lexer::Span::new(base + start_u32, base + end_u32),
                    0,
                    false,
                    crate::naming::qualify(here, &text),
                )
                .with_original_input(original_input),
            );
        }
    }

    /// Record `namespace import ?-force? PATTERN ...` declarations.
    ///
    /// Literal patterns are recorded in ``result.namespace_imports`` (the
    /// W123 unresolved-command pass suppresses an unqualified call whose
    /// name glob-matches a recorded pattern's tail); a *dynamic* pattern
    /// (``$``/``[…]`` substitution) can't be qualified statically, so it
    /// flips ``has_dynamic_providers`` instead — the imported namespace may
    /// provide any name at runtime, which suppresses W120/W123 file-wide.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceImport`] (stamped
    /// on `namespace`'s `import` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_import_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        // Skip the subcommand word + the leading flag words the registry says
        // `namespace import` consumes (`-force`, at most one). Both facts are
        // read from the spec rather than matched by name or bounded by a
        // literal here — see [`Self::namespace_leading_flag_words`]. That the
        // option word was consumed *is* `-force` (`IMPORT_OPTIONS` declares
        // exactly one option and `max_leading_option_words` caps it at one),
        // so the conflict semantics below need no name match either — the
        // same reading `namespace export`'s `-clear` tombstone already gets.
        let Some(first) = arg_tokens
            .get(1)
            .and_then(|token| self.original_source_namespace_operand(token.span, scope_path))
        else {
            return;
        };
        let first_bytes = [first.name_input().bytes()];
        let Some((forced, _)) = tcl_registry::NamespaceTransition::import_pattern_byte_operands(
            &first_bytes,
            first.name_input().policy(),
        ) else {
            return;
        };
        let mut idx = 1 + usize::from(forced);
        // `namespace import` imports into the namespace current at the call —
        // the command-resolution namespace, so an import inside
        // `proc ::ns::p {}` lands in `::ns`.
        let importing_ns = self.command_resolution_namespace(scope_path);
        while idx < args.len() && idx < arg_tokens.len() {
            let pat_raw = args[idx].clone();
            let original = self
                .original_source_namespace_operand(arg_tokens[idx].span, scope_path)
                .and_then(|operand| {
                    crate::signature_scan::original_name::SourceNamespacePattern::from_original(
                        operand,
                        tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
                        arg_tokens[idx].span,
                        forced,
                    )
                });
            let Some(original) = original else {
                if pat_raw.contains(['$', '[']) {
                    self.result.has_dynamic_providers = true;
                }
                idx += 1;
                continue;
            };
            let source =
                crate::signature_scan::types::SignatureNamespaceImportSource::from_native_pattern(
                    original.parts(),
                );
            self.result.original_namespace_patterns.push(original);
            let Some(source) = source else {
                idx += 1;
                continue;
            };
            let pat = source.constructed_pattern();
            self.result.namespace_imports.push(
                crate::signature_scan::types::SignatureNamespaceImport {
                    ns: importing_ns.clone(),
                    pattern: pat,
                    source: Some(source),
                    range: arg_tokens[idx].span,
                    conjectured: false,
                    forced,
                },
            );
            idx += 1;
        }
    }

    /// Record `namespace forget ?PATTERN ...?` events — the removal half of
    /// the import edge's lifecycle log.
    ///
    /// A `namespace import` is not a permanent name: `namespace forget`
    /// removes the alias and a later bare call raises `invalid command name`
    /// (oracle in
    /// [`crate::signature_scan::types::SignatureNamespaceForget`], tclsh
    /// 8.6.14 / 9.0.4 byte-identical). Recorded as an ordered event beside
    /// the `namespace export` tombstones so the resolvers in `tcl-lsp-core`
    /// can ask "does this namespace still hold an alias for NAME *here*?"
    /// rather than "was one ever installed"
    /// (`tcl_lsp_core::namespace_import::alias_live_at`).
    ///
    /// The pattern shapes follow `Tcl_ForgetImport` exactly: a qualified
    /// pattern names the *source* namespace whose commands lose their import
    /// here, a simple one matches this namespace's own imported command
    /// names whatever their origin. Both are oracle-confirmed; see the record
    /// type.
    ///
    /// A dynamic pattern (`$`/`[` substitution) can't be statically resolved
    /// to a glob text and is skipped rather than guessed at: revoking an
    /// alias on a guess would silently drop real references, the same
    /// abstain-toward-answering direction the unordered `-clear` takes.
    /// `namespace forget` declares no options, so — unlike `import` /
    /// `export` — there is no leading flag word to consume; the registry's
    /// own [`Self::namespace_leading_flag_words`] is still what says so.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceForget`] (stamped on
    /// `namespace`'s `forget` subcommand); `args[0]` is still the subcommand
    /// word.
    pub fn handle_namespace_forget_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        let mut idx = 1 + self.namespace_leading_flag_words("forget", &args[1..]);
        // As for `import` / `export`: the namespace losing the aliases is the
        // one current at the call, not the lexically enclosing one.
        let forgetting_ns = self.command_resolution_namespace(scope_path);
        while idx < args.len() && idx < arg_tokens.len() {
            let original = self
                .original_source_namespace_operand(arg_tokens[idx].span, scope_path)
                .and_then(|operand| {
                    crate::signature_scan::original_name::SourceNamespacePattern::from_original(
                        operand,
                        tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern,
                        arg_tokens[idx].span,
                        false,
                    )
                });
            let Some(original) = original else {
                idx += 1;
                continue;
            };
            let parts = original.parts().clone();
            self.result.original_namespace_patterns.push(original);
            let source_ns =
                crate::signature_scan::types::SignatureNamespaceImportSource::from_native_pattern(
                    &parts,
                )
                .map(|source| source.namespace);
            let Ok(pattern) = parts.tail.try_utf8() else {
                idx += 1;
                continue;
            };
            self.result.namespace_forgets.push(
                crate::signature_scan::types::SignatureNamespaceForget {
                    ns: forgetting_ns.clone(),
                    source_ns,
                    pattern: pattern.to_owned(),
                    range: arg_tokens[idx].span,
                },
            );
            idx += 1;
        }
    }

    /// Record `namespace export ?-clear? PATTERN ...` declarations.
    ///
    /// Real Tcl's `namespace import NS::*` only ever imports names `NS` has
    /// actually exported (`Tcl_Export`, `tclNamesp.c`) — an unexported
    /// sibling command living in `NS` is not reachable through the import at
    /// all (tclsh9.0/8.6-verified: `invalid command name` calling it bare).
    /// Recorded in
    /// `result.namespace_exports` so the wildcard-import bareword resolvers
    /// in `tcl-lsp-core` (same-document) and the workspace index
    /// (cross-document) can gate a would-be import target on whether its
    /// source namespace actually exports it.
    ///
    /// `-clear` is recorded as an ordered **tombstone** entry rather than
    /// applied by dropping the namespace's earlier entries: an import
    /// snapshots the export list as it stood when the import ran, so a
    /// `-clear` written *after* an import must not revoke what that import
    /// already bound (oracle in
    /// [`crate::signature_scan::types::SignatureNamespaceExport`]).
    /// Collapsing it eagerly destroys exactly the ordering the snapshot needs. Consumers reconstruct the
    /// export set as of an offset with
    /// `tcl_lsp_core::namespace_import::exported_at_import_site`.
    ///
    /// A dynamic pattern (`$`/`[` substitution) can't be statically resolved
    /// to a glob text, so it is silently skipped — the wildcard-import
    /// resolver then correctly abstains for names it might have covered,
    /// rather than guessing. A dynamic word cannot hide a `-clear`, which is
    /// a literal flag word or nothing. It is skipped rather than treated as
    /// the abort below, for the same reason: whether it resolves to a
    /// qualified pattern is unknowable here.
    ///
    /// A **qualified** pattern is a guaranteed runtime error (`Tcl_Export`
    /// rejects it), and C `return`s from the pattern loop on that failure, so
    /// recording stops there — see the loop for the oracle.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceExport`] (stamped on
    /// `namespace`'s `export` subcommand); `args[0]` is still the subcommand
    /// word.
    pub fn handle_namespace_export_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        self.record_original_namespace_exports(args, arg_tokens, scope_path);
        let mut idx = 1;
        // As for `import`: the exporting namespace is the one current at the
        // call, not the lexically enclosing one.
        let exporting_ns = self.command_resolution_namespace(scope_path);
        // Which leading words are flags — and how many of them the command
        // consumes — is registry data (`EXPORT_OPTIONS` plus
        // `max_leading_option_words`), not a name match or a loop bound here.
        // `-clear` is the only option and at most one is consumed, so a
        // second `-clear` falls through to the pattern loop below and is
        // recorded as the export pattern real Tcl treats it as. Each consumed
        // flag lands as a tombstone event at its own token so the ordering
        // survives.
        let flag_words = self.namespace_leading_flag_words("export", &args[1..]);
        for _ in 0..flag_words {
            if idx >= args.len() || idx >= arg_tokens.len() {
                break;
            }
            self.result.namespace_exports.push(
                crate::signature_scan::types::SignatureNamespaceExport {
                    ns: exporting_ns.clone(),
                    pattern: String::new(),
                    range: arg_tokens[idx].span,
                    clears: true,
                },
            );
            idx += 1;
        }
        while idx < args.len() && idx < arg_tokens.len() {
            let pattern = args[idx].clone();
            if pattern.contains('$') || pattern.contains('[') {
                idx += 1;
                continue;
            }
            let selected = self
                .declaration_name_dialect()
                .authored_name_policy()
                .and_then(|policy| {
                    policy
                        .recipe()
                        .namespace_pattern_input(
                            pattern.as_bytes(),
                            tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
                        )
                        .ok()
                        .and_then(|projection| {
                            std::str::from_utf8(projection.selected())
                                .ok()
                                .map(str::to_owned)
                        })
                });
            let Some(pattern) = selected else {
                idx += 1;
                continue;
            };
            // `NamespaceExportCmd` calls `Tcl_Export` per pattern and
            // `return`s on the first failure, so a qualified pattern aborts
            // the loop: every *later* word is never exported at all. The
            // earlier ones stay — C commits each pattern as it goes.
            // Oracle (tclsh 8.6.16 / 9.0.4): `namespace export one ::bad
            // three` leaves exactly `one` exported, and `namespace export
            // ::bad ok` leaves nothing. Recording past the bad word let a
            // wildcard-import bareword resolve where real Tcl would not.
            if tcl_syntax::naming::is_qualified(pattern.as_bytes()) {
                break;
            }
            self.result.namespace_exports.push(
                crate::signature_scan::types::SignatureNamespaceExport {
                    ns: exporting_ns.clone(),
                    pattern,
                    range: arg_tokens[idx].span,
                    clears: false,
                },
            );
            idx += 1;
        }
    }

    fn original_source_namespace_operand(
        &self,
        span: tcl_lexer::Span,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNamespaceOperand> {
        self.head_identities
            .original_namespace_operand_at_span_in_source(
                &tcl_lexer::SourceImage::document(&self.source),
                span,
                self.file_lexer_config(),
            )
            .or_else(|| {
                let original = self.original_static_source_name_at_span(span)?;
                (original.name_input().policy() == self.declaration_name_policy()?)
                    .then_some(())?;
                crate::signature_scan::original_name::SourceOriginalNamespaceOperand::from_conditional_declaration(
                    original,
                    self.conditional_source_namespace_scope(scope_path)?,
                )
            })
    }

    fn conditional_source_namespace_scope(
        &self,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        let policy = self.declaration_name_policy()?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let mut selected = self.declaration_namespace_scope(&[])?;
        let mut scope = &self.result.global_scope;
        for &index in scope_path {
            scope = scope.children.get(index)?;
            if scope.kind != super::super::types::ScopeKind::Namespace {
                return None;
            }
            let name = self.original_static_source_name_at_span(scope.name_span?)?;
            let key = name.name_input();
            if key.policy() != policy
                || key.lexer_config() != self.file_lexer_config()
                || key.source_image() != &image
            {
                return None;
            }
            selected =
                selected.child_from_input(&SignatureSourceNameInput::OriginalWord(key.clone()))?;
            if scope.naming_scope.as_ref() != Some(&selected) {
                return None;
            }
        }
        Some(selected)
    }

    fn record_original_namespace_exports(
        &mut self,
        args: &[String],
        tokens: &[Token],
        scope_path: &[usize],
    ) {
        use crate::signature_scan::original_name::SourceNamespaceExport;
        let mut operands = Vec::new();
        for token in tokens.iter().take(args.len()).skip(1) {
            let Some(operand) = self.original_source_namespace_operand(token.span, scope_path)
            else {
                self.result
                    .original_namespace_export_unknowns
                    .push(token.span);
                return;
            };
            operands.push((operand, token.span));
        }
        let Some((first, _)) = operands.first() else {
            return;
        };
        let policy = first.name_input().policy();
        if operands.iter().any(|(operand, _)| {
            operand.name_input().policy() != policy
                || operand.context() != first.context()
                || operand.site() != first.site()
        }) {
            return;
        }
        let bytes = operands
            .iter()
            .map(|(operand, _)| operand.name_input().bytes())
            .collect::<Vec<_>>();
        let Some((clear, _)) =
            tcl_registry::NamespaceTransition::export_pattern_byte_operands(&bytes, policy)
        else {
            return;
        };
        for (index, (operand, span)) in operands.into_iter().enumerate() {
            let Some(event) =
                SourceNamespaceExport::from_original(operand, span, clear && index == 0)
            else {
                // A qualified pattern aborts this handler after committing its
                // preceding pattern events; it supplies no enclosing Normal.
                break;
            };
            self.result.original_namespace_exports.push(event);
        }
    }

    /// How many of `args`' leading words `namespace SUB` consumes as option
    /// flags, entirely from registry data: a word that matches a declared
    /// flag option of that subcommand, capped by the subcommand's declared
    /// [`tcl_registry::SubCommand::max_leading_option_words`].
    ///
    /// `args` is the word list *after* the subcommand word.
    ///
    /// Two registry facts, no literal in the walker:
    ///
    /// - **Which** words are flags — `namespace import`'s `-force` and
    ///   `namespace export`'s `-clear` (`IMPORT_OPTIONS` / `EXPORT_OPTIONS`).
    ///   Matching is exact (`OptionSpec::matches`, canonical name or declared
    ///   alias), never the generic unique-prefix rule, because both
    ///   subcommands hand-parse with `strcmp` in C: oracle (tclsh 8.6.14 /
    ///   9.0.4) `namespace export -c p` exports `-c` and `p`, and `namespace
    ///   import -f ::src::p` aborts with `no namespace specified in import
    ///   pattern "-f"`.
    /// - **How many** — one, declared as `max_leading_option_words`. A second
    ///   `-clear` is an ordinary export pattern (and `-clear` is a perfectly
    ///   valid, importable command name); a second `-force` is an import
    ///   pattern that aborts the script.
    ///
    /// The scan also stops at the first non-flag word, which is what makes
    /// `namespace export a -clear` export both (the flag is only ever the
    /// first word — oracle-verified).
    ///
    /// `0` when no registry is attached (the analyser runs registry-less in
    /// some unit tests): a flagless read then treats `-clear` as an ordinary
    /// pattern, which is inert — nothing else records a command by that name.
    pub(super) fn namespace_leading_flag_words(&self, sub: &str, args: &[String]) -> usize {
        let Some((spec, sub_spec)) = self
            .registry
            .as_deref()
            .and_then(|r| r.get("namespace"))
            .and_then(|spec| spec.subcommand(sub).map(|s| (spec, s)))
        else {
            return 0;
        };
        let options = self
            .analysis_context()
            .context()
            .available_sub_option_specs(spec, sub_spec);
        let cap = sub_spec
            .max_leading_option_words
            .map_or(usize::MAX, usize::from);
        args.iter()
            .take(cap)
            .take_while(|w| {
                options
                    .iter()
                    .any(|o| o.matches(w.as_str()) && !o.takes_value())
            })
            .count()
    }

    /// Handle `namespace unknown HANDLER` — installing a per-namespace
    /// unknown handler (TIP 181, `NamespaceUnknownCmd` in `tclNamesp.c`)
    /// makes command resolution unknowable, exactly like a dynamic user
    /// `proc unknown`: the handler runs for every failed lookup and may
    /// resolve anything.  Flips ``has_dynamic_providers``, which suppresses
    /// W120/W123 file-wide (matching the proc-unknown behaviour).
    ///
    /// Only the *setter* form suppresses: the bare query (`namespace
    /// unknown`) installs nothing, and an empty handler (`namespace unknown
    /// {}`) resets to the default lookup (`Tcl_SetNamespaceUnknownHandler`
    /// treats an empty list as a reset).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceUnknown`]
    /// (stamped on `namespace`'s `unknown` subcommand); `args[0]` is
    /// still the subcommand word.
    pub fn handle_namespace_unknown_command(&mut self, args: &[String]) {
        if args.len() < 2 {
            return;
        }
        if args[1].trim().is_empty() {
            return;
        }
        self.result.has_dynamic_providers = true;
    }

    /// Recognise the tcllib ``<NS>::import <ALIAS>`` wrapper idiom
    /// and record a conjectured `NamespaceImport`.
    ///
    /// A common tcllib convention is a proc ``some::ns::import`` that
    /// ``uplevel``s ``namespace eval <alias> {namespace import some::ns::*}``.
    /// The wrapper bodies are not statically detected, but
    /// the call shape is unambiguous — its qualified name ends in
    /// ``::import`` and its single argument is a static namespace
    /// identifier.
    pub fn handle_tcllib_import_wrapper(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        args: &[String],
        scope_path: &[usize],
    ) {
        let [alias] = args else {
            return;
        };
        let current_ns = self.command_resolution_namespace(scope_path);
        let Some(hint) = crate::naming::authored_import_wrapper_hint(
            cmd_name,
            alias,
            &current_ns,
            crate::naming::AuthoredImportAliasSpelling::Written,
        ) else {
            return;
        };
        self.result.namespace_imports.push(
            crate::signature_scan::types::SignatureNamespaceImport {
                ns: hint.alias_namespace,
                pattern: hint.pattern,
                source: Some(
                    crate::signature_scan::types::SignatureNamespaceImportSource {
                        native_source: None,
                        namespace: hint.source_namespace,
                        tail_pattern: "*".to_owned(),
                    },
                ),
                range: cmd_tok.span,
                conjectured: true,
                // The wrapper's own body is not read, so whether the
                // `namespace import` it `uplevel`s carries `-force` is
                // unknown. `false` is the conservative reading for the one
                // thing the flag decides: a conjectured import never claims
                // to have replaced a command the target namespace already
                // holds.
                forced: false,
            },
        );
    }
}
