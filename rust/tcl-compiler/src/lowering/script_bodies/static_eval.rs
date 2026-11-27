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

//! `eval` and `uplevel` whose script is statically known: level parsing, list-literal bodies and the dynamic barriers that stop inlining.

use super::super::{Lowerer, SUBST_NOCOMMANDS_KINDS};
use crate::ir::Statement;
use tcl_lexer::{SourceSegmentationPurpose, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};
use tcl_source::segment::SegmentedCommand;

/// If *`cmd_text`* is `list lit1 lit2 ...` with all-literal
/// arguments, return the body text the list would evaluate to —
/// otherwise `None`.
///
/// Literal means `Esc` / `Str` token only: no `$var` substitution,
/// no nested command substitution. `Str` (`{...}`) tokens are
/// re-braced in the synthesised body so list-canonicalisation
/// stays correct (we trust the segmenter's `single_token_word`
/// flag plus the absence of `$` / `[` in `Esc` text).
pub(in super::super) fn eval_list_literal_body(
    cmd_text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let image = tcl_lexer::SourceImage::native(cmd_text.as_bytes());
    let inner = tcl_source::segment::segment_commands_image_with_offset_and_config(
        &image,
        0,
        config.nested(),
    )?;
    if inner.len() != 1 {
        return None;
    }
    let inner_cmd = &inner[0];
    if inner_cmd.texts.is_empty() || inner_cmd.texts[0] != "list" {
        return None;
    }
    let argv = inner_cmd.arg_tokens();
    let texts = inner_cmd.args();
    let single = &inner_cmd.single_token_word;
    // Each element after ``list`` must be a single-token literal.
    for (i, tok) in argv.iter().enumerate() {
        // ``single`` is per-word over the full argv (including
        // the command word at index 0) — argv here starts at
        // index 1 of the original argv, so the matching single-
        // token-word index is ``i + 1``.
        if !single.get(i + 1).copied().unwrap_or(false) {
            return None;
        }
        if !matches!(tok.kind, TokenType::Esc | TokenType::Str) {
            return None;
        }
        if tok.kind == TokenType::Esc {
            let text = &texts[i];
            if text.contains('$') || text.contains('[') {
                return None;
            }
        }
    }
    let mut parts: Vec<String> = Vec::with_capacity(argv.len());
    for (i, tok) in argv.iter().enumerate() {
        let text = &texts[i];
        if tok.kind == TokenType::Str {
            parts.push(format!("{{{text}}}"));
        } else {
            parts.push(text.clone());
        }
    }
    Some(parts.join(" "))
}

