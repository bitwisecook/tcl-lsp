//! Port of `tests/test_command_segmenter.py` — the command segmenter +
//! error-recovery suite. Verifies the Rust `segment_commands` splits a Tcl
//! script into commands and words the same way the Python segmenter did.
//!
//! C-Tcl proof: command/word splitting is exactly Tcl's own parsing, verified
//! against tclsh8.6/9.0 — `set a 1` is a command with two args
//! (`proc cnt args {llength $args}; cnt a 1` → 2); `puts "hello $x"` passes a
//! single multi-token word (`cnt "hello $x"` → 1); an unclosed `proc x {} {`
//! is incomplete (`info complete` → 0), which is what `is_partial` marks.

use std::collections::HashSet;
use tcl_compiler::segmenter::{segment_commands, segment_commands_with_recovery, SegmentedCommand};

fn names(cmds: &[SegmentedCommand]) -> Vec<&str> {
    cmds.iter().map(SegmentedCommand::name).collect()
}

// -- Basic segmentation (TestSegmentCommands) --

#[test]
fn single_command() {
    let cmds = segment_commands("set a 1");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].name(), "set");
    assert_eq!(cmds[0].args(), ["a", "1"]);
}

#[test]
fn two_commands_split_on_newline() {
    let cmds = segment_commands("set a 1\nset b 2");
    assert_eq!(cmds.len(), 2);
    assert_eq!(names(&cmds), ["set", "set"]);
    assert_eq!(cmds[1].args(), ["b", "2"]);
}

#[test]
fn semicolon_separator() {
    let cmds = segment_commands("set a 1; set b 2");
    assert_eq!(cmds.len(), 2);
    assert_eq!(names(&cmds), ["set", "set"]);
}

#[test]
fn empty_source_is_no_commands() {
    assert!(segment_commands("").is_empty());
}

#[test]
fn comment_only_is_no_commands() {
    assert!(segment_commands("# just a comment").is_empty());
}

#[test]
fn preceding_comment_attached() {
    let cmds = segment_commands("# note\nset a 1");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].preceding_comment.as_deref(), Some("note"));
}

#[test]
fn multiline_preceding_comment_concatenated() {
    let cmds = segment_commands("# line one\n# line two\nset a 1");
    assert_eq!(cmds.len(), 1);
    assert_eq!(
        cmds[0].preceding_comment.as_deref(),
        Some("line one\nline two")
    );
}

#[test]
fn blank_line_breaks_comment_accumulation() {
    let cmds = segment_commands("# orphan\n\nset a 1");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].preceding_comment, None);
}

#[test]
fn variable_word_piece_normalised() {
    // `$x` reconstructs to the brace-normalised `${x}` word.
    let cmds = segment_commands("puts $x");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].args(), ["${x}"]);
}

#[test]
fn command_substitution_word_piece() {
    let cmds = segment_commands("puts [clock seconds]");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].args(), ["[clock seconds]"]);
}

#[test]
fn multi_token_word_flagged() {
    // tclsh: `cnt "hello $x"` → 1 word (a single multi-token argument).
    let cmds = segment_commands("puts \"hello $name\"");
    assert_eq!(cmds.len(), 1);
    assert!(!cmds[0].single_token_word.last().copied().unwrap());
}

#[test]
fn normal_commands_are_not_partial() {
    let cmds = segment_commands("set a 1\nset b 2");
    assert!(cmds.iter().all(|c| !c.is_partial));
}

// -- Error recovery (TestErrorRecovery) --

fn unclosed_source() -> String {
    // An unclosed proc brace that consumes to EOF, then valid commands.
    "proc broken {} {\n    set inner 1\n    set inner2 2\n    # missing close brace\nset x 1\nset after_recovery 42".to_string()
}

#[test]
fn recovery_finds_known_command_and_marks_partial() {
    let known: HashSet<&str> = ["proc", "set", "puts", "return", "if"].into_iter().collect();
    let src = unclosed_source();
    let cmds = segment_commands_with_recovery(&src, &known);
    let partial = cmds.iter().filter(|c| c.is_partial).count();
    let valid = cmds.iter().filter(|c| !c.is_partial).count();
    assert!(partial >= 1, "an unclosed brace yields a partial command");
    assert!(valid >= 1, "recovery resumes at the next known command");
}

#[test]
fn no_recovery_for_valid_source() {
    let known: HashSet<&str> = ["proc", "set", "puts", "return"].into_iter().collect();
    let src = "proc foo {} {\n    set a 1\n    return $a\n}\nset b 2";
    let cmds = segment_commands_with_recovery(src, &known);
    assert!(cmds.iter().all(|c| !c.is_partial));
}

#[test]
fn plain_segmentation_never_marks_partial_on_balanced_source() {
    // The body-token path (no recovery) never flags partials on well-formed,
    // multi-line input.
    let cmds = segment_commands("set a 1\nset b 2\nset c 3\nset d 4");
    assert!(cmds.iter().all(|c| !c.is_partial));
}

#[test]
fn recovered_commands_have_correct_names() {
    let known: HashSet<&str> = ["proc", "set", "puts", "return"].into_iter().collect();
    let src = unclosed_source();
    let cmds = segment_commands_with_recovery(&src, &known);
    // The recovered (non-partial) tail commands are all `set`s.
    let recovered: Vec<&str> = cmds
        .iter()
        .filter(|c| !c.is_partial)
        .map(SegmentedCommand::name)
        .collect();
    assert!(recovered.contains(&"set"));
}

// -- Word shapes --

#[test]
fn braced_word_is_single_token() {
    let cmds = segment_commands("proc p {a b} {body}");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].name(), "proc");
    // proc has three args: name, params, body.
    assert_eq!(cmds[0].args().len(), 3);
}

#[test]
fn expansion_marker_recorded() {
    // `{*}$args` marks the word for expansion.
    let cmds = segment_commands("puts {*}$args");
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].expand_word.is_some(), "{{*}} expansion recorded");
}
