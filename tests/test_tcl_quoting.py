"""Tests for shared.tcl_quoting — the canonical Tcl word/list-element quoter."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from shared.tcl_quoting import tcl_list_quote


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
