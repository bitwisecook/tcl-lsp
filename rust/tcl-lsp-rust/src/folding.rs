//! Folding range provider — Rust port of `lsp/features/folding.py`.
//!
//! Emits LSP folding ranges for proc bodies, namespace bodies, comment
//! blocks, and control-structure bodies (`if`, `while`, `for`,
//! `foreach`, `switch`, …).  Mirrors the Python algorithm: a scope
//! walk over the analyser's [`AnalysisResult`], a comment-line
//! collector, and a registry-driven body-argument walker that
//! recurses through nested braced bodies.
//!
//! The result is a fully line-resolved `Vec<FoldingRange>`.  The
//! `PyO3` binding (in `lib.rs`) emits these as plain dicts; the
//! Python dispatcher in `lsp/features/folding.py` materialises
//! [`lsprotocol.types.FoldingRange`] values and runs
//! `_normalise_overlaps` on them — keeping the overlap-normalisation
//! algorithm in Python preserves the
//! `_normalise_overlaps` test surface in `tests/test_folding.py`.
//!
//! [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult

use std::collections::HashSet;

use tcl_compiler::analyser::{Analyser, Scope, ScopeKind};
use tcl_compiler::segmenter::segment_commands_with_offset;
use tcl_lexer::{LineIndex, TokenType};
use tcl_registry::dialects::DialectSet;
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
/// Overlap normalisation is left to the Python dispatcher so the
/// `_normalise_overlaps` test surface in `tests/test_folding.py`
/// keeps working unchanged; the cargo tests in this module validate
/// the collector outputs directly.
#[must_use]
pub fn folding_ranges(source: &str, dialect: &str) -> Vec<FoldingRange> {
    if source.is_empty() {
        return Vec::new();
    }

    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, dialect);

    let mut registry = CommandRegistry::build_default();
    if let Some(d) = DialectSet::parse(dialect) {
        registry.load_dialect(d);
    }

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
    collect_body_folds(
        source,
        &registry,
        &line_index,
        source,
        0,
        0,
        &mut seen,
        &mut ranges,
    );

    ranges
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

