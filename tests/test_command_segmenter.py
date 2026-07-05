# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the command segmenter and error recovery."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from analyser.semantic_model import Range
from compiler.parsing.command_segmenter import (
    SegmentedCommand,
    UnclosedDelimiter,
    _find_recovery_offset,
    _has_suspicious_token,
    find_first_dirty_chunk,
    segment_commands,
    segment_top_level_chunks,
)
from compiler.registry.dialect import dialect_scope
from shared.tokens import SourcePosition, Token, TokenType


@pytest.fixture
def irules_dialect():
    """Analyse this test's iRules snippet under the f5-irules dialect.

    These recovery tests wrap their input in a ``when EVENT { … }`` handler.
    ``when`` is an iRules-only builtin, so only under f5-irules is its body a
    script the analyser recurses into (collecting the inner commands the
    assertions inspect).  Under a non-iRules dialect ``when`` is an unknown
    command whose braced body is opaque data — correct, but not what these
    tests exercise.
    """
    with dialect_scope("f5-irules"):
        yield


class TestSegmentCommands:
    """Basic segmentation without error recovery."""

    def test_single_command(self):
        cmds = segment_commands("set a 1")
        assert len(cmds) == 1
        assert cmds[0].name == "set"
        assert cmds[0].args == ["a", "1"]

    def test_two_commands(self):
        cmds = segment_commands("set a 1\nset b 2")
        assert len(cmds) == 2
        assert cmds[0].name == "set"
        assert cmds[1].name == "set"
        assert cmds[1].args == ["b", "2"]

    def test_semicolon_separator(self):
        cmds = segment_commands("set a 1; set b 2")
        assert len(cmds) == 2
        assert cmds[0].name == "set"
        assert cmds[1].name == "set"

    def test_empty_source(self):
        cmds = segment_commands("")
        assert cmds == []

    def test_comment_only(self):
        cmds = segment_commands("# just a comment")
        assert cmds == []

    def test_preceding_comment_attached(self):
        cmds = segment_commands("# note\nset a 1")
        assert len(cmds) == 1
        assert cmds[0].preceding_comment == "note"

    def test_multiline_preceding_comment(self):
        cmds = segment_commands("# line one\n# line two\nset a 1")
        assert len(cmds) == 1
        assert cmds[0].preceding_comment == "line one\nline two"

    def test_blank_line_breaks_comment_accumulation(self):
        cmds = segment_commands("# orphan\n\nset a 1")
        assert len(cmds) == 1
        assert cmds[0].preceding_comment is None

    def test_body_token_segmentation(self):
        tok = Token(
            type=TokenType.STR,
            text="set a 1\nset b 2",
            start=SourcePosition(line=0, character=1, offset=1),
            end=SourcePosition(line=1, character=5, offset=16),
        )
        cmds = segment_commands(tok.text, body_token=tok)
        assert len(cmds) == 2

    def test_variable_word_piece(self):
        cmds = segment_commands("puts $x")
        assert len(cmds) == 1
        assert cmds[0].args == ["${x}"]

    def test_command_substitution_word_piece(self):
        cmds = segment_commands("puts [clock seconds]")
        assert len(cmds) == 1
        assert cmds[0].args == ["[clock seconds]"]

    def test_multi_token_word(self):
        cmds = segment_commands('puts "hello $name"')
        assert len(cmds) == 1
        assert not cmds[0].single_token_word[-1]

    def test_is_partial_false_for_normal_commands(self):
        cmds = segment_commands("set a 1\nset b 2")
        assert all(not cmd.is_partial for cmd in cmds)


