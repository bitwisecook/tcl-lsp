"""Unit tests for ``core.analysis.auto_path_eval.evaluate_auto_path_expr``.

The mini-evaluator is consulted by cross-file indexing / package
discovery for every ``lappend auto_path …`` call. The Codex review
on PR #214 (file ``core/analysis/auto_path_eval.py:49``) flagged
that the parser/evaluator was non-trivially complex and lacked
unit-test coverage. This file pins the supported idioms and
every documented failure mode so a regression in tokenisation
or AST evaluation is caught early.

Coverage:

* literal forms — bare path, brace-quoted path, double-quoted path
* ``[info script]`` substitution (with / without ``info_script``)
* ``[file dirname …]`` (single-level + nested + over an absolute path)
* ``[file join …]`` (two args + many args + with ``..``)
* combination — ``[file join [file dirname [info script]] ..]``
* ``~`` expansion to ``$HOME``
* failure modes — unbalanced ``{`` / ``[``, unterminated ``"``,
  ``$var`` substitutions, ``$env(…)``, unsupported commands
  (``linsert``, ``lsort``, ``set``, …), bad arity for ``info`` /
  ``file``, empty input, whitespace-only input.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.auto_path_eval import evaluate_auto_path_expr


class TestLiterals:
    def test_bare_absolute_path(self):
        assert evaluate_auto_path_expr("/abs/path", None) == os.path.abspath("/abs/path")

    def test_brace_quoted_path(self):
        # Brace-quoted words preserve internal spaces.
        result = evaluate_auto_path_expr("{/abs/path with spaces}", None)
        assert result == os.path.abspath("/abs/path with spaces")

    def test_double_quoted_path(self):
        result = evaluate_auto_path_expr('"/abs/path"', None)
        assert result == os.path.abspath("/abs/path")

    def test_relative_path_resolved_to_absolute(self):
        # Relative paths get cwd-prefixed by ``os.path.abspath``.
        result = evaluate_auto_path_expr("rellib", None)
        assert result is not None
        assert os.path.isabs(result)
        assert result.endswith("rellib")

    def test_empty_input_returns_none(self):
        assert evaluate_auto_path_expr("", None) is None

    def test_whitespace_only_input_returns_none(self):
        assert evaluate_auto_path_expr("   \t\n", None) is None


class TestTildeExpansion:
    def test_tilde_expands_to_home(self):
        # ``~/lib`` expands to ``$HOME/lib`` then absolutifies.
        result = evaluate_auto_path_expr("~/lib", None)
        assert result == os.path.abspath(os.path.expanduser("~/lib"))

    def test_tilde_inside_braces_also_expands(self):
        # Brace-quoting strips the braces but keeps ``~`` for
        # ``os.path.expanduser`` to handle.
        result = evaluate_auto_path_expr("{~/lib}", None)
        assert result == os.path.abspath(os.path.expanduser("~/lib"))


class TestInfoScript:
    def test_info_script_resolves_to_supplied_path(self):
        result = evaluate_auto_path_expr(
            "[info script]",
            info_script="/proj/main.tcl",
        )
        assert result == os.path.abspath("/proj/main.tcl")

    def test_info_script_without_info_script_returns_none(self):
        # If the caller doesn't know the script path, we can't
        # evaluate ``[info script]``.
        assert evaluate_auto_path_expr("[info script]", None) is None

    def test_info_unknown_subcommand_returns_none(self):
        # Only ``[info script]`` is supported.
        assert evaluate_auto_path_expr("[info hostname]", "/x.tcl") is None

    def test_info_with_extra_args_returns_none(self):
        assert evaluate_auto_path_expr("[info script extra]", "/x.tcl") is None


class TestFileDirname:
    def test_single_level(self):
        result = evaluate_auto_path_expr(
            "[file dirname [info script]]",
            info_script="/proj/sub/main.tcl",
        )
        assert result == os.path.abspath("/proj/sub")

    def test_two_levels_nested(self):
        result = evaluate_auto_path_expr(
            "[file dirname [file dirname [info script]]]",
            info_script="/proj/sub/main.tcl",
        )
        assert result == os.path.abspath("/proj")

    def test_dirname_over_literal_path(self):
        result = evaluate_auto_path_expr(
            "[file dirname /opt/app/main.tcl]",
            info_script=None,
        )
        assert result == os.path.abspath("/opt/app")

    def test_dirname_arity_zero_returns_none(self):
        assert evaluate_auto_path_expr("[file dirname]", "/x.tcl") is None


class TestFileJoin:
    def test_two_literal_parts(self):
        result = evaluate_auto_path_expr(
            "[file join /opt lib]",
            info_script=None,
        )
        assert result == os.path.abspath("/opt/lib")

    def test_many_parts(self):
        result = evaluate_auto_path_expr(
            "[file join /opt sub a b c]",
            info_script=None,
        )
        assert result == os.path.abspath("/opt/sub/a/b/c")

    def test_join_with_dot_dot(self):
        # Idiomatic shape — ``..`` walks up one level.
        result = evaluate_auto_path_expr(
            "[file join /proj/sub ..]",
            info_script=None,
        )
        # ``os.path.join`` followed by ``os.path.abspath`` collapses
        # the ``..`` into the parent dir.
        assert result == os.path.abspath("/proj/sub/..")
        assert result == os.path.abspath("/proj")

    def test_join_zero_args_returns_none(self):
        # ``[file join]`` is treated as malformed by the validator
        # (``len(args) >= 2`` requires the subcommand + at least
        # one path component).
        assert evaluate_auto_path_expr("[file join]", None) is None


class TestCombinedShape:
    def test_canonical_idiom(self):
        # The canonical Tcl idiom for "go to my parent directory":
        # ``lappend auto_path [file join [file dirname [info script]] ..]``.
        result = evaluate_auto_path_expr(
            "[file join [file dirname [info script]] ..]",
            info_script="/proj/sub/main.tcl",
        )
        assert result == os.path.abspath("/proj")

    def test_double_dirname_then_join(self):
        # ``[file join [file dirname [file dirname [info script]]] lib]``
        # — go up two levels then into a sibling ``lib`` dir.
        result = evaluate_auto_path_expr(
            "[file join [file dirname [file dirname [info script]]] lib]",
            info_script="/proj/sub/main.tcl",
        )
        # ``/proj/sub/main.tcl`` → dirname → ``/proj/sub`` → dirname
        # → ``/proj`` → joined with ``lib`` → ``/proj/lib``.
        assert result == os.path.abspath("/proj/lib")


class TestUnsupportedCommands:
    def test_set_command_returns_none(self):
        assert evaluate_auto_path_expr("[set foo]", None) is None

    def test_linsert_command_returns_none(self):
        assert evaluate_auto_path_expr("[linsert {a b} 0 x]", None) is None

    def test_lsort_command_returns_none(self):
        assert evaluate_auto_path_expr("[lsort {a b c}]", None) is None

    def test_eval_command_returns_none(self):
        assert evaluate_auto_path_expr("[eval {/abs/path}]", None) is None


class TestVariableSubstitutions:
    def test_dollar_variable_returns_none(self):
        assert evaluate_auto_path_expr("$auto_path", None) is None

    def test_env_array_returns_none(self):
        assert evaluate_auto_path_expr("$env(HOME)", None) is None

    def test_braced_dollar_returns_none(self):
        assert evaluate_auto_path_expr('"${auto_path}"', None) is None

    def test_dollar_inside_path_returns_none(self):
        # Even a literal-looking word is rejected if it embeds a
        # ``$`` — we never guess.
        assert evaluate_auto_path_expr("/opt/$user/lib", None) is None


class TestMalformedInput:
    def test_unbalanced_open_brace(self):
        assert evaluate_auto_path_expr("{unterminated", None) is None

    def test_unbalanced_open_bracket(self):
        assert evaluate_auto_path_expr("[file dirname /x", None) is None

    def test_unterminated_double_quote(self):
        assert evaluate_auto_path_expr('"unterminated', None) is None

    def test_stray_close_bracket(self):
        # A bare ``]`` with no matching ``[`` is rejected.
        assert evaluate_auto_path_expr("]", None) is None

    def test_extra_words_after_first(self):
        # The evaluator must consume the entire input. Trailing
        # words after the first parsed expression should fail.
        assert evaluate_auto_path_expr("/abs/path extra", None) is None

    def test_command_substitution_without_name(self):
        # ``[]`` — no command name to dispatch on.
        assert evaluate_auto_path_expr("[]", None) is None


class TestNestedBraces:
    def test_nested_braces_kept_as_single_word(self):
        # Brace nesting inside a brace word is balanced and the
        # whole content is one literal.
        result = evaluate_auto_path_expr("{/path/with/{nested}/braces}", None)
        assert result == os.path.abspath("/path/with/{nested}/braces")
