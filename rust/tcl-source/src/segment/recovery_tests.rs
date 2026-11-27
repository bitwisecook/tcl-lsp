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

//! Tests for [`segment_commands_with_recovery`].
//!
//! When an unclosed `{` / `[` / `"` causes the lexer to
//! consume the rest of the file as one giant token, the
//! segmenter scans the inner text for a line whose first word
//! is a known command, marks the broken command `is_partial`,
//! and re-segments from the recovery point so later commands
//! still surface to consumers (workspace index, signature
//! scan).

use super::*;
use std::collections::HashSet;

fn known<const N: usize>(names: [&'static str; N]) -> HashSet<&'static str> {
    HashSet::from(names)
}

#[test]
fn unclosed_brace_recovers_at_known_command() {
    // Recovery from an unclosed brace scans forward for the next
    // line whose first word is in
    // the known-commands set. Without recovery the
    // segmenter would yield a single command — `proc early` —
    // whose body STR token swallows everything to EOF.
    let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
    let cmds = segment_commands_with_recovery(src, &known(["proc"]));
    assert_eq!(cmds.len(), 2, "expected partial + recovered, got {cmds:?}");
    assert!(cmds[0].is_partial, "first command should be marked partial");
    assert_eq!(cmds[1].name(), "proc");
    // Verify the recovered command's argv span points at the
    // *outer* source buffer's offset of `proc late`, not the
    // local-to-slice offset.
    let argv0 = cmds[1].argv[0];
    assert_eq!(&src[argv0.span.as_range()], "proc");
}

#[test]
fn recovery_records_partial_delimiter() {
    // Unclosed brace → BRACE; the recovered partial carries the
    // precise delimiter for the E200 message + stolen-brace gate.
    let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
    let cmds = segment_commands_with_recovery(src, &known(["proc"]));
    assert_eq!(cmds[0].partial_delimiter, Some(UnclosedDelimiter::Brace));
    // Unclosed bracket → BRACKET (no line-span threshold for `[`).
    let src = "set x [foo\nputs done\n";
    let cmds = segment_commands_with_recovery(src, &known(["set", "puts"]));
    assert!(cmds[0].is_partial);
    assert_eq!(cmds[0].partial_delimiter, Some(UnclosedDelimiter::Bracket));
    // A complete command records no partial delimiter.
    let cmds = segment_commands_with_recovery("set x 1\n", &known(["set"]));
    assert_eq!(cmds[0].partial_delimiter, None);
}

#[test]
fn unclosed_brace_without_known_command_keeps_swallowed_input() {
    // No recovery point reachable — the broken command stays
    // as the only command and is *not* marked partial (recovery
    // never fires).
    let src = "proc early {} {\n    line one\n    line two\n";
    let cmds = segment_commands_with_recovery(src, &known(["proc"]));
    assert_eq!(cmds.len(), 1);
    assert!(!cmds[0].is_partial);
}

#[test]
fn unclosed_brace_below_line_threshold_skipped() {
    // Multi-line braced literals shorter than the line
    // threshold (3 lines) are NOT treated as suspicious — they
    // are likely legitimate string content.
    let src = "set x {\n  line two\n";
    let cmds = segment_commands_with_recovery(src, &known(["proc", "set"]));
    // Recovery does not fire; we still get the one (over-large)
    // command without a partial split.
    assert_eq!(cmds.len(), 1);
    assert!(!cmds[0].is_partial);
}

#[test]
fn unclosed_bracket_recovers_without_line_threshold() {
    // Unclosed `[` is always suspicious (no line span check) —
    // a valid `[…]` always closes. Note: braced empty bodies
    // `{}` round-trip through `word_piece` as empty strings
    // because `SourceMap::token_text` strips the opening `{`,
    // leaving the brace's content (empty) behind.
    let src = "set x [foo\nproc late {} {}\n";
    let cmds = segment_commands_with_recovery(src, &known(["proc"]));
    assert!(
        cmds.len() >= 2,
        "expected partial + recovered, got {cmds:?}"
    );
    let recovered = cmds.last().unwrap();
    assert_eq!(recovered.name(), "proc");
    assert_eq!(recovered.args(), ["late", "", ""]);
}

#[test]
fn empty_known_commands_set_never_recovers() {
    let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
    let cmds = segment_commands_with_recovery(src, &HashSet::new());
    assert_eq!(cmds.len(), 1);
    assert!(!cmds[0].is_partial);
}

#[test]
fn closed_input_is_unaffected_by_recovery() {
    // Recovery must not perturb well-formed inputs — same
    // commands as `segment_commands(src)` would produce.
    let src = "proc foo {} {}\nproc bar {} {}";
    let kc = known(["proc"]);
    let recovered = segment_commands_with_recovery(src, &kc);
    let raw = segment_commands(src);
    assert_eq!(recovered.len(), raw.len());
    for (r, p) in recovered.iter().zip(raw.iter()) {
        assert_eq!(r.name(), p.name());
        assert_eq!(r.args(), p.args());
        assert_eq!(r.is_partial, p.is_partial);
    }
}

#[test]
fn closed_command_sub_followed_by_trailing_newline_not_suspicious() {
    // Regression for the EOF off-by-one: ``Span::end()`` is
    // exclusive, so a closed ``[cmd]`` whose ``]`` sits at
    // ``source_len - 2`` (followed by a single ``\n``) ends
    // at ``source_len - 1``. The previous ``end + 1 < source_len``
    // check incorrectly accepted such tokens as "reaches EOF",
    // which spuriously triggered recovery on ``Cmd`` tokens
    // (no line-span check) when their inner text contained a
    // known command name like ``proc``.
    //
    // After the fix: only ``end == source_len`` qualifies as
    // EOF, so this input segments cleanly without recovery.
    let src = "set x [proc foo {} {}]\n";
    let kc = known(["proc"]);
    let cmds = segment_commands_with_recovery(src, &kc);
    // Should be exactly one command (`set x [...]`); no
    // partial split, no spurious extra recovered commands.
    assert_eq!(cmds.len(), 1, "expected one command, got {cmds:?}");
    assert_eq!(cmds[0].name(), "set");
    assert!(!cmds[0].is_partial);
}

#[test]
fn closed_multiline_brace_followed_by_trailing_newline_not_suspicious() {
    // Same regression but for the ``Str`` (brace) path. A
    // closed multi-line braced string spanning more than the
    // ``RECOVERY_LINE_THRESHOLD`` lines, with a trailing
    // newline, must not be treated as suspicious.
    let src = "set x {\n  line1\n  line2\n  line3\n}\n";
    let kc = known(["set"]);
    let cmds = segment_commands_with_recovery(src, &kc);
    assert_eq!(cmds.len(), 1, "expected one command, got {cmds:?}");
    assert_eq!(cmds[0].name(), "set");
    assert!(!cmds[0].is_partial);
}

#[test]
fn recovery_picks_first_matching_line_not_later_ones() {
    // Two `set` commands appear in the broken body. The
    // segmenter must recover at the first match — the line
    // starting with `set y 1` — not skip ahead to the later
    // `set z 2`. The recovered slice is then re-segmented on
    // its own, picking up `foo bar` and the second `set` too.
    let src = "proc early {} {\n    set y 1\nfoo bar\nset z 2\n";
    let cmds = segment_commands_with_recovery(src, &known(["set"]));
    assert!(cmds.len() >= 2);
    assert!(cmds[0].is_partial);
    // First recovered command points at `set y 1`, not `set z 2`.
    assert_eq!(cmds[1].name(), "set");
    assert_eq!(cmds[1].args(), ["y", "1"]);
    // The later `set z 2` command is also reachable in the
    // recovered slice.
    let last_set = cmds
        .iter()
        .rev()
        .find(|c| c.name() == "set")
        .expect("at least one set");
    assert_eq!(last_set.args(), ["z", "2"]);
}

#[test]
fn recovery_skips_indented_lines_and_finds_unindented_match() {
    // Recovery scans for the first line whose first word is
    // known, regardless of indentation — the line lookup
    // strips leading whitespace before the word match.
    let src = "proc early {} {\n    not_known foo\n    proc indented {} {}\n";
    let cmds = segment_commands_with_recovery(src, &known(["proc"]));
    assert_eq!(cmds.len(), 2);
    assert!(cmds[0].is_partial);
    assert_eq!(cmds[1].name(), "proc");
    // `{}` empty bodies become empty strings in `texts` (the
    // segmenter strips the opening delimiter before emitting
    // the word piece).
    assert_eq!(cmds[1].args(), ["indented", "", ""]);
}

#[test]
fn recovery_from_offset_into_source_uses_absolute_offsets() {
    // The recovered slice must use absolute (outer-source)
    // offsets so downstream consumers (signature_scan, the
    // workspace index) can slice the recovered token text from
    // the original buffer without further bookkeeping.
    let src = "proc early {} {\n    # broken\n\nproc late {} {}\n";
    let cmds = segment_commands_with_recovery(src, &known(["proc"]));
    let late = cmds.iter().find(|c| !c.is_partial).expect("recovered cmd");
    let argv0 = late.argv[0];
    assert_eq!(&src[argv0.span.as_range()], "proc");
    // Confirm the recovered command's argv tokens span the
    // outer-source bytes (the `late` name token text matches
    // the source byte range).
    let name_tok = late.argv[1];
    assert_eq!(&src[name_tok.span.as_range()], "late");
}
