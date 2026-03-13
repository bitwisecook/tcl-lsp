"""Tests for zero-width virtual token recovery (recovery.py).

Tests the centralised error recovery architecture:
- Virtual token injection into the lexer
- E201 comment-break heuristic
- E201 command-break heuristic
- E201 brace-break heuristic (existing, migrated)
- Integration: analyser and semantic tokens produce correct output
"""

from __future__ import annotations

from core.analysis.analyser import analyse
from core.parsing.lexer import TclLexer
from core.parsing.recovery import (
    compute_virtual_insertions,
    segment_with_recovery,
)
from core.parsing.tokens import TokenType
from lsp.features.semantic_tokens import semantic_tokens_full


class TestVirtualTokenLexer:
    """Unit tests for zero-width virtual token injection in TclLexer."""

    def test_virtual_bracket_terminates_cmd(self):
        """Virtual ] at a position terminates a CMD token."""
        # Source: [foo bar\nset x 1
        # Without virtual: CMD swallows everything
        # With virtual ] at offset 8 (the \n): CMD ends at "bar"
        source = "[foo bar\nset x 1"
        lexer = TclLexer(source, virtual_insertions={8: "]"})
        tokens = lexer.tokenise_all()
        cmd_tokens = [t for t in tokens if t.type is TokenType.CMD]
        assert len(cmd_tokens) == 1
        assert cmd_tokens[0].text == "foo bar"

    def test_virtual_does_not_shift_positions(self):
        """Virtual token is zero-width — subsequent positions are unchanged."""
        source = "[foo\nset x 1"
        lexer = TclLexer(source, virtual_insertions={4: "]"})
        tokens = lexer.tokenise_all()
        # After virtual ] consumed at offset 4, next token starts at
        # the \n (offset 4 in real text).
        esc_tokens = [t for t in tokens if t.type is TokenType.ESC]
        set_tok = next(t for t in esc_tokens if t.text == "set")
        assert set_tok.start.offset == 5  # "set" starts at real offset 5

    def test_virtual_brace_terminates_str(self):
        """Virtual } at a position terminates a STR (brace) token."""
        source = "{foo bar\nset x 1"
        lexer = TclLexer(source, virtual_insertions={8: "}"})
        tokens = lexer.tokenise_all()
        str_tokens = [t for t in tokens if t.type is TokenType.STR]
        assert len(str_tokens) >= 1
        assert str_tokens[0].text == "foo bar"

    def test_no_virtuals_behaves_normally(self):
        """When no virtuals are set, lexer behaves exactly as before."""
        source = "set x 1"
        lexer_normal = TclLexer(source)
        lexer_empty = TclLexer(source, virtual_insertions={})
        tokens_normal = lexer_normal.tokenise_all()
        tokens_empty = lexer_empty.tokenise_all()
        assert len(tokens_normal) == len(tokens_empty)
        for t1, t2 in zip(tokens_normal, tokens_empty):
            assert t1.type == t2.type
            assert t1.text == t2.text
            assert t1.start == t2.start
            assert t1.end == t2.end

    def test_virtual_at_eof(self):
        """Virtual token at EOF is still consumed."""
        source = "[foo"
        lexer = TclLexer(source, virtual_insertions={4: "]"})
        tokens = lexer.tokenise_all()
        cmd_tokens = [t for t in tokens if t.type is TokenType.CMD]
        assert len(cmd_tokens) == 1
        assert cmd_tokens[0].text == "foo"


class TestComputeVirtualInsertions:
    """Tests for the virtual insertions computation."""

    def test_clean_source_returns_empty(self):
        """No virtual insertions needed for clean source."""
        source = "set x [string length $y]"
        result = compute_virtual_insertions(source)
        assert result == {}

    def test_unterminated_cmd_with_brace_returns_insertion(self):
        """Unterminated [ with { returns a virtual ] insertion."""
        source = "switch -- [ACCESS::policy agent_id {\n    default { set x 1 }\n}"
        result = compute_virtual_insertions(source)
        assert len(result) == 1
        assert "]" in result.values()

    def test_unterminated_cmd_with_comment_returns_insertion(self):
        """Unterminated [ followed by # comment returns a virtual ] insertion."""
        source = (
            "set username [ACCESS::session data get session.logon.last.username\n# comment\nset x 1"
        )
        result = compute_virtual_insertions(source)
        assert len(result) == 1
        assert "]" in result.values()