class TestErrorRecovery:
    """Error recovery for unclosed braces/brackets."""

    def _make_unclosed_source(self, valid_cmd: str = "set x 1") -> str:
        """Create a source with an unclosed brace followed by valid commands.

        The unclosed brace must span enough lines and reach EOF to
        trigger recovery.
        """
        # Build a broken proc with unclosed brace, followed by valid code
        return (
            f"proc broken {{}} {{\n"
            f"    set inner 1\n"
            f"    set inner2 2\n"
            f"    # missing close brace\n"
            f"{valid_cmd}\n"
            f"set after_recovery 42"
        )

    def test_recovery_finds_known_command(self):
        """When an unclosed brace causes a STR to consume to EOF,
        recovery should find the next known command and resume."""
        known = frozenset(["proc", "set", "puts", "return", "if"])
        source = self._make_unclosed_source()
        cmds = segment_commands(source, known_commands=known)

        # Should have partial + recovered commands
        partial_cmds = [c for c in cmds if c.is_partial]
        valid_cmds = [c for c in cmds if not c.is_partial]

        assert len(partial_cmds) >= 1
        assert len(valid_cmds) >= 1

    def test_partial_command_marked(self):
        known = frozenset(["proc", "set", "puts", "return"])
        source = self._make_unclosed_source()
        cmds = segment_commands(source, known_commands=known)

        partial = [c for c in cmds if c.is_partial]
        assert len(partial) >= 1
        assert partial[0].is_partial

    def test_no_recovery_for_valid_source(self):
        """No error recovery should fire for well-formed source."""
        known = frozenset(["proc", "set", "puts", "return"])
        source = "proc foo {} {\n    set a 1\n    return $a\n}\nset b 2"
        cmds = segment_commands(source, known_commands=known)
        assert all(not c.is_partial for c in cmds)

    def test_no_recovery_for_body_token(self):
        """Error recovery is disabled inside body tokens to avoid
        false positives on legitimate multi-line strings."""
        known = frozenset(["set", "puts"])
        source = "set a 1\nset b 2\nset c 3\nset d 4"
        tok = Token(
            type=TokenType.STR,
            text=source,
            start=SourcePosition(line=0, character=0, offset=0),
            end=SourcePosition(line=3, character=7, offset=len(source)),
        )
        cmds = segment_commands(source, body_token=tok, known_commands=known)
        assert all(not c.is_partial for c in cmds)

    def test_recovered_commands_have_correct_names(self):
        known = frozenset(["proc", "set", "puts", "return"])
        source = self._make_unclosed_source("set x 1")
        cmds = segment_commands(source, known_commands=known)
        valid_names = [c.name for c in cmds if not c.is_partial]
        assert "set" in valid_names

    def test_partial_delimiter_brace(self):
        known = frozenset(["proc", "set", "puts", "return"])
        source = self._make_unclosed_source("set x 1")
        cmds = segment_commands(source, known_commands=known)
        partial = [c for c in cmds if c.is_partial]
        assert len(partial) >= 1
        assert partial[0].partial_delimiter is UnclosedDelimiter.BRACE

    def test_recovery_from_unclosed_bracket(self):
        """Unclosed [ swallows to EOF; recovery finds known commands."""
        known = frozenset(["set", "puts"])
        # Unclosed [ on first line, then valid commands on later lines.
        source = "set x [foo\nset y 2\nset z 3\nputs hello"
        cmds = segment_commands(source, known_commands=known)
        partial = [c for c in cmds if c.is_partial]
        valid = [c for c in cmds if not c.is_partial]
        assert len(partial) >= 1
        assert partial[0].partial_delimiter is UnclosedDelimiter.BRACKET
        assert len(valid) >= 1

    def test_recovery_from_unclosed_quote(self):
        """Unclosed " swallows to EOF; recovery finds known commands."""
        known = frozenset(["set", "puts"])
        source = 'set x "hello\nset y 2\nset z 3\nputs hello'
        cmds = segment_commands(source, known_commands=known)
        partial = [c for c in cmds if c.is_partial]
        valid = [c for c in cmds if not c.is_partial]
        assert len(partial) >= 1
        assert partial[0].partial_delimiter is UnclosedDelimiter.QUOTE
        assert len(valid) >= 1


class TestHasSuspiciousToken:
    def test_not_suspicious_for_short_str(self):
        cmd = SegmentedCommand(
            range=Range(
                start=SourcePosition(line=0, character=0, offset=0),
                end=SourcePosition(line=0, character=0, offset=0),
            ),
            argv=[],
            texts=[],
            single_token_word=[],
            all_tokens=[
                Token(
                    type=TokenType.STR,
                    text="short",
                    start=SourcePosition(line=0, character=0, offset=0),
                    end=SourcePosition(line=0, character=5, offset=5),
                )
            ],
        )
        assert _has_suspicious_token(cmd, 100) is None

    def test_suspicious_when_spans_lines_and_reaches_eof(self):
        cmd = SegmentedCommand(
            range=Range(
                start=SourcePosition(line=0, character=0, offset=0),
                end=SourcePosition(line=0, character=0, offset=0),
            ),
            argv=[],
            texts=[],
            single_token_word=[],
            all_tokens=[
                Token(
                    type=TokenType.STR,
                    text="line1\nline2\nline3\nline4",
                    start=SourcePosition(line=0, character=0, offset=0),
                    end=SourcePosition(line=3, character=5, offset=22),
                )
            ],
        )
        assert _has_suspicious_token(cmd, 23) is not None

    def test_not_suspicious_when_not_reaching_eof(self):
        cmd = SegmentedCommand(
            range=Range(
                start=SourcePosition(line=0, character=0, offset=0),
                end=SourcePosition(line=0, character=0, offset=0),
            ),
            argv=[],
            texts=[],
            single_token_word=[],
            all_tokens=[
                Token(
                    type=TokenType.STR,
                    text="line1\nline2\nline3\nline4",
                    start=SourcePosition(line=0, character=0, offset=0),
                    end=SourcePosition(line=3, character=5, offset=22),
                )
            ],
        )
        # Source is much longer than the token — not suspicious
        assert _has_suspicious_token(cmd, 200) is None

    def test_suspicious_single_line_cmd_token(self):
        """CMD token on a single line reaching EOF is suspicious — no threshold."""
        cmd = SegmentedCommand(
            range=Range(
                start=SourcePosition(line=0, character=0, offset=0),
                end=SourcePosition(line=0, character=0, offset=0),
            ),
            argv=[],
            texts=[],
            single_token_word=[],
            all_tokens=[
                Token(
                    type=TokenType.CMD,
                    text="foo bar",
                    start=SourcePosition(line=0, character=4, offset=4),
                    end=SourcePosition(line=0, character=10, offset=10),
                )
            ],
        )
        result = _has_suspicious_token(cmd, 11)
        assert result is not None
        assert result[1] is UnclosedDelimiter.BRACKET

    def test_suspicious_cmd_token(self):
        """CMD token spanning many lines and reaching EOF is suspicious."""
        cmd = SegmentedCommand(
            range=Range(
                start=SourcePosition(line=0, character=0, offset=0),
                end=SourcePosition(line=0, character=0, offset=0),
            ),
            argv=[],
            texts=[],
            single_token_word=[],
            all_tokens=[
                Token(
                    type=TokenType.CMD,
                    text="foo\nbar\nbaz\nqux",
                    start=SourcePosition(line=0, character=0, offset=0),
                    end=SourcePosition(line=3, character=3, offset=15),
                )
            ],
        )
        result = _has_suspicious_token(cmd, 16)
        assert result is not None
        assert result[1] is UnclosedDelimiter.BRACKET

    def test_suspicious_esc_token(self):
        """ESC token spanning many lines and reaching EOF is suspicious."""
        cmd = SegmentedCommand(
            range=Range(
                start=SourcePosition(line=0, character=0, offset=0),
                end=SourcePosition(line=0, character=0, offset=0),
            ),
            argv=[],
            texts=[],
            single_token_word=[],
            all_tokens=[
                Token(
                    type=TokenType.ESC,
                    text="hello\nworld\nfoo\nbar",
                    start=SourcePosition(line=0, character=0, offset=0),
                    end=SourcePosition(line=3, character=3, offset=19),
                )
            ],
        )
        result = _has_suspicious_token(cmd, 20)
        assert result is not None
        assert result[1] is UnclosedDelimiter.QUOTE


