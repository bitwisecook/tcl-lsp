//! Inlay-hints provider — Rust port of
//! `lsp/features/inlay_hints.py` (parameter-name hints).
//!
//! Surfaces `param_name:` hints at each positional argument
//! of every user-proc call site within the requested document
//! range.  The hints make it easy to see which argument goes
//! where without having to hover or look up the proc
//! signature.
//!
//! Example: for `proc greet {name greeting} { ... }` and a
//! call site `greet alice hello`, the provider emits:
//!
//! ```text
//! greet (name:)alice (greeting:)hello
//! ```
//!
//! Implementation: re-segments the source on each request so
//! per-argument token spans are available (the analyser
//! records the command-head span on `command_invocations` but
//! not per-arg spans).  Re-segmenting is cheap relative to
//! the LSP request rate; the keystone async-diagnostics
//! cached-analysis surface (`S-async-diagnostics`) will
//! eventually let this share a cached segmenter pass.
//!
//! What is *deferred*:
//!
//! * Built-in command hints — Python's provider also surfaces
//!   parameter names for selected built-in commands (`set`,
//!   `lassign`, etc.).  Needs the `argument_values` /
//!   `arg_role` machinery the Rust registry doesn't yet
//!   surface in the same shape; deferred until the registry's
//!   argument-name surface lands.
//! * Type / inferred-trait annotations on hints (Python's
//!   richer mode shows `name:string`, `count:int`).  Same
//!   gating as the `S-hover-rich` `_infer_var_type` follow-up.
//! * Method-call hints inside class bodies — needs the
//!   analyser's method-resolution machinery that
//!   `S-references-rich` will eventually land.

use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_lexer::LineIndex;

use crate::definition::LspRange;

/// One inlay-hint entry — position plus label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayHint {
    /// Anchor position for the hint (typically the start of
    /// an argument token).
    pub position_line: u32,
    /// Anchor character.
    pub position_character: u32,
    /// Hint label (e.g. `name:`).
    pub label: String,
}

/// Compute inlay hints for `range` in `source`.
///
/// `analysis` provides the proc-name → parameter-list lookup
/// the hints need.  When `analysis` is `None` (a stub-only
/// caller from the minimal port), returns an empty vector.
#[must_use]
pub fn inlay_hints(
    source: &str,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
) -> Vec<InlayHint> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let segments = tcl_compiler::segmenter::segment_commands(source);
    let mut out = Vec::new();

    for seg in &segments {
        if seg.texts.is_empty() || seg.argv.is_empty() {
            continue;
        }
        let cmd_name = &seg.texts[0];
        let Some(proc_def) = lookup_proc(analysis, cmd_name) else {
            continue;
        };
        emit_hints_for_call(seg, proc_def, &line_index, range, &mut out);
    }

    out
}

fn lookup_proc<'a>(analysis: &'a AnalysisResult, name: &str) -> Option<&'a ProcDef> {
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == name || qname == name || qname == &format!("::{name}") {
            return Some(proc_def);
        }
    }
    None
}

/// Walk a single segmented command, emit a hint per argument
/// that falls inside `range`.  Stops at the proc's parameter
/// count — extra arguments (e.g. an `args`-tail proc) don't
/// produce hints, mirroring Python's parameter-by-parameter
/// loop.
fn emit_hints_for_call(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    proc_def: &ProcDef,
    line_index: &LineIndex,
    range: LspRange,
    out: &mut Vec<InlayHint>,
) {
    // argv[0] is the command head; positional args start at
    // index 1.
    for (arg_idx, arg_tok) in seg.argv.iter().enumerate().skip(1) {
        let param_idx = arg_idx - 1;
        let Some(param) = proc_def.params.get(param_idx) else {
            // Past the declared parameter count — proc may
            // have an `args` tail, but we don't emit hints
            // for those (no individual name).
            break;
        };
        // `args` is the conventional tail-collector; skip it
        // even when present.
        if param.name == "args" {
            continue;
        }
        let pos = line_index.position_at(arg_tok.span.start());
        if !position_within_range(pos.line, pos.character, range) {
            continue;
        }
        out.push(InlayHint {
            position_line: pos.line,
            position_character: pos.character,
            label: format!("{}:", param.name),
        });
    }
}

fn position_within_range(line: u32, character: u32, range: LspRange) -> bool {
    if line < range.start_line {
        return false;
    }
    if line > range.end_line {
        return false;
    }
    if line == range.start_line && character < range.start_character {
        return false;
    }
    if line == range.end_line && character > range.end_character {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    fn whole_document_range(source: &str) -> LspRange {
        let line_count = source.lines().count().max(1);
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: u32::try_from(line_count - 1).unwrap_or(0),
            end_character: u32::MAX,
        }
    }

    #[test]
    fn empty_hints_when_analysis_is_none() {
        let hints = inlay_hints("set x 1\n", whole_document_range("set x 1\n"), None);
        assert!(hints.is_empty());
    }

    #[test]
    fn hints_emitted_for_user_proc_call() {
        let src = "proc greet {name greeting} {}\ngreet alice hello\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis));
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(
            labels.contains(&"name:"),
            "expected `name:` hint; got {labels:?}",
        );
        assert!(
            labels.contains(&"greeting:"),
            "expected `greeting:` hint; got {labels:?}",
        );
    }

    #[test]
    fn hints_anchored_at_argument_start() {
        let src = "proc greet {name} {}\ngreet alice\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis));
        assert_eq!(hints.len(), 1);
        let h = &hints[0];
        assert_eq!(h.position_line, 1);
        // `greet ` is 6 chars; `alice` starts at column 6.
        assert_eq!(h.position_character, 6);
        assert_eq!(h.label, "name:");
    }

    #[test]
    fn no_hints_for_unknown_command() {
        let src = "unknown_cmd a b c\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis));
        assert!(hints.is_empty(), "{hints:?}");
    }

    #[test]
    fn no_hints_for_args_tail_parameter() {
        // `args` is the variadic-tail collector — we skip it
        // because there's no individual name to surface.
        let src = "proc many {first args} {}\nmany 1 2 3 4\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis));
        // Only the `first` arg gets a hint.
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label, "first:");
    }

    #[test]
    fn extra_args_past_param_count_not_hinted() {
        // `proc one {a} {}` then `one 1 2 3` — only `a` has
        // a corresponding parameter; the extra args produce
        // no hints (no name to attach).
        let src = "proc one {a} {}\none 1 2 3\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis));
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label, "a:");
    }

    #[test]
    fn hints_filtered_by_range() {
        // Three lines, each with a proc call.  Range covers
        // only line 2.
        let src = "proc greet {name} {}\ngreet alice\ngreet bob\ngreet charlie\n";
        let analysis = analyse(src);
        let range = LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: u32::MAX,
        };
        let hints = inlay_hints(src, range, Some(&analysis));
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert_eq!(hints[0].position_line, 2);
    }
}