/// Drop const-map entries that *stmt* may have overwritten.
/// Straight-line assignments pop just the named variable;
/// structured IR and barriers conservatively clear the whole map
/// because their child scopes (or runtime side effects) could
/// touch any tracked name.
/// Token-level check that a relaxed-eval /
/// relaxed-uplevel body is free of nested dynamic-shape barriers.
///
/// When the eval/uplevel hooks consider relaxing a braced-literal
/// body to inline IR, they first walk the body's command words and
/// reject any nested `eval`/`uplevel` whose own body argument is
/// substitution-bearing (`$var` / `[cmd]` / multi-token).  Without
/// this gate, a static braced `eval {uplevel 1 $x}` would relax to
/// IR that runs a compiled `uplevel` with a still-dynamic body.
///
/// The walk is deliberately shallow and token-based:
///
/// 1. Segment the body into commands.
/// 2. For each command the registry marks as evaluating code, ask the
///    registry which words its script is made of and poison unless
///    every one of them is a braced literal (`Str`) word.
/// 3. Recurse into nested braced bodies and into braced-arg shapes
///    of non-barrier commands so a nested
///    `if { … } { eval $x }` still trips the gate.
///
/// Which word is the script is **never** re-derived here:
/// [`CommandRegistry::arg_indices_for_role`] answers it, so `uplevel`'s
/// optional-level shape is resolved by the one contract-tested
/// `uplevel_arg_roles` resolver instead of a second, subtly different
/// level-word sniff.  The barrier test likewise composes subcommand
/// traits ([`CommandRegistry::invocation_traits`]), so the compound
/// eval-family members — `namespace eval`, `namespace inscope`,
/// `interp eval`, all of which carry the eval-family bits on the
/// *subcommand* — flow through the same path a bare `eval` does.
///
/// Returns `true` when the body contains a nested dynamic-shape
/// barrier (the caller should fall back to [`Statement::Barrier`]); `false`
/// when the body is safe to relax.
pub(in super::super) fn body_has_dynamic_barrier(
    body_text: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> bool {
    use tcl_lexer::TokenType;
    use tcl_registry::prelude::Traits;
    let image = tcl_lexer::SourceImage::native(body_text.as_bytes());
    let Some(commands) = tcl_source::segment::segment_commands_image_with_offset_and_config(
        &image,
        0,
        config.nested(),
    ) else {
        return true;
    };
    for sc in &commands {
        if sc.argv.is_empty() || sc.texts.is_empty() {
            continue;
        }
        let raw_name = sc.texts[0].as_str();
        let selected = registry
            .get(raw_name)
            .map(tcl_registry::CommandSpec::descriptor_key);
        let name = selected.map_or(raw_name, tcl_registry::CanonicalCommandDescriptorKey::label);
        let args: Vec<&str> = sc.texts[1..].iter().map(String::as_str).collect();
        // A "dynamic barrier" command evaluates a script (`eval`,
        // `uplevel`, `namespace eval`, `interp eval`, …).  Sourced from
        // the registry's `EVALUATES_CODE` trait rather than a hardcoded
        // name list.  `None` because the question is the
        // command's *shape*, not its availability: a barrier is a barrier
        // whichever dialect the file is analysed as.  Deliberate under
        // invariant I4 — a barrier answer *widens* (falls back to
        // `Statement::Barrier`), never specialises on an unproved
        // binding.
        let traits = registry.invocation_traits(name, &args, None);
        if !traits.contains(Traits::EVALUATES_CODE) {
            // Recurse into braced args of non-barrier commands so
            // nested barriers still trip the gate.
            for (i, tok) in sc.argv.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                if tok.kind != TokenType::Str {
                    continue;
                }
                let arg_text = sc.texts.get(i).map_or("", String::as_str);
                if body_has_dynamic_barrier(arg_text, registry, config) {
                    return true;
                }
            }
            continue;
        }
        // Name is a barrier — inspect the words its script is made of.
        let body_indices = registry.arg_indices_for_role(name, &args, ArgRole::Body);
        let Some(&first_body) = body_indices.first() else {
            // The registry can point at no script word: a malformed call
            // (`uplevel 1` — a wrong-#args error) or an evaluator whose
            // code argument is a command *prefix* rather than a script
            // (`coroprobe` / `coroinject`).  Poison so the outer hook
            // falls back to Statement::Barrier and the runtime decides.
            return true;
        };
        // `SCRIPT_CONCATENATES_ARGS`: the trailing words concatenate into
        // the one script Tcl evaluates, so `uplevel 1 {set x 1} $tail`
        // is dynamic even though the marked body word is a literal.  Check
        // the whole tail from the first script word, not just the marked
        // ones.
        let script_words: Vec<usize> = if traits.contains(Traits::SCRIPT_CONCATENATES_ARGS) {
            (first_body..args.len()).collect()
        } else {
            body_indices
        };
        for idx in script_words {
            // `sc.argv` / `sc.texts` include the command name at 0, so the
            // arg-relative index shifts by one.
            let (Some(tok), Some(text)) = (sc.argv.get(idx + 1), sc.texts.get(idx + 1)) else {
                return true;
            };
            if tok.kind != TokenType::Str {
                return true;
            }
            // Recurse into the literal nested script word.
            if body_has_dynamic_barrier(text, registry, config) {
                return true;
            }
        }
    }
    false
}

