"""Tests for the diagnostic-repro minimizer (Phase 9)."""

from __future__ import annotations

import pytest

from server.features.diagnostics import get_diagnostics
from server.features.minimize import minimize_diagnostic


def _codes(src: str) -> set[str | int | None]:
    return {d.code for d in get_diagnostics(src)}


class TestMinimizeReduces:
    def test_reduces_dead_store_to_one_line(self):
        src = (
            "proc helper {alpha beta} {\n"
            "    set unused_local 99\n"
            "    return [expr {$alpha + $beta}]\n"
            "}\n"
            "set globalThing 5\n"
            "set deadStore 1\n"
            "puts done\n"
        )
        r = minimize_diagnostic(src, "W220")
        assert r.reproduces
        assert r.reduced_lines < r.original_lines
        # The minimal reproducer still fires W220.
        assert "W220" in _codes(r.source)

    def test_read_before_set_reduces_and_renames(self):
        src = "proc foo {} {\n    set y $undefinedVar\n    return $y\n}\n"
        r = minimize_diagnostic(src, "W210")
        assert r.reproduces
        assert "W210" in _codes(r.source)
        assert r.renamed
        # Identifiers renamed to short a/b names — no original long names left.
        assert "undefinedVar" not in r.source

    def test_rename_is_verify_gated_and_safe(self):
        # Even when renaming can't apply (vars inside braced exprs), the result
        # still reproduces and is never broken.
        src = 'set longName "a"\nif {$longName == ""} { puts x }\n'
        r = minimize_diagnostic(src, "W110")
        assert r.reproduces
        assert "W110" in _codes(r.source)


class TestMinimizeContract:
    def test_missing_code_raises(self):
        with pytest.raises(ValueError):
            minimize_diagnostic("puts hello", "W220")

    def test_result_never_larger_than_input(self):
        src = "set a 1\nset b 2\nset c 3\nset dead 4\n"
        r = minimize_diagnostic(src, "W220")
        assert r.reduced_lines <= r.original_lines
        assert r.reproduces

    def test_reproduces_property_across_codes(self):
        # For a handful of diagnostics, the minimized output must reproduce the
        # same code (the core verify-preserving invariant).
        cases = [
            ("proc f {} { set x 1 }", "W220"),
            ("proc f {} { return $never }", "W210"),
            ('if {$x == ""} { puts a }', "W110"),
        ]
        for src, code in cases:
            if code not in _codes(src):
                continue
            r = minimize_diagnostic(src, code)
            assert r.reproduces, f"{code} repro lost"
            assert code in _codes(r.source), f"{code} not in minimized output"


class TestMinimizeNoRename:
    def test_rename_disabled_keeps_names(self):
        src = "proc foo {} {\n    set descriptiveName $undefinedThing\n    return $descriptiveName\n}\n"
        r = minimize_diagnostic(src, "W210", rename=False)
        assert r.reproduces
        assert not r.renamed
