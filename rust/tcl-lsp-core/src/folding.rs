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

//! Folding range provider.
//!
//! Emits LSP folding ranges for proc bodies, namespace bodies, comment
//! blocks, and control-structure bodies (`if`, `while`, `for`,
//! `foreach`, `switch`, …).  The algorithm is a scope
//! walk over the analyser's [`AnalysisResult`], a comment-line
//! collector, and a registry-driven body-argument walker that
//! recurses through nested braced bodies, followed by an overlap
//! normalisation post-pass that trims partially overlapping
//! siblings so VS Code's folding tree-builder doesn't drop them.
//!
//! The result is a fully line-resolved `Vec<FoldingRange>`.  The
//! normalisation pass is idempotent, so running it again over this
//! output is harmless.  The Rust LSP server (`tcl-lsp-server`) gets
//! the normalised output it needs directly from this function.
//!
//! [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult

use std::collections::BTreeSet;

use rustc_hash::FxHashSet;

use tcl_compiler::analyser::{Analyser, Scope, ScopeKind};
use tcl_compiler::lambda_literal::split_lambda_literal;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{Lexer, LineIndex, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use crate::oo_body::{HeadWords, is_member, member_body_indices_in, next_definition_grammar};
use tcl_registry::definer::DefinitionBodyGrammar;

/// LSP folding-range kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldKind {
    /// Generic foldable region (proc/namespace/control body).
    Region,
    /// Consecutive `#` comment lines.
    Comment,
}

impl FoldKind {
    /// Lower-case wire form expected by `lsprotocol`
    /// (`"region"`, `"comment"`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Region => "region",
            Self::Comment => "comment",
        }
    }
}

/// A single folding range — start/end lines (inclusive, 0-based) and
/// the LSP kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldingRange {
    /// First line of the fold (0-based, inclusive).
    pub start_line: u32,
    /// Last line of the fold (0-based, inclusive).
    pub end_line: u32,
    /// LSP fold kind.
    pub kind: FoldKind,
}

/// Compute folding ranges for a Tcl source document.
///
/// Runs the Rust analyser internally for scope folds, then walks
/// the segmented token stream for body-argument and comment folds.
///
/// `registry` is the [`CommandRegistry`] consulted for body-arg
/// roles. The caller owns the registry — typically the LSP server's
/// `Backend` builds and dialect-loads it once per session, and the
/// `PyO3` binding caches a default instance — so this function never
/// rebuilds it. `dialect` is forwarded to the analyser for
/// dialect-specific scope semantics.
///
/// Overlap normalisation runs as a post-pass via
/// [`normalise_overlaps`] so the returned vector is always disjoint or
/// properly nested — VS Code's folding tree-builder rejects
/// partially overlapping siblings, and the Rust LSP server consumes
/// this output directly.  The pass is idempotent.
#[must_use]
pub fn folding_ranges(
    source: &str,
    dialect: &str,
    registry: &CommandRegistry,
) -> Vec<FoldingRange> {
    if source.is_empty() {
        return Vec::new();
    }

    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, dialect);

    let line_index = LineIndex::new(source);
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    let mut ranges: Vec<FoldingRange> = Vec::new();

    collect_scope_folds(
        &analysis.global_scope,
        source,
        &line_index,
        &mut seen,
        &mut ranges,
    );
    collect_comment_folds(source, &mut seen, &mut ranges);
    collect_continuation_folds(source, &line_index, &mut seen, &mut ranges);
    let identities =
        tcl_compiler::head_identity::command_head_identities(source, dialect, registry);
    let mut ctx = FoldCtx {
        registry,
        availability: tcl_dialect::DialectProfile::by_name(dialect).availability_mask,
        identities: &identities,
        line_index: &line_index,
        original_source: source,
        seen: &mut seen,
        ranges: &mut ranges,
        config: tcl_lexer::LexerConfig::for_file_dialect(dialect),
    };
    collect_body_folds(
        source, 0, 0, None, // top-level body is not inside a definition body
        &mut ctx,
    );

    normalise_overlaps(ranges)
}

/// Return a fold end line that leaves the closing ``}`` visible.
///
/// When a braced body's last content byte sits on the same line as
/// its closing ``}``, trim the fold end up by one so sibling folds
/// remain disjoint (the `} else {` regression — VS Code's folding
/// tree-builder rejects partially overlapping siblings).  Bodies
/// terminated by a newline before ``}`` keep their end line as-is;
/// unterminated bodies (no closing ``}`` yet — common while the
/// user is mid-edit) also keep their end line.
fn adjust_body_end_line(source: &str, end_offset: usize, end_line: u32) -> u32 {
    let bytes = source.as_bytes();
    if end_offset >= bytes.len() {
        return end_line;
    }
    if bytes[end_offset] == b'\n' {
        return end_line;
    }
    if end_offset + 1 < bytes.len() && bytes[end_offset + 1] == b'}' {
        return end_line.saturating_sub(1);
    }
    end_line
}

fn push_unique(
    seen: &mut FxHashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
    start_line: u32,
    end_line: u32,
    kind: FoldKind,
) {
    if end_line > start_line && seen.insert((start_line, end_line)) {
        ranges.push(FoldingRange {
            start_line,
            end_line,
            kind,
        });
    }
}

fn emit_body_span_fold(
    span: tcl_lexer::Span,
    source: &str,
    line_index: &LineIndex,
    seen: &mut FxHashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    if span.is_empty() {
        return;
    }
    let start_line = line_index.line_at(span.start());
    let end_offset = span.end().saturating_sub(1) as usize;
    let raw_end_line = line_index.line_at(span.end().saturating_sub(1));
    if start_line < raw_end_line {
        let end_line = adjust_body_end_line(source, end_offset, raw_end_line);
        push_unique(seen, ranges, start_line, end_line, FoldKind::Region);
    }
}

/// Walk the analyser scope tree and emit folds for proc bodies and
/// namespace bodies.
fn collect_scope_folds(
    scope: &Scope,
    source: &str,
    line_index: &LineIndex,
    seen: &mut FxHashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    for proc_def in scope.procs.values() {
        emit_body_span_fold(proc_def.body_span, source, line_index, seen, ranges);
    }
    for child in &scope.children {
        if matches!(child.kind, ScopeKind::Namespace)
            && let Some(span) = child.body_span
        {
            emit_body_span_fold(span, source, line_index, seen, ranges);
        }
        collect_scope_folds(child, source, line_index, seen, ranges);
    }
}

/// Walk lines and emit folds for runs of consecutive ``#`` comment
/// lines.  An internal block (one followed by a non-comment line)
/// needs at least three lines; a trailing block at end-of-file needs
/// at least two.
///
/// Shared with the BIG-IP folding path ([`crate::bigip::folding_ranges`]),
/// which reuses it verbatim so a comment block folds identically whether
/// it sits in Tcl source or in a `.conf`.
pub(crate) fn collect_comment_folds(
    source: &str,
    seen: &mut FxHashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut block_start: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim_start();
        if stripped.starts_with('#') {
            if block_start.is_none() {
                block_start = Some(i);
            }
        } else {
            if let Some(start) = block_start
                && i - start >= 2
            {
                let s = u32::try_from(start).expect("line index fits u32");
                let e = u32::try_from(i - 1).expect("line index fits u32");
                if seen.insert((s, e)) {
                    ranges.push(FoldingRange {
                        start_line: s,
                        end_line: e,
                        kind: FoldKind::Comment,
                    });
                }
            }
            block_start = None;
        }
    }
    // Trailing comment block at EOF.
    if let Some(start) = block_start {
        let end = lines.len().saturating_sub(1);
        if end > start {
            let s = u32::try_from(start).expect("line index fits u32");
            let e = u32::try_from(end).expect("line index fits u32");
            if seen.insert((s, e)) {
                ranges.push(FoldingRange {
                    start_line: s,
                    end_line: e,
                    kind: FoldKind::Comment,
                });
            }
        }
    }
}