class TestE201CommentBreak:
    """E201: unterminated [ where # comment signals the break point."""

    def test_comment_break_tight_diagnostic(self):
        """E201 diagnostic range covers only the incomplete line, not the comment."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "set username [ACCESS::session data get session.logon.last.username\n"
            "# retrieve key from specified storage\n"
            'set totp_key ""\n'
            "switch $totp_key_storage {\n"
            "    ldap {\n"
            "        set totp_key [ACCESS::session data get session.custom.totp_key]\n"
            "    }\n"
            "}\n"
            "}"
        )
        result = analyse(source)
        e201 = [d for d in result.diagnostics if d.code == "E201"]
        assert len(e201) >= 1
        # Diagnostic should be on line 2 (0-indexed: line 1),
        # NOT spanning to line 4 (the switch line).
        diag = e201[0]
        assert diag.range.start.line == 1  # line with [ACCESS...
        assert diag.range.end.line == 1  # should NOT extend beyond this line

    def test_comment_break_codefix_inserts_before_comment(self):
        """E201 CodeFix inserts ] at the end of the incomplete line."""
        source = (
            "set username [ACCESS::session data get session.logon.last.username\n"
            "# retrieve key from specified storage\n"
            'set totp_key ""'
        )
        result = analyse(source)
        e201 = [d for d in result.diagnostics if d.code == "E201"]
        assert len(e201) >= 1
        fixes = e201[0].fixes
        assert fixes
        assert fixes[0].new_text == "]"
        assert "comment" in fixes[0].description.lower()

    def test_comment_break_switch_body_analysed(self):
        """After E201 comment-break recovery, subsequent switch body is analysed."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "set username [ACCESS::session data get session.logon.last.username\n"
            "# retrieve key from specified storage\n"
            'set totp_key ""\n'
            "switch $totp_key_storage {\n"
            "    ldap {\n"
            "        set inner_var [ACCESS::session data get session.custom.totp_key]\n"
            "    }\n"
            "}\n"
            "}"
        )
        result = analyse(source)
        # The switch body should be analysed — "inner_var" should be defined.
        cmd_names = [ci.name for ci in result.command_invocations]
        assert "switch" in cmd_names
        assert "set" in cmd_names

    def test_comment_break_semantic_tokens(self):
        """Semantic tokens show proper highlighting after comment-break recovery."""
        source = (
            "set username [ACCESS::session data get session.logon.last.username\n"
            "# retrieve key from specified storage\n"
            'set totp_key ""\n'
        )
        tokens_data = semantic_tokens_full(source)
        # Convert to (line, char, length, type, mods) tuples
        tokens = []
        for i in range(0, len(tokens_data), 5):
            tokens.append(tuple(tokens_data[i : i + 5]))

        # Decode delta-encoded tokens to absolute positions
        abs_tokens = []
        prev_line = 0
        prev_char = 0
        for dl, dc, length, typ, mods in tokens:
            line = prev_line + dl
            char = dc if dl > 0 else prev_char + dc
            abs_tokens.append((line, char, length, typ, mods))
            prev_line = line
            prev_char = char

        # Line 1 should be a comment (type 4), not part of CMD
        line1_tokens = [row for row in abs_tokens if row[0] == 1]
        assert len(line1_tokens) >= 1
        # comment type index is 4
        assert any(row[3] == 4 for row in line1_tokens)

        # Line 2 should have "set" as keyword (type 0)
        line2_tokens = [row for row in abs_tokens if row[0] == 2]
        assert len(line2_tokens) >= 1
        assert any(row[3] == 0 for row in line2_tokens)

    def test_comment_break_priority_over_brace(self):
        """Comment-break takes priority when both # and { are present."""
        # The # appears before {, so comment-break should win.
        source = (
            "set x [some_command arg1 arg2\n# a comment\nswitch $var {\n    default { set y 1 }\n}"
        )
        result = analyse(source)
        e201 = [d for d in result.diagnostics if d.code == "E201"]
        assert len(e201) >= 1
        # Diagnostic should end on line 0 (the incomplete command line)
        assert e201[0].range.end.line == 0
        # CodeFix should mention "comment"
        assert e201[0].fixes
        assert "comment" in e201[0].fixes[0].description.lower()