class TestFindRecoveryOffset:
    def test_finds_known_command_on_later_line(self):
        text = "    set inner 1\n    set inner2 2\nset x 1"
        known = frozenset(["set", "puts"])
        # token_start_offset is the position of the opening brace
        offset = _find_recovery_offset(text, 10, known)
        assert offset is not None

    def test_returns_none_when_no_known_command(self):
        text = "    foo bar\n    baz quux"
        known = frozenset(["set", "puts"])
        offset = _find_recovery_offset(text, 0, known)
        assert offset is None

    def test_skips_first_line(self):
        # First line is always part of the broken command
        text = "set x 1\nset y 2"
        known = frozenset(["set"])
        offset = _find_recovery_offset(text, 0, known)
        # Should find "set" on second line, not first
        assert offset is not None
        # The offset should point past the first line
        assert offset > len("set x 1")


class TestAnalyserErrorRecovery:
    """Integration: analyser emits E200 for partial commands and still
    analyses recovered commands after the break."""

    def test_emits_e200_or_e203_for_unclosed_brace(self):
        source = (
            "proc broken {} {\n"
            "    set inner 1\n"
            "    set inner2 2\n"
            "    # missing close brace\n"
            "set x 1\n"
            "set y 2"
        )
        result = analyse(source)
        codes = [d.code for d in result.diagnostics]
        # E203 fires when a de-indented known command signals where }
        # should be inserted; E200 is the generic fallback.
        assert "E200" in codes or "E203" in codes

    def test_valid_commands_still_analysed_after_recovery(self):
        source = (
            "proc broken {} {\n"
            "    set inner 1\n"
            "    set inner2 2\n"
            "    # missing close brace\n"
            "set x 1\n"
            "set y 2"
        )
        result = analyse(source)
        # After recovery, 'set x 1' and 'set y 2' should produce
        # variable definitions in the global scope.
        var_names = set(result.global_scope.variables.keys())
        assert "x" in var_names or "y" in var_names


class TestE201UnterminatedBracket:
    """E201: detect unterminated [ in CMD tokens (e.g. { inside [...])."""

    def test_e201_brace_inside_bracket(self, irules_dialect):
        """{ inside [ prevents ] from being found — produces E201."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch -- [ACCESS::policy agent_id {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e201 = [d for d in result.diagnostics if d.code == "E201"]
        assert len(e201) >= 1
        assert e201[0].message == "missing close-bracket"

    def test_e201_codefix_inserts_bracket_before_brace(self, irules_dialect):
        """E201 CodeFix inserts '] ' before the stray '{'."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch -- [ACCESS::policy agent_id {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e201 = [d for d in result.diagnostics if d.code == "E201"]
        assert len(e201) >= 1
        fixes = e201[0].fixes
        assert fixes
        assert fixes[0].new_text == "]"
        assert "Insert missing ']'" in fixes[0].description

    def test_no_e201_for_valid_cmd_substitution(self):
        """Properly closed [string length $x] produces no E201."""
        source = "set n [string length $x]"
        result = analyse(source)
        e201 = [d for d in result.diagnostics if d.code == "E201"]
        assert len(e201) == 0

    def test_e201_single_line_top_level(self):
        """Single-line unterminated [ at top level also triggers E201."""
        source = "set x [string length"
        result = analyse(source)
        codes = [d.code for d in result.diagnostics]
        # Could be E200 (recovery) or E201 — either way, an error is raised
        assert "E200" in codes or "E201" in codes


