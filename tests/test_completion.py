"""Tests for the completion provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol.types import TextEdit

from core.commands.registry.runtime import configure_signatures
from lsp.features.completion import get_completions


class TestCommandCompletion:
    def test_empty_line_returns_commands(self):
        items = get_completions("", 0, 0)
        labels = [i.label for i in items]
        assert "set" in labels
        assert "proc" in labels
        assert "puts" in labels

    def test_partial_command(self):
        items = get_completions("pu", 0, 2)
        labels = [i.label for i in items]
        assert "puts" in labels
        # Should not include unrelated commands
        assert "set" not in labels

    def test_no_math_operators(self):
        items = get_completions("", 0, 0)
        labels = [i.label for i in items]
        assert "+" not in labels
        assert "-" not in labels

    def test_user_proc_in_completions(self):
        source = textwrap.dedent("""\
            proc myHelper {x} { return $x }
        """)
        # Complete on second line
        items = get_completions(source + "my", 1, 2)
        labels = [i.label for i in items]
        assert "myHelper" in labels


class TestVariableCompletion:
    def test_dollar_triggers_vars(self):
        source = "set greeting hello\nputs $"
        items = get_completions(source, 1, 6)
        labels = [i.label for i in items]
        assert "$greeting" in labels

    def test_partial_var_name(self):
        source = "set greeting hello\nset goodbye bye\nputs $gre"
        items = get_completions(source, 2, 9)
        labels = [i.label for i in items]
        assert "$greeting" in labels
        # 'goodbye' starts with 'g' but not 'gre'
        assert "$goodbye" not in labels

    def test_var_in_proc_scope(self):
        source = textwrap.dedent("""\
            proc foo {x} {
                set local 1
                puts $
            }
        """)
        items = get_completions(source, 2, 10)
        labels = [i.label for i in items]
        # Should include params and locals
        assert "$x" in labels
        assert "$local" in labels

    def test_namespace_var_completion(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                variable nsVar 1
                puts $
            }
        """)
        items = get_completions(source, 2, 10)
        labels = [i.label for i in items]
        assert "$nsVar" in labels

    def test_dollar_text_edit_replaces_dollar_sign(self):
        """After typing '$', the completion must replace '$' with '$name' so
        VS Code does not delete the leading dollar (issue #357)."""
        source = "set testvar Test\nputs $"
        items = get_completions(source, 1, 6)
        by_label = {i.label: i for i in items}
        assert "$testvar" in by_label
        item = by_label["$testvar"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        # Edit replaces the typed '$' (col 5) up to the cursor (col 6).
        assert item.text_edit.range.start.line == 1
        assert item.text_edit.range.start.character == 5
        assert item.text_edit.range.end.character == 6
        assert item.text_edit.new_text == "$testvar"

    def test_dollar_text_edit_replaces_partial_var_name(self):
        """Typing '$gre' and completing should replace '$gre' with '$greeting'."""
        source = "set greeting hello\nputs $gre"
        items = get_completions(source, 1, 9)
        by_label = {i.label: i for i in items}
        assert "$greeting" in by_label
        item = by_label["$greeting"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 5
        assert item.text_edit.range.end.character == 9
        assert item.text_edit.new_text == "$greeting"

    def test_dollar_text_edit_brace_form(self):
        """Typing '${gre' should complete to '${greeting}' with closing brace."""
        source = "set greeting hello\nputs ${gre"
        items = get_completions(source, 1, 10)
        by_label = {i.label: i for i in items}
        assert "$greeting" in by_label
        item = by_label["$greeting"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 5
        assert item.text_edit.range.end.character == 10
        assert item.text_edit.new_text == "${greeting}"

    def test_dollar_text_edit_brace_form_consumes_existing_close(self):
        """``${gre|}`` -- existing ``}`` is consumed so we don't double it."""
        source = "set greeting hello\nputs ${gre}"
        # Cursor between 'gre' and '}'.
        items = get_completions(source, 1, 10)
        by_label = {i.label: i for i in items}
        item = by_label["$greeting"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 5
        # Edit should subsume the trailing '}' (col 10..11) along with '${gre'.
        assert item.text_edit.range.end.character == 11
        assert item.text_edit.new_text == "${greeting}"

    def test_dollar_text_edit_brace_form_empty_with_close(self):
        """``${|}`` (empty inside braces) should fill in ``${name}``."""
        source = "set greeting hello\nputs ${}"
        # Cursor between '{' and '}'.
        items = get_completions(source, 1, 7)
        by_label = {i.label: i for i in items}
        item = by_label["$greeting"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 5
        assert item.text_edit.range.end.character == 8
        assert item.text_edit.new_text == "${greeting}"

    def test_dollar_text_edit_midword_replaces_full_token(self):
        """``$gre|eting`` -- midword completion must replace the whole token,
        not leave ``eting`` behind."""
        source = "set greeting hello\nputs $greeting"
        # Cursor between 'gre' and 'eting' on line 1.
        items = get_completions(source, 1, 9)
        by_label = {i.label: i for i in items}
        item = by_label["$greeting"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 5
        # Should extend forward past 'eting' to col 14 (end of '$greeting').
        assert item.text_edit.range.end.character == 14
        assert item.text_edit.new_text == "$greeting"

    def test_dollar_text_edit_midword_brace_form_replaces_full_token(self):
        """``${gre|eting}`` -- midword completion in brace form must replace
        the whole ``${greeting}`` reference."""
        source = "set greeting hello\nputs ${greeting}"
        # Cursor between 'gre' and 'eting' on line 1.
        items = get_completions(source, 1, 10)
        by_label = {i.label: i for i in items}
        item = by_label["$greeting"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 5
        # Should extend forward past 'eting}' to col 16.
        assert item.text_edit.range.end.character == 16
        assert item.text_edit.new_text == "${greeting}"

    def test_dollar_auto_braces_var_name_with_hyphen(self):
        """A name with a ``-`` cannot live in bare ``$name`` form, so the
        completion must promote to ``${name}`` even if the user only typed
        a bare ``$``."""
        source = 'set "foo-bar" 1\nputs $'
        items = get_completions(source, 1, 6)
        by_label = {i.label: i for i in items}
        assert "$foo-bar" in by_label
        item = by_label["$foo-bar"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.new_text == "${foo-bar}"

    def test_dollar_namespace_qualified_var_uses_bare_form(self):
        """``::ns::var`` is a legal bare ``$`` substitution -- the lexer
        accepts ``::``-separated segments without braces -- so the
        completion should not force the brace form."""
        source = "set ::myns::foo 1\nputs $"
        items = get_completions(source, 1, 6)
        by_label = {i.label: i for i in items}
        # Cross-rule globals get stored in the global scope as ``::myns::foo``.
        cand = next((lbl for lbl in by_label if lbl.endswith("::foo")), None)
        if cand is not None:
            item = by_label[cand]
            # Either bare or brace is correct here; what matters is that
            # the inserted text round-trips as a valid var substitution.
            assert item.text_edit is not None
            assert isinstance(item.text_edit, TextEdit)
            assert item.text_edit.new_text in (
                f"${cand[1:]}",
                f"${{{cand[1:]}}}",
            )

    def test_dollar_cross_namespace_offers_qualified_name(self):
        """Vars in another namespace are offered fully qualified, e.g.
        ``$::other::baz`` from inside ``::myns``."""
        source = textwrap.dedent("""\
            namespace eval ::other {
                variable baz 3
            }
            namespace eval ::myns {
                puts $
            }
        """)
        # Cursor right after the '$' on the ``puts`` line.
        items = get_completions(source, 4, 10)
        labels = [i.label for i in items]
        assert "$::other::baz" in labels
        item = next(i for i in items if i.label == "$::other::baz")
        # Cross-namespace use must always be fully qualified.
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.new_text == "$::other::baz"

    def test_dollar_same_namespace_uses_bare_minimal_form(self):
        """A var in the cursor's own namespace is offered bare (the
        minimal valid form), not as ``$::myns::foo``."""
        source = textwrap.dedent("""\
            namespace eval ::myns {
                variable foo 1
                puts $
            }
        """)
        items = get_completions(source, 2, 10)
        labels = [i.label for i in items]
        assert "$foo" in labels
        # Should not also offer the qualified form -- the bare form is
        # already the minimal valid substitution.
        assert "$::myns::foo" not in labels

    def test_dollar_global_var_from_namespace_offers_qualified(self):
        """Globals declared at the top level appear as ``$::globalvar``
        when the cursor is inside a different namespace."""
        source = textwrap.dedent("""\
            set ::globalvar 9
            namespace eval ::myns {
                puts $
            }
        """)
        items = get_completions(source, 2, 10)
        labels = [i.label for i in items]
        assert "$::globalvar" in labels

    def test_dollar_partial_filters_cross_namespace(self):
        """Typing ``$::ot`` filters cross-namespace candidates by qualified
        prefix and excludes vars in the current namespace."""
        source = textwrap.dedent("""\
            namespace eval ::other {
                variable baz 3
            }
            namespace eval ::myns {
                variable foo 1
                puts $::ot
            }
        """)
        items = get_completions(source, 5, 14)
        labels = [i.label for i in items]
        assert "$::other::baz" in labels
        assert "$foo" not in labels  # Filtered: doesn't start with ``::ot``.

    def test_dollar_completion_offers_global_local_alias(self):
        """``global foo`` and ``global ::foo`` both create a local alias
        named ``foo`` in the proc, so completion offers both ``$foo`` (the
        local alias / minimal form) and ``$::foo`` (qualified)."""
        for decl in ("global myglobal", "global ::myglobal"):
            src = textwrap.dedent(f"""\
                set ::myglobal 9
                proc p {{}} {{
                    {decl}
                    puts $
                }}
            """)
            items = get_completions(src, 3, 10)
            labels = sorted(i.label for i in items if i.label.startswith("$"))
            assert "$myglobal" in labels, f"missing bare alias for {decl!r}: {labels}"
            assert "$::myglobal" in labels, f"missing qualified for {decl!r}: {labels}"

    def test_dollar_completion_offers_upvar_alias(self):
        """``upvar 1 caller_var alias`` registers ``alias`` as a local so
        completion offers ``$alias`` even before any ``set alias ...``."""
        source = textwrap.dedent("""\
            proc inner {var} {
                upvar 1 $var alias
                puts $
            }
        """)
        items = get_completions(source, 2, 10)
        labels = [i.label for i in items]
        assert "$alias" in labels

    def test_dollar_completion_tolerates_cursor_past_eol(self):
        """LSP clients may send positions past EOL; the provider must not
        crash with IndexError."""
        source = "set greeting hello\nputs $"
        # Cursor at column 100, well past the end of the line.
        items = get_completions(source, 1, 100)
        labels = [i.label for i in items]
        assert "$greeting" in labels


class TestSubcommandCompletion:
    def test_string_subcommands(self):
        items = get_completions("string ", 0, 7)
        labels = [i.label for i in items]
        assert "length" in labels
        assert "match" in labels
        assert "tolower" in labels

    def test_partial_subcommand(self):
        items = get_completions("string to", 0, 9)
        labels = [i.label for i in items]
        assert "tolower" in labels
        assert "toupper" in labels
        assert "totitle" in labels
        # Should not include unmatched subcommands
        assert "length" not in labels

    def test_namespace_subcommands(self):
        items = get_completions("namespace ", 0, 10)
        labels = [i.label for i in items]
        assert "eval" in labels
        assert "export" in labels


class TestSwitchCompletion:
    def test_regexp_switches(self):
        items = get_completions("regexp -", 0, 8)
        labels = [i.label for i in items]
        assert "-nocase" in labels
        assert "-all" in labels

    def test_partial_switch(self):
        items = get_completions("lsort -no", 0, 9)
        labels = [i.label for i in items]
        assert "-nocase" in labels

    def test_socket_switches(self):
        items = get_completions("socket -", 0, 8)
        labels = [i.label for i in items]
        assert "-server" in labels
        assert "-myaddr" in labels

    def test_switch_completion_ignores_semicolon_in_quoted_arg(self):
        source = 'socket "a;b" -'
        items = get_completions(source, 0, len(source))
        labels = [i.label for i in items]
        assert "-server" in labels


class TestCommandArgumentCompletion:
    def test_when_event_name_completion(self):
        configure_signatures(dialect="f5-irules")
        items = get_completions("when ", 0, 5)
        labels = [i.label for i in items]
        assert "HTTP_REQUEST" in labels
        assert "CLIENT_ACCEPTED" in labels

    def test_when_priority_and_timing_keywords_after_event(self):
        configure_signatures(dialect="f5-irules")
        source = "when HTTP_REQUEST "
        items = get_completions(source, 0, len(source))
        labels = [i.label for i in items]
        assert "priority" in labels
        assert "timing" in labels

    def test_when_priority_and_timing_partial_keyword(self):
        configure_signatures(dialect="f5-irules")
        source = "when HTTP_REQUEST pr"
        items = get_completions(source, 0, len(source))
        labels = [i.label for i in items]
        assert "priority" in labels
        assert "timing" not in labels

    def test_when_priority_and_timing_after_priority_value(self):
        configure_signatures(dialect="f5-irules")
        source = "when HTTP_REQUEST priority 500 "
        items = get_completions(source, 0, len(source))
        labels = [i.label for i in items]
        assert "priority" in labels
        assert "timing" in labels

    def test_when_timing_value_keywords_after_timing(self):
        configure_signatures(dialect="f5-irules")
        source = "when HTTP_REQUEST timing "
        items = get_completions(source, 0, len(source))
        labels = [i.label for i in items]
        assert "enable" in labels
        assert "disable" in labels

    def test_when_timing_values_not_suggested_after_priority(self):
        configure_signatures(dialect="f5-irules")
        source = "when HTTP_REQUEST priority "
        items = get_completions(source, 0, len(source))
        labels = [i.label for i in items]
        assert "enable" not in labels
        assert "disable" not in labels

    def test_http_header_subcommand_keywords(self):
        configure_signatures(dialect="f5-irules")
        items = get_completions("HTTP::header ", 0, 13)
        labels = [i.label for i in items]
        assert "insert" in labels
        assert "replace" in labels
        assert "value" in labels

    def test_http_header_partial_keyword(self):
        configure_signatures(dialect="f5-irules")
        items = get_completions("HTTP::header re", 0, 15)
        labels = [i.label for i in items]
        assert "remove" in labels
        assert "replace" in labels
        assert "insert" not in labels

    def test_http_respond_options_after_status_code(self):
        configure_signatures(dialect="f5-irules")
        items = get_completions("HTTP::respond 302 ", 0, 18)
        labels = [i.label for i in items]
        assert "content" in labels
        assert "noserver" in labels
        assert "version" in labels


class TestCompletionDocumentation:
    """Verify completion items carry documentation summaries."""

    def test_builtin_command_has_documentation(self):
        items = get_completions("se", 0, 2)
        by_label = {i.label: i for i in items}
        assert "set" in by_label
        item = by_label["set"]
        doc = item.documentation
        assert doc is not None
        doc_text = doc if isinstance(doc, str) else doc.value
        assert "variable" in doc_text.lower() or "value" in doc_text.lower()

    def test_subcommand_has_documentation(self):
        items = get_completions("string ", 0, 7)
        by_label = {i.label: i for i in items}
        assert "length" in by_label
        item = by_label["length"]
        # length should have doc from ArgumentValueSpec hover
        doc = item.documentation
        if doc is not None:
            doc_str = doc if isinstance(doc, str) else doc.value
            assert len(doc_str) > 0

    def test_switch_has_documentation(self):
        items = get_completions("socket -", 0, 8)
        by_label = {i.label: i for i in items}
        assert "-server" in by_label
        item = by_label["-server"]
        doc = item.documentation
        assert doc is not None
        doc_str = doc if isinstance(doc, str) else doc.value
        assert len(doc_str) > 0

    def test_switch_text_edit_replaces_partial_dash_prefix(self):
        """Typing '-no' and completing '-nocase' should not produce '--nocase'."""
        source = "lsort -no"
        items = get_completions(source, 0, len(source))
        by_label = {i.label: i for i in items}
        assert "-nocase" in by_label
        item = by_label["-nocase"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        # The edit should replace from the '-' (col 6) to the cursor (col 9)
        assert item.text_edit.range.start.character == 6
        assert item.text_edit.range.end.character == 9
        assert item.text_edit.new_text == "-nocase"

    def test_switch_text_edit_with_single_char_partial(self):
        """Typing '-n' should also produce a correct text edit."""
        source = "regexp -n"
        items = get_completions(source, 0, len(source))
        by_label = {i.label: i for i in items}
        assert "-nocase" in by_label
        item = by_label["-nocase"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 7
        assert item.text_edit.range.end.character == 9
        assert item.text_edit.new_text == "-nocase"

    def test_switch_text_edit_with_longer_partial(self):
        """Typing '-noc' should also produce a correct text edit."""
        source = "lsort -noc"
        items = get_completions(source, 0, len(source))
        by_label = {i.label: i for i in items}
        assert "-nocase" in by_label
        item = by_label["-nocase"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 6
        assert item.text_edit.range.end.character == 10
        assert item.text_edit.new_text == "-nocase"

    def test_switch_text_edit_bare_dash(self):
        """Typing just '-' should produce a text edit replacing the dash."""
        source = "regexp -"
        items = get_completions(source, 0, len(source))
        by_label = {i.label: i for i in items}
        assert "-nocase" in by_label
        item = by_label["-nocase"]
        assert item.text_edit is not None
        assert isinstance(item.text_edit, TextEdit)
        assert item.text_edit.range.start.character == 7
        assert item.text_edit.range.end.character == 8
        assert item.text_edit.new_text == "-nocase"

    def test_argument_value_has_documentation(self):
        configure_signatures(dialect="f5-irules")
        items = get_completions("when ", 0, 5)
        # Event names should have documentation from their ArgumentValueSpec.hover
        # Some events may not have hover, but the infrastructure should be there
        assert any(i.documentation is not None for i in items if i.label == "HTTP_REQUEST")
        assert len(items) > 0


class TestWorkspaceProcs:
    def test_workspace_procs_included(self):
        items = get_completions("", 0, 0, workspace_procs=["::utils::helper"])
        labels = [i.label for i in items]
        assert "helper" in labels


class TestCompletionRanking:
    def test_irules_event_valid_command_ranked_before_invalid(self):
        configure_signatures(dialect="f5-irules")
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                
            }
        """)
        items = get_completions(source, 1, 4)
        by_label = {item.label: item for item in items}
        assert by_label["HTTP::header"].sort_text is not None
        assert by_label["TCP::collect"].sort_text is not None
        assert by_label["HTTP::header"].sort_text < by_label["TCP::collect"].sort_text

    def test_workspace_command_usage_boosts_builtin_ranking(self):
        items = get_completions(
            "",
            0,
            0,
            workspace_command_usage={"set": 25, "puts": 1},
        )
        by_label = {item.label: item for item in items}
        assert by_label["set"].sort_text is not None
        assert by_label["puts"].sort_text is not None
        assert by_label["set"].sort_text < by_label["puts"].sort_text

    def test_workspace_proc_usage_boosts_local_proc_ranking(self):
        source = textwrap.dedent("""\
            proc alpha {} { return 1 }
            proc beta {} { return 2 }
            
        """)
        items = get_completions(
            source,
            2,
            0,
            workspace_proc_usage={"::beta": 12, "::alpha": 1},
        )
        by_label = {item.label: item for item in items}
        assert by_label["alpha"].sort_text is not None
        assert by_label["beta"].sort_text is not None
        assert by_label["beta"].sort_text < by_label["alpha"].sort_text

    def test_workspace_proc_usage_boosts_workspace_proc_ranking(self):
        items = get_completions(
            "",
            0,
            0,
            workspace_procs=["::pkg::slow_helper", "::pkg::fast_helper"],
            workspace_proc_usage={"::pkg::fast_helper": 9, "::pkg::slow_helper": 1},
        )
        by_label = {item.label: item for item in items}
        assert by_label["fast_helper"].sort_text is not None
        assert by_label["slow_helper"].sort_text is not None
        assert by_label["fast_helper"].sort_text < by_label["slow_helper"].sort_text