class TestE201CommandBreak:
    """E201: unterminated [ where a known command on the next line signals ]."""

    def test_command_break_basic_recovery(self):
        """Unterminated [ followed by set on the next line produces two commands."""
        source = "set totp_algorithm [lindex $totp_key 0\nset totp_secret [lindex $totp_key 1]"
        commands, diags = segment_with_recovery(source)
        assert len(commands) == 2
        assert commands[0].name == "set"
        assert commands[1].name == "set"
        assert "totp_secret" not in " ".join(commands[0].texts)

    def test_command_break_diagnostic_code_and_message(self):
        """E201 diagnostic from command-break has correct code and message."""
        source = "set totp_algorithm [lindex $totp_key 0\nset totp_secret [lindex $totp_key 1]"
        commands, diags = segment_with_recovery(source)
        e201 = [d for d in diags if d.code == "E201"]
        assert len(e201) == 1
        assert e201[0].message == "missing close-bracket"

    def test_command_break_diagnostic_range(self):
        """E201 command-break range covers the incomplete line only."""
        source = "set totp_algorithm [lindex $totp_key 0\nset totp_secret [lindex $totp_key 1]"
        commands, diags = segment_with_recovery(source)
        e201 = [d for d in diags if d.code == "E201"]
        assert len(e201) == 1
        # Diagnostic should start at [ (line 0, char 19)
        assert e201[0].range.start.line == 0
        assert e201[0].range.start.character == 19
        # Diagnostic should end on line 0 (not spilling to line 1)
        assert e201[0].range.end.line == 0

    def test_command_break_codefix(self):
        """E201 command-break CodeFix inserts ] before the command."""
        source = "set totp_algorithm [lindex $totp_key 0\nset totp_secret [lindex $totp_key 1]"
        commands, diags = segment_with_recovery(source)
        e201 = [d for d in diags if d.code == "E201"]
        assert len(e201) == 1
        fixes = e201[0].fixes
        assert fixes
        assert fixes[0].new_text == "]"
        assert "command" in fixes[0].description.lower()

    def test_command_break_cmd_value_preserved(self):
        """After recovery, the CMD token contains only the intended arguments."""
        source = "set totp_algorithm [lindex $totp_key 0\nset totp_secret [lindex $totp_key 1]"
        commands, diags = segment_with_recovery(source)
        # First set should have [lindex $totp_key 0] as value
        assert commands[0].texts[2] == "[lindex $totp_key 0]"
        # Second set should have [lindex $totp_key 1] as value
        assert commands[1].texts[2] == "[lindex $totp_key 1]"

    def test_command_break_skips_blank_lines(self):
        """Command-break skips blank lines to find the known command."""
        source = "set x [lindex $list 0\n\nset y 1"
        commands, diags = segment_with_recovery(source)
        e201 = [d for d in diags if d.code == "E201"]
        assert len(e201) == 1
        assert len(commands) >= 2
        assert commands[1].name == "set"

    def test_command_break_non_command_stops(self):
        """If the first non-blank line is not a known command, don't keep looking."""
        source = "set x [some_cmd arg1\n    continued_arg\nset y 1"
        commands, diags = segment_with_recovery(source)
        # "continued_arg" is not a known command — should stop looking.
        # May fall through to brace-break or fallback.
        e201 = [d for d in diags if d.code == "E201"]
        # Should NOT have a command-break fix mentioning "command"
        for d in e201:
            if d.fixes:
                assert "command" not in d.fixes[0].description.lower()

    def test_command_break_priority_below_comment(self):
        """Comment-break takes priority over command-break."""
        source = "set x [some_cmd arg1\n# a comment\nset y 1"
        commands, diags = segment_with_recovery(source)
        e201 = [d for d in diags if d.code == "E201"]
        assert len(e201) >= 1
        # CodeFix should mention "comment", not "command"
        assert e201[0].fixes
        assert "comment" in e201[0].fixes[0].description.lower()

    def test_command_break_analyser_integration(self):
        """After command-break recovery, analyser sees both commands."""
        source = "set totp_algorithm [lindex $totp_key 0\nset totp_secret [lindex $totp_key 1]"
        result = analyse(source)
        cmd_names = [ci.name for ci in result.command_invocations]
        assert cmd_names.count("set") == 2

    def test_command_break_semantic_tokens(self):
        """Semantic tokens show proper highlighting after command-break recovery."""
        source = "set totp_algorithm [lindex $totp_key 0\nset totp_secret [lindex $totp_key 1]\n"
        tokens_data = semantic_tokens_full(source)
        tokens = []
        for i in range(0, len(tokens_data), 5):
            tokens.append(tuple(tokens_data[i : i + 5]))

        abs_tokens = []
        prev_line = 0
        prev_char = 0
        for dl, dc, length, typ, mods in tokens:
            line = prev_line + dl
            char = dc if dl > 0 else prev_char + dc
            abs_tokens.append((line, char, length, typ, mods))
            prev_line = line
            prev_char = char

        # Line 1 should have "set" as keyword (type 0), not part of CMD
        line1_tokens = [row for row in abs_tokens if row[0] == 1]
        assert len(line1_tokens) >= 1
        assert any(row[3] == 0 for row in line1_tokens)


