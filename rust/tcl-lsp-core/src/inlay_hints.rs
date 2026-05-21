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
//! Built-in command hints also land: when a registry is
//! provided and the call's head matches a built-in command (or
//! `cmd subcommand`), the provider parses the spec's synopsis
//! for positional parameter names and labels the matching
//! call-site args.  Synopsis flags (`?-nocase?`,
//! `?-length length?`) are skipped on both sides — synopsis
//! parsing drops flag tokens, and call-site args that look
//! like flags (start with `-`) don't consume a positional
//! slot.  Varargs (`?name ...?`) stop the parse.
//!
//! What is *deferred*:
//!
//! * Type / inferred-trait annotations on hints (Python's
//!   richer mode shows `name:string`, `count:int`).  Same
//!   gating as the `S-hover-rich` `_infer_var_type` follow-up.
//! * Method-call hints inside class bodies — needs the
//!   analyser's method-resolution machinery that
//!   `S-references-rich` will eventually land.

use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

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
/// `registry`, when `Some`, additionally surfaces parameter-
/// name hints for built-in commands via their synopsis.
#[must_use]
pub fn inlay_hints(
    source: &str,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
    registry: Option<&CommandRegistry>,
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
        if let Some(proc_def) = lookup_proc(analysis, cmd_name) {
            emit_hints_for_call(seg, proc_def, &line_index, range, &mut out);
            continue;
        }
        // Built-in command — parse the registry synopsis for
        // positional parameter names.  User procs take
        // precedence (handled above).
        if let Some(registry) = registry {
            if let Some(spec) = registry.get(cmd_name) {
                emit_builtin_hints(seg, spec, &line_index, range, &mut out);
            }
        }
    }

    out
}

/// Emit parameter-name hints for a built-in command call by
/// parsing the spec's synopsis.  Handles the `cmd subcommand`
/// shape: when the spec declares subcommands and the call's
/// first arg names one, the subcommand's synopsis drives the
/// hints (and the subcommand keyword itself isn't labelled).
fn emit_builtin_hints(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    spec: &tcl_registry::CommandSpec,
    line_index: &LineIndex,
    range: LspRange,
    out: &mut Vec<InlayHint>,
) {
    // Resolve synopsis, the call-arg index at which positional
    // arguments begin, and the option set the registry declares
    // for this command / subcommand.  argv[0] is the command head.
    let (synopsis, skip_words, first_arg_idx, options): (
        &str,
        usize,
        usize,
        &[tcl_registry::prelude::OptionSpec],
    ) = if spec.subcommands.is_empty() {
        let Some(hover) = spec.hover.as_ref() else {
            return;
        };
        let Some(line) = hover.synopsis.first() else {
            return;
        };
        // Synopsis like `set varName ?value?` — skip the
        // command word (1); positional call args start at
        // argv[1].
        (*line, 1, 1, spec.options)
    } else {
        // Subcommand shape: argv[1] should name a subcommand.
        let Some(sub_name) = seg.texts.get(1) else {
            return;
        };
        let Some(sub) = spec.subcommands.iter().find(|s| s.name == sub_name) else {
            return;
        };
        // Synopsis like `string length string` — skip the
        // command + subcommand words (2); positional call args
        // start at argv[2].
        (sub.synopsis, 2, 2, sub.options)
    };

    let param_names = param_names_from_synopsis(synopsis, skip_words);
    if param_names.is_empty() {
        return;
    }

    // Walk call args, assigning each positional the next param name.
    // Whether a `-`-prefixed token is an option is decided by the
    // registry, not its spelling: only tokens matching a declared
    // `OptionSpec` are skipped (and their value too, when the option
    // `takes_value`).  This keeps real positionals like the `-1` in
    // `string index $s -1` — which is no command's option — labelled
    // correctly.  `argv` and `texts` are parallel, indexed by `arg_idx`.
    let mut slot = 0;
    let mut arg_idx = first_arg_idx;
    let mut options_ended = false;
    while arg_idx < seg.argv.len() && slot < param_names.len() {
        let arg_text = seg.texts.get(arg_idx).map_or("", String::as_str);
        if !options_ended && arg_text.starts_with('-') && arg_text != "-" {
            // `--` ends option parsing; everything after is positional.
            if arg_text == "--" {
                options_ended = true;
                arg_idx += 1;
                continue;
            }
            if let Some(opt) = options.iter().find(|o| o.name == arg_text) {
                arg_idx += 1;
                if opt.takes_value && arg_idx < seg.argv.len() {
                    arg_idx += 1;
                }
                continue;
            }
        }
        let arg_tok = &seg.argv[arg_idx];
        arg_idx += 1;
        let pos = line_index.position_at(arg_tok.span.start());
        slot += 1;
        if !position_within_range(pos.line, pos.character, range) {
            continue;
        }
        out.push(InlayHint {
            position_line: pos.line,
            position_character: pos.character,
            label: format!("{}:", param_names[slot - 1]),
        });
    }
}

/// Parse positional parameter names out of a command synopsis,
/// dropping the leading `skip_words` command/subcommand tokens.
///
/// Token grammar (best-effort):
/// * `name` — required positional → emitted.
/// * `?name?` — optional positional → emitted (stripped).
/// * `?-flag?` / `-flag` / `?-flag value?` — flag → skipped.
/// * `?name ...?` / `...` — varargs → stops the parse.
fn param_names_from_synopsis(synopsis: &str, skip_words: usize) -> Vec<String> {
    let groups = synopsis_groups(synopsis);
    let mut names = Vec::new();
    for group in groups.into_iter().skip(skip_words) {
        // Varargs anywhere → stop.
        if group.contains("...") {
            break;
        }
        // Optional group `?...?`.
        if let Some(inner) = group.strip_prefix('?').and_then(|g| g.strip_suffix('?')) {
            let inner = inner.trim();
            if inner.starts_with('-') {
                // Optional flag (possibly `-flag value`) — skip.
                continue;
            }
            if inner.is_empty() || inner.contains(char::is_whitespace) {
                // Multi-word optional that isn't a plain name —
                // skip conservatively.
                continue;
            }
            names.push(inner.to_string());
            continue;
        }
        // Bare flag.
        if group.starts_with('-') {
            continue;
        }
        // Plain required positional.
        if !group.is_empty() {
            names.push(group);
        }
    }
    names
}

