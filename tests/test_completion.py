"""White-box completion internals with no JSON-RPC surface.

The completion provider's request/response behaviour is exercised end-to-end
against the packaged server in ``tests/lsp_e2e/test_completion_e2e.py`` (and the
F5 iRules argument/keyword cases in ``tests/lsp_e2e/test_irules_e2e.py``).

What remains here cannot be expressed over the wire:

* ``_var_needs_braces`` / ``split_array_name`` — pure helpers.
* the ``analysis=`` snapshot-redirect path (``copy_for_snapshot``).
* ranking driven by the ``workspace_procs`` / ``workspace_command_usage`` /
  ``workspace_proc_usage`` arguments, which the server computes from its
  workspace index rather than from request parameters.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.completion import get_completions


class TestVariableHelpers:
    def test_var_needs_braces_unicode_aware(self):
        """The lexer's bare-var rule uses ``ch.isalnum()`` (Unicode), so
        non-ASCII names like ``ñame`` lex as a single bare token.  An
        ASCII-only regex would force ``${...}`` unnecessarily."""
        from server.features.completion import _var_needs_braces

        for name in ["foo", "foo_bar", "::foo", "::ns::foo", "ñame", "café", "__中文__"]:
            assert not _var_needs_braces(name), f"{name!r} should not need braces"
        for name in ["foo-bar", "foo bar", "foo.bar", "", "::", "a::"]:
            assert _var_needs_braces(name), f"{name!r} should need braces"

    def test_split_array_name_does_not_misclassify_brace_then_paren(self):
        """``${arr}(foo)`` is parsed by Tcl as scalar ``${arr}`` followed
        by literal characters ``(foo)`` -- not as an array reference."""
        from shared.naming import split_array_name

        assert split_array_name("${arr}(foo)") == ("arr", None)
        assert split_array_name("${arr}") == ("arr", None)
        assert split_array_name("${arr(foo)}") == ("arr", "foo")
        assert split_array_name("$arr(foo)") == ("arr", "foo")
        assert split_array_name("arr(foo)") == ("arr", "foo")
        assert split_array_name("arr") == ("arr", None)


class TestAnalysisSnapshotRedirect:
    def test_dollar_completion_uplevel_zero_survives_analysis_copy(self):
        """The ``uplevel #0`` synthetic scope must round-trip through
        ``AnalysisResult.copy_for_snapshot`` -- the scope tree is copied
        recursively and the parent pointer is rewritten to the new lexical
        parent, so the redirect-to-global behaviour can't depend on a parent
        pointer that points outside the local subtree."""
        from analyser import analyse

        source = textwrap.dedent("""\
            set ::globalvar 9
            proc p {} {
                set local_var 1
                uplevel #0 {
                    set inside_var 99
                    puts $
                }
            }
        """)
        snapshot = analyse(source).copy_for_snapshot()
        labels = [i.label for i in get_completions(source, 5, 18, analysis=snapshot)]
        assert "$inside_var" in labels
        assert "$::globalvar" in labels
        assert "$local_var" not in labels


class TestWorkspaceProcs:
    def test_workspace_procs_included(self):
        items = get_completions("", 0, 0, workspace_procs=["::utils::helper"])
        assert "helper" in [i.label for i in items]


class TestCompletionRanking:
    def test_workspace_command_usage_boosts_builtin_ranking(self):
        items = get_completions("", 0, 0, workspace_command_usage={"set": 25, "puts": 1})
        by_label = {item.label: item for item in items}
        set_sort = by_label["set"].sort_text
        puts_sort = by_label["puts"].sort_text
        assert set_sort is not None and puts_sort is not None
        assert set_sort < puts_sort

    def test_workspace_proc_usage_boosts_local_proc_ranking(self):
        source = "proc alpha {} { return 1 }\nproc beta {} { return 2 }\n\n"
        items = get_completions(source, 2, 0, workspace_proc_usage={"::beta": 12, "::alpha": 1})
        by_label = {item.label: item for item in items}
        beta_sort = by_label["beta"].sort_text
        alpha_sort = by_label["alpha"].sort_text
        assert beta_sort is not None and alpha_sort is not None
        assert beta_sort < alpha_sort

    def test_workspace_proc_usage_boosts_workspace_proc_ranking(self):
        items = get_completions(
            "",
            0,
            0,
            workspace_procs=["::pkg::slow_helper", "::pkg::fast_helper"],
            workspace_proc_usage={"::pkg::fast_helper": 9, "::pkg::slow_helper": 1},
        )
        by_label = {item.label: item for item in items}
        fast_sort = by_label["fast_helper"].sort_text
        slow_sort = by_label["slow_helper"].sort_text
        assert fast_sort is not None and slow_sort is not None
        assert fast_sort < slow_sort