/// Collect BODY indices for ``property`` / ``oo::define …
/// property`` flag pairs (``-set BODY`` / ``-get BODY``).
/// `start` is the index of the first option flag — `0` for the inner
/// ``property`` command, `2` for the ``oo::define Target property``
/// shape.
fn collect_property_body_indices(args: &[&str], start: usize) -> Vec<usize> {
    let n = args.len();
    if n == 0 {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .skip(start)
        .take(n.saturating_sub(start + 1))
        .filter_map(|(i, &a)| ((a == "-set" || a == "-get") && i + 1 < n).then_some(i + 1))
        .collect()
}

/// Subcommands recognised by ``oo::define`` / ``oo::objdefine``.
/// Used to disambiguate the script-form (``oo::define Target {body}``)
/// from a subcommand call where ``args[1]`` is one of these words.
const OO_DEFINE_SUBCOMMANDS: &[&str] = &[
    "constructor",
    "destructor",
    "method",
    "classmethod",
    "initialise",
    "initialize",
    "private",
    "self",
    "property",
    "filter",
    "export",
    "unexport",
    "deletemethod",
    "renamemethod",
    "forward",
    "mixin",
    "superclass",
    "variable",
];

/// Return BODY argument indices for `TclOO` commands.
///
/// Mirrors three Python helpers that the Rust command registry
/// hasn't yet absorbed: ``_oo_definition_body_indices``
/// (inner-script commands — ``method``, ``constructor``, ``destructor``,
/// ``self constructor``, ``property -set/-get``, …) in
/// ``core/commands/registry/runtime.py``, plus the
/// ``arg_role_resolver`` callbacks on ``oo::class`` (the
/// ``create``/``new``/``createWithNamespace`` metaclass shapes) and
/// ``oo::define`` / ``oo::objdefine`` (both the script form and the
/// subcommand-driven shape) in
/// ``core/commands/registry/tcl/oo_class.py`` and
/// ``oo_define.py``.  These commands are context-sensitive — a user
/// proc named ``method`` outside an OO block must not be
/// misidentified — so they aren't regular registry entries; the
/// body walker checks them as a priority before falling back to the
/// registry.
//
// `match_same_arms`: keeping each command word on its own arm
// mirrors the Python helpers' shape and reads as a lookup table,
// which is the point of the function — collapsing the
// ``-> vec![0]`` arms together loses that reading.
#[allow(clippy::match_same_arms)]
fn oo_definition_body_indices(command: &str, args: &[&str]) -> Vec<usize> {
    let n = args.len();
    match command {
        // Inner OO definition-script commands.
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
        "property" => collect_property_body_indices(args, 0),
        // Outer OO metaclass commands — ``oo::class create Foo
        // {body}`` etc.  Mirrors ``_oo_metaclass_arg_roles``.
        "oo::class" if n >= 2 => match args[0] {
            "create" if n >= 3 => vec![2],
            "new" if n >= 2 => vec![1],
            "createWithNamespace" if n >= 4 => vec![3],
            _ => Vec::new(),
        },
        // ``oo::define Target {body}`` script form, plus the
        // subcommand-driven shape.  Mirrors ``_oo_define_arg_roles``.
        "oo::define" | "oo::objdefine" => {
            if n == 2 && !OO_DEFINE_SUBCOMMANDS.contains(&args[1]) {
                return vec![1];
            }
            if n < 2 {
                return Vec::new();
            }
            match args[1] {
                "constructor" if n >= 4 => vec![3],
                "destructor" if n >= 3 => vec![2],
                "method" | "classmethod" if n >= 5 => vec![n - 1],
                "initialise" | "initialize" if n >= 3 => vec![2],
                "private" if n >= 3 => vec![2],
                "self" if n >= 3 => match args[2] {
                    "constructor" if n >= 5 => vec![4],
                    "destructor" if n >= 4 => vec![3],
                    "method" | "classmethod" if n >= 6 => vec![n - 1],
                    _ => Vec::new(),
                },
                "property" => collect_property_body_indices(args, 2),
                _ => Vec::new(),
            }
        }
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
/// Recursion depth is capped at 20 to mirror the Python guard.
#[allow(clippy::too_many_arguments)]
fn collect_body_folds(
    body_source: &str,
    registry: &CommandRegistry,
    line_index: &LineIndex,
    original_source: &str,
    base_offset: u32,
    depth: u32,
    seen: &mut HashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
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
        // TclOO definition-script commands (`method`, `constructor`,
        // `destructor`, `self constructor`, `property -set/-get`, …)
        // are context-sensitive — a user proc named ``method`` outside
        // an OO block must not be misidentified — so they aren't
        // regular registry entries.  Mirror Python's
        // ``_oo_definition_body_indices`` priority in
        // ``core/commands/registry/runtime.py`` so braced bodies in
        // ``oo::define`` blocks still fold.
        let oo_body = oo_definition_body_indices(cmd.name(), &args_borrow);
        let body_indices: Vec<usize> = if oo_body.is_empty() {
            registry.arg_indices_for_role(cmd.name(), &args_borrow, ArgRole::Body)
        } else {
            oo_body
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
            emit_body_span_fold(body_tok.span, original_source, line_index, seen, ranges);

            // Recurse into the body's content. The lexer's STR span
            // includes the opening ``{``; for closed non-empty bodies
            // it ends just before the closing ``}``, for closed empty
            // bodies it includes the ``}`` (degenerate clamp), and for
            // unclosed bodies it runs to EOF.  Slice the absolute
            // source so recursive spans stay in original-source space.
            let span = body_tok.span;
            let content_start = span.start() as usize + body_tok.content_offset as usize;
            let raw_end = span.end() as usize;
            let bytes = original_source.as_bytes();
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
            let inner = &original_source[content_start..content_end];
            collect_body_folds(
                inner,
                registry,
                line_index,
                original_source,
                u32::try_from(content_start).expect("content offset fits u32"),
                depth + 1,
                seen,
                ranges,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(folding_ranges("", "tcl8.6").is_empty());
    }

    #[test]
    fn single_line_proc_has_no_fold() {
        // Mirrors `test_single_line_no_fold` in tests/test_folding.py.
        let ranges = folding_ranges("proc foo {} { return 1 }\n", "tcl8.6");
        assert!(fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    #[test]
    fn proc_body_folds_to_close_brace_minus_one() {
        // Mirrors `test_proc_body`.
        let source = "proc greet {name} {\n    puts \"Hello\"\n    puts \"$name\"\n}\n";
        let ranges = folding_ranges(source, "tcl8.6");
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
        let ranges = folding_ranges(source, "tcl8.6");
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
        let ranges = folding_ranges(source, "tcl8.6");
        let comments = fold_lines(&ranges, FoldKind::Comment);
        assert_eq!(comments, vec![(0, 2)]);
    }

    #[test]
    fn single_comment_line_does_not_fold() {
        // Mirrors `test_single_comment_no_fold`.
        let ranges = folding_ranges("# Just one comment\n", "tcl8.6");
        assert!(fold_lines(&ranges, FoldKind::Comment).is_empty());
    }

    #[test]
    fn if_body_emits_at_least_one_region_fold() {
        // Mirrors `test_if_body`.
        let source = "if {1} {\n    puts \"yes\"\n    puts \"really\"\n}\n";
        let ranges = folding_ranges(source, "tcl8.6");
        assert!(!fold_lines(&ranges, FoldKind::Region).is_empty());
    }

    #[test]
    fn while_body_emits_at_least_one_region_fold() {
        // Mirrors `test_while_body`.
        let source = "while {1} {\n    puts \"loop\"\n    puts \"again\"\n}\n";
        let ranges = folding_ranges(source, "tcl8.6");
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
        let ranges = folding_ranges(source, "tcl8.6");
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
        let ranges = folding_ranges(source, "tcl8.6");
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
        let ranges = folding_ranges(source, "tcl8.6");
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
        let ranges = folding_ranges(source, "tcl8.6");
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
                assert!(!overlaps, "non-nested overlap between {a:?} and {b:?}",);
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
        let ranges = folding_ranges(source, "tcl8.6");
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
    fn oo_definition_body_indices_table() {
        assert_eq!(
            oo_definition_body_indices("constructor", &["{}", "body"]),
            vec![1]
        );
        assert_eq!(oo_definition_body_indices("destructor", &["body"]), vec![0]);
        assert_eq!(
            oo_definition_body_indices("method", &["name", "{}", "body"]),
            vec![2],
        );
        assert_eq!(
            oo_definition_body_indices("classmethod", &["name", "{args}", "body"]),
            vec![2],
        );
        assert_eq!(
            oo_definition_body_indices("self", &["constructor", "{}", "body"]),
            vec![2],
        );
        assert_eq!(
            oo_definition_body_indices("self", &["destructor", "body"]),
            vec![1],
        );
        assert_eq!(
            oo_definition_body_indices("self", &["method", "name", "{}", "body"]),
            vec![3],
        );
        assert_eq!(
            oo_definition_body_indices("property", &["name", "-set", "setter", "-get", "getter"]),
            vec![2, 4],
        );
        // Non-OO command: empty.
        assert!(oo_definition_body_indices("set", &["x", "1"]).is_empty());
        // Too few args: empty.
        assert!(oo_definition_body_indices("method", &["name"]).is_empty());
        // ``oo::class create Foo {body}`` — body at index 2.
        assert_eq!(
            oo_definition_body_indices("oo::class", &["create", "Foo", "body"]),
            vec![2],
        );
        assert_eq!(
            oo_definition_body_indices("oo::class", &["new", "body"]),
            vec![1],
        );
        assert_eq!(
            oo_definition_body_indices(
                "oo::class",
                &["createWithNamespace", "Foo", "::Foo::ns", "body"],
            ),
            vec![3],
        );
        // ``oo::define Target {body}`` script form — body at 1 only
        // when arg 1 is not a recognised subcommand.
        assert_eq!(
            oo_definition_body_indices("oo::define", &["Foo", "body"]),
            vec![1],
        );
        // ``oo::define Foo method ...`` — body resolution falls
        // through to the subcommand shape.
        assert_eq!(
            oo_definition_body_indices("oo::define", &["Foo", "method", "name", "{}", "body"]),
            vec![4],
        );
        assert_eq!(
            oo_definition_body_indices("oo::define", &["Foo", "constructor", "{}", "body"]),
            vec![3],
        );
        assert_eq!(
            oo_definition_body_indices(
                "oo::define",
                &["Foo", "self", "method", "name", "{}", "body"],
            ),
            vec![5],
        );
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
        let ranges = folding_ranges(source, "tcl8.6");
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