class TestE100StrayCloseBracketRecovery:
    """E100: recover from stray ']' (missing '[') for switch body analysis."""

    def test_e100_switch_recovers_compact_form(self, irules_dialect):
        """Stray ']' merges into virtual CMD → switch sees compact form."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        # E100 diagnostic should still fire for the stray ]
        e100 = [d for d in result.diagnostics if d.code == "E100"]
        assert len(e100) >= 1
        # Recovery should make switch see compact form, so "set x 1"
        # is analysed inside the pattern body (x becomes a variable).
        cmd_names = [ci.name for ci in result.command_invocations]
        assert "set" in cmd_names

    def test_e100_switch_body_not_treated_as_command(self, irules_dialect):
        """After recovery, "get_totp_key" is a switch pattern, not a command."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        cmd_names = [ci.name for ci in result.command_invocations]
        # "get_totp_key" should NOT be treated as a command name
        assert '"get_totp_key"' not in cmd_names
        assert "get_totp_key" not in cmd_names

    def test_no_e100_recovery_without_known_command(self):
        """Stray ']' without a known command backward doesn't merge."""
        source = "set x foobar]"
        result = analyse(source)
        e100 = [d for d in result.diagnostics if d.code == "E100"]
        assert len(e100) >= 1
        # No recovery should have occurred — set still has 2 args
        cmd_names = [ci.name for ci in result.command_invocations]
        assert "set" in cmd_names

    def test_no_recovery_for_valid_bracket(self, irules_dialect):
        """Valid [cmd] produces no stray bracket recovery."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e100 = [d for d in result.diagnostics if d.code == "E100"]
        assert len(e100) == 0
        # Switch should still parse correctly
        cmd_names = [ci.name for ci in result.command_invocations]
        assert "set" in cmd_names

    def test_e100_no_false_positive_on_quoted_close_bracket(self):
        """A "]" string literal is a character, not an unmatched bracket."""
        source = 'puts "]"\n'
        result = analyse(source)
        e100 = [d for d in result.diagnostics if d.code == "E100"]
        assert len(e100) == 0

    def test_e100_no_false_positive_on_close_bracket_in_quoted_string(self):
        """ "foo ]" should not trigger E100."""
        source = 'puts "foo ]"\n'
        result = analyse(source)
        e100 = [d for d in result.diagnostics if d.code == "E100"]
        assert len(e100) == 0

    def test_e100_no_false_positive_on_close_bracket_after_cmd_subst_in_quote(self):
        """ "[cmd] ]" — trailing ']' after a CMD in the same quoted word."""
        source = 'puts "[set x 1] ]"\n'
        result = analyse(source)
        e100 = [d for d in result.diagnostics if d.code == "E100"]
        assert len(e100) == 0

    def test_e100_still_fires_on_actual_stray_bracket(self):
        """Genuine stray ']' must still produce E100."""
        source = "set x foobar]\n"
        result = analyse(source)
        e100 = [d for d in result.diagnostics if d.code == "E100"]
        assert len(e100) >= 1

    def test_no_stray_bracket_recovery_on_quoted_close_bracket(self):
        """Analyser must not trigger recovery on a quoted ``"]"``.

        Previously the analyser's ``_recover_stray_close_bracket``
        merged any ESC ending in ``]`` into a virtual CMD, which would
        have misclassified ``set x "foo bar]"`` as a command
        substitution and corrupted argv.
        """
        source = 'set x "foo bar]"\n'
        result = analyse(source)
        assert not any(d.code == "E100" for d in result.diagnostics)
        cmd_names = [ci.name for ci in result.command_invocations]
        assert "set" in cmd_names


class TestE101MissingOpenBrace:
    """E101: detect missing '{' on switch and recover orphaned case commands."""

    def test_e101_switch_missing_brace_multi_case(self, irules_dialect):
        """switch with missing { and 2+ orphaned cases emits E101."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] \n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e101 = [d for d in result.diagnostics if d.code == "E101"]
        assert len(e101) == 1
        assert "Missing '{'" in e101[0].message

    def test_e101_switch_missing_brace_no_trailing_space(self, irules_dialect):
        """E101 fires when there's no trailing space after ]."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id]\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e101 = [d for d in result.diagnostics if d.code == "E101"]
        assert len(e101) == 1

    def test_e101_codefix_inserts_brace(self, irules_dialect):
        """E101 CodeFix offers to insert '{'."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] \n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e101 = [d for d in result.diagnostics if d.code == "E101"]
        assert e101[0].fixes
        assert " {" in e101[0].fixes[0].new_text

    def test_e101_recovery_analyses_both_case_bodies(self, irules_dialect):
        """After E101 recovery, variables in ALL case bodies are detected."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] \n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        cmd_names = [ci.name for ci in result.command_invocations]
        # Both set commands should be analysed
        assert cmd_names.count("set") == 2
        # Orphaned "other_key" should NOT appear as a command
        assert "other_key" not in cmd_names

    def test_e101_suppresses_e002(self, irules_dialect):
        """When E101 fires, E002 (too few args) should not also fire."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id]\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e002 = [d for d in result.diagnostics if d.code == "E002"]
        assert len(e002) == 0

    def test_no_e101_on_valid_switch_form2(self):
        """Valid switch with { does not emit E101."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e101 = [d for d in result.diagnostics if d.code == "E101"]
        assert len(e101) == 0

    def test_no_e101_on_valid_switch_form1(self):
        """Valid switch in Form 1 (explicit pairs) does not emit E101."""
        source = 'switch $x "a" {\n    set y 1\n} "b" {\n    set y 2\n}\n'
        result = analyse(source)
        e101 = [d for d in result.diagnostics if d.code == "E101"]
        assert len(e101) == 0

    def test_e101_with_options(self):
        """switch with options and missing { also detected."""
        source = 'switch -exact -- $x\n    "pat" {\n        set y 1\n    }\n'
        result = analyse(source)
        e101 = [d for d in result.diagnostics if d.code == "E101"]
        assert len(e101) >= 1


class TestE102StrayCloseBrace:
    """E102: detect unmatched '}' at any nesting level."""

    def test_e102_stray_brace_at_top_level(self):
        """Bare } at top level emits E102."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] \n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) >= 1

    def test_e102_no_false_positive_in_valid_code(self):
        """} inside a body does not emit E102."""
        source = "if {1} {\n    set x 1\n}\n"
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) == 0

    def test_e102_no_false_positive_on_trailing_empty_braces(self):
        """Issue #527: a body ending in an empty `{}` argument is valid Tcl.

        ``if {[llength $domain] != 2} {return {}}`` closes the body's brace right
        after the empty ``{}``.  The empty word's token already ends on its own
        closer, so the command span must not absorb the body's ``}`` and report
        it as a stray brace.
        """
        source = "if {[llength $domain] != 2} {return {}}\n"
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert e102 == []

    def test_e102_no_false_positive_on_trailing_empty_braces_in_proc(self):
        """The same construct nested inside a proc body must also stay clean."""
        source = "proc foo {domain} {\n    if {[llength $domain] != 2} {return {}}\n}\n"
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert e102 == []

    def test_e102_standalone_close_brace(self):
        """A standalone } line at top level emits E102."""
        source = "set x 1\n}\n"
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) >= 1

    def test_e102_codefix_removes_stray_brace_line(self):
        """CodeFix removes the entire line containing the stray }."""
        source = "proc foo {} {\n    set x 1\n}\n}\n"
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) == 1
        assert len(e102[0].fixes) == 1
        fix = e102[0].fixes[0]
        assert fix.new_text == ""
        # Applying the fix should remove the stray '}' line entirely.
        fixed = source[: fix.range.start.offset] + fix.new_text + source[fix.range.end.offset :]
        assert fixed == ("proc foo {} {\n    set x 1\n}\n")

    def test_e102_no_false_positive_on_quoted_close_brace(self):
        """A "}" string literal is a close brace character, not a stray brace."""
        source = 'append result "}"\n'
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) == 0

    def test_e102_no_false_positive_on_quoted_close_brace_in_proc(self):
        """String literals inside a proc body must not trigger E102."""
        source = (
            "proc getInfoCmd {name} {\n"
            '    append result "proc $name {[info args $name]} {"\n'
            "    append result [info body $name]\n"
            '    append result "}"\n'
            "    return $result\n"
            "}\n"
        )
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) == 0

    def test_e102_no_false_positive_on_close_brace_after_cmd_subst_in_quote(self):
        """A "[cmd]}" ESC token carries no delimiter but is still in-quote."""
        source = 'puts "[set x 1]}"\n'
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) == 0

    def test_e102_no_false_positive_on_close_brace_after_var_in_quote(self):
        """A "$var}" tail should not trigger E102 when text is bare '}'."""
        source = 'set x 1; puts "$x}"\n'
        result = analyse(source)
        e102 = [d for d in result.diagnostics if d.code == "E102"]
        assert len(e102) == 0


