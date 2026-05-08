//! Folding range provider — Rust port of `lsp/features/folding.py`.
//!
//! Emits LSP folding ranges for proc bodies, namespace bodies, comment
//! blocks, and control-structure bodies (`if`, `while`, `for`,
//! `foreach`, `switch`, …).  Mirrors the Python algorithm: a scope
//! walk over the analyser's [`AnalysisResult`], a comment-line
//! collector, and a registry-driven body-argument walker that
//! recurses through nested braced bodies, followed by the
//! `_normalise_overlaps` post-pass that trims partially overlapping
//! siblings so VS Code's folding tree-builder doesn't drop them.
//!
//! The result is a fully line-resolved `Vec<FoldingRange>`.  The
//! `PyO3` binding (`super::folding_binding`) emits these as plain
//! dicts; the Python dispatcher in `lsp/features/folding.py`
//! materialises [`lsprotocol.types.FoldingRange`] values and re-runs
//! its own `_normalise_overlaps` over them — running the
//! idempotent normalisation pass twice is harmless and keeps the
//! Python `_normalise_overlaps` test surface in
//! `tests/test_folding.py` working unchanged while the Rust LSP
//! server (`tcl-lsp-server`) gets the normalised output it needs
//! directly from this function.
//!
//! [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult

use std::collections::HashSet;

use tcl_compiler::analyser::{Analyser, Scope, ScopeKind};
use tcl_compiler::segmenter::segment_commands_with_offset;
use tcl_lexer::{LineIndex, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

/// LSP folding-range kind.  Mirrors `lsprotocol.types.FoldingRangeKind`.
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
/// Mirrors `lsp/features/folding.py::get_folding_ranges` with the
/// `analysis` argument present (the Python optimisation that skipped
/// scope folds when `analysis is None` is unnecessary in Rust — the
/// analyser itself is fast enough).
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
/// this output directly without a Python-side cleanup pass.  The
/// pass is idempotent, so the Python dispatcher's own
/// `_normalise_overlaps` over `PyO3`-binding output stays harmless
/// for the legacy path.
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
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    let mut ranges: Vec<FoldingRange> = Vec::new();

    collect_scope_folds(
        &analysis.global_scope,
        source,
        &line_index,
        &mut seen,
        &mut ranges,
    );
    collect_comment_folds(source, &mut seen, &mut ranges);
    let mut ctx = FoldCtx {
        registry,
        line_index: &line_index,
        original_source: source,
        seen: &mut seen,
        ranges: &mut ranges,
    };
    collect_body_folds(
        source, 0, 0, false, // top-level body is not inside an OO definition
        &mut ctx,
    );

    normalise_overlaps(ranges)
}

/// Return a fold end line that leaves the closing ``}`` visible.
///
/// Mirrors `_adjust_body_end_line` in `lsp/features/folding.py`:
/// when a braced body's last content byte sits on the same line as
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
    seen: &mut HashSet<(u32, u32)>,
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

