"""Tests for the command segmenter and error recovery."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Range
from core.parsing.command_segmenter import (
    SegmentedCommand,
    UnclosedDelimiter,
    _find_recovery_offset,
    _has_suspicious_token,
    find_first_dirty_chunk,
    segment_commands,
    segment_top_level_chunks,
)
from core.parsing.tokens import SourcePosition, Token, TokenType


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

    def test_e201_brace_inside_bracket(self):
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

    def test_e201_codefix_inserts_bracket_before_brace(self):
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

    def test_e100_switch_recovers_compact_form(self):
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

    def test_e100_switch_body_not_treated_as_command(self):
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

    def test_no_recovery_for_valid_bracket(self):
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


class TestE101MissingOpenBrace:
    """E101: detect missing '{' on switch and recover orphaned case commands."""

    def test_e101_switch_missing_brace_multi_case(self):
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

    def test_e101_switch_missing_brace_no_trailing_space(self):
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

    def test_e101_codefix_inserts_brace(self):
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

    def test_e101_recovery_analyses_both_case_bodies(self):
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

    def test_e101_suppresses_e002(self):
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


class TestDocumentStateIncremental:
    """Integration: DocumentState skips re-analysis for unchanged sources."""

    def test_skips_reanalysis_for_identical_source(self):
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://a")
        state.update("set a 1\nset b 2")
        analysis_1 = state.analysis
        state.update("set a 1\nset b 2")
        # Same source — should reuse analysis (identity check)
        assert state.analysis is analysis_1

    def test_reanalyses_on_change(self):
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://b")
        state.update("set a 1")
        analysis_1 = state.analysis
        state.update("set a 99")
        assert state.analysis is not analysis_1

    def test_chunks_updated_on_change(self):
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://c")
        state.update("set a 1")
        assert len(state.chunks) == 1
        state.update("set a 1\nset b 2")
        assert len(state.chunks) == 2