/// Split a synopsis into whitespace tokens, re-joining
/// `?...?` optional groups that span multiple tokens (e.g.
/// `?-length length?` → one group).
fn synopsis_groups(synopsis: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut current: Option<String> = None;
    for tok in synopsis.split_whitespace() {
        match &mut current {
            Some(buf) => {
                buf.push(' ');
                buf.push_str(tok);
                if tok.ends_with('?') {
                    groups.push(current.take().unwrap());
                }
            }
            None => {
                if tok.starts_with('?') && !tok.ends_with('?') {
                    current = Some(tok.to_string());
                } else {
                    groups.push(tok.to_string());
                }
            }
        }
    }
    // Unterminated optional group — keep what we have.
    if let Some(buf) = current {
        groups.push(buf);
    }
    groups
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
        let hints = inlay_hints("set x 1\n", whole_document_range("set x 1\n"), None, None);
        assert!(hints.is_empty());
    }

    #[test]
    fn hints_emitted_for_user_proc_call() {
        let src = "proc greet {name greeting} {}\ngreet alice hello\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), None);
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
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), None);
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
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), None);
        assert!(hints.is_empty(), "{hints:?}");
    }

    #[test]
    fn no_hints_for_args_tail_parameter() {
        // `args` is the variadic-tail collector — we skip it
        // because there's no individual name to surface.
        let src = "proc many {first args} {}\nmany 1 2 3 4\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), None);
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
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), None);
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
        let hints = inlay_hints(src, range, Some(&analysis), None);
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert_eq!(hints[0].position_line, 2);
    }

    // -- S-inlay-hints-rich: built-in command synopsis hints --------

    fn registry() -> tcl_registry::CommandRegistry {
        tcl_registry::CommandRegistry::build_default()
    }

    #[test]
    fn synopsis_groups_rejoins_optional_flag_value() {
        let g = synopsis_groups("string compare ?-nocase? ?-length length? a b");
        assert_eq!(
            g,
            vec![
                "string",
                "compare",
                "?-nocase?",
                "?-length length?",
                "a",
                "b"
            ],
        );
    }

    #[test]
    fn param_names_skips_flags_and_keeps_positionals() {
        // After `string compare`, the flags drop out leaving the
        // two positionals.
        let names = param_names_from_synopsis(
            "string compare ?-nocase? ?-length length? string1 string2",
            2,
        );
        assert_eq!(names, vec!["string1", "string2"]);
    }

    #[test]
    fn param_names_stops_at_varargs() {
        let names = param_names_from_synopsis("string cat ?string1? ?string2 ...?", 2);
        // `?string1?` is an optional positional; `?string2 ...?`
        // is varargs → stop.
        assert_eq!(names, vec!["string1"]);
    }

    #[test]
    fn builtin_hint_for_subcommand_positional() {
        // `string index $s 3` — subcommand `index`, synopsis
        // `string index string charIndex`.
        let src = "string index $s 3\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), Some(&reg));
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"string:"), "{hints:?}");
        assert!(labels.contains(&"charIndex:"), "{hints:?}");
    }

    #[test]
    fn builtin_hint_skips_call_site_flags() {
        // `string compare -nocase $a $b` — `-nocase` is a flag,
        // so $a→string1, $b→string2.
        let src = "string compare -nocase $a $b\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), Some(&reg));
        // The flag token shouldn't be labelled.
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"string1:"), "{hints:?}");
        assert!(labels.contains(&"string2:"), "{hints:?}");
        // No hint anchored on the `-nocase` flag.
        for h in &hints {
            assert_ne!(h.position_character, 15, "flag should not be hinted: {h:?}");
        }
    }

    #[test]
    fn builtin_hint_treats_negative_number_as_positional() {
        // `string index $s -1` — the `index` subcommand declares no
        // `-1` option, so the registry-driven walk keeps `-1` as the
        // `charIndex` positional rather than skipping it as a flag.
        let src = "string index $s -1\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), Some(&reg));
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"string:"), "{hints:?}");
        assert!(labels.contains(&"charIndex:"), "{hints:?}");
    }

    #[test]
    fn builtin_hint_consumes_value_taking_option() {
        // `string compare -length 3 $a $b` — `-length` takes a value,
        // so both `-length` and `3` are skipped; $a→string1, $b→string2.
        let src = "string compare -length 3 $a $b\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), Some(&reg));
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        // Only the two positionals are labelled — `-length` and its
        // value `3` are both consumed.
        assert_eq!(labels, vec!["string1:", "string2:"], "{hints:?}");
    }

    #[test]
    fn builtin_hint_not_emitted_without_registry() {
        // Same source, no registry — no built-in hints.
        let src = "string index $s 3\n";
        let analysis = analyse(src);
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), None);
        assert!(hints.is_empty(), "{hints:?}");
    }

    #[test]
    fn user_proc_takes_precedence_over_builtin() {
        // A user proc named `string` (contrived) wins over the
        // built-in.  Here we just confirm a user proc still
        // gets its param hints when a registry is also present.
        let src = "proc greet {name} {}\ngreet alice\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(src, whole_document_range(src), Some(&analysis), Some(&reg));
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"name:"), "{hints:?}");
    }
}
