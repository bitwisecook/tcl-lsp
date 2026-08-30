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

//! Command segmenter + error-recovery suite. Verifies `segment_commands`
//! splits a Tcl script into commands and words.
//!
//! C-Tcl proof: command/word splitting is exactly Tcl's own parsing, verified
//! against tclsh8.6/9.0 — `set a 1` is a command with two args
//! (`proc cnt args {llength $args}; cnt a 1` → 2); `puts "hello $x"` passes a
//! single multi-token word (`cnt "hello $x"` → 1); an unclosed `proc x {} {`
//! is incomplete (`info complete` → 0), which is what `is_partial` marks.

use std::collections::HashSet;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands, segment_commands_with_recovery};

fn names(cmds: &[SegmentedCommand]) -> Vec<&str> {
    cmds.iter().map(SegmentedCommand::name).collect()
}

// -- Basic segmentation --

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

// -- Error recovery --

fn unclosed_source() -> String {
    // An unclosed proc brace that consumes to EOF, then valid commands.
    "proc broken {} {\n    set inner 1\n    set inner2 2\n    # missing close brace\nset x 1\nset after_recovery 42".to_string()
}

#[test]
fn recovery_finds_known_command_and_marks_partial() {
    let known: HashSet<&str> = ["proc", "set", "puts", "return", "if"]
        .into_iter()
        .collect();
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

// -- N5: the F5 `if` else/elseif lookahead across a single newline --
//
// `docs/design/bigip-irule-parser-measurements.md` §2 N5 (measured on TMM in
// a cli script reproducing the parser): `else` / `elseif` are a *separate*
// lookahead performed by `if` itself — picked up across a single newline,
// but NOT across a blank line, where they fall back to being an unknown
// command (`undefined procedure: else`). A trunk fact, gated on the F5
// grammar (the same `brace_line_continuation` axis the §2 N-rules ride).

fn segment_f5(src: &str) -> Vec<SegmentedCommand> {
    tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        src,
        0,
        tcl_lexer::LexerConfig::for_dialect("f5-irules"),
    )
}

#[test]
fn n5_else_across_a_single_newline_belongs_to_the_if() {
    let cmds = segment_f5("if {0} {set a 1}\nelse {set a 2}");
    assert_eq!(names(&cmds), ["if"], "the else line is the if's own");
    assert_eq!(cmds[0].texts, ["if", "0", "set a 1", "else", "set a 2"]);
    assert!(cmds[0].word_views_aligned());
}

#[test]
fn n5_elseif_chain_across_single_newlines_belongs_to_the_if() {
    let cmds = segment_f5("if {0} {set a 1}\nelseif {1} {set a 2}\nelse {set a 3}");
    assert_eq!(names(&cmds), ["if"]);
    assert_eq!(cmds[0].texts.len(), 8);
    assert!(cmds[0].word_views_aligned());
}

#[test]
fn n5_else_across_a_blank_line_stays_a_standalone_command() {
    // §2 N5: not across a blank line — `undefined procedure: else` on TMM,
    // so the segmentation stays exactly as stock.
    let cmds = segment_f5("if {0} {set a 1}\n\nelse {set a 2}");
    assert_eq!(names(&cmds), ["if", "else"]);
}

#[test]
fn n5_else_after_a_comment_line_stays_a_standalone_command() {
    let cmds = segment_f5("if {0} {set a 1}\n# note\nelse {set a 2}");
    assert_eq!(
        names(&cmds),
        ["if", "else"],
        "a comment line blocks the lookahead"
    );
}

#[test]
fn n5_else_after_a_semicolon_stays_a_standalone_command() {
    let cmds = segment_f5("if {0} {set a 1};\nelse {set a 2}");
    assert_eq!(names(&cmds), ["if", "else"]);
}

#[test]
fn n5_does_not_apply_outside_the_f5_grammar() {
    let cmds = segment_commands("if {0} {set a 1}\nelse {set a 2}");
    assert_eq!(names(&cmds), ["if", "else"]);
}

#[test]
fn n5_does_not_attach_else_to_a_non_if_command() {
    // The lookahead is `if`'s own — a preceding non-`if` command never
    // absorbs an `else` line.
    let cmds = segment_f5("set a 1\nelse {set a 2}");
    assert_eq!(names(&cmds), ["set", "else"]);
}