impl Lowerer<'_> {
    /// Lower `namespace eval ns body`.
    /// Try to lower `uplevel ?level? {body}` to a static-body
    /// [`Statement::UpFrame`] when:
    ///
    /// 1. The body argument is a brace-string token (`TokenType::Str`),
    ///    and
    /// 2. The level argument (if present) parses as a positive integer
    ///    or `#0` / `#N` global form.
    ///
    /// Returns `None` if the call doesn't match the static shape, in
    /// which case the caller falls back to [`Self::lower_default`]
    /// (producing a runtime [`Statement::Barrier`]).
    pub(in super::super) fn try_lower_uplevel_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;

        // Static-body `uplevel` lowers to `Statement::UpFrame` so the analysers
        // (interproc purity, code-sinking, var-escape, the inline_uplevel pass)
        // see a structured frame-shift with a lowered body instead of an opaque
        // runtime barrier. Runtime correctness is preserved at codegen: a
        // surviving `UpFrame` re-emits the original `uplevel <level> {body}`
        // invoke from its preserved command tokens (see
        // `codegen::statements`), producing byte-identical bytecode to the old
        // barrier path and dispatching through the `uplevel` builtin's correct
        // frame semantics. The whole-callee inline_uplevel splice (which would
        // need frame-shift opcodes to run a *bare* UpFrame against the right
        // activation) is an opt-in optimiser pass, not wired into codegen.
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let tokens = self.cmd_tokens(seg);
        let invocation = self.resolved_metadata_invocation(&tokens)?;
        let frame = invocation.facts.frame_effect?;
        if frame.layout != tcl_registry::frame_effect::FrameArgLayout::ScriptInSelectedFrame {
            return None;
        }
        let offset = invocation.facts.argument_offset;
        let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
            level_word_len,
            level,
        } = invocation.with_argument_words(|words| {
            frame.resolve_arguments(words.arguments().slice_from(offset))
        })
        else {
            return None;
        };
        // A multiword body first undergoes the selected engine's concat.
        // Only an original single body operand can use the direct-script path.
        let body_argument = offset.checked_add(level_word_len)?;
        if invocation.arguments.len() != body_argument.checked_add(1)? {
            return None;
        }
        let body_tok_idx = invocation.effective.written_argument(body_argument)?;
        let (shift, absolute) = match level {
            tcl_registry::FrameLevel::Relative(number) => (i32::try_from(number).ok()?, false),
            tcl_registry::FrameLevel::Absolute(number) => (i32::try_from(number).ok()?, true),
            tcl_registry::FrameLevel::Dynamic => return None,
        };
        let body_tok = arg_tokens.get(body_tok_idx)?;
        // Barrier gate: see `try_lower_eval_static` for the
        // rationale.  A static-body `uplevel 1 {eval $x}` would
        // relax to inline IR with a still-dynamic `eval`; reject.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[body_tok_idx];
            if body_has_dynamic_barrier(body_text, self.registry, self.config) {
                return None;
            }
        }
        let body_namespace = match (absolute, shift) {
            (true, 0) => "::",
            // A non-zero relative frame is selected only at runtime. Keep the
            // lexical fallback for lowering; ExecutionNamespace later marks
            // the body runtime-selected and invalidates relative typed proof.
            _ => namespace,
        };
        let body = if body_tok.kind == TokenType::Str {
            self.lower_body_from_tok(&args[body_tok_idx], Some(body_tok), body_namespace)
        } else if body_tok.kind == TokenType::Var {
            // Retain the original Native argument or quiet Logical source value.
            let literal = self
                .original_native_written_argument_value(seg, body_tok_idx)
                .or_else(|| self.const_map_lookup(&args[body_tok_idx]))?;
            self.lower_script(
                &literal,
                body_namespace,
                SourceSegmentationPurpose::NestedScript,
            )
        } else {
            return None;
        };
        Some(Statement::UpFrame {
            span: seg.span,
            frame_shift: shift,
            absolute,
            body,
            tokens: Some(tokens),
        })
    }

    /// If *`cmd_text`* is a call whose template-word plan
    /// (`docs/design/compiler/value-transfers.md` § *The template-word
    /// plan*) runs exactly [`SUBST_NOCOMMANDS_KINDS`] over a braced
    /// template AND every variable that template reads is in the current
    /// const-map, return the substituted string. Otherwise `None` so the
    /// caller falls back to runtime dispatch.
    ///
    /// Used to materialise the tcltest-style `Option` factory body
    /// at compile time when the surrounding proc has all the
    /// template vars const-tracked.
    ///
    /// The plan is the registry's answer over the call's literal words: any
    /// other set of kinds is a call this evaluator does not reproduce, and a
    /// computed switch word is no spelling a release accepts, so it folds
    /// nothing.
    pub(in super::super) fn eval_subst_nocommands_body(&self, cmd_text: &str) -> Option<String> {
        use tcl_lexer::TokenType;
        let inner = self.segment_source(cmd_text, 0, SourceSegmentationPurpose::NestedScript)?;
        let [inner_cmd] = inner.as_slice() else {
            return None;
        };
        let head = inner_cmd.texts.first()?;
        let texts = inner_cmd.args();
        let arg_refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        // A braced word is one `Str` token; only it is a template this can
        // substitute at compile time.
        let source = |index: usize| {
            let braced = inner_cmd
                .single_token_word
                .get(index + 1)
                .copied()
                .unwrap_or(false)
                && inner_cmd
                    .arg_tokens()
                    .get(index)
                    .is_some_and(|token| token.kind == TokenType::Str);
            crate::value_transfer::SourceWord::of(arg_refs.get(index).copied(), braced)
        };
        let plan =
            crate::value_transfer::literal_template_plan(self.registry, head, &arg_refs, source)?;
        if plan.kinds != SUBST_NOCOMMANDS_KINDS || self.proc_depth == 0 {
            return None;
        }
        let template = arg_refs.get(plan.operand.0)?;
        let scope = self.const_map_stack.last()?;
        crate::subst_nocommands::subst_nocommands(template, &plan, scope)
    }

    /// Try to lower `eval ?body?` to a static-body
    /// [`Statement::Block`] when the body is a brace-string literal,
    /// a const-mapped `$var`, or an `eval [list lit1 lit2 ...]`
    /// command-substitution shape. Returns `None` for dynamic
    /// bodies so the caller falls through to runtime barrier
    /// dispatch.
    pub(in super::super) fn try_lower_eval_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        error_context: Option<tcl_registry::InlineBodyErrorContext>,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        // Single-body shape only: ``eval $body`` / ``eval {body}``.
        // The list form (``eval cmd arg1 arg2``) keeps runtime
        // semantics — joining the words with spaces is observable.
        if args.len() != 1 || arg_tokens.is_empty() {
            return None;
        }
        let body_tok = arg_tokens[0];
        // Barrier gate: a braced literal body might contain a
        // nested ``eval $x`` / ``uplevel $lvl {...}`` whose own body
        // is still dynamic.  Relaxing the outer barrier in that case
        // produces IR that runs a compiled inner barrier with a
        // still-dynamic shape — we'd lose the runtime barrier without
        // gaining static knowledge.  Reject and fall back to the
        // default Statement::Barrier dispatch.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[0];
            if body_has_dynamic_barrier(body_text, self.registry, self.config) {
                return None;
            }
            // Parse gate: inlining lowers the body with the LSP-lenient
            // lowering, which gives malformed text a meaning C never gives it
            // — `eval {set y "a"b}` would quietly mean `set y ab` instead of
            // raising `extra characters after close-quote` (#1829).  C parses
            // an `eval` body when the command runs, so decline the relaxation
            // and fall back to the runtime barrier, which is the path a
            // non-literal `eval $body` already takes and which reports the
            // error correctly.
            if !matches!(
                tcl_lexer::first_parse_cut_image_checked(
                    &tcl_lexer::SourceImage::from_bytes(
                        self.guarded_body_text(body_tok, body_text).as_bytes(),
                        self.current_source_channel()
                    ),
                    self.config.nested()
                ),
                Ok(None)
            ) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            self.lower_body_from_tok(&args[0], Some(&body_tok), namespace)
        } else if body_tok.kind == TokenType::Var {
            // The script `eval` evaluates at run time is a unit of its own
            // (8.4 runs it directly), which creates no locals and reaches only
            // those the procedure compiled (`TclFindCompiledLocal`), and whose
            // errors name its own commands; folded into the body it would
            // create them and be logged as the body's text. Analysis keeps the
            // structure it can see.
            if self.target.is_bytecode() {
                return None;
            }
            let literal = self
                .original_native_written_argument_value(seg, 0)
                .or_else(|| self.const_map_lookup(&args[0]))?;
            if body_has_dynamic_barrier(&literal, self.registry, self.config) {
                return None;
            }
            self.lower_script(&literal, namespace, SourceSegmentationPurpose::NestedScript)
        } else if body_tok.kind == TokenType::Cmd {
            // ``eval [list lit1 lit2 ...]`` — synthesise the
            // body by joining the list's literal arguments and
            // re-lowering. The bracket-substitution text retains
            // the surrounding ``[...]``; strip them via
            // ``content_offset`` if present, otherwise the helper
            // strips them itself.
            let inner_text = if body_tok.content_offset > 0 {
                let start = u32::from(body_tok.content_offset) as usize;
                &args[0][start..args[0].len() - start]
            } else {
                args[0].trim_start_matches('[').trim_end_matches(']')
            };
            let synthesised = eval_list_literal_body(inner_text, self.config)?;
            // `eval` runs the list as one command without compiling it; a word
            // the release compiles apart stays with the runtime `eval`.
            let release = self
                .dialect
                .map_or(tcl_dialect::TclVersion::V9_0, |profile| {
                    profile.vm_runtime_version
                });
            if crate::const_subst::folded_literal_reads_apart(&synthesised, release) {
                return None;
            }
            self.lower_script(
                &synthesised,
                namespace,
                SourceSegmentationPurpose::NestedScript,
            )
        } else {
            return None;
        };
        Some(Statement::Block {
            span: seg.span,
            body,
            namespace: namespace.to_string(),
            tokens: Some(self.cmd_tokens(seg)),
            error_context,
        })
    }
}