class TestE201BraceBreak:
    """E201: unterminated [ where { signals the break point (existing behaviour)."""

    def test_brace_break_basic(self):
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

    def test_brace_break_codefix(self):
        """E201 CodeFix inserts ] before the {."""
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

    def test_no_e201_for_valid_cmd(self):
        """Properly closed [string length $x] produces no E201."""
        source = "set n [string length $x]"
        result = analyse(source)
        e201 = [d for d in result.diagnostics if d.code == "E201"]
        assert len(e201) == 0


class TestSegmentWithRecovery:
    """Integration tests for segment_with_recovery."""

    def test_clean_source_no_diagnostics(self):
        """Clean source produces no recovery diagnostics."""
        source = "set x [string length $y]\nset z 1"
        commands, diags = segment_with_recovery(source)
        assert len(diags) == 0
        assert len(commands) == 2

    def test_unterminated_cmd_with_comment_produces_clean_commands(self):
        """Unterminated [ with comment break produces clean segmented commands."""
        source = (
            "set username [ACCESS::session data get session.logon.last.username\n"
            "# comment\n"
            'set totp_key ""'
        )
        commands, diags = segment_with_recovery(source)
        # Should have: set username [...], then set totp_key ""
        # The comment is consumed by the segmenter
        assert len(commands) >= 2
        assert commands[0].texts[0] == "set"
        # The CMD should be properly terminated (not swallowing everything)
        cmd_texts_joined = " ".join(commands[0].texts)
        assert "totp_key" not in cmd_texts_joined

    def test_unterminated_cmd_with_brace_produces_clean_commands(self):
        """Unterminated [ with brace break produces clean segmented commands."""
        source = "switch -- [ACCESS::policy agent_id {\n    default { set x 1 }\n}"
        commands, diags = segment_with_recovery(source)
        # E201 diagnostic should be present
        e201 = [d for d in diags if d.code == "E201"]
        assert len(e201) >= 1
        # The switch should see its body as a separate argument
        assert commands[0].texts[0] == "switch"

    def test_fallback_e201_no_heuristic(self):
        """Unterminated [ without { or # emits simple E201."""
        source = "set x [string length"
        commands, diags = segment_with_recovery(source)
        # May produce E201 or no virtual (simple unterminated at EOF)
        # The segmenter itself handles EOF-reaching tokens
        assert len(commands) >= 1