/// Collect BODY indices for the inner `property` form's
/// `-set BODY` / `-get BODY` flag pairs. Always called with
/// `args` from an inner `property NAME ?-set BODY? ?-get BODY?`
/// invocation.
fn collect_property_body_indices(args: &[&str]) -> Vec<usize> {
    let n = args.len();
    if n == 0 {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .take(n.saturating_sub(1))
        .filter_map(|(i, &a)| ((a == "-set" || a == "-get") && i + 1 < n).then_some(i + 1))
        .collect()
}

/// Outer (context-free) OO definition commands — `oo::class` /
/// `oo::define` / `oo::objdefine`. The bodies of these commands
/// are OO definition bodies; recursing into one switches the
/// walker into "inside OO body" mode where the inner-OO commands
/// (`method`, `constructor`, …) become body-bearing.
fn is_outer_oo_definition_command(name: &str) -> bool {
    matches!(name, "oo::class" | "oo::define" | "oo::objdefine")
}

/// Inner OO definition-script commands. These are
/// context-sensitive: a top-level user proc named `method`
/// outside an OO block must not be folded as if it were an OO
/// `method` definition. The walker only consults
/// [`inner_oo_body_indices`] when `inside_oo_body == true`.
///
/// Recursing into one of these inner bodies takes us out of OO
/// definition context — methods / constructors / destructors hold
/// regular Tcl code, so the next recursion runs with
/// `inside_oo_body = false`.
fn is_inner_oo_definition_command(name: &str) -> bool {
    matches!(
        name,
        "method"
            | "classmethod"
            | "constructor"
            | "destructor"
            | "initialise"
            | "initialize"
            | "private"
            | "self"
            | "property"
    )
}

/// Return BODY argument indices for inner OO definition-script
/// commands. Only consulted by the walker when
/// `inside_oo_body == true` — i.e. we are recursing through the
/// body of an `oo::class create … { … }` / `oo::define … { … }`
/// block. Outside that context these commands are treated as
/// regular calls (a user proc named `method` shadows nothing).
//
// `match_same_arms`: keeping each command word on its own arm
// reads as a lookup table, which is the point of the function.
#[allow(clippy::match_same_arms)]
fn inner_oo_body_indices(command: &str, args: &[&str]) -> Vec<usize> {
    let n = args.len();
    match command {
        "constructor" if n >= 2 => vec![1],
        "destructor" if n >= 1 => vec![0],
        "method" | "classmethod" if n >= 3 => vec![n - 1],
        "initialise" | "initialize" if n >= 1 => vec![0],
        "private" if n >= 1 => vec![0],
        "self" if n >= 1 => match args[0] {
            "constructor" if n >= 3 => vec![2],
            "destructor" if n >= 2 => vec![1],
            "method" | "classmethod" if n >= 4 => vec![n - 1],
            _ => Vec::new(),
        },
        "property" => collect_property_body_indices(args),
        _ => Vec::new(),
    }
}

fn emit_body_span_fold(
    span: tcl_lexer::Span,
    source: &str,
    line_index: &LineIndex,
    seen: &mut HashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    if span.is_empty() {
        return;
    }
    let start_line = line_index.position_at(span.start()).line;
    let end_offset = span.end().saturating_sub(1) as usize;
    let raw_end_line = line_index.position_at(span.end().saturating_sub(1)).line;
    if start_line < raw_end_line {
        let end_line = adjust_body_end_line(source, end_offset, raw_end_line);
        push_unique(seen, ranges, start_line, end_line, FoldKind::Region);
    }
}

/// Walk the analyser scope tree and emit folds for proc bodies and
/// namespace bodies.  Mirrors `_collect_scope_folds`.
fn collect_scope_folds(
    scope: &Scope,
    source: &str,
    line_index: &LineIndex,
    seen: &mut HashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    for proc_def in scope.procs.values() {
        emit_body_span_fold(proc_def.body_span, source, line_index, seen, ranges);
    }
    for child in &scope.children {
        if matches!(child.kind, ScopeKind::Namespace) {
            if let Some(span) = child.body_span {
                emit_body_span_fold(span, source, line_index, seen, ranges);
            }
        }
        collect_scope_folds(child, source, line_index, seen, ranges);
    }
}

/// Walk lines and emit folds for runs of consecutive ``#`` comment
/// lines.  Mirrors `_collect_comment_folds`: an internal block (one
/// followed by a non-comment line) needs at least three lines; a
/// trailing block at end-of-file needs at least two.
fn collect_comment_folds(
    source: &str,
    seen: &mut HashSet<(u32, u32)>,
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
            if let Some(start) = block_start {
                if i - start >= 2 {
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

/// Recursively segment commands and emit folds for every multi-line
/// `BODY`-roled argument.  Mirrors `_collect_body_folds`.
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
    line_index: &'a LineIndex,
    original_source: &'a str,
    seen: &'a mut HashSet<(u32, u32)>,
    ranges: &'a mut Vec<FoldingRange>,
}

/// Recursion depth is capped at 20 to mirror the Python guard.
fn collect_body_folds(
    body_source: &str,
    base_offset: u32,
    depth: u32,
    inside_oo_body: bool,
    ctx: &mut FoldCtx<'_>,
) {
    if depth > 20 {
        return;
    }
    let commands = segment_commands_with_offset(body_source, base_offset);
    for cmd in &commands {
        if cmd.argv.is_empty() {
            continue;
        }
        let args_borrow: Vec<&str> = cmd.args().iter().map(String::as_str).collect();

        // Outer OO definition commands (`oo::class`, `oo::define`,
        // `oo::objdefine`) carry their body-arg shapes in the
        // registry now — `arg_indices_for_role` returns the right
        // index. Inner OO commands (`method`, `constructor`,
        // `destructor`, `self`, `property`, …) are
        // context-sensitive and only count when we are walking
        // through an outer OO body; their indices come from
        // [`inner_oo_body_indices`].
        let body_indices: Vec<usize> =
            if inside_oo_body && is_inner_oo_definition_command(cmd.name()) {
                inner_oo_body_indices(cmd.name(), &args_borrow)
            } else {
                ctx.registry
                    .arg_indices_for_role(cmd.name(), &args_borrow, ArgRole::Body)
            };

        // What "inside_oo_body" should the recursion into THIS
        // command's bodies see?
        //
        //  - Outer OO commands (`oo::class create Foo {...}`):
        //    the body IS the OO definition body → set true.
        //  - Inner OO commands inside an OO body
        //    (`method foo {} {...}`): the body is plain Tcl code
        //    → set false.
        //  - Anything else (`if`, `while`, `for`, …): inherit the
        //    current flag — control-flow nesting inside an OO
        //    body keeps us in OO context.
        let next_inside_oo_body = if is_outer_oo_definition_command(cmd.name()) {
            true
        } else if inside_oo_body && is_inner_oo_definition_command(cmd.name()) {
            false
        } else {
            inside_oo_body
        };

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
                next_inside_oo_body,
                ctx,
            );
        }
    }
}

/// Trim partially overlapping sibling folds so the returned ranges
/// are pairwise disjoint or properly nested.
///
/// Mirrors `_normalise_overlaps` in `lsp/features/folding.py`. VS
/// Code's folding tree-builder silently drops or misplaces ranges
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
    let mut seen: HashSet<(u32, u32, FoldKind)> = HashSet::new();
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
        // Mirrors `test_single_line_no_fold` in tests/test_folding.py.
        let ranges = folding_ranges_default("proc foo {} { return 1 }\n", "tcl8.6");
        assert!(fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    #[test]
    fn proc_body_folds_to_close_brace_minus_one() {
        // Mirrors `test_proc_body`.
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
        // Mirrors `test_namespace_body`.
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
        // Mirrors `test_comment_block`.
        let source = "# This is a comment block\n# that spans multiple lines\n# explaining something important\nproc foo {} { return }\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        let comments = fold_lines(&ranges, FoldKind::Comment);
        assert_eq!(comments, vec![(0, 2)]);
    }

    #[test]
    fn single_comment_line_does_not_fold() {
        // Mirrors `test_single_comment_no_fold`.
        let ranges = folding_ranges_default("# Just one comment\n", "tcl8.6");
        assert!(fold_lines(&ranges, FoldKind::Comment).is_empty());
    }

    #[test]
    fn if_body_emits_at_least_one_region_fold() {
        // Mirrors `test_if_body`.
        let source = "if {1} {\n    puts \"yes\"\n    puts \"really\"\n}\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        assert!(!fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    #[test]
    fn while_body_emits_at_least_one_region_fold() {
        // Mirrors `test_while_body`.
        let source = "while {1} {\n    puts \"loop\"\n    puts \"again\"\n}\n";
        let ranges = folding_ranges_default(source, "tcl8.6");
        assert!(!fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    #[test]
    fn if_else_bodies_are_disjoint() {
        // Mirrors `test_if_else_bodies_are_disjoint` — `} else {` must
        // not put body1 and body2 on the same line.
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
        // Mirrors `test_incomplete_body_still_folds`: an unterminated
        // proc body should still be foldable so the fold doesn't
        // flicker off mid-edit.
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
        // Mirrors `test_elseif_chain_disjoint`.
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
        // Mirrors `test_nested_if_else_no_overlapping_siblings`: every
        // pair of folds is either properly nested or disjoint.
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
        // ``oo::define`` are context-sensitive and not in the registry.
        // Python's ``_oo_definition_body_indices`` carried that special
        // case; the Rust port must mirror it.
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
    fn inner_oo_body_indices_table() {
        // Inner OO definition-script commands (only consulted when
        // `inside_oo_body == true`).
        assert_eq!(
            inner_oo_body_indices("constructor", &["{}", "body"]),
            vec![1]
        );
        assert_eq!(inner_oo_body_indices("destructor", &["body"]), vec![0]);
        assert_eq!(
            inner_oo_body_indices("method", &["name", "{}", "body"]),
            vec![2],
        );
        assert_eq!(
            inner_oo_body_indices("classmethod", &["name", "{args}", "body"]),
            vec![2],
        );
        assert_eq!(
            inner_oo_body_indices("self", &["constructor", "{}", "body"]),
            vec![2],
        );
        assert_eq!(
            inner_oo_body_indices("self", &["destructor", "body"]),
            vec![1],
        );
        assert_eq!(
            inner_oo_body_indices("self", &["method", "name", "{}", "body"]),
            vec![3],
        );
        assert_eq!(
            inner_oo_body_indices("property", &["name", "-set", "setter", "-get", "getter"]),
            vec![2, 4],
        );
        // Non-OO command: empty (the walker never even consults this
        // function for non-inner-OO commands, but the helper itself
        // is still defensive).
        assert!(inner_oo_body_indices("set", &["x", "1"]).is_empty());
        // Too few args: empty.
        assert!(inner_oo_body_indices("method", &["name"]).is_empty());
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
    /// The body walker only consults [`inner_oo_body_indices`] when
    /// `inside_oo_body == true` — at top level it falls through to
    /// the registry, which has no `method` entry. Regression for
    /// the architecture review on PR #231.
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
        // Mirrors `test_normalise_overlaps_shared_boundary_trims_earlier`
        // in tests/test_folding.py.
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
        // Mirrors `test_normalise_overlaps_dedups_after_trimming`:
        // parent [0, 10] with child [3, 8] and a sibling [3, 12]
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
        // Running normalisation twice should produce the same set —
        // important because the Python dispatcher in
        // `lsp/features/folding.py` re-runs `_normalise_overlaps`
        // over the binding output, and the Rust LSP server bypasses
        // that path entirely.
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
        // Mirrors `test_if_else_proc_body_close_brace_visible`.
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
}
