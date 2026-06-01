"""Tests for shared.tcl_list — the canonical Tcl word/list-element quoter."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from shared.tcl_list import TclListError, tcl_list_join, tcl_list_quote, tcl_list_split


class TestTclListQuote:
    @pytest.mark.parametrize(
        ("value", "expected"),
        [
            ("", "{}"),
            ("simple", "simple"),
            ("foo/bar:baz-1.5", "foo/bar:baz-1.5"),
            ("Hello World", "{Hello World}"),
            ("a $b [c]", "{a $b [c]}"),
            ('say "hi"', '{say "hi"}'),
        ],
    )
    def test_known_renderings(self, value: str, expected: str) -> None:
        assert tcl_list_quote(value, first=False) == expected

    @pytest.mark.parametrize(
        "value",
        [
            "",
            "simple",
            "Hello World",
            "a $b [c]",
            'has "quotes"',
            "semi;colon",
            "unbalanced{brace",
            "trailing\\backslash\\",
            "newline\nhere",
            "#leading-hash",
        ],
    )
    def test_matches_tcl_list_builtin(self, value: str) -> None:
        """Our quoting must match Tcl's own ``list`` single-element output.

        ``list`` builds the canonical string rep of a one-element list, which
        is exactly a single list element — Tcl's reference for ``first=True``
        (a leading ``#`` is quoted, matching position-0 of ``UpdateStringOfList``).
        The value is passed via the environment so no Tcl word parsing mangles
        it on the way in.
        """
        tclsh = shutil.which("tclsh9.0") or shutil.which("tclsh8.6")
        if tclsh is None:
            pytest.skip("no tclsh on PATH")
        out = subprocess.run(
            [tclsh],
            input="puts -nonewline [list $env(V)]",
            capture_output=True,
            text=True,
            check=True,
            env={**os.environ, "V": value},
        ).stdout
        assert out == tcl_list_quote(value, first=True)


class TestTclListSplit:
    @pytest.mark.parametrize(
        ("text", "expected"),
        [
            ("", []),
            ("a b c", ["a", "b", "c"]),
            ("  a   b  ", ["a", "b"]),
            ("{a b} c", ["a b", "c"]),
            ('a "b c" d', ["a", "b c", "d"]),
            ("{nested {braces} here}", ["nested {braces} here"]),
            ("a\\ b c", ["a\\ b", "c"]),
            ("{}", [""]),
        ],
    )
    def test_known_splits(self, text: str, expected: str) -> None:
        assert tcl_list_split(text) == expected

    @pytest.mark.parametrize("text", ["{unmatched", 'a "open', "{a}x", '"a"x'])
    def test_strict_rejects_malformed(self, text: str) -> None:
        with pytest.raises(TclListError):
            tcl_list_split(text, strict=True)

    @pytest.mark.parametrize("text", ["{unmatched", 'a "open', "{a}x"])
    def test_lenient_tolerates_malformed(self, text: str) -> None:
        # Lenient mode never raises; it recovers *some* element list.
        assert isinstance(tcl_list_split(text, strict=False), list)


class TestTclListJoinSplitRoundTrip:
    @pytest.mark.parametrize(
        "elements",
        [
            [],
            ["a", "b", "c"],
            ["hello world", "x"],
            ["a $b [c]", "plain"],
            ["", "{}", "{nested}"],
            ["#hash", "second"],
            ["with\ttab", "with space"],
        ],
    )
    def test_join_then_split_is_identity(self, elements: list[str]) -> None:
        # Holds for elements that quote via bare word or balanced braces — the
        # split recovers source-form-minus-delimiters, which equals the value
        # whenever no backslash escaping was needed to render the element.
        assert tcl_list_split(tcl_list_join(elements), strict=True) == elements

    def test_escape_quoted_element_returns_raw_source(self) -> None:
        # ``}`` can't be brace-quoted, so join escapes it to ``\}``; split
        # returns that source form verbatim (it strips delimiters but does not
        # apply backslash substitution — that's a separate decoding step).
        joined = tcl_list_join(["}"])
        assert joined == "\\}"
        assert tcl_list_split(joined, strict=True) == ["\\}"]

    @pytest.mark.parametrize(
        "elements",
        [["a", "b"], ["hello world", "x"], ["a $b [c]", "p"], ["#hash", "x"]],
    )
    def test_join_matches_tcl_list_builtin(self, elements: list[str]) -> None:
        tclsh = shutil.which("tclsh9.0") or shutil.which("tclsh8.6")
        if tclsh is None:
            pytest.skip("no tclsh on PATH")
        # Pass elements via a newline-delimited env var; rebuild + ``list`` them.
        script = (
            "set parts [split $env(ELEMS) \\n]\n"
            "puts -nonewline [list {*}$parts]"
        )
        out = subprocess.run(
            [tclsh],
            input=script,
            capture_output=True,
            text=True,
            check=True,
            env={**os.environ, "ELEMS": "\n".join(elements)},
        ).stdout
        assert out == tcl_list_join(elements)