/// Collect folds for commands spread across multiple physical lines
/// via `\<newline>` continuations.
///
/// Each backslash-newline at a word boundary lexes to a `Sep` token
/// that starts at the backslash (ordinary whitespace Seps start with a
/// space/tab), so the set of continued lines — the lines bearing a
/// trailing `\` — falls straight out of the token stream. A command
/// spanning lines `a..=a+k` has continuations on lines `a..=a+k-1`;
/// consecutive continued lines therefore fold into one region from the
/// first continued line through the line after the last continuation.
fn collect_continuation_folds(
    source: &str,
    line_index: &LineIndex,
    seen: &mut FxHashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
        return;
    };
    let bytes = source.as_bytes();
    let mut continued: BTreeSet<u32> = BTreeSet::new();
    for tok in &tokens {
        if tok.kind != TokenType::Sep {
            continue;
        }
        let start = tok.span.start();
        if bytes.get(start as usize) == Some(&b'\\') {
            continued.insert(line_index.line_at(start));
        }
    }

    // Emit one fold per maximal run of consecutive continued lines.
    let mut iter = continued.into_iter();
    let Some(mut run_start) = iter.next() else {
        return;
    };
    let mut prev = run_start;
    for line in iter {
        if line != prev + 1 {
            push_unique(seen, ranges, run_start, prev + 1, FoldKind::Region);
            run_start = line;
        }
        prev = line;
    }
    push_unique(seen, ranges, run_start, prev + 1, FoldKind::Region);
}

/// Recursively segment commands and emit folds for every multi-line
/// `BODY`-roled argument.
///
/// `inside_oo_body` tracks whether we are recursing through the
/// body of an outer OO definition command (`oo::class create` /
/// `oo::define` / `oo::objdefine`). The inner OO commands
/// (`method`, `constructor`, …) are body-bearing only inside that
/// context — outside it, a user proc named `method` must not be
/// misidentified.
///
/// Per-walk context for [`collect_body_folds`].
///
/// Bundles the immutable references that don't change across the
/// recursive walk (`registry`, `line_index`, `original_source`)
/// and the two mutable accumulators (`seen`, `ranges`).  Only
/// `body_source`, `base_offset`, `depth`, and `inside_oo_body`
/// vary per call — they stay as direct parameters.
struct FoldCtx<'a> {
    registry: &'a CommandRegistry,
    /// Selected profile availability. Member layouts use this to reject a
    /// version-gated option instead of treating it as a fixed-tail word.
    availability: tcl_dialect::DialectSet,
    /// The document's statically proven command-identity facts, so a body-arg
    /// role is resolved against the command a head *is* rather than the one it
    /// is spelled as (issue #1275).  Empty — and lookup-free — for the
    /// overwhelmingly common document that binds nothing.
    identities: &'a tcl_compiler::head_identity::HeadIdentityMap,
    line_index: &'a LineIndex,
    original_source: &'a str,
    seen: &'a mut FxHashSet<(u32, u32)>,
    ranges: &'a mut Vec<FoldingRange>,
    /// Dialect lexer config so body re-segmentation honours `{*}` / `}{`.
    /// Whole-file lexer config; nested body slices take
    /// [`tcl_lexer::LexerConfig::at_depth`] of it.
    config: tcl_lexer::LexerConfig,
}

/// Recursion depth is capped at 20.
/// Defensive recursion bound for nested-body fold collection, set to match the
/// compiler analyser's `MAX_BODY_DEPTH` so deeply (but validly) nested code
/// keeps full folding support. Real source never nests anywhere near this.
const MAX_FOLD_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// One segmented command's head, as written and as it resolves.
///
/// The written spelling is sliced from the command itself; the resolved name
/// comes from the document's [`HeadIdentityMap`](tcl_compiler::head_identity::HeadIdentityMap)
/// at the head's own byte offset, so a binding never retroactively re-tags an
/// earlier call.  A document that binds nothing skips the lookup entirely.
fn resolve_head<'a>(
    identities: &'a tcl_compiler::head_identity::HeadIdentityMap,
    cmd: &'a tcl_compiler::segmenter::SegmentedCommand,
) -> HeadWords<'a> {
    let at = cmd.argv.first().map_or(0, |t| t.span.start());
    identities.head_words(cmd.name(), at)
}

fn collect_body_folds(
    body_source: &str,
    base_offset: u32,
    depth: u32,
    oo_grammar: Option<&'static DefinitionBodyGrammar>,
    ctx: &mut FoldCtx<'_>,
) {
    if MAX_FOLD_DEPTH.exceeded(depth) {
        return;
    }
    // The whole-file lexer rule (which may skip a leading byte-order mark)
    // applies at the top level only — a mark at the head of a *nested* body
    // slice is ordinary data (issue #1243).
    let commands = segment_commands_with_offset_and_config(
        body_source,
        base_offset,
        ctx.config.at_depth(depth),
    );
    for cmd in &commands {
        if cmd.argv.is_empty() {
            continue;
        }
        let args_borrow: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
        // The head's *effective command identity*, resolved exactly as the
        // semantic-token walk resolves it: a proven `interp alias` / `rename` /
        // `namespace import` answers with the command the head really names, and
        // a spelling whose binding was provably taken over answers with nothing,
        // so no registry grammar is applied to it (issue #1275).  The member
        // sub-keyword test below deliberately keeps the *written* spelling —
        // `method` inside a class body is a lexical keyword, not a command
        // binding a top-level `rename` could move.
        let head = resolve_head(ctx.identities, cmd);

        // Outer definer commands (`oo::class`, `oo::define`, `snit::type`, …)
        // carry their body-arg shapes in the registry — `arg_indices_for_role`
        // returns the right index. The context-sensitive member sub-keywords
        // (`method`, `constructor`, `typemethod`, `self`, `property`, …) have
        // no `CommandSpec`; their body indices come from the enclosing
        // definition-body grammar ([`crate::oo_body`]).
        let body_indices: Vec<usize> = match oo_grammar {
            Some(g) if is_member(g, head.written) => {
                member_body_indices_in(g, head.written, &args_borrow, ctx.availability)
            }
            _ => ctx
                .registry
                .arg_indices_for_role(head.resolved, &args_borrow, ArgRole::Body),
        };

        // A clause-list command (`switch … {pat body …}`, Expect's `expect
        // {…}`) marks that single trailing word `ArgRole::Body` too, but the
        // word is a *list* of pattern/body pairs, not a script: re-segmenting
        // it below would read each `pat body` pair as one bogus command, find
        // no body role on it, and emit nothing for the arms — only the outer
        // block folded (issue #1216).  Which word holds the list, and how the
        // list is shaped, is registry data (`CommandSpec::case_list`), so this
        // walk names no command.
        let case_list = ctx
            .registry
            .get(head.resolved)
            .and_then(|s| s.case_list)
            .and_then(|spec| {
                let dialect = ctx
                    .registry
                    .profile()
                    .map_or_else(tcl_dialect::DialectSet::empty, |profile| {
                        profile.availability_mask
                    });
                ctx.registry
                    .case_invocation(head.resolved, &args_borrow, dialect)
                    .and_then(|(_, invocation)| {
                        invocation.clause_list_index.map(|index| (spec, index))
                    })
            });

        // The grammar the recursion into THIS command's bodies should carry:
        // outer definer bodies switch to their grammar, member bodies switch
        // off, everything else inherits.
        let next_grammar = next_definition_grammar(head, &args_borrow, oo_grammar, ctx.registry);

        for idx in body_indices {
            let arg_tokens = cmd.arg_tokens();
            if idx >= arg_tokens.len() {
                continue;
            }
            let body_tok = arg_tokens[idx];
            if !matches!(body_tok.kind, TokenType::Str) {
                continue;
            }
            emit_body_span_fold(
                body_tok.span,
                ctx.original_source,
                ctx.line_index,
                ctx.seen,
                ctx.ranges,
            );

            // The clause-list word folds as one block (just emitted), then
            // each arm's own body folds and is recursed as the script it is.
            if let Some((spec, case_idx)) = case_list
                && case_idx == idx
            {
                collect_clause_folds(body_tok, spec, depth, ctx);
                continue;
            }

            // Recurse into the body's content. The lexer's STR span
            // includes the opening ``{``; for closed non-empty bodies
            // it ends just before the closing ``}``, for closed empty
            // bodies it includes the ``}`` (degenerate clamp), and for
            // unclosed bodies it runs to EOF.  Slice the absolute
            // source so recursive spans stay in original-source space.
            let span = body_tok.span;
            let content_start = span.start() as usize + body_tok.content_offset as usize;
            let raw_end = span.end() as usize;
            let bytes = ctx.original_source.as_bytes();
            // Strip the trailing ``}`` for the empty-body clamp case
            // so the sub-lexer doesn't see a stray close-brace.
            let content_end = if raw_end > content_start
                && raw_end - content_start == 1
                && bytes.get(raw_end - 1) == Some(&b'}')
            {
                content_start
            } else {
                raw_end
            };
            if content_end <= content_start {
                continue;
            }
            let inner = &ctx.original_source[content_start..content_end];
            collect_body_folds(
                inner,
                u32::try_from(content_start).expect("content offset fits u32"),
                depth + 1,
                next_grammar,
                ctx,
            );
        }

        collect_lambda_folds(cmd, head.resolved, &args_borrow, depth, ctx);
    }
}

