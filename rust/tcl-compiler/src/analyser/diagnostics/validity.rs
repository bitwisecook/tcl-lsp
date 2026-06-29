//! Command validity and arity checks emitted during the command walk.
//!
//! These diagnostics decide whether a command invocation is well-formed
//! against the registry and the active dialect: an unknown subcommand
//! (W001), a command disabled in the dialect (W002), an invalid dialect
//! option (W004) or expression operator (W003), wrong argument counts
//! (the arity diagnostics), a malformed `if` (E004), a missing `--` option
//! terminator before a value that looks like an option (W304), and a stub
//! `proc` that shadows a built-in command or `expr` function (W116, W117).
//! The disabled-command, arity, and W304 emitters buffer their candidates
//! and flush them after the walk.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_core_types::DiagCode;

use super::helpers::{is_ident_continue, is_integer_word};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::expr_ast::{BinOp, ExprNode};

impl Analyser {
    /// **W001.** Emit "Unknown subcommand" warning for commands
    /// whose registry signature is a [`SubcommandSig`](super::dispatch::SubcommandSig)
    /// when the first argument doesn't resolve to a known subcommand.
    ///
    /// Skips:
    ///
    /// - commands the registry doesn't know (no signature),
    /// - simple-command signatures (no subcommand dispatch),
    /// - signatures with `allow_unknown == true` (generated
    ///   dialect packs),
    /// - first-arg values containing ``$`` / ``[`` (dynamic
    ///   substitution — runtime-resolved),
    /// - empty arg lists (handled by the E001 emitter).
    ///
    /// When emission is warranted, includes a "did you mean…?"
    /// suffix using [`crate::text::suggest_similar`] over the
    /// known subcommand set (max 1 suggestion within edit
    /// distance 3).
    ///
    /// One case is not handled: a subcommand position that is
    /// ``{*}``-expanded (``arg_expand[0]``). ``process_command`` does
    /// not currently thread the expansion flag through; the literal-
    /// text ``$`` / ``[`` gate covers the dynamic-substitution case,
    /// and ``{*}LITERAL`` for an unknown subcommand is rare enough in
    /// practice that the gap is acceptable.
    /// **W002** — the command is disabled in the active dialect profile: it
    /// exists in the registry but not for the active dialect (e.g. `dict` under
    /// `tcl8.4`, added in 8.5).  Only a *literal* command head is checked — a
    /// `$obj` / `[cmd]` head is W307's concern — and an earlier unconditional
    /// user-proc definition that shadows the built-in suppresses it (Tcl
    /// resolves the proc at the call site).
    pub(in crate::analyser) fn emit_w002_disabled_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
    ) {
        use tcl_registry::prelude::DialectSet;
        // A dynamic command head (`$obj method`, `[lookup] arg`) is resolved at
        // runtime — W307 handles it, not W002.
        if matches!(
            cmd_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) {
            return;
        }
        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let bare = cmd_name.trim_start_matches(':');
        if bare.is_empty() {
            return;
        }
        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        // EXISTS in the active dialect → fine.  UNKNOWN everywhere → W123's
        // concern.  Only DISALLOWED (exists in some dialect, not this one)
        // fires.  Existence must be checked *dialect-agnostically*: the
        // analyser registry only loads the active dialect, so `get(bare)`
        // misses an iRules command like `when`/`log`/`session` under
        // tcl8.6, so use the dialect-independent `known_in_any_dialect`.
        if registry.get_for_dialect(bare, dialect).is_some() || !registry.known_in_any_dialect(bare)
        {
            return;
        }
        // An earlier *unconditional* user proc with this name shadows the
        // would-be-disabled built-in at the call site.
        let qualified = crate::naming::normalise_qualified_name(bare);
        if let Some(def) = self.result.all_procs.get(&qualified)
            && def.name_span.start() < cmd_tok.span.start()
        {
            return;
        }
        let diag = super::types::Diagnostic {
            code: DiagCode::W002,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is disabled in the active dialect profile"),
            severity: Severity::Warning,
            fixes: Vec::new(),
        };
        // Per-item path (isolated body): the body's own `all_procs` couldn't
        // prove a shadow, but a *sibling/enclosing* user proc still might.  That
        // is a cross-item fact, so defer the shadow re-check to the tail (over
        // the merged `all_procs`).  `capture_global_reads.is_some()` marks the
        // isolated-body analysis; on the whole-file path it is `None` and W002 is
        // emitted inline exactly as before.
        if self.capture_global_reads.is_some() {
            self.pending_disabled_commands.push((qualified, diag));
        } else {
            self.result.diagnostics.push(diag);
        }
    }

    pub(in crate::analyser) fn emit_w001_unknown_subcommand(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use super::dispatch::{CommandSignature, signature_for_command};
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let Some(first_arg) = args.first() else {
            // Empty arg list — E001 path; not in scope here.
            return;
        };
        // Dynamic-value subcommand position — can't resolve statically.
        if first_arg.contains('$') || first_arg.contains('[') {
            return;
        }
        // Tk geometry/widget ensemble commands (`grid` / `pack` / `wm` / …)
        // are recognised for the unknown-subcommand check regardless of the
        // active Tcl dialect — a `.tcl` script may `package require Tk` at
        // runtime, and W001 fires on `grid bogus` under every dialect.
        let dialect =
            DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL) | DialectSet::TK;
        let Some(CommandSignature::WithSubcommands(sig)) =
            signature_for_command(registry, cmd_name, dialect)
        else {
            return;
        };
        if sig.allow_unknown {
            return;
        }
        // `after` dispatches on `cancel` / `idle` / `info`, but its first word
        // may instead be a millisecond delay (`after 200 {…}`).  An integer
        // first word is a valid time argument, not an unknown subcommand, so
        // it must not trip W001.  (Non-integer, non-subcommand words such as
        // `after foo` remain genuine errors and still fire.)
        if cmd_name == "after" && is_integer_word(first_arg) {
            return;
        }
        // Tk geometry managers accept `manager pathName ?args?` as a shortcut
        // for `manager configure pathName ?args?` (grid.n / pack.n / place.n).
        // A window path starts with `.`, which is not a valid subcommand-name
        // first character, so this is unambiguous.
        if matches!(cmd_name, "grid" | "pack" | "place") && first_arg.starts_with('.') {
            return;
        }
        if sig.subcommands.contains_key(first_arg) {
            return;
        }
        let mut message = format!("Unknown subcommand '{first_arg}' for '{cmd_name}'");
        let candidates: Vec<&str> = sig.subcommands.keys().map(String::as_str).collect();
        let suggestions = crate::text::suggest_similar(first_arg, candidates.iter().copied(), 1, 3);
        let mut fixes: Vec<super::types::CodeFix> = Vec::new();
        if let Some(best) = suggestions.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            if let Some(sub_tok) = arg_tokens.first() {
                // Target the *content* range of the subcommand
                // token rather than its full span.  Wrapper tokens
                // (`Str` braced, `Esc` quoted) carry the opening
                // delimiter via ``content_offset`` and intentionally
                // exclude the closing delimiter from ``span.end``;
                // replacing the full span would leave a stray
                // ``}`` / ``"`` behind (e.g. ``string {lenght}`` →
                // ``string length}``).  Using the content range
                // ([span.start + content_offset, span.end)) gives
                // ``{length}`` / ``"length"`` for the wrapped forms
                // and remains identical to the full span for bare
                // ``Esc`` words (``content_offset == 0``).
                let content_start = sub_tok.span.start() + u32::from(sub_tok.content_offset);
                let fix_span = tcl_lexer::Span::new(content_start, sub_tok.span.end());
                fixes.push(super::types::CodeFix {
                    span: fix_span,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
        }
        // Anchor at the command-head + subcommand-name range so
        // the squiggle covers ``cmd subname`` rather than the
        // entire invocation: combine the command token with the
        // subcommand arg token.
        let span = match arg_tokens.first() {
            Some(sub_tok) => tcl_lexer::Span::new(cmd_tok.span.start(), sub_tok.span.end()),
            None => cmd_tok.span,
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W001,
            span,
            message,
            severity: Severity::Warning,
            fixes,
        });
    }

    /// **E002 / E003.** Argument-count check for simple (non-
    /// subcommand) commands: skip leading declared
    /// option flags, then compare the positional-argument count
    /// against the registry signature's arity bounds.
    ///
    /// Option skipping uses the dialect-filtered
    /// [`CommandSig::leading_options`](super::dispatch::CommandSig::leading_options)
    /// set, so switches introduced in a later Tcl release (e.g.
    /// `regsub -command`, 9.0+) are only skipped under a dialect that
    /// declares them.  This prevents both a false positive (declared
    /// switches counted as positional → spurious E003) and a dialect
    /// leak (9.0-only switches skipped under 8.x).
    ///
    /// `arg_expand[i]` marks an argument preceded by the Tcl 8.5+
    /// `{*}` expansion prefix.  A `{*}`-expanded word contributes an
    /// unknown number of runtime arguments, so option skipping stops
    /// at the first such word and the positional upper bound becomes
    /// unbounded — only the count of *non-expanded* positional words
    /// can still trip E003.
    ///
    /// **Intentional gaps:**
    /// - The `leading_options` skip is name-only, so the *value*
    ///   of a value-taking leading option is **not** skipped.
    /// - Statically-resolvable literal `{*}` expansions (`{*}{a b c}`)
    ///   are not refined to their element count; the conservative form
    ///   here can miss a genuine over-arity but never invents a false
    ///   positive.
    ///
    /// Subcommand-dispatch commands are handled by
    /// [`Self::emit_w001_unknown_subcommand`] and skipped here;
    /// per-subcommand arity is not checked.
    pub(in crate::analyser) fn emit_arity_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand_in: &[bool],
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        use super::dispatch::{CommandSignature, signature_for_command};
        use tcl_registry::prelude::DialectSet;

        // `arg_expand_in` is parallel to the full argv (command name at
        // index 0); drop that slot so it lines up with `args`.
        let arg_expand: &[bool] = arg_expand_in.get(1..).unwrap_or(&[]);

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        match signature_for_command(registry, cmd_name, dialect) {
            Some(CommandSignature::Simple(sig)) => {
                self.check_simple_arity(
                    cmd_name, cmd_name, &sig, args, arg_tokens, arg_expand, cmd_tok, scope_path,
                );
            }
            Some(CommandSignature::WithSubcommands(sig)) => {
                // Per-subcommand arity on `args[1:]`.  The W001
                // unknown-subcommand path is handled separately by
                // [`Self::emit_w001_unknown_subcommand`].
                let Some(sub_name) = args.first() else {
                    // **E001.** A subcommand-dispatch command invoked with no
                    // subcommand at all (`string` / `dict` / `info` on its
                    // own).  Queued as a
                    // `pending_arity` candidate so an earlier shadowing user
                    // proc / class / alias / ensemble / stub suppresses it,
                    // exactly like the E002 / E003 paths.
                    let ns = self.command_resolution_namespace(scope_path);
                    let enforce_order = !self.scope_path_in_proc_body(scope_path);
                    self.pending_arity.push((
                        cmd_name.to_string(),
                        ns,
                        enforce_order,
                        super::types::Diagnostic {
                            code: DiagCode::E001,
                            span: cmd_tok.span,
                            message: format!("'{cmd_name}' requires a subcommand"),
                            severity: Severity::Error,
                            fixes: Vec::new(),
                        },
                    ));
                    return;
                };
                // A `{*}`-expanded subcommand word resolves to an unknown
                // name at runtime; skip resolution and arity entirely.
                if arg_expand.first().copied().unwrap_or(false) {
                    return;
                }
                // Dynamic subcommand value — can't resolve statically.
                if sub_name.contains('$') || sub_name.contains('[') {
                    return;
                }
                let Some(sub_sig) = sig.subcommands.get(sub_name) else {
                    // Unknown subcommand — W001's job, not arity.
                    return;
                };
                let display_name = format!("{cmd_name} {sub_name}");
                self.check_simple_arity(
                    cmd_name,
                    &display_name,
                    sub_sig,
                    &args[1..],
                    arg_tokens.get(1..).unwrap_or(&[]),
                    arg_expand.get(1..).unwrap_or(&[]),
                    cmd_tok,
                    scope_path,
                );
            }
            None => {}
        }
    }

    /// Compare a positional-argument count against a single
    /// [`CommandSig`]'s arity bounds and queue an E002 / E003
    /// candidate.  Shared by the simple-command and per-subcommand
    /// arity paths in [`Self::emit_arity_diagnostics`].
    ///
    /// `resolution_name` is the base command name used by the
    /// post-walk [`Self::flush_arity_diagnostics`] to honour a
    /// shadowing user proc / class / alias (e.g. `file` for the
    /// `file link` subcommand check), while `display_name` is the
    /// human-facing name shown in the message (`file link`).
    ///
    /// `args` / `arg_tokens` / `arg_expand` are the slices *after*
    /// whatever prefix the caller has already consumed (the command
    /// name for the simple path; the command name and subcommand word
    /// for the subcommand path), so the leading-option scan and
    /// positional count operate on the same coordinate system as
    /// `sig`.
    #[allow(clippy::too_many_arguments)]
    fn check_simple_arity(
        &mut self,
        resolution_name: &str,
        display_name: &str,
        sig: &super::dispatch::CommandSig,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand: &[bool],
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        let expanded = |i: usize| arg_expand.get(i).copied().unwrap_or(false);

        // Skip leading declared option flags.  Stop at the first
        // non-option word, the option terminator `--` (consumed), or
        // a `{*}`-expanded word (whose value can't be classified).
        let mut positional_start = 0usize;
        if !sig.leading_options.is_empty() {
            for (i, arg) in args.iter().enumerate() {
                if expanded(i) {
                    break;
                }
                if sig.leading_options.contains(arg) {
                    positional_start = i + 1;
                    if arg == "--" {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        let positional_any_expand = (positional_start..args.len()).any(expanded);
        // `nargs_min` is the *lower bound* on the positional-argument
        // count: the non-expanded words, since each `{*}` word
        // contributes 0..N more at runtime.  E003 ("too many") fires
        // when even this lower bound exceeds `max`.  E002 ("too few")
        // needs an *upper bound* on the count, which becomes unbounded
        // once any `{*}` expansion is present — so E002 only fires when
        // there is no expansion and the count is therefore exact.
        let nargs_min = if positional_any_expand {
            (positional_start..args.len())
                .filter(|&i| !expanded(i))
                .count()
        } else {
            args.len() - positional_start
        };
        let min = usize::from(sig.arity.min);
        let max = usize::from(sig.arity.max);

        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };

        // Capture the call-site command-resolution namespace so the
        // post-walk flush can resolve this command the Tcl way (current
        // namespace → global) and only suppress the arity check when
        // the call actually resolves to a user definition — not to any
        // same-tail-named proc elsewhere in the file. Uses the proc's
        // *defining* namespace (so `close` inside a body of
        // `proc ::ns::x` resolves through `::ns`), not just lexical
        // `namespace eval` nesting.
        let ns = self.command_resolution_namespace(scope_path);

        // Top-level calls (module body, `namespace eval` bodies, and
        // conditionals) execute in source order during load, so a
        // shadowing proc only silences the builtin arity check when its
        // definition lexically precedes the call.  Calls inside a proc
        // body resolve after the whole script has loaded, so order is
        // not enforced there.
        let enforce_order = !self.scope_path_in_proc_body(scope_path);

        // Collect as a *candidate*; the post-walk
        // [`Self::flush_arity_diagnostics`] drops it if the call
        // resolves to a user proc / class / alias / ensemble / stub.
        // A class / alias / ensemble / stub match suppresses regardless
        // of definition order; a *proc* match additionally honours
        // `enforce_order` (in-order/reachability gate).
        if !positional_any_expand && (args.len() - positional_start) < min {
            let got = args.len() - positional_start;
            self.pending_arity.push((
                resolution_name.to_string(),
                ns,
                enforce_order,
                super::types::Diagnostic {
                    code: DiagCode::E002,
                    span: full_span,
                    message: format!(
                        "Too few arguments for '{display_name}': expected at least {min}, got {got}"
                    ),
                    severity: Severity::Error,
                    fixes: Vec::new(),
                },
            ));
        } else if !sig.arity.is_unlimited() && nargs_min > max {
            self.pending_arity.push((
                resolution_name.to_string(),
                ns,
                enforce_order,
                super::types::Diagnostic {
                    code: DiagCode::E003,
                    span: full_span,
                    message: format!(
                        "Too many arguments for '{display_name}': expected at most {max}, got {nargs_min}"
                    ),
                    severity: Severity::Error,
                    fixes: Vec::new(),
                },
            ));
        }
    }

    /// Post-walk flush of the [`Self::pending_arity`] candidates
    /// collected by [`Self::emit_arity_diagnostics`].
    ///
    /// Runs after the command walk completes, when `all_procs`,
    /// `all_classes`, `command_aliases`, `ensemble_namespaces` and the
    /// inline stub set are fully populated.  A candidate is dropped
    /// only when the call **resolves to** a user definition rather than
    /// the builtin whose registry arity produced it — resolution
    /// follows Tcl's rule for unqualified commands (the call-site
    /// namespace, then global `::`), using the namespace captured at
    /// emit time.  So `proc ::ns::close {...}` suppresses a `close`
    /// call inside `::ns` (and a qualified `::ns::close ...`), but a
    /// `close` call in another namespace still resolves to the builtin
    /// and is checked.  Document-global declarations — inline
    /// `# tcl-lsp: stub`s — suppress by bare name regardless of
    /// namespace.
    ///
    /// Suppression by a shadowing **proc** also honours definition
    /// reachability: a top-level call (one whose
    /// `enforce_order` flag is set — module body, `namespace eval`
    /// body, or a conditional) is silenced only when the proc's
    /// definition lexically precedes it, since top-level commands run
    /// in source order during load (so a `close x y z` *before* a later
    /// `proc close` still reaches the builtin).  Proc-body calls run
    /// after load and are not order-gated.  Classes / aliases /
    /// ensembles / stubs always exist at run time and are never
    /// order-gated.  (Excluding *conditionally* defined procs would
    /// need the CFG dominator model, which is not modelled here.)
    ///
    /// Emit the per-item path's pending W002 (disabled-in-dialect command)
    /// diagnostics, re-applying the user-proc-shadowing suppression against the
    /// merged `all_procs` (a cross-item fact unavailable to an isolated body).
    /// No-op on the whole-file `analyse` path (W002 is emitted inline there, so
    /// `pending_disabled_commands` is empty) — keeping the two paths
    /// byte-identical.  The position guard (`name_span.start() < call.start()`)
    /// matches the inline check, so a unique-named proc resolves identically
    /// whether checked inline or here (duplicate proc names already force the
    /// per-item path to fall back).
    pub(in crate::analyser) fn flush_disabled_command_diagnostics(&mut self) {
        let pending = std::mem::take(&mut self.pending_disabled_commands);
        for (qualified, diag) in pending {
            if let Some(def) = self.result.all_procs.get(&qualified)
                && def.name_span.start() < diag.span.start()
            {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    /// Idempotent: drains `pending_arity`, so a second call is a
    /// no-op.
    pub fn flush_arity_diagnostics(&mut self) {
        if self.pending_arity.is_empty() {
            return;
        }
        // Fully-qualified non-proc user-command names the calls may
        // resolve to (classes / aliases keyed by qualified name;
        // ensemble namespaces *are* the command name).  These always
        // exist by the time the script runs, so they suppress the
        // builtin arity check regardless of definition order.
        let mut non_proc_qnames: FxHashSet<&str> = FxHashSet::default();
        non_proc_qnames.extend(self.result.all_classes.keys().map(String::as_str));
        non_proc_qnames.extend(self.result.command_aliases.keys().map(String::as_str));
        non_proc_qnames.extend(self.ensemble_namespaces.iter().map(String::as_str));
        // Qualified proc name → definition offset (the proc-name
        // token start).  A shadowing proc only silences a *top-level*
        // call (`enforce_order`) when its definition lexically
        // precedes the call; proc-body calls are not order-gated.
        // Conditional / nested definitions are still treated as
        // shadowing here — distinguishing unconditionally-reachable
        // definitions needs the CFG dominator model, which is not
        // modelled here.
        let proc_offsets: FxHashMap<&str, u32> = self
            .result
            .all_procs
            .iter()
            .map(|(qname, def)| (qname.as_str(), def.name_span.start()))
            .collect();
        // Inline stubs are document-global and unqualified.
        let stub_names = super::utils::scan_stub_command_names(&self.source);

        // Qualify an unqualified command against a namespace, mirroring
        // `resolve_command_qualified_name` (`::` root → `::cmd`).
        let join = |ns: &str, cmd: &str| -> String {
            if ns == "::" {
                format!("::{cmd}")
            } else {
                format!("{ns}::{cmd}")
            }
        };

        let pending = std::mem::take(&mut self.pending_arity);
        for (cmd_name, ns, enforce_order, diag) in pending {
            let bare = cmd_name.rsplit("::").next().unwrap_or(&cmd_name);
            // Candidate qualified names this call could resolve to.
            let candidates: Vec<String> = if cmd_name.contains("::") {
                // Already qualified — absolutise like
                // `resolve_command_qualified_name` does.
                let abs = if cmd_name.starts_with("::") {
                    cmd_name.clone()
                } else {
                    format!("::{cmd_name}")
                };
                vec![abs]
            } else {
                // Unqualified — current namespace, then global.
                vec![join(&ns, &cmd_name), format!("::{cmd_name}")]
            };
            // A proc shadows only when reachable at the call: top-level
            // calls require the definition to lexically precede them
            // (`def_off < call_off`); proc-body calls accept any
            // same-named definition.  Classes / aliases / ensembles /
            // stubs are not order-gated.
            let call_off = diag.span.start();
            let resolves_to_user = candidates.iter().any(|c| {
                non_proc_qnames.contains(c.as_str())
                    || proc_offsets
                        .get(c.as_str())
                        .is_some_and(|&def_off| !enforce_order || def_off < call_off)
            }) || stub_names.contains(bare);
            if resolves_to_user {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    /// **E004.** Emit "Malformed `if` command" / "Extra words after
    /// `else` clause" errors when an `if` invocation's structural
    /// shape doesn't match `if COND BODY ?elseif COND BODY ...?
    /// ?else BODY?`.
    ///
    /// Fires when an `if` invocation's syntactic shape is invalid.
    /// The cases:
    ///
    /// - `"malformed if"` — empty arg list, or no clauses after
    ///   the full walk.
    /// - `"malformed if else clause"` — bare `else` with no body
    ///   following.
    /// - `'extra words after "else" clause'` — `else BODY` with
    ///   one or more trailing words.
    /// - `"malformed if clause"` — condition with no body
    ///   (with or without an intervening `then` keyword).
    ///
    /// Detected analyser-side at the `if`-command dispatch site
    /// rather than by walking lowered IR, matching the established
    /// W302 / W001 dispatch-site pattern.  This also covers a case
    /// `lowering/structured.rs::lower_if` doesn't: it currently
    /// doesn't produce an "extra words after else" barrier at all.
    ///
    /// Severity: `Error`.  No code fixes.  Span anchors at the
    /// command-head token through the last argument-token end (the
    /// full command source range).
    pub(in crate::analyser) fn emit_e004_malformed_if(
        &mut self,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };
        let push_malformed = |this: &mut Self| {
            this.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::E004,
                span: full_span,
                message: "Malformed 'if' command".to_string(),
                severity: Severity::Error,
                fixes: Vec::new(),
            });
        };
        let push_extra_words = |this: &mut Self| {
            this.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::E004,
                span: full_span,
                message: "Extra words after \"else\" clause in \"if\" command".to_string(),
                severity: Severity::Error,
                fixes: Vec::new(),
            });
        };

        if args.is_empty() {
            push_malformed(self);
            return;
        }

        let mut i = 0;
        let mut clause_count: usize = 0;
        while i < args.len() {
            if args[i] == "elseif" {
                i += 1;
                continue;
            }
            if args[i] == "else" {
                if i + 1 >= args.len() {
                    // Bare ``else`` with no body following.
                    push_malformed(self);
                    return;
                }
                if i + 2 < args.len() {
                    // ``else BODY <extra...>``.
                    push_extra_words(self);
                    return;
                }
                // ``else BODY`` — well-formed terminator.  An else-only
                // clause does not count as a clause, so ``if else BODY``
                // produces a ``"malformed if"`` barrier; leave
                // ``clause_count`` unchanged in this arm.
                break;
            }

            // Condition + (optional ``then``) + body shape.
            i += 1;
            if i < args.len() && args[i] == "then" {
                i += 1;
            }
            if i >= args.len() {
                // Condition with no following body.
                push_malformed(self);
                return;
            }
            clause_count += 1;
            i += 1;
        }

        if clause_count == 0 {
            // E.g. ``if elseif`` / ``if else`` after the elseif-skip
            // / else-skip branches consume their keywords without
            // producing a clause.
            push_malformed(self);
        }
    }

    /// **W304.** Emit "Missing option terminator (`--`)" diagnostics
    /// for option-bearing commands whose first positional argument
    /// could be misinterpreted as an option.
    ///
    /// Resolves the command's option-
    /// terminator profile via
    /// [`tcl_registry::CommandRegistry::resolve_option_terminator`],
    /// scans for the first positional argument that lacks a
    /// preceding `--`, and emits a tristate-severity diagnostic:
    ///
    /// - **OFF** (no diagnostic) — the value is provably non-`-`-
    ///   prefixed (a non-dynamic literal whose representative token
    ///   isn't a `Var`/`Cmd` and whose text doesn't start with `-`).
    /// - **INFO** — dynamic value (`Var` / `Cmd` token) with no
    ///   proof of starting with `-`.  When the value is a single-
    ///   token `Var` whose most recent literal `set` resolves to a
    ///   non-`-`-prefixed value, an additional "origin" diagnostic
    ///   is emitted at the resolution site to explain the INFO
    ///   downgrade.
    /// - **WARNING** — the value is known to start with `-`: either
    ///   a literal whose first character is `-`, or a `Var` whose
    ///   constant-propagated value starts with `-`.
    ///
    /// The diagnostic carries a code-fix that prepends `"-- "` to
    /// the positional-argument span (with a one-byte extension for
    /// `Cmd` tokens whose lexer span excludes the closing `]`).
    ///
    /// **Note on `warn_without_terminator`:** the registry's
    /// `Traits::WARN_WITHOUT_TERMINATOR` flag (set on `regexp` only
    /// today) is plumbed onto [`tcl_registry::ResolvedTerminator`]
    /// but is not consumed.  The OFF gate
    /// fires uniformly for non-dynamic, non-`-`-prefixed values
    /// regardless of the trait.
    pub(in crate::analyser) fn emit_w304_missing_option_terminator(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }

        // Resolve the option-terminator profile *dialect-agnostically*:
        // resolving with no dialect means W304 still fires on a command
        // that the active dialect disables (e.g. `exec` / `glob` under
        // f5-irules, which also draw W002 / W123).  Passing the dialect
        // here would over-filter via `get_for_dialect` and silently drop
        // those W304s.
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(profile) =
            registry.resolve_option_terminator(cmd_name, &arg_strs, DialectSet::empty())
        else {
            return;
        };

        // The braced pattern-list switch form ``switch $x { pat body … }``
        // is NOT a runtime hazard: Tcl unambiguously identifies the
        // trailing brace as the pattern list and never consumes the
        // preceding word as an option.  Detect the two-arg braced form
        // (the last arg is a brace-enclosed `Str` token) and exempt it
        // entirely.  The SPLIT form (`switch $x -nocase {body} …`, 3+
        // args) is still flagged.
        if cmd_name == "switch"
            && arg_tokens.len() == 2
            && arg_tokens.last().map(|t| t.kind) == Some(tcl_lexer::TokenType::Str)
        {
            return;
        }

        let Some(positional_idx) = first_positional_without_terminator(args, &profile) else {
            return;
        };
        if positional_idx >= arg_tokens.len() {
            return;
        }

        let tok = arg_tokens[positional_idx];
        let text = &args[positional_idx];

        let is_dynamic = matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        );
        let looks_like_option = text.starts_with('-');

        // OFF — non-dynamic value that does not start with `-` can
        // never be confused with an option.
        if !is_dynamic && !looks_like_option {
            return;
        }

        let command_label = match profile.subcommand {
            Some(sub) => format!("{cmd_name} {sub}"),
            None => cmd_name.to_string(),
        };

        // Build the code-fix span.  For ``Cmd`` (`[…]`) tokens the
        // lexer span covers ``[inner`` but excludes the closing
        // ``]``; extend by one byte when the byte after ``span.end``
        // is ``]`` so the replacement encompasses the bracket pair.
        // (Body-local: the fix text is the argument's own source slice, so it is
        // computable in an isolated body and rebased by the graft.)
        let (fix_span, diag_end) = self.compute_w304_fix_span(tok);
        let fix_text = format!(
            "-- {}",
            &self.source[fix_span.start() as usize..fix_span.end() as usize]
        );
        let fixes = vec![super::types::CodeFix {
            span: fix_span,
            new_text: fix_text,
            description: "Insert '--' option terminator".to_string(),
        }];
        let diag_span = tcl_lexer::Span::new(tok.span.start(), diag_end);
        // Suppress unused-warning on the rare path where `cmd_tok`
        // isn't needed (the diagnostic anchors at the positional
        // arg's span, not the command head).
        let _ = cmd_tok;

        // The `Var` dynamic-not-option branch of `classify_w304` resolves the
        // variable against the most recent literal `set` in the *whole file*
        // (`last_literal_set_value_for_var` scans `self.source`).  An isolated
        // proc body's `self.source` is only the body, so an enclosing-scope set
        // would be missed.  On the per-item path, defer that one source-dependent
        // case to the tail (where `self.source` is the full file); every other
        // branch is body-local and emitted inline.
        if self.capture_global_reads.is_some()
            && is_dynamic
            && !looks_like_option
            && matches!(tok.kind, tcl_lexer::TokenType::Var)
        {
            self.pending_w304
                .push((tok, command_label, fixes, diag_span));
            return;
        }

        let (severity, message, origin) =
            self.classify_w304(tok, is_dynamic, looks_like_option, &command_label);
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W304,
            span: diag_span,
            message,
            severity,
            fixes,
        });
        if let Some(origin_diag) = origin {
            self.result.diagnostics.push(origin_diag);
        }
    }

    /// Emit the per-item path's pending W304 diagnostics, classifying each
    /// `$var` against the **full-file** most-recent-literal-`set` resolution
    /// (impossible inside an isolated body, whose `self.source` is only the
    /// body).  All inputs are absolute by the time the tail runs (the graft
    /// rebased the token, fix, and diagnostic spans), so the result is identical
    /// to the inline whole-file emission.  No-op on the `analyse` path
    /// (`pending_w304` empty).
    pub(in crate::analyser) fn flush_w304_diagnostics(&mut self) {
        let pending = std::mem::take(&mut self.pending_w304);
        for (tok, command_label, fixes, diag_span) in pending {
            let (severity, message, origin) = self.classify_w304(tok, true, false, &command_label);
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W304,
                span: diag_span,
                message,
                severity,
                fixes,
            });
            if let Some(origin_diag) = origin {
                self.result.diagnostics.push(origin_diag);
            }
        }
    }

    /// **W116 / W117.** Stub command / expression definition shadows a
    /// built-in.  Post-walk check.  W116 fires when a `# tcl-lsp:
    /// stub` command name (with leading `::` stripped) collides with a
    /// registered command; W117 when a stub expr function/operator name
    /// collides with a built-in `expr` function or operator.
    pub(in crate::analyser) fn emit_w116_w117_stub_shadows(&mut self) {
        use super::types::{Diagnostic, Severity};

        if self.result.stub_commands.is_empty() && self.result.stub_expr_defs.is_empty() {
            return;
        }

        // W116 — stub command shadows a built-in command.  Build the
        // dialect command-name set locally.
        if !self.result.stub_commands.is_empty() {
            use tcl_registry::CommandRegistry;
            use tcl_registry::prelude::DialectSet;
            let mut registry = CommandRegistry::build_default();
            if let Some(d) = DialectSet::parse(&self.dialect) {
                registry.load_dialect(d);
            }
            let commands: std::collections::HashSet<&str> = registry.command_names().collect();
            let hits: Vec<(String, tcl_lexer::Span)> = self
                .result
                .stub_commands
                .iter()
                .filter(|s| commands.contains(s.name.trim_start_matches(':')))
                .map(|s| (s.name.clone(), s.range))
                .collect();
            for (name, span) in hits {
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::W116,
                    span,
                    message: format!("Stub command '{name}' shadows built-in command."),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }

        // W117 — stub expr function/operator shadows a built-in.
        if !self.result.stub_expr_defs.is_empty() {
            let irules = self.dialect == "f5-irules";
            let hits: Vec<(String, String, tcl_lexer::Span)> = self
                .result
                .stub_expr_defs
                .iter()
                .filter(|s| {
                    BUILTIN_MATH_FUNCTIONS.contains(&s.name.as_str())
                        || BUILTIN_EXPR_OPS.contains(&s.name.as_str())
                        || (irules && IRULES_EXPR_OPS.contains(&s.name.as_str()))
                })
                .map(|s| (s.name.clone(), s.kind.clone(), s.range))
                .collect();
            for (name, kind, span) in hits {
                let kind_label = if kind == "function" {
                    "function"
                } else {
                    "operator"
                };
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::W117,
                    span,
                    message: format!(
                        "Stub expression {kind_label} '{name}' shadows built-in {kind_label}."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **IRULE2002.** Warn when a deprecated iRules command is used —
    /// the command's spec carries a `deprecated_replacement`.  Only fires
    /// under the `f5-irules` dialect.
    pub(in crate::analyser) fn emit_irule2002_deprecated_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
    ) {
        if self.dialect != "f5-irules" {
            return;
        }
        let Some(replacement) = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|s| s.deprecated_replacement)
        else {
            return;
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::Irule2002,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is deprecated in iRules. Use '{replacement}' instead."),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE2001.** Warn that `matchclass` is deprecated — use
    /// `class match` instead.  Only fires under the `f5-irules` dialect.
    /// This fires *alongside* IRULE2002 at the same span (the
    /// command head): `matchclass` carries both a `deprecated_replacement`
    /// (→ IRULE2002) and a dedicated rule (→ IRULE2001).
    pub(in crate::analyser) fn emit_irule2001_matchclass(
        &mut self,
        cmd_name: &str,
        arg_tokens: &[tcl_lexer::Token],
        cmd_tok: tcl_lexer::Token,
    ) {
        if self.dialect != "f5-irules" || cmd_name != "matchclass" {
            return;
        }
        // Auto-fix `matchclass` → `class match`, a 1:1 rename (same argument
        // order).  The iRules forms are:
        //   * 3-arg `matchclass <item> <operator> <class>` → preserve all three
        //     verbatim as `class match <item> <operator> <class>`.
        //   * 2-arg shorthand `matchclass <item> <class>` → expand with the
        //     default operator: `class match <item> equals <class>`.
        // Any other arity is ambiguous, so we still warn but offer NO quick-fix
        // rather than corrupt the command.  (Gating on `>= 2` and always forcing
        // `equals` mangled the 3-arg form — e.g. `matchclass [HTTP::uri]
        // starts_with $::admin_paths` became `class match [HTTP::uri] equals
        // starts_with`, dropping the real class and operator.)  The raw source
        // slices preserve `$var` / `[cmd]` substitutions verbatim (the
        // substituted `args` values would drop them).  The lexer reports
        // representative spans for `[cmd …]` / `${name}` / `"…"` words without
        // their closing delimiter, so each slice — and the whole-command fix
        // range — is widened through trailing closers; otherwise
        // `[HTTP::uri]` would round-trip as `[HTTP::uri`.
        let word_end = |t: &tcl_lexer::Token| {
            crate::optimiser::helpers::spans::full_rewrite_span(&self.source, t.span).end()
        };
        let raw = |t: &tcl_lexer::Token| {
            self.source[t.span.start() as usize..word_end(t) as usize].to_string()
        };
        let new_text = match arg_tokens {
            [item, cls] => Some(format!("class match {} equals {}", raw(item), raw(cls))),
            [item, operator, cls] => Some(format!(
                "class match {} {} {}",
                raw(item),
                raw(operator),
                raw(cls)
            )),
            _ => None,
        };
        let fixes = new_text
            .map(|new_text| {
                let end = arg_tokens.last().map_or(cmd_tok.span.end(), word_end);
                vec![super::types::CodeFix {
                    span: tcl_lexer::Span::new(cmd_tok.span.start(), end),
                    new_text,
                    description: "Replace with 'class match'".to_string(),
                }]
            })
            .unwrap_or_default();
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::Irule2001,
            span: cmd_tok.span,
            message: "'matchclass' is deprecated since BIG-IP v10. \
Use 'class match <item> <operator> <class>' instead."
                .to_string(),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// Classify the positional value for W304: tristate severity,
    /// human-readable message, and an optional "origin" diagnostic
    /// for the constant-propagated INFO path.  Split out of
    /// [`Self::emit_w304_missing_option_terminator`] to keep that
    /// method's body within the clippy `too_many_lines` budget.
    fn classify_w304(
        &self,
        tok: tcl_lexer::Token,
        is_dynamic: bool,
        looks_like_option: bool,
        command_label: &str,
    ) -> (Severity, String, Option<super::types::Diagnostic>) {
        if is_dynamic && !looks_like_option {
            if matches!(tok.kind, tcl_lexer::TokenType::Var) {
                let var_name = self.var_name_from_token(tok);
                let resolved = var_name.and_then(|name| {
                    last_literal_set_value_for_var(
                        &self.source,
                        &name,
                        tok.span.start(),
                        self.lexer_config(),
                    )
                });
                if let Some((resolved_text, resolved_span, var_text)) = resolved {
                    if resolved_text.starts_with('-') {
                        let message = format!(
                            "'{command_label}' parses leading '-' as options. \
This value currently resolves to '{resolved_text}', so add '--' to force \
data parsing."
                        );
                        return (Severity::Warning, message, None);
                    }
                    let message = format!(
                        "'{command_label}' parses leading '-' as options. \
This value is reported at INFO because '{var_text}' currently resolves to \
static literal '{resolved_text}'. Keep '--' to guard against future \
option-injection regressions if the variable changes."
                    );
                    let origin = super::types::Diagnostic {
                        code: DiagCode::W304,
                        span: resolved_span,
                        message: format!(
                            "'{var_text}' is currently assigned static \
literal '{resolved_text}' here; this is why the diagnostic is INFO."
                        ),
                        severity: Severity::Suggestion,
                        fixes: Vec::new(),
                    };
                    return (Severity::Suggestion, message, Some(origin));
                }
            }
            // Command substitution / unresolved variable — INFO
            // with the substituted-input message.
            let message = format!(
                "'{command_label}' parses leading '-' as options. \
Insert '--' before substituted input to reduce option-injection risk."
            );
            return (Severity::Suggestion, message, None);
        }
        // ALWAYS: literal value that starts with `-`.
        let message = format!(
            "'{command_label}' argument starts with '-'. Add '--' \
before this value so it is treated as data, not an option."
        );
        (Severity::Warning, message, None)
    }

    /// Extract the variable name for a `Var` token using the
    /// lexer-provided token-text semantics
    /// ([`tcl_lexer::SourceMap::token_text`]).  Preserves the
    /// `Var`-specific normalisation rules (notably the trailing
    /// `}` strip for the `${}` degenerate case where the lexer
    /// extends the span by one byte to cover the closing brace),
    /// so this stays in sync with the rest of the analyser's
    /// token-text usage and avoids edge-case mismatches that a
    /// raw `self.source[..]` slice would introduce.  Returns
    /// `None` when the extracted text is empty.
    fn var_name_from_token(&self, tok: tcl_lexer::Token) -> Option<String> {
        let sm = tcl_lexer::SourceMap::new(&self.source);
        let text = sm.token_text(tok);
        if text.is_empty() {
            return None;
        }
        Some(text.to_string())
    }

    /// Compute the W304 code-fix span and diagnostic end position.
    ///
    /// For `Cmd` tokens (`[…]`) the lexer span excludes the closing
    /// `]`; we extend the span by one byte when the next character
    /// is `]` so the prepended ``-- `` doesn't split the bracket
    /// pair.  All other token kinds use the lexer span directly.
    fn compute_w304_fix_span(&self, tok: tcl_lexer::Token) -> (tcl_lexer::Span, u32) {
        let span_start = tok.span.start();
        let span_end = tok.span.end();
        if matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            let after = span_end as usize;
            if after < self.source.len() && self.source.as_bytes()[after] == b']' {
                let extended = span_end + 1;
                return (tcl_lexer::Span::new(span_start, extended), extended);
            }
        }
        (tcl_lexer::Span::new(span_start, span_end), span_end)
    }

    /// **W004.** Emit "Command option is not available in the active
    /// dialect" warning for option-bearing commands invoked with an
    /// option whose registry entry restricts it to a dialect that
    /// doesn't include the active one.
    ///
    /// Examples:
    /// `lsearch -stride` on Tcl 8.4 / 8.5 (option is 8.6+),
    /// `regsub -command` / `clock scan -validate` /
    /// `fconfigure -nodelay` on Tcl 8.x (options are 9.0+).
    ///
    /// Walks args looking for `-foo`-shaped flags, asks the registry
    /// for the matching `OptionSpec`, and fires when
    /// `OptionSpec::supports_dialect` returns false.  Substituted
    /// flag values (`-foo $bar`, `-foo [cmd]`) are skipped because
    /// the dispatching is only on the *flag name*; we don't have to
    /// inspect the value.  `--` terminates the scan.
    ///
    /// Subcommand-scoped options consult the subcommand's
    /// `OptionSpec` table when the first arg matches a known
    /// subcommand.
    pub(in crate::analyser) fn emit_w004_dialect_invalid_option(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::dialects::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };

        // Resolve subcommand-level options when the first arg names
        // one.
        let sub_match = (!spec.subcommands.is_empty())
            .then(|| spec.subcommands.iter().find(|s| s.name == args[0].as_str()))
            .flatten();
        let (options, parent_dialects, start_idx) = if let Some(sub) = sub_match {
            (sub.options, sub.dialects.or(spec.dialects), 1usize)
        } else {
            (spec.options, spec.dialects, 0usize)
        };

        if options.is_empty() {
            return;
        }

        let mut i = start_idx;
        while i < args.len() {
            let arg = args[i].as_str();
            if arg == "--" {
                break;
            }
            if !arg.starts_with('-') || arg.len() < 2 {
                i += 1;
                continue;
            }
            // Skip negative number literals (`-1`, `-1.5`).
            let rest = &arg[1..].trim_start_matches('-');
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
                i += 1;
                continue;
            }
            // Skip dynamic-value args (Var / Cmd tokens).  The flag
            // name itself comes from the arg text, but if the
            // representative token is a substitution we can't know
            // it's actually `-foo`.
            if i < arg_tokens.len() {
                let tok = arg_tokens[i];
                if matches!(
                    tok.kind,
                    tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                ) {
                    i += 1;
                    continue;
                }
            }
            // Find a matching OptionSpec; if found and dialect-gated
            // out, emit W004.
            if let Some(opt) = options.iter().find(|o| o.name == arg)
                && !opt.supports_dialect(Some(active), parent_dialects)
            {
                let span = if i < arg_tokens.len() {
                    arg_tokens[i].span
                } else {
                    continue;
                };
                // Message
                // exactly: `Option 'X' on 'cmd'[ sub] is not available in the
                // active dialect (D).`
                let sub_suffix = sub_match.map_or(String::new(), |s| format!(" {}", s.name));
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W004,
                    span,
                    message: format!(
                        "Option '{arg}' on '{cmd_name}'{sub_suffix} is not available \
in the active dialect ({}).",
                        self.dialect
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
            i += 1;
        }
    }

    /// **W003.** Emit "Expression operator not available in active
    /// dialect" warning for expressions that use a Tcl 9.0 string-
    /// comparison operator (`lt` / `le` / `gt` / `ge`, TIP 461) in a
    /// pre-9.0 dialect, or `in` / `ni` (TIP 201, Tcl 8.5+) in
    /// Tcl 8.4 / f5-irules.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_operator(
        &mut self,
        expr_text: &str,
        diag_span: tcl_lexer::Span,
    ) {
        use tcl_registry::dialects::DialectSet;

        // Quick lexical bail-out — the gated operators are short
        // word-shaped keywords; if none appear as a whole word we
        // can skip the parse.  Boundary check uses ASCII identifier
        // continuation so `tab`-, `newline`-, and start/end-of-text
        // boundaries all count (mirrors Tcl expr's whitespace
        // tolerance — `$x\tlt\t$y` and a wrapped `in` expression
        // both qualify).
        if !contains_gated_word(expr_text) {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        // Pre-Tcl-8.5 dialects don't accept `in` / `ni` (TIP 201).
        let pre_85 = !DialectSet::TCL85_PLUS.contains(active);
        // Pre-Tcl-9.0 dialects don't accept `lt` / `le` / `gt` / `ge`
        // (TIP 461).
        let pre_90 = !DialectSet::from_iter([DialectSet::TCL90]).contains(active);
        if !pre_85 && !pre_90 {
            return;
        }

        let parsed = crate::parse_expr(expr_text.trim(), Some(self.dialect.as_str()));
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        let mut found: Vec<&'static str> = Vec::new();
        walk_dialect_invalid_ops(&parsed, pre_85, pre_90, &mut found);
        for op_name in found {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W003,
                span: diag_span,
                message: format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'.",
                    self.dialect
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }
}

/// Built-in `expr` math functions.  Used by the
/// W117 stub-shadow check.
const BUILTIN_MATH_FUNCTIONS: &[&str] = &[
    "abs", "acos", "asin", "atan", "atan2", "bool", "ceil", "cos", "cosh", "double", "entier",
    "exp", "floor", "fmod", "hypot", "int", "isinf", "isnan", "isqrt", "log", "log10", "max",
    "min", "pow", "rand", "round", "sin", "sinh", "sqrt", "srand", "tan", "tanh", "wide",
];

/// Built-in `expr` operators.
const BUILTIN_EXPR_OPS: &[&str] = &[
    "!", "!=", "%", "&", "&&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=", ">>",
    "^", "eq", "ge", "gt", "in", "le", "lt", "ne", "ni", "|", "||", "~",
];

/// iRules-only `expr` operators.
const IRULES_EXPR_OPS: &[&str] = &[
    "and",
    "contains",
    "ends_with",
    "equals",
    "matches_glob",
    "matches_regex",
    "not",
    "or",
    "starts_with",
];

/// Scan `args` for the first positional argument that lacks a
/// preceding `--` terminator.
///
/// Skips option words (text starts with `-`); skips an additional
/// argument when the option's [`OptionSpec`](tcl_registry::prelude::OptionSpec)
/// in [`ResolvedTerminator::options`](tcl_registry::ResolvedTerminator)
/// has `takes_value == true`.  Linear scan over the borrowed
/// option slice — per-command option counts are small (≤ a dozen
/// for the largest specs in practice), so this is cheaper than a
/// per-resolve `HashSet` allocation on the analyser hot path.
/// Returns `None` when a `--` is encountered (positional arguments
/// after `--` are explicitly terminated).
fn first_positional_without_terminator(
    args: &[String],
    profile: &tcl_registry::ResolvedTerminator,
) -> Option<usize> {
    let mut i = profile.scan_start;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            return None;
        }
        if arg.starts_with('-') {
            i += 1;
            let consumes_value = profile
                .options
                .iter()
                .any(|o| o.name == arg && o.takes_value);
            if consumes_value && i < args.len() {
                i += 1;
            }
            continue;
        }
        return Some(i);
    }
    None
}

/// Locate the most-recent literal `set var value` assignment whose
/// command-head precedes `before_offset`.
///
/// Returns `Some((value_text, value_span, var_text))` when the
/// nearest preceding `set` is a fully-literal three-arg form.
/// Returns `None` when the latest assignment is dynamic / multi-
/// token (the runtime value cannot be proven statically).
fn last_literal_set_value_for_var(
    source: &str,
    var_name: &str,
    before_offset: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, tcl_lexer::Span, String)> {
    if var_name.is_empty() || before_offset == 0 {
        return None;
    }
    let head = before_offset as usize;
    if head > source.len() {
        return None;
    }
    let prefix = &source[..head];
    let segments = crate::segmenter::segment_commands_with_offset_and_config(prefix, 0, config);

    for cmd in segments.iter().rev() {
        // Cross-scope guard: stop the backward scan at a `proc NAME
        // {PARAMS} BODY` whose body *contains* the use offset and whose
        // params include `var_name` — the parameter shadows any outer
        // scope, so an outer `set` must not be attributed to the inner
        // use.  The use is inside the proc body iff that proc is the one
        // left unclosed by the truncation at `before_offset`: its span
        // then reaches the last truncated byte (`end + 1 >= head`).  A
        // *complete* proc before the use ends well before that and does
        // not shadow.
        let use_inside_proc = cmd.span.end() as usize + 1 >= head;
        if use_inside_proc
            && cmd.texts.first().map(String::as_str) == Some("proc")
            && cmd.texts.len() >= 4
            && cmd.texts[2].contains(var_name)
        {
            let shadows = crate::tcl_expr_eval::split_tcl_list(&cmd.texts[2])
                .iter()
                .any(|el| el.split_whitespace().next() == Some(var_name));
            if shadows {
                return None;
            }
        }

        if cmd.texts.first().map(String::as_str) != Some("set") {
            continue;
        }
        if cmd.texts.len() < 3 {
            continue;
        }
        if cmd.texts[1] != var_name {
            continue;
        }
        // Most recent assignment wins.  If it's dynamic, the
        // runtime value can't be proven statically.
        if cmd.single_token_word.get(2).copied() != Some(true) {
            return None;
        }
        if cmd.argv.len() < 3 {
            return None;
        }
        let value_tok = cmd.argv[2];
        if !matches!(
            value_tok.kind,
            tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str
        ) {
            return None;
        }
        return Some((cmd.texts[2].clone(), value_tok.span, var_name.to_string()));
    }
    None
}

/// Return `true` if `text` contains any of the dialect-gated
/// expression operator keywords (`lt`, `le`, `gt`, `ge`, `in`, `ni`)
/// as a whole word — i.e. surrounded by non-identifier bytes or
/// the text boundary.  Used as a fast prefilter to skip the
/// expression parse for expressions that obviously can't trigger
/// W003.
///
/// Whitespace-aware: tabs, newlines, and any other non-identifier
/// byte (parentheses, operators, comparison glyphs, etc.) count
/// as word boundaries.  Matches Tcl expr's tolerance for
/// arbitrary whitespace between tokens.
pub(super) fn contains_gated_word(text: &str) -> bool {
    const GATED: &[&[u8]] = &[b"lt", b"le", b"gt", b"ge", b"in", b"ni"];
    let bytes = text.as_bytes();
    for needle in GATED {
        let n = needle.len();
        let mut i = 0;
        while i + n <= bytes.len() {
            if &bytes[i..i + n] == *needle {
                let before_ok = i == 0 || !is_ident_continue(bytes[i - 1]);
                let after_ok = i + n == bytes.len() || !is_ident_continue(bytes[i + n]);
                if before_ok && after_ok {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

fn walk_dialect_invalid_ops(
    node: &ExprNode,
    pre_85: bool,
    pre_90: bool,
    found: &mut Vec<&'static str>,
) {
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_dialect_invalid_ops(left, pre_85, pre_90, found);
            walk_dialect_invalid_ops(right, pre_85, pre_90, found);
            match op {
                BinOp::In if pre_85 => found.push("in"),
                BinOp::Ni if pre_85 => found.push("ni"),
                BinOp::StrLt if pre_90 => found.push("lt"),
                BinOp::StrLe if pre_90 => found.push("le"),
                BinOp::StrGt if pre_90 => found.push("gt"),
                BinOp::StrGe if pre_90 => found.push("ge"),
                _ => {}
            }
        }
        ExprNode::Unary { operand, .. } => {
            walk_dialect_invalid_ops(operand, pre_85, pre_90, found);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_dialect_invalid_ops(condition, pre_85, pre_90, found);
            walk_dialect_invalid_ops(true_branch, pre_85, pre_90, found);
            walk_dialect_invalid_ops(false_branch, pre_85, pre_90, found);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_dialect_invalid_ops(arg, pre_85, pre_90, found);
            }
        }
        _ => {}
    }
}