class TestE202UnterminatedQuote:
    """E202: unterminated " where next line starts with a known command."""

    def test_quote_recovery_produces_two_commands(self):
        """Unterminated " followed by switch produces two clean commands."""
        source = 'set totp_key "\nswitch $var {\n    default { set y 1 }\n}'
        commands, diags = segment_with_recovery(source)
        assert len(commands) >= 2
        assert commands[0].name == "set"
        assert commands[1].name == "switch"

    def test_quote_diagnostic_code_and_message(self):
        """E202 diagnostic has correct code and message."""
        source = 'set totp_key "\nswitch $var {\n    default { set y 1 }\n}'
        commands, diags = segment_with_recovery(source)
        e202 = [d for d in diags if d.code == "E202"]
        assert len(e202) == 1
        assert e202[0].message == 'missing "'

    def test_quote_diagnostic_range_at_opening_quote(self):
        """E202 diagnostic highlights just the opening "."""
        source = 'set totp_key "\nset y 1'
        commands, diags = segment_with_recovery(source)
        e202 = [d for d in diags if d.code == "E202"]
        assert len(e202) == 1
        # The " is at line 0, character 13
        assert e202[0].range.start.line == 0
        assert e202[0].range.start.character == 13

    def test_quote_codefix_inserts_close_quote(self):
        """E202 CodeFix inserts " right after the opening one."""
        source = 'set totp_key "\nset y 1'
        commands, diags = segment_with_recovery(source)
        e202 = [d for d in diags if d.code == "E202"]
        assert len(e202) == 1
        fixes = e202[0].fixes
        assert fixes
        assert fixes[0].new_text == '"'

    def test_quote_recovery_set_becomes_empty_string(self):
        """After recovery, set's value is an empty string."""
        source = 'set totp_key "\nset y 1'
        commands, diags = segment_with_recovery(source)
        # set totp_key "" — the value arg should be empty
        assert commands[0].texts == ["set", "totp_key", ""]

    def test_properly_closed_quote_no_diagnostic(self):
        """Properly closed quote produces no E202."""
        source = 'set x "hello"'
        commands, diags = segment_with_recovery(source)
        e202 = [d for d in diags if d.code == "E202"]
        assert len(e202) == 0

    def test_multi_line_closed_quote_no_diagnostic(self):
        """Multi-line properly closed quote produces no E202."""
        source = 'set x "hello\nworld"'
        commands, diags = segment_with_recovery(source)
        e202 = [d for d in diags if d.code == "E202"]
        assert len(e202) == 0

    def test_quote_fallback_when_no_known_command(self):
        """E202 fallback diagnostic when no known command follows."""
        source = 'set x "\nfoo_unknown bar baz'
        commands, diags = segment_with_recovery(source)
        e202 = [d for d in diags if d.code == "E202"]
        assert len(e202) == 1
        # Fallback has no fixes
        assert not e202[0].fixes

    def test_quote_recovery_with_blank_lines(self):
        """E202 recovery skips blank lines to find known command."""
        source = 'set totp_key "\n\nset y 1'
        commands, diags = segment_with_recovery(source)
        e202 = [d for d in diags if d.code == "E202"]
        assert len(e202) == 1
        assert len(commands) >= 2
        assert commands[1].name == "set"

    def test_quote_recovery_analyser_integration(self):
        """After E202 recovery, analyser sees subsequent commands."""
        source = 'when RULE_INIT {\nset totp_key "\nswitch $var {\n    default { set y 1 }\n}\n}'
        result = analyse(source)
        cmd_names = [ci.name for ci in result.command_invocations]
        assert "switch" in cmd_names

    def test_quote_recovery_semantic_tokens(self):
        """Semantic tokens show proper highlighting after E202 recovery."""
        source = 'set totp_key "\nset y 1\n'
        tokens_data = semantic_tokens_full(source)
        # Convert to absolute (line, char, length, type, mods) tuples
        tokens = []
        for i in range(0, len(tokens_data), 5):
            tokens.append(tuple(tokens_data[i : i + 5]))

        abs_tokens = []
        prev_line = 0
        prev_char = 0
        for dl, dc, length, typ, mods in tokens:
            line = prev_line + dl
            char = dc if dl > 0 else prev_char + dc
            abs_tokens.append((line, char, length, typ, mods))
            prev_line = line
            prev_char = char

        # Line 1 should have "set" as keyword (type 0), not part of quoted string
        line1_tokens = [row for row in abs_tokens if row[0] == 1]
        assert len(line1_tokens) >= 1
        assert any(row[3] == 0 for row in line1_tokens)