/// `apply {argList body ?ns?} …` (and any future command sharing the shape) —
/// fold the whole lambda literal as one region, but recurse only into the real
/// body element (`split_lambda_literal`, issue #954): the argument-list element
/// is a plain word/list, not code, so re-segmenting the whole literal as a
/// script previously mis-read the params word as a command name and never found
/// the real body's own nested folds.
fn collect_lambda_folds(
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
    resolved_head: &str,
    args: &[&str],
    depth: u32,
    ctx: &mut FoldCtx<'_>,
) {
    for idx in ctx
        .registry
        .arg_indices_for_role(resolved_head, args, ArgRole::LambdaLiteral)
    {
        let arg_tokens = cmd.arg_tokens();
        let Some(&lambda_tok) = arg_tokens.get(idx) else {
            continue;
        };
        if !matches!(lambda_tok.kind, TokenType::Str) {
            continue;
        }
        emit_body_span_fold(
            lambda_tok.span,
            ctx.original_source,
            ctx.line_index,
            ctx.seen,
            ctx.ranges,
        );
        let Some(elems) = split_lambda_literal(ctx.original_source, lambda_tok) else {
            continue;
        };
        let Some(body_span) = elems.body else {
            continue;
        };
        let (bstart, bend) = (body_span.start() as usize, body_span.end() as usize);
        if bend <= bstart {
            continue;
        }
        let Some(inner) = ctx.original_source.get(bstart..bend) else {
            continue;
        };
        // The body runs in a fresh, non-OO frame (`apply`'s own scope),
        // never the enclosing definer's grammar.
        collect_body_folds(inner, body_span.start(), depth + 1, None, ctx);
    }
}

/// Fold every arm of a clause list: one region per arm body, recursing into
/// each as a script so nested `if` / `foreach` / `proc` blocks inside an arm
/// fold too.
///
/// `list_tok` is the braced clause-list word itself (already folded as one
/// block by the caller).  The split is [`tcl_syntax::case_list`]'s — the same
/// one the semantic-token walker and the iRules object walker use, so the
/// three cannot disagree about where an arm's body is.  It is a **list**
/// split, not a command segmentation: `;` and `#` are ordinary pattern text
/// inside a clause list, which is Tcl's "comments don't work in `switch`"
/// gotcha.
///
/// A `-` fall-through body and a single-line arm both fold to nothing — the
/// shared `end_line > start_line` guard drops them — so no arm-specific
/// filtering is needed here.
fn collect_clause_folds(
    list_tok: tcl_lexer::Token,
    spec: &'static tcl_registry::CaseListSpec,
    depth: u32,
    ctx: &mut FoldCtx<'_>,
) {
    let content_start = list_tok.span.start() as usize + list_tok.content_offset as usize;
    let content_end = list_tok.span.end() as usize;
    if content_end <= content_start {
        return;
    }
    let Some(inner) = ctx.original_source.get(content_start..content_end) else {
        return;
    };
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: spec.clause_flags,
        clause_value_flags: spec.clause_value_flags,
    };
    for clause in tcl_syntax::case_list::split_case_list(inner, &shape) {
        let Some(body) = clause.body else {
            continue;
        };
        // `Element::start` includes the opening `{` and `Element::end` sits
        // *at* the closing `}` — the lexer's own inner-end convention, so the
        // span drops straight into the shared emitter.
        let (abs_start, abs_end) = (content_start + body.start, content_start + body.end);
        let (Ok(start), Ok(end)) = (u32::try_from(abs_start), u32::try_from(abs_end)) else {
            continue;
        };
        emit_body_span_fold(
            tcl_lexer::Span::new(start, end),
            ctx.original_source,
            ctx.line_index,
            ctx.seen,
            ctx.ranges,
        );
        // Only a braced arm body is a script whose source slice is what runs;
        // a bare or quoted one is substituted before the command sees it, so
        // its text is not the script (the same rule `apply`'s lambda body
        // follows).
        if !body.braced {
            continue;
        }
        let inner_start = abs_start + 1;
        if abs_end <= inner_start {
            continue;
        }
        let Some(arm) = ctx.original_source.get(inner_start..abs_end) else {
            continue;
        };
        let Ok(base) = u32::try_from(inner_start) else {
            continue;
        };
        // An arm body runs in the caller's frame, never a definition-body
        // grammar of its own.
        collect_body_folds(arm, base, depth + 1, None, ctx);
    }
}