class TestE103MissingCloseBrace:
    """E103: detect when an inner body steals the enclosing scope's '}'."""

    def test_e103_switch_body_missing_close(self):
        """Missing } on switch body inside when emits E103."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    \n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        assert len(e103) == 1
        diag = e103[0]
        # The stolen '}' is on line 9 (0-indexed).
        assert diag.range.start.line == 9

    def test_e103_switch_single_case(self):
        """Missing } on switch body with only one case emits E103."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    \n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        assert len(e103) == 1
        assert e103[0].range.start.line == 6

    def test_e103_if_body_missing_close(self):
        """Missing } on if body inside when emits E103."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            '    if {[ACCESS::policy agent_id] eq "foo"} {\n'
            "        set x 1\n"
            "    \n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        assert len(e103) == 1
        assert e103[0].range.start.line == 4

    def test_e103_replaces_e200(self):
        """E103 fires instead of E200 when stolen brace is detected."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    \n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        e200 = [d for d in result.diagnostics if d.code == "E200"]
        assert len(e103) == 1
        assert len(e200) == 0

    def test_e103_codefix_inserts_brace(self):
        """CodeFix inserts } at correct indentation before the stolen brace."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    \n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        assert len(e103) == 1
        fix = e103[0].fixes[0]
        # The inserted text should have 4-space indentation matching 'switch'.
        assert fix.new_text == "    }\n"
        # Insertion is at the start of the stolen '}' line.
        assert fix.range.start.line == 9
        assert fix.range.start.character == 0

    def test_no_e103_on_valid_code(self):
        """Valid nested bodies produce no E103."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        assert len(e103) == 0

    def test_e103_recovery_still_works(self):
        """Segmenter recovery commands still get processed after E103."""
        # After the stolen brace, a new 'when' block follows.
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    \n"
            "}\n"
            "\n"
            "when HTTP_REQUEST {\n"
            "    set y 2\n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        assert len(e103) == 1
        # The recovered 'when HTTP_REQUEST' should still be processed —
        # verify we get its diagnostics but no E200 for it.
        e200 = [d for d in result.diagnostics if d.code == "E200"]
        assert len(e200) == 0

    def test_e103_deeply_nested(self):
        """E103 detects stolen brace through multiple nesting levels."""
        source = (
            "when HTTP_REQUEST {\n"
            "    if {1} {\n"
            "        switch $x {\n"
            "            a { set y 1 }\n"
            "        }\n"
            "    \n"
            "}\n"
        )
        result = analyse(source)
        e103 = [d for d in result.diagnostics if d.code == "E103"]
        assert len(e103) == 1
        # The stolen '}' is on line 6 — it closes 'if' instead of 'when'.
        assert e103[0].range.start.line == 6