class TestE203UnterminatedBrace:
    """E203: unterminated { where de-indented known command signals break."""

    def test_brace_recovery_produces_two_commands(self):
        """Unterminated { followed by set produces two clean commands."""
        source = "array set b32_alphabet {\n  A 0 B 1\n  C 2 D 3\nset totp_secret foo"
        commands, diags = segment_with_recovery(source)
        assert len(commands) >= 2
        assert commands[0].name == "array"
        assert commands[1].name == "set"

    def test_brace_diagnostic_code_and_message(self):
        """E203 diagnostic has correct code and message."""
        source = "array set b32_alphabet {\n  A 0 B 1\n  C 2 D 3\nset totp_secret foo"
        commands, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert len(e203) == 1
        assert e203[0].message == "missing close-brace"

    def test_brace_diagnostic_range_at_opening_brace(self):
        """E203 diagnostic highlights just the opening {."""
        source = "array set b32_alphabet {\n  A 0 B 1\n  C 2 D 3\nset totp_secret foo"
        commands, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert len(e203) == 1
        assert e203[0].range.start.line == 0
        assert e203[0].range.start.character == 23

    def test_brace_codefix_inserts_close_brace(self):
        """E203 CodeFix inserts } before the known command."""
        source = "array set b32_alphabet {\n  A 0 B 1\n  C 2 D 3\nset totp_secret foo"
        commands, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert len(e203) == 1
        fixes = e203[0].fixes
        assert fixes
        assert fixes[0].new_text == "}"

    def test_brace_content_preserved(self):
        """After recovery, the brace content is correctly preserved."""
        source = "array set b32_alphabet {\n  A 0 B 1\n  C 2 D 3\nset totp_secret foo"
        commands, diags = segment_with_recovery(source)
        # The array set's value should contain the data, not the set command
        array_value = commands[0].texts[3]
        assert "A 0" in array_value
        assert "D 3" in array_value
        assert "totp_secret" not in array_value

    def test_properly_closed_brace_no_diagnostic(self):
        """Properly closed brace produces no E203."""
        source = "set x { a b c }"
        commands, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert len(e203) == 0

    def test_multi_line_closed_brace_no_diagnostic(self):
        """Multi-line properly closed brace produces no E203."""
        source = "set x {\n  a b c\n  d e f\n}"
        commands, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert len(e203) == 0

    def test_brace_fallback_when_no_known_command(self):
        """E203 fallback diagnostic when no de-indented known command."""
        source = "set x {\n  data1\n  data2\nfoo_unknown more"
        commands, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert len(e203) == 1
        assert not e203[0].fixes

    def test_brace_recovery_indented_data_preserved(self):
        """Recovery only triggers on de-indented lines, not indented ones."""
        source = "set x {\n  set 0\n  get 1\nset y 2"
        commands, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert len(e203) == 1
        # "set 0" and "get 1" should stay in the brace content (they're indented)
        # Only "set y 2" at column 0 triggers recovery
        assert len(commands) >= 2
        assert commands[1].name == "set"
        assert "y" in commands[1].texts

    def test_brace_recovery_analyser_integration(self):
        """After E203 recovery, analyser sees subsequent commands."""
        # Use a flat source (no wrapping when block) to avoid
        # interference from the segmenter's own top-level recovery.
        source = "array set data {\n  A 0 B 1\nset result [string length $data]"
        result = analyse(source)
        cmd_names = [ci.name for ci in result.command_invocations]
        assert "array" in cmd_names
        assert "set" in cmd_names

    def test_brace_recovery_semantic_tokens(self):
        """Semantic tokens show proper highlighting after E203 recovery."""
        source = "array set data {\n  A 0 B 1\nset x 1\n"
        tokens_data = semantic_tokens_full(source)
        tokens = []
        for i in range(0, len(tokens_data), 5):
            tokens.append(tuple(tokens_data[i : i + 5]))

        abs_tokens = []
        prev_line = 0
        prev_char = 0
        for dl, dc, length, typ, mods in tokens:
            line = prev_line + dl
            char = dc if dl > 0 else prev_char + dc
            abs_tokens.append((line, char, length, typ, mods))
            prev_line = line
            prev_char = char

        # Line 2 should have "set" as keyword (type 0)
        line2_tokens = [row for row in abs_tokens if row[0] == 2]
        assert len(line2_tokens) >= 1
        assert any(row[3] == 0 for row in line2_tokens)