/// Trim partially overlapping sibling folds so the returned ranges
/// are pairwise disjoint or properly nested.
///
/// VS Code's folding tree-builder silently drops or misplaces ranges
/// that share a boundary line without one containing the other; the
/// individual collectors already try to avoid this via
/// [`adjust_body_end_line`], but a belt-and-suspenders post-pass
/// keeps the output well-formed even if a future collector forgets
/// the invariant.
///
/// `FoldingRange::end_line` is inclusive, so two ranges that share a
/// boundary line (e.g. `[0, 5]` and `[5, 10]`) both include the
/// shared line and are neither disjoint nor strictly nested. When
/// that pattern slips past the collectors:
///
/// * A previously-emitted parent that overlaps the next range gets
///   trimmed back by one line, or dropped if trimming would leave
///   a degenerate fold.
/// * A range that extends past its (new) parent gets trimmed down
///   to the parent's end, or dropped if that would leave it
///   degenerate.
///
/// A final dedup pass drops any duplicate `(start, end, kind)`
/// triples produced by the trimming.
///
/// The pass is idempotent — running it on already-normalised input
/// returns the same set.
#[must_use]
pub fn normalise_overlaps(ranges: Vec<FoldingRange>) -> Vec<FoldingRange> {
    if ranges.is_empty() {
        return ranges;
    }

    // Sort by start ascending, end descending so parents come before
    // children and equal-start ranges with larger spans come first.
    let mut ordered = ranges;
    ordered.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| b.end_line.cmp(&a.end_line))
    });

    // working[i] may be replaced in-place (to trim a previously-emitted
    // parent) or set to None to drop it outright. stack holds indices
    // of currently-open ancestors; entries always reference a live
    // (non-None) working slot — we only set an entry to None
    // immediately before popping its index off the stack.
    let mut working: Vec<Option<FoldingRange>> = Vec::with_capacity(ordered.len());
    let mut stack: Vec<usize> = Vec::new();

    'next: for mut r in ordered {
        // Close or trim ancestors that conflict with r's start.
        while let Some(&top) = stack.last() {
            // Defensive: a stack entry should always reference a live
            // working slot (we only set a slot to None immediately
            // before popping). Bail out safely if the invariant ever
            // breaks instead of unwrapping into a panic.
            let Some(parent) = working[top] else {
                stack.pop();
                continue;
            };
            if parent.end_line < r.start_line {
                stack.pop();
                continue;
            }
            if parent.end_line == r.start_line {
                // Inclusive end_line: a shared boundary still
                // overlaps, so trim the parent back by one line if
                // that leaves a useful fold, otherwise drop it.
                if parent.end_line.saturating_sub(1) > parent.start_line {
                    working[top] = Some(FoldingRange {
                        start_line: parent.start_line,
                        end_line: parent.end_line - 1,
                        kind: parent.kind,
                    });
                } else {
                    working[top] = None;
                }
                stack.pop();
                continue;
            }
            break;
        }

        // Trim r down to fit inside its (new) parent, if any.
        let parent_end = stack
            .last()
            .and_then(|&top| working[top])
            .and_then(|p| (p.end_line < r.end_line).then_some((p.end_line, p.start_line)));
        if let Some((p_end, _p_start)) = parent_end {
            if p_end <= r.start_line {
                // Trim would leave r degenerate or inverted — drop it.
                continue 'next;
            }
            r = FoldingRange {
                start_line: r.start_line,
                end_line: p_end,
                kind: r.kind,
            };
        }

        working.push(Some(r));
        stack.push(working.len() - 1);
    }

    // De-duplicate: trimming may have collapsed distinct inputs onto
    // the same (start, end, kind) triple.
    let mut seen: FxHashSet<(u32, u32, FoldKind)> = FxHashSet::default();
    let mut result: Vec<FoldingRange> = Vec::with_capacity(working.len());
    for slot in working {
        let Some(r) = slot else { continue };
        if seen.insert((r.start_line, r.end_line, r.kind)) {
            result.push(r);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Build a fresh registry for a test.
    ///
    /// Tests build a per-test registry rather than caching one
    /// statically — `CommandRegistry::build_default()` is cheap
    /// (~118 specs of `&'static` data) and avoids a static
    /// `OnceLock` in this crate.
    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Convenience wrapper for the original two-argument test
    /// signature; threads a fresh registry through the new API.
    fn folding_ranges_default(source: &str, dialect: &str) -> Vec<FoldingRange> {
        folding_ranges(source, dialect, &registry())
    }

    fn fold_lines(ranges: &[FoldingRange], kind: FoldKind) -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = ranges
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| (r.start_line, r.end_line))
            .collect();
        out.sort_unstable();
        out
    }

    #[test]
    fn fold_kind_wire_form() {
        assert_eq!(FoldKind::Region.as_str(), "region");
        assert_eq!(FoldKind::Comment.as_str(), "comment");
    }

    #[test]
    fn empty_source_yields_no_folds() {
        assert!(folding_ranges_default("", "tcl8.6").is_empty());
    }

    #[test]
    fn single_line_proc_has_no_fold() {
        let ranges = folding_ranges_default("proc foo {} { return 1 }\n", "tcl8.6");
        assert!(fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    // backslash line-continuation folding

    #[test]
    fn backslash_continuation_folds_first_to_last_line() {
        // `puts hello \` + `world` is one logical command spanning two
        // physical lines — fold line 0 → 1.
        let src = "puts hello \\\n     world\n";
        let ranges = folding_ranges_default(src, "tcl8.6");
        assert!(
            fold_lines(&ranges, FoldKind::Region).contains(&(0, 1)),
            "{ranges:?}",
        );
    }

    #[test]
    fn backslash_continuation_spans_three_lines() {
        let src = "mycmd a \\\nb \\\nc\n";
        let ranges = folding_ranges_default(src, "tcl8.6");
        assert!(
            fold_lines(&ranges, FoldKind::Region).contains(&(0, 2)),
            "{ranges:?}",
        );
    }

    #[test]
    fn no_continuation_fold_without_backslash() {
        // A plain two-line script has no continuation → no region fold.
        let src = "puts hello\nputs world\n";
        let ranges = folding_ranges_default(src, "tcl8.6");
        assert!(
            fold_lines(&ranges, FoldKind::Region).is_empty(),
            "{ranges:?}",
        );
    }

    #[test]
    fn two_separate_continued_commands_fold_independently() {
        let src = "aa x \\\ny\nbb p \\\nq\n";
        let ranges = folding_ranges_default(src, "tcl8.6");
        let region = fold_lines(&ranges, FoldKind::Region);
        assert!(region.contains(&(0, 1)), "{region:?}");
        assert!(region.contains(&(2, 3)), "{region:?}");
    }

    #[test]
    fn proc_body_folds_to_close_brace_minus_one() {
        let source = "proc greet {name} {\n    puts \"Hello\"\n    puts \"$name\"\n}\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        let regions = fold_lines(&ranges, FoldKind::Region);
        assert!(!regions.is_empty(), "expected a region fold for the proc");
        assert!(
            regions.iter().any(|&(s, e)| s == 0 && e >= 2),
            "expected fold starting at line 0 with end >= 2, got {regions:?}",
        );
    }

    #[test]
    fn namespace_body_emits_fold_at_line_zero() {
        let source = "namespace eval myns {\n    proc helper {} { return }\n}\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        let starts: HashSet<u32> = ranges
            .iter()
            .filter(|r| r.kind == FoldKind::Region)
            .map(|r| r.start_line)
            .collect();
        assert!(
            starts.contains(&0),
            "expected a region fold starting at line 0, got {starts:?}",
        );
    }

    #[test]
    fn comment_block_of_three_lines_folds() {
        let source = "# This is a comment block\n# that spans multiple lines\n# explaining something important\nproc foo {} { return }\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        let comments = fold_lines(&ranges, FoldKind::Comment);
        assert_eq!(comments, vec![(0, 2)]);
    }

    #[test]
    fn single_comment_line_does_not_fold() {
        let ranges = folding_ranges_default("# Just one comment\n", "tcl8.6");
        assert!(fold_lines(&ranges, FoldKind::Comment).is_empty());
    }

    #[test]
    fn if_body_emits_at_least_one_region_fold() {
        let source = "if {1} {\n    puts \"yes\"\n    puts \"really\"\n}\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        assert!(!fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    /// Issue #1216: `switch`'s braced form is a single clause-list argument,
    /// so the generic body walk saw one block and the individual arms could
    /// not be folded — and sticky scroll, which rides the folding provider
    /// for Tcl, pinned only the `switch` header from inside a long arm.
    #[test]
    fn switch_arms_each_fold() {
        let source = concat!(
            "switch -glob $uri {\n",       // 0
            "    \"/admin*\" {\n",         // 1
            "        respond 403\n",       // 2
            "        log denied\n",        // 3
            "    }\n",                     // 4
            "    default {\n",             // 5
            "        header insert X 1\n", // 6
            "        log allowed\n",       // 7
            "    }\n",                     // 8
            "}\n",                         // 9
        );
        let regions = fold_lines(&folding_ranges_default(source, "tcl8.6"), FoldKind::Region);
        assert!(
            regions.iter().any(|&(s, _)| s == 0),
            "the outer switch block must still fold: {regions:?}",
        );
        assert!(
            regions.contains(&(1, 3)),
            "the `/admin*` arm body must fold: {regions:?}",
        );
        assert!(
            regions.contains(&(5, 7)),
            "the `default` arm body must fold: {regions:?}",
        );
    }

    /// An arm body is a real script: blocks nested inside it fold too.  The
    /// old walk re-segmented the whole clause list as a script, read
    /// `pattern body` as one bogus command, and never descended at all.
    #[test]
    fn switch_arm_bodies_recurse_into_nested_blocks() {
        let source = concat!(
            "switch $x {\n",             // 0
            "    a {\n",                 // 1
            "        foreach y $ys {\n", // 2
            "            puts $y\n",     // 3
            "            incr n\n",      // 4
            "        }\n",               // 5
            "    }\n",                   // 6
            "}\n",                       // 7
        );
        let regions = fold_lines(&folding_ranges_default(source, "tcl8.6"), FoldKind::Region);
        assert!(
            regions.contains(&(2, 4)),
            "the `foreach` inside the arm must fold: {regions:?}",
        );
    }

    /// The inline `switch $x pat body pat body` form leaves each body as an
    /// ordinary script argument the generic role walk already reaches — the
    /// clause-list unpacking must not disturb it.
    #[test]
    fn switch_inline_pair_form_still_folds_each_body() {
        let source = concat!(
            "switch $x a {\n",  // 0
            "    puts one\n",   // 1
            "    puts two\n",   // 2
            "} b {\n",          // 3
            "    puts three\n", // 4
            "    puts four\n",  // 5
            "}\n",              // 6
        );
        let regions = fold_lines(&folding_ranges_default(source, "tcl8.6"), FoldKind::Region);
        assert!(regions.contains(&(0, 2)), "{regions:?}");
        assert!(regions.contains(&(3, 5)), "{regions:?}");
    }

    /// A `-` fall-through arm shares the *next* arm's body and has none of
    /// its own, so it contributes no fold; the arm it falls through to still
    /// does.
    #[test]
    fn switch_fallthrough_arm_contributes_no_fold() {
        let source = concat!(
            "switch $x {\n",    // 0
            "    a -\n",        // 1
            "    b {\n",        // 2
            "        puts 1\n", // 3
            "        puts 2\n", // 4
            "    }\n",          // 5
            "}\n",              // 6
        );
        let regions = fold_lines(&folding_ranges_default(source, "tcl8.6"), FoldKind::Region);
        assert!(regions.contains(&(2, 4)), "{regions:?}");
        assert!(
            !regions.iter().any(|&(s, _)| s == 1),
            "the `-` fall-through arm must not fold: {regions:?}",
        );
    }

    #[test]
    fn while_body_emits_at_least_one_region_fold() {
        let source = "while {1} {\n    puts \"loop\"\n    puts \"again\"\n}\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        assert!(!fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    /// Issue #954: `apply`'s lambda-literal argument is
    /// `ArgRole::LambdaLiteral`, not `Body` — re-segmenting the whole
    /// `{argList} {body}` blob as a script (the old generic-`Body` path)
    /// misread the parameter word as a command name, so a nested foldable
    /// region *inside* the real body (here, the `if`) was never found. A
    /// fold must cover the whole lambda literal (line 0) and the walk must
    /// also recurse far enough to find the `if` block's own nested fold.
    #[test]
    fn apply_lambda_body_folds_and_recurses_into_nested_blocks() {
        let source = "apply {dir {\n    if {1} {\n        puts \"yes\"\n        puts \"again\"\n    }\n}} /tmp\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        let regions = fold_lines(&ranges, FoldKind::Region);
        assert!(
            regions.iter().any(|&(s, _)| s == 0),
            "expected a region fold for the whole lambda literal at line 0, got {regions:?}"
        );
        assert!(
            regions.iter().any(|&(s, _)| s == 1),
            "expected the `if` block nested inside the real body to fold too \
             (proves recursion reached the actual body, not a params-as-head \
             misparse), got {regions:?}"
        );
    }

    #[test]
    fn deeply_nested_bodies_fold_past_the_old_depth_cap() {
        // Regression for the lifted `MAX_FOLD_DEPTH` (was 20): 40 nested `while`
        // bodies — each on its own line — must all yield a region fold, so the
        // innermost levels are no longer dropped by the descent guard.
        const LEVELS: usize = 40;
        let mut source = String::new();
        for i in 0..LEVELS {
            source.push_str(&"    ".repeat(i));
            source.push_str("while {1} {\n");
        }
        source.push_str(&"    ".repeat(LEVELS));
        source.push_str("puts deep\n");
        for i in (0..LEVELS).rev() {
            source.push_str(&"    ".repeat(i));
            source.push_str("}\n");
        }
        let ranges = folding_ranges_default(&source, "tcl8.6");
        let regions = fold_lines(&ranges, FoldKind::Region).len();
        assert!(
            regions >= LEVELS - 1,
            "expected ~{LEVELS} nested region folds, got {regions} (depth cap too low?)",
        );
    }

    #[test]
    fn if_else_bodies_are_disjoint() {
        // `} else {` must not put body1 and body2 on the same line.
        let source = concat!(
            "if {1} {\n",
            "    puts \"yes\"\n",
            "    puts \"really\"\n",
            "} else {\n",
            "    puts \"no\"\n",
            "    puts \"nope\"\n",
            "}\n",
        );
        let ranges = folding_ranges_default(source, "tcl8.6");
        let mut bodies: Vec<(u32, u32)> = ranges
            .iter()
            .filter(|r| r.kind == FoldKind::Region && (r.start_line == 0 || r.start_line == 3))
            .map(|r| (r.start_line, r.end_line))
            .collect();
        bodies.sort_unstable();
        assert_eq!(
            bodies.len(),
            2,
            "expected two sibling folds, got {bodies:?}"
        );
        let (first, second) = (bodies[0], bodies[1]);
        assert!(
            first.1 < second.0,
            "body1 {first:?} overlaps body2 {second:?}",
        );
    }

    #[test]
    fn incomplete_body_still_folds() {
        // An unterminated proc body should still be foldable so the
        // fold doesn't flicker off mid-edit.
        let source = "proc foo {} {\n    puts hi\n    puts there\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        let regions = fold_lines(&ranges, FoldKind::Region);
        assert!(
            !regions.is_empty(),
            "expected a fold range for an unterminated proc body",
        );
        assert!(
            regions.iter().any(|&(s, _)| s == 0),
            "expected a fold starting at line 0, got {regions:?}",
        );
    }

    #[test]
    fn elseif_chain_yields_four_disjoint_folds() {
        let source = concat!(
            "if {1} {\n",
            "    puts a\n",
            "} elseif {2} {\n",
            "    puts b\n",
            "} elseif {3} {\n",
            "    puts c\n",
            "} else {\n",
            "    puts d\n",
            "}\n",
        );
        let ranges = folding_ranges_default(source, "tcl8.6");
        let regions = fold_lines(&ranges, FoldKind::Region);
        assert_eq!(
            regions.len(),
            4,
            "expected four sibling folds, got {regions:?}",
        );
        for w in regions.windows(2) {
            assert!(
                w[0].1 < w[1].0,
                "elseif siblings {a:?} and {b:?} share a line",
                a = w[0],
                b = w[1],
            );
        }
    }

    #[test]
    fn nested_if_else_keeps_siblings_disjoint() {
        // Every pair of folds is either properly nested or disjoint.
        let source = concat!(
            "proc demo {x} {\n",
            "    if {$x} {\n",
            "        if {$x > 1} {\n",
            "            puts \"big\"\n",
            "            puts \"really big\"\n",
            "        } else {\n",
            "            puts \"small\"\n",
            "            puts \"really small\"\n",
            "        }\n",
            "    } else {\n",
            "        puts \"zero\"\n",
            "        puts \"none\"\n",
            "    }\n",
            "}\n",
        );
        let ranges = folding_ranges_default(source, "tcl8.6");
        for (i, a) in ranges.iter().enumerate() {
            for b in &ranges[i + 1..] {
                if a.start_line == b.start_line && a.end_line == b.end_line {
                    continue;
                }
                let contains = |outer: &FoldingRange, inner: &FoldingRange| {
                    outer.start_line <= inner.start_line && inner.end_line <= outer.end_line
                };
                if contains(a, b) || contains(b, a) {
                    continue;
                }
                let overlaps = a.end_line >= b.start_line && a.start_line <= b.end_line;
                assert!(!overlaps, "non-nested overlap between {a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn tcloo_method_body_inside_define_emits_a_fold() {
        // Regression: ``method``/``constructor``/``destructor`` inside
        // ``oo::define`` are context-sensitive and not in the registry,
        // so a special case handles them.
        let source = concat!(
            "oo::class create Foo {\n",
            "    method greet {name} {\n",
            "        puts \"hello $name\"\n",
            "        puts \"goodbye $name\"\n",
            "    }\n",
            "    constructor {} {\n",
            "        set count 0\n",
            "        set total 0\n",
            "    }\n",
            "}\n",
        );
        let ranges = folding_ranges_default(source, "tcl8.6");
        let regions = fold_lines(&ranges, FoldKind::Region);
        // Method body (``method greet … { … }``): start line 1, body
        // spans through line 3 — the closing ``}`` sits on line 4.
        assert!(
            regions.iter().any(|&(s, e)| s == 1 && e >= 3),
            "expected method-body fold starting at line 1, got {regions:?}",
        );
        // Constructor body: start line 5, body spans through line 7.
        assert!(
            regions.iter().any(|&(s, e)| s == 5 && e >= 7),
            "expected constructor-body fold starting at line 5, got {regions:?}",
        );
    }

    #[test]
    fn tcloo_method_visibility_folding_respects_the_selected_profile() {
        let source = concat!(
            "oo::class create Foo {\n",
            "    method hidden -private {} {\n",
            "        puts hidden\n",
            "        puts again\n",
            "    }\n",
            "}\n",
        );

        let tcl86 = fold_lines(&folding_ranges_default(source, "tcl8.6"), FoldKind::Region);
        let tcl90 = fold_lines(&folding_ranges_default(source, "tcl9.0"), FoldKind::Region);

        assert!(
            !tcl86.iter().any(|&(start, _)| start == 1),
            "Tcl 8.6 must not treat a Tcl 9 visibility flag as a method-body layout: {tcl86:?}",
        );
        assert!(
            tcl90.iter().any(|&(start, end)| start == 1 && end >= 3),
            "Tcl 9 should fold the visibility-qualified method body: {tcl90:?}",
        );
    }

    // The inner-OO body-index table itself is unit-tested in
    // [`crate::oo_body`]; here we cover the folding-level behaviour it
    // drives.

    #[test]
    fn tcloo_method_body_inside_configurable_emits_a_fold() {
        // Regression for #747: `oo::configurable` is a metaclass like
        // `oo::class`, so a method body inside its definition block must
        // fold — the registry-driven outer-command detection now covers it.
        let source = concat!(
            "oo::configurable create Widget {\n",
            "    property color\n",
            "    method paint {} {\n",
            "        set x 1\n",
            "        set y 2\n",
            "    }\n",
            "}\n",
        );
        let ranges = folding_ranges_default(source, "tcl9.0");
        let regions = fold_lines(&ranges, FoldKind::Region);
        // Method body (`method paint {} { … }`): starts on line 2, spans
        // through line 4 (the closing `}` sits on line 5).
        assert!(
            regions.iter().any(|&(s, e)| s == 2 && e >= 4),
            "expected method-body fold starting at line 2, got {regions:?}",
        );
    }

    /// Top-level OO body shapes are now driven by registry
    /// `arg_role_resolver` callbacks on `oo::class` / `oo::define` /
    /// `oo::objdefine` rather than by a hardcoded helper inside the
    /// folding provider. Pin the registry-side resolver so the
    /// folding walker keeps working off authoritative metadata.
    #[test]
    fn outer_oo_body_indices_resolve_via_registry() {
        let reg = registry();

        // `oo::class create Foo body` — body at index 2.
        assert_eq!(
            reg.arg_indices_for_role("oo::class", &["create", "Foo", "body"], ArgRole::Body,),
            vec![2],
        );
        assert_eq!(
            reg.arg_indices_for_role("oo::class", &["new", "body"], ArgRole::Body),
            vec![1],
        );
        assert_eq!(
            reg.arg_indices_for_role(
                "oo::class",
                &["createWithNamespace", "Foo", "::Foo::ns", "body"],
                ArgRole::Body,
            ),
            vec![3],
        );

        // `oo::define Target body` script form — body at 1 only when
        // arg 1 is not a recognised subcommand.
        assert_eq!(
            reg.arg_indices_for_role("oo::define", &["Foo", "body"], ArgRole::Body),
            vec![1],
        );
        // `oo::define Foo method ...` — body resolution falls
        // through to the subcommand shape.
        assert_eq!(
            reg.arg_indices_for_role(
                "oo::define",
                &["Foo", "method", "name", "{}", "body"],
                ArgRole::Body,
            ),
            vec![4],
        );
        assert_eq!(
            reg.arg_indices_for_role(
                "oo::define",
                &["Foo", "constructor", "{}", "body"],
                ArgRole::Body,
            ),
            vec![3],
        );
        assert_eq!(
            reg.arg_indices_for_role(
                "oo::define",
                &["Foo", "self", "method", "name", "{}", "body"],
                ArgRole::Body,
            ),
            vec![5],
        );

        // `oo::objdefine` shares the resolver with `oo::define`.
        assert_eq!(
            reg.arg_indices_for_role(
                "oo::objdefine",
                &["$obj", "method", "name", "{}", "body"],
                ArgRole::Body,
            ),
            vec![4],
        );
    }

    /// Context-sensitivity guard: a top-level user proc named
    /// `method` must NOT be treated as an OO method definition.
    /// The body walker only consults [`member_body_indices`] when an
    /// enclosing definition-body grammar is present — at top level it
    /// falls through to the registry, which has no `method` entry.
    #[test]
    fn top_level_method_is_not_an_oo_definition() {
        let source = concat!(
            "proc method {a b c} {\n",
            "    puts \"shadow attempt\"\n",
            "    puts \"second line\"\n",
            "}\n",
            // A bare invocation of the user proc — args don't
            // form a valid `method name args body` shape.
            "method 1 2 3\n",
        );
        let ranges = folding_ranges_default(source, "tcl8.6");
        let regions = fold_lines(&ranges, FoldKind::Region);
        // The proc body fold (line 0..2) is fine — that comes from
        // `proc`, not from misidentifying `method`. But there must
        // be no spurious fold attributable to `method 1 2 3`
        // (it has no braced body, so even if we erroneously
        // applied OO rules, no STR token would match — but the
        // assertion documents the intent).
        for (s, _e) in &regions {
            // `method 1 2 3` is on line 4; no fold should start
            // there.
            assert_ne!(*s, 4, "method 1 2 3 must not produce a fold");
        }
    }

    #[test]
    fn normalise_overlaps_shared_boundary_trims_earlier() {
        let input = vec![
            FoldingRange {
                start_line: 0,
                end_line: 5,
                kind: FoldKind::Region,
            },
            FoldingRange {
                start_line: 5,
                end_line: 10,
                kind: FoldKind::Region,
            },
        ];
        let normalised = normalise_overlaps(input);
        assert_eq!(normalised.len(), 2, "expected both ranges to survive");
        let mut sorted = normalised;
        sorted.sort_by_key(|r| r.start_line);
        let (a, b) = (sorted[0], sorted[1]);
        assert!(
            a.end_line < b.start_line,
            "{a:?} and {b:?} still overlap after normalisation",
        );
    }

    #[test]
    fn normalise_overlaps_dedups_after_trimming() {
        // Parent [0, 10] with child [3, 8] and a sibling [3, 12]
        // that trims to [3, 10]; another collector emitting [3, 10]
        // natively must not survive as a duplicate.
        let input = vec![
            FoldingRange {
                start_line: 0,
                end_line: 10,
                kind: FoldKind::Region,
            },
            FoldingRange {
                start_line: 3,
                end_line: 10,
                kind: FoldKind::Region,
            },
            FoldingRange {
                start_line: 3,
                end_line: 12,
                kind: FoldKind::Region,
            },
        ];
        let normalised = normalise_overlaps(input);
        let original_len = normalised.len();
        let unique: HashSet<(u32, u32, FoldKind)> = normalised
            .iter()
            .map(|r| (r.start_line, r.end_line, r.kind))
            .collect();
        assert_eq!(
            unique.len(),
            original_len,
            "duplicates remain after normalisation: {normalised:?}",
        );
    }

    #[test]
    fn normalise_overlaps_is_idempotent() {
        // Running normalisation twice should produce the same set.
        let input = vec![
            FoldingRange {
                start_line: 0,
                end_line: 5,
                kind: FoldKind::Region,
            },
            FoldingRange {
                start_line: 5,
                end_line: 10,
                kind: FoldKind::Region,
            },
            FoldingRange {
                start_line: 2,
                end_line: 4,
                kind: FoldKind::Comment,
            },
        ];
        let once = normalise_overlaps(input);
        let twice = normalise_overlaps(once.clone());
        assert_eq!(once, twice, "normalise_overlaps should be idempotent");
    }

    #[test]
    fn normalise_overlaps_empty_input() {
        // Pure smoke — empty input stays empty without panicking.
        assert!(normalise_overlaps(Vec::new()).is_empty());
    }

    #[test]
    fn proc_fold_end_leaves_outer_close_brace_visible() {
        let source = concat!(
            "proc demo {} {\n",
            "    if {1} {\n",
            "        puts \"yes\"\n",
            "    } else {\n",
            "        puts \"no\"\n",
            "    }\n",
            "}\n",
        );
        let ranges = folding_ranges_default(source, "tcl8.6");
        let proc_folds: Vec<&FoldingRange> = ranges
            .iter()
            .filter(|r| r.kind == FoldKind::Region && r.start_line == 0)
            .collect();
        assert!(!proc_folds.is_empty(), "expected a proc-level fold");
        let close_idx: u32 = source
            .lines()
            .enumerate()
            .filter_map(|(i, line)| (line == "}").then_some(u32::try_from(i).unwrap()))
            .max()
            .expect("source has a `}` line");
        for fold in proc_folds {
            assert!(
                fold.end_line < close_idx,
                "proc fold end {} hides closing brace on line {close_idx}",
                fold.end_line,
            );
        }
    }

    /// A `TclOO` class whose closing `}` is the file's own last line, with no
    /// trailing newline.  This is the tightest bound the provider can hit:
    /// there is no line after the closer to absorb an off-by-one, so an
    /// `end_line` at or past `line_count` would address a line the client
    /// does not have.  VS Code discards a whole sticky-scroll candidate
    /// *and its subtree* when the range is out of bounds, so an overflow
    /// here silently removes the outline from sticky scroll (issue #1122).
    ///
    /// The shape (a versioned `.tm` module holding one top-level
    /// `oo::class create` with a superclass, instance variables, a
    /// constructor, and methods, closing on the final line) mirrors the
    /// module in the issue #1122 report; the names here are invented.
    const CLASS_AT_EOF: &str = concat!(
        "oo::class create Widget {\n",
        "    superclass WidgetBase\n",
        "    variable label\n",
        "    constructor {text} {\n",
        "        set label $text\n",
        "    }\n",
        "    method render {} {\n",
        "        puts \"widget $label\"\n",
        "    }\n",
        "}",
    );

    /// Number of lines the LSP client sees for `source` (a trailing newline
    /// opens a final, empty line).
    fn lsp_line_count(source: &str) -> u32 {
        u32::try_from(tcl_lexer::LineIndex::new_lsp(source).line_count())
            .expect("line count fits u32")
    }

    /// `source` with every `\n` rewritten to `\r\n` — the reporter is on
    /// Windows, and a trailing `\r` on the closing-brace line is exactly the
    /// kind of off-by-one that would push a fold past the last line.
    fn crlf(source: &str) -> String {
        source.replace('\n', "\r\n")
    }

    fn assert_folds_in_bounds(source: &str, label: &str) {
        let line_count = lsp_line_count(source);
        let ranges = folding_ranges_default(source, "tcl8.6");
        assert!(!ranges.is_empty(), "{label}: expected at least one fold");
        for r in &ranges {
            assert!(
                r.end_line < line_count,
                "{label}: fold {}..{} ends past the last line (line_count {line_count})",
                r.start_line,
                r.end_line,
            );
            assert!(
                r.start_line < r.end_line,
                "{label}: degenerate fold {}..{}",
                r.start_line,
                r.end_line,
            );
        }
    }

    #[test]
    fn tcloo_class_closing_at_eof_without_trailing_newline_stays_in_bounds() {
        assert_folds_in_bounds(CLASS_AT_EOF, "class at EOF, no trailing newline");
    }

    #[test]
    fn tcloo_class_closing_at_eof_with_trailing_newline_stays_in_bounds() {
        assert_folds_in_bounds(
            &format!("{CLASS_AT_EOF}\n"),
            "class at EOF, trailing newline",
        );
    }

    #[test]
    fn tcloo_class_closing_at_eof_with_crlf_endings_stays_in_bounds() {
        assert_folds_in_bounds(&crlf(CLASS_AT_EOF), "CRLF class at EOF, no final newline");
        assert_folds_in_bounds(
            &crlf(&format!("{CLASS_AT_EOF}\n")),
            "CRLF class at EOF, final newline",
        );
    }

    /// The remaining shapes that end flush with EOF: an unterminated body
    /// (mid-edit), a trailing comment block, a trailing backslash
    /// continuation, and CRLF / lone-CR line endings — the last two because
    /// this module indexes lines on `\n` only while the client counts `\r`
    /// as a break too.
    #[test]
    fn no_fold_ends_past_the_last_line_for_eof_shapes() {
        let cases: [(&str, &str); 8] = [
            (
                "unterminated class",
                "oo::class create C {\n  method m {} {\n    puts hi\n",
            ),
            ("unterminated proc", "proc p {} {\n    set x 1\n"),
            ("trailing comment block", "# one\n# two\n# three"),
            ("trailing continuation", "set x $a \\\n    $b \\\n"),
            ("continuation to EOF", "foo a \\\nbar b"),
            (
                "crlf proc at EOF",
                "proc p {} {\r\n    set x 1\r\n    set y 2\r\n}",
            ),
            (
                "lone-cr proc at EOF",
                "proc p {} {\r    set x 1\r    set y 2\r}",
            ),
            (
                "nested namespace at EOF",
                "namespace eval n {\n    proc a {} {\n        set x 1\n    }\n}",
            ),
        ];
        for (label, source) in cases {
            let line_count = lsp_line_count(source);
            for r in folding_ranges_default(source, "tcl8.6") {
                assert!(
                    r.end_line < line_count,
                    "{label}: fold {}..{} ends past the last line (line_count {line_count})",
                    r.start_line,
                    r.end_line,
                );
            }
        }
    }

    /// The sticky-scroll contract for a realistic versioned-module class.
    ///
    /// VS Code (since 1.105) rejects a sticky candidate whose 1-based
    /// `foldEnd + 1` exceeds `lineCount` — in the 0-based terms this module
    /// uses, `end_line + 2 > line_count`.  A *top-level* range that ends on
    /// the document's last content line trips that unavoidably, and that is
    /// VS Code's own bug: contorting the fold output to dodge it would break
    /// the folding UI for every other client.  What we can guarantee, and
    /// what this test pins, is that we never make it worse — no range may
    /// exceed the last line, and the nested member folds (the ones sticky
    /// scroll actually shows while scrolling through a method) must clear
    /// the filter with room to spare.
    #[test]
    fn tcloo_class_fixture_meets_the_sticky_scroll_bounds_contract() {
        let source = format!("{CLASS_AT_EOF}\n");
        let line_count = lsp_line_count(&source);
        let ranges = folding_ranges_default(&source, "tcl8.6");
        assert!(!ranges.is_empty(), "expected folds for the class fixture");

        for r in &ranges {
            assert!(
                r.end_line < line_count,
                "fold {}..{} ends past the last line (line_count {line_count})",
                r.start_line,
                r.end_line,
            );
            assert!(r.start_line < r.end_line, "degenerate fold {r:?}");
        }

        // Every member fold (anything nested inside the class) survives
        // VS Code's `foldEnd + 1 > lineCount` rejection.
        let members: Vec<&FoldingRange> = ranges.iter().filter(|r| r.start_line > 0).collect();
        assert!(
            !members.is_empty(),
            "expected constructor/method folds inside the class",
        );
        for r in members {
            assert!(
                r.end_line + 2 <= line_count,
                "member fold {}..{} would be rejected by VS Code's sticky filter \
                 (line_count {line_count})",
                r.start_line,
                r.end_line,
            );
        }
    }

    /// Issue #1275 — folding must resolve a command head's *effective
    /// identity*, not its written spelling.
    ///
    /// `while`'s second word is `ArgRole::Body`; nothing else in the provider
    /// folds it (the analyser's scope walk only covers `proc` / `namespace
    /// eval` bodies), so a fold starting on the call's own line is a clean
    /// witness that the body role was resolved.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical): after `interp alias {}
    /// loop {} while` a `loop` call runs `while`; after `rename while loop` the
    /// same holds and `while` itself is gone (`info commands while` → empty);
    /// after `proc while …` the call runs the user proc.
    fn body_fold_on_call_line(source: &str) -> bool {
        folding_ranges_default(source, "tcl8.6")
            .iter()
            .any(|r| r.kind == FoldKind::Region && r.start_line == 1)
    }

    const REBOUND_CALL: &str = "loop {$x} {\n    puts a\n    puts b\n}\n";
    const WRITTEN_CALL: &str = "while {$x} {\n    puts a\n    puts b\n}\n";

    #[test]
    fn folding_folds_an_aliased_body_command() {
        assert!(
            body_fold_on_call_line(&format!(
                "interp alias {{}} loop {{}} while\n{REBOUND_CALL}"
            )),
            "an alias of `while` must contribute `while`'s body fold"
        );
        // The `::`-qualified spelling of the alias classifies alike.
        assert!(
            body_fold_on_call_line(
                "interp alias {} loop {} while\n::loop {$x} {\n    puts a\n    puts b\n}\n"
            ),
            "`::loop` must fold exactly like `loop`"
        );
        // Guard: the same call with no alias folds nothing — `loop` is an
        // ordinary unknown command with no body argument.
        assert!(
            !body_fold_on_call_line(&format!("set y 1\n{REBOUND_CALL}")),
            "an unbound `loop` has no body role"
        );
    }

    #[test]
    fn folding_folds_a_renamed_body_command_and_drops_the_old_name() {
        assert!(
            body_fold_on_call_line(&format!("rename while loop\n{REBOUND_CALL}")),
            "`rename while loop` must move `while`'s body grammar to `loop`"
        );
        // … and the old spelling is gone from the rename onwards, so it must
        // abstain rather than keep folding.
        assert!(
            !body_fold_on_call_line(&format!("rename while loop\n{WRITTEN_CALL}")),
            "a renamed-away `while` must not keep the built-in's body grammar"
        );
    }

    #[test]
    fn folding_abstains_for_a_builtin_shadowed_by_a_user_proc() {
        assert!(
            !body_fold_on_call_line(&format!(
                "proc while {{c b}} {{ return 1 }}\n{WRITTEN_CALL}"
            )),
            "a user `proc while` takes the name over; no registry body grammar applies"
        );
        // Guard: without the shadowing proc the very same call folds.
        assert!(
            body_fold_on_call_line(&format!("set y 1\n{WRITTEN_CALL}")),
            "the unshadowed built-in still folds"
        );
    }

    #[test]
    fn folding_abstains_for_a_dynamic_binding() {
        // `rename $old loop` proves nothing about either name.
        assert!(
            !body_fold_on_call_line(&format!("rename $old loop\n{REBOUND_CALL}")),
            "a dynamic rename must not give `loop` a body grammar"
        );
        assert!(
            body_fold_on_call_line(&format!("rename $old loop\n{WRITTEN_CALL}")),
            "a dynamic rename must not take `while`'s body grammar away either"
        );
    }
}