class TestTopLevelChunks:
    """Tests for segment_top_level_chunks."""

    def test_one_chunk_per_command(self):
        source = "set a 1\nset b 2\nset c 3"
        chunks = segment_top_level_chunks(source)
        assert len(chunks) == 3

    def test_chunk_indices_sequential(self):
        source = "set a 1\nset b 2\nset c 3"
        chunks = segment_top_level_chunks(source)
        assert [c.index for c in chunks] == [0, 1, 2]

    def test_chunks_tile_source(self):
        source = "set a 1\nset b 2\nset c 3"
        chunks = segment_top_level_chunks(source)
        # First chunk starts at 0, last chunk ends at len(source)
        assert chunks[0].start_offset == 0
        assert chunks[-1].end_offset == len(source)
        # Chunks are contiguous
        for i in range(len(chunks) - 1):
            assert chunks[i].end_offset == chunks[i + 1].start_offset

    def test_hash_changes_with_content(self):
        src_a = "set a 1\nset b 2"
        src_b = "set a 1\nset b 99"
        chunks_a = segment_top_level_chunks(src_a)
        chunks_b = segment_top_level_chunks(src_b)
        # First chunk unchanged
        assert chunks_a[0].source_hash == chunks_b[0].source_hash
        # Second chunk changed
        assert chunks_a[1].source_hash != chunks_b[1].source_hash

    def test_empty_source(self):
        chunks = segment_top_level_chunks("")
        assert chunks == []

    def test_proc_is_single_chunk(self):
        source = "proc foo {} {\n    set a 1\n    return $a\n}\nset b 2"
        chunks = segment_top_level_chunks(source)
        # proc is one chunk, set b 2 is another
        assert len(chunks) == 2
        assert chunks[0].commands[0].name == "proc"
        assert chunks[1].commands[0].name == "set"


class TestFindFirstDirtyChunk:
    """Tests for dirty-suffix detection."""

    def test_identical_sources(self):
        src = "set a 1\nset b 2\nset c 3"
        old = segment_top_level_chunks(src)
        new = segment_top_level_chunks(src)
        assert find_first_dirty_chunk(old, new) == 3

    def test_last_chunk_changed(self):
        old = segment_top_level_chunks("set a 1\nset b 2\nset c 3")
        new = segment_top_level_chunks("set a 1\nset b 2\nset c 99")
        assert find_first_dirty_chunk(old, new) == 2

    def test_first_chunk_changed(self):
        old = segment_top_level_chunks("set a 1\nset b 2")
        new = segment_top_level_chunks("set a 99\nset b 2")
        assert find_first_dirty_chunk(old, new) == 0

    def test_chunk_added(self):
        old = segment_top_level_chunks("set a 1")
        new = segment_top_level_chunks("set a 1\nset b 2")
        assert find_first_dirty_chunk(old, new) == 1

    def test_chunk_removed(self):
        old = segment_top_level_chunks("set a 1\nset b 2")
        new = segment_top_level_chunks("set a 1")
        assert find_first_dirty_chunk(old, new) == 1

    def test_both_empty(self):
        assert find_first_dirty_chunk([], []) == 0

    def test_position_shift_is_dirty(self):
        # A chunk whose text is unchanged but which moved (blank line inserted
        # above) is dirty — its cached IR/diagnostics carry stale absolute
        # positions.  The first shifted chunk is the dirty index.
        old = segment_top_level_chunks("set a 1\nset b 2")
        new = segment_top_level_chunks("\nset a 1\nset b 2")
        assert find_first_dirty_chunk(old, new) == 0

    def test_blank_line_between_chunks_marks_suffix_dirty(self):
        old = segment_top_level_chunks("set a 1\nset b 2\nset c 3")
        new = segment_top_level_chunks("set a 1\n\nset b 2\nset c 3")
        # a is unshifted; b (and c) shifted down → first dirty is index 1.
        assert find_first_dirty_chunk(old, new) == 1

    def test_append_still_not_dirty(self):
        # Appending a command must not shift existing chunks' offsets.
        old = segment_top_level_chunks("set a 1\nset b 2")
        new = segment_top_level_chunks("set a 1\nset b 2\nset c 3")
        assert find_first_dirty_chunk(old, new) == 2


class TestDocumentStateIncremental:
    """Integration: DocumentState skips re-analysis for unchanged sources."""

    def test_skips_reanalysis_for_identical_source(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://a")
        state.update("set a 1\nset b 2")
        analysis_1 = state.analysis
        state.update("set a 1\nset b 2")
        # Same source — should reuse analysis (identity check)
        assert state.analysis is analysis_1

    def test_reanalyses_on_change(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://b")
        state.update("set a 1")
        analysis_1 = state.analysis
        state.update("set a 99")
        assert state.analysis is not analysis_1

    def test_chunks_updated_on_change(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://c")
        state.update("set a 1")
        assert len(state.chunks) == 1
        state.update("set a 1\nset b 2")
        assert len(state.chunks) == 2


class TestStableChunkHashes:
    """``TopLevelChunk.source_hash`` must be deterministic across processes:
    fresh analysis runs in a ``forkserver`` pool worker (fresh ``PYTHONHASHSEED``)
    and returns chunks the main process compares against locally-segmented ones.
    A salted builtin ``hash()`` made unchanged chunks look dirty and collapsed
    cache reuse."""

    _SRC = "proc alpha {} { return 1 }\nproc beta {} { return 2 }\nset c 3\n"

    def _hashes_under_seed(self, seed: str) -> tuple[str, str]:
        import os
        import subprocess
        import sys

        code = (
            "from compiler.parsing.command_segmenter import segment_top_level_chunks\n"
            f"src = {self._SRC!r}\n"
            "chunks = segment_top_level_chunks(src)\n"
            "print('CHUNK ' + ','.join(str(c.source_hash) for c in chunks))\n"
            "print('BUILTIN ' + str(hash(src)))\n"
        )
        proc = subprocess.run(
            [sys.executable, "-c", code],
            capture_output=True,
            text=True,
            env={**os.environ, "PYTHONHASHSEED": seed},
        )
        assert proc.returncode == 0, proc.stderr
        chunk_line = next(ln for ln in proc.stdout.splitlines() if ln.startswith("CHUNK "))
        builtin_line = next(ln for ln in proc.stdout.splitlines() if ln.startswith("BUILTIN "))
        return chunk_line[len("CHUNK ") :], builtin_line[len("BUILTIN ") :]

    def test_source_hash_identical_across_hash_seeds(self):
        chunks1, builtin1 = self._hashes_under_seed("1")
        chunks2, builtin2 = self._hashes_under_seed("2")
        # Guard: the two runs really used different seeds (builtin hash differs).
        assert builtin1 != builtin2, "PYTHONHASHSEED did not vary between runs"
        # The chunk source hashes must match regardless of seed.
        assert chunks1 == chunks2
        assert chunks1  # non-empty


class TestPartialCommandHashCoversTail:
    """Regression: a partial (unclosed) command's ``source_hash`` must cover its
    whole tile, not just the parsed ``range.end``.

    A partial command's ``range.end`` stops at the parse-failure point, so text
    edited in the unparsed tail still sits inside the chunk's tile but past
    ``cmd_end``.  If the hash only covered ``source[start:cmd_end+1]`` an edit
    there would leave the hash unchanged, ``find_first_dirty_chunk`` would treat
    the chunk as clean, and the incremental cache would serve stale per-chunk
    semantic tokens (observed as a token whose length lags the buffer)."""

    def test_edit_in_unparsed_tail_changes_hash(self):
        # In ``set b [ex\npr {\n1 + x]`` the command is *partial* (the ``[`` opens
        # a substitution whose ``{`` never closes); its ``range.end`` stops early,
        # leaving an unparsed tail inside the chunk's tile.  An edit there must
        # still change the chunk hash.
        before = "set a 1\nset b [ex\npr {\n1 + x]\nset c 3\n"
        after = "set a 1\nset b [ex\npr {\n1 + xYYY]\nset c 3\n"
        c_before = segment_top_level_chunks(before)[1]
        c_after = segment_top_level_chunks(after)[1]
        assert c_before.commands[0].is_partial, "expected the broken command to be partial"
        assert c_before.source_hash != c_after.source_hash, (
            "editing a partial command's unparsed tail must change its chunk hash"
        )

    def test_dirty_detection_flags_the_partial_chunk(self):
        old = segment_top_level_chunks("set a 1\nset b [ex\npr {\n1 + x]\nset c 3\n")
        new = segment_top_level_chunks("set a 1\nset b [ex\npr {\n1 + xYYY]\nset c 3\n")
        # Chunk 0 (``set a 1``) is unchanged; the partial chunk (index 1) is dirty.
        assert find_first_dirty_chunk(old, new) == 1

    def test_well_formed_command_hash_unaffected_by_appended_command(self):
        # The append-invariant still holds: a well-formed command's hash does not
        # change when a new command is appended after it.
        one = segment_top_level_chunks("set a 1\n")[0]
        two = segment_top_level_chunks("set a 1\nset b 2\n")[0]
        assert one.source_hash == two.source_hash

    def test_trailing_layout_whitespace_is_folded(self):
        # When the gap after the last token is pure layout whitespace it is
        # trivia: ``_chunk_content_end`` drops it, so it must not affect the hash
        # (this is what keeps the append-invariant).
        plain = segment_top_level_chunks("set a 1\n")[0]
        spaced = segment_top_level_chunks("set a 1   \t\n")[0]
        assert plain.source_hash == spaced.source_hash

    def test_trailing_semicolon_is_kept_distinct(self):
        # ``;`` is a syntactic command terminator, not layout whitespace, so the
        # gap after the last token is *not* pure-whitespace and the chunk keeps
        # its whole tile.  Folding ``set a 1;`` onto ``set a 1`` would collide
        # chunks the incremental builder has to tell apart (a real divergence seen
        # under the random-edit storm); guard against treating ``;`` as trivia.
        plain = segment_top_level_chunks("set a 1\n")[0]
        semi = segment_top_level_chunks("set a 1;\n")[0]
        assert plain.source_hash != semi.source_hash

    def test_trailing_unicode_whitespace_is_not_over_stripped(self):
        # The trivia test names the lexer's ASCII whitespace set, so a trailing
        # non-ASCII space the lexer treats as word content makes the gap non-empty
        # and still changes the hash (a bare ``str.rstrip()`` would eat it).
        plain = segment_top_level_chunks("set a 1\n")[0]
        nbsp = segment_top_level_chunks("set a 1\u00a0\n")[0]  # U+00A0 NBSP
        assert plain.source_hash != nbsp.source_hash


class TestUnclosedDelimiterSwallowsTrailingWhitespace:
    """Regression: an unclosed delimiter at EOF consumes trailing whitespace into
    its token, so that whitespace sets the token's rendered length and is *not*
    trivia.  ``_chunk_content_end`` extends the hash to the last token's end (the
    gap after it is empty, not pure whitespace), so editing the swallowed
    whitespace changes the hash \u2014 otherwise the per-chunk token cache serves a
    stale length."""

    @pytest.mark.parametrize(
        "more,less",
        [
            ("set x {abc   ", "set x {abc  "),  # unclosed brace
            ('puts "abc   ', 'puts "abc  '),  # unclosed quote
            ("set y [abc   ", "set y [abc  "),  # unclosed bracket
        ],
    )
    def test_trailing_ws_inside_unclosed_token_changes_hash(self, more, less):
        c_more = segment_top_level_chunks(more)[0]
        c_less = segment_top_level_chunks(less)[0]
        # The unterminated token reaches (near) EOF \u2014 that's what makes the
        # trailing whitespace part of a token rather than inter-command trivia.
        assert c_more.commands[0].all_tokens[-1].end.offset >= len(more) - 2
        assert c_more.source_hash != c_less.source_hash

    def test_well_formed_trailing_ws_still_folded(self):
        # The append-invariant is unharmed: with no unclosed token swallowing it,
        # trailing whitespace is trivia and does not change the hash.
        a = segment_top_level_chunks("set x 1   \n")[0]
        b = segment_top_level_chunks("set x 1  \n")[0]
        assert a.source_hash == b.source_hash


class TestEqualLengthLineShiftIsDirty:
    """Regression: ``find_first_dirty_chunk`` keys reuse on the full start
    *position*, not just ``start_offset``.  Leading/inter-command whitespace lives
    outside chunk tiles, so an equal-length edit there (e.g. a space replaced by a
    newline) shifts every following chunk's line without changing its offset or
    hash.  Cached tokens are absolute, so such a chunk must be treated as dirty."""

    def test_leading_space_to_newline_flags_first_chunk_dirty(self):
        a = "  set x 1\nset y 2\n"
        b = " \nset x 1\nset y 2\n"  # one leading space -> newline (same length)
        ca, cb = segment_top_level_chunks(a), segment_top_level_chunks(b)
        assert len(a) == len(b)
        # offsets and hashes are unchanged; only the start line shifts.
        assert ca[0].start_offset == cb[0].start_offset
        assert ca[0].source_hash == cb[0].source_hash
        assert ca[0].commands[0].range.start.line != cb[0].commands[0].range.start.line
        assert find_first_dirty_chunk(ca, cb) == 0

    def test_pure_append_stays_clean(self):
        # The position check must not over-invalidate: appending a command shifts
        # no existing chunk's position, so all existing chunks stay clean.
        ca = segment_top_level_chunks("set x 1\nset y 2\n")
        cb = segment_top_level_chunks("set x 1\nset y 2\nset z 3\n")
        assert find_first_dirty_chunk(ca, cb) == 2
