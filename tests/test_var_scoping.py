"""Tests for the shared var-scoping detection helpers."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.var_scoping import (
    global_declaration_indices,
    upvar_local_declaration_indices,
    variable_declaration_indices,
)


class TestGlobalDeclarationIndices:
    def test_multiple_names(self):
        assert global_declaration_indices(["a", "b", "c"]) == [0, 1, 2]

    def test_skips_substituted_refs(self):
        # `global $dynamic` is a runtime-resolved name; the compiler ignores
        # it as a static declaration, so we do too.
        assert global_declaration_indices(["$dynamic", "b"]) == [1]

    def test_accepts_token_like_args(self):
        class Tok:
            def __init__(self, text: str) -> None:
                self.text = text

        assert global_declaration_indices([Tok("a"), Tok("b")]) == [0, 1]


class TestVariableDeclarationIndices:
    def test_even_indices_only(self):
        # variable name1 value1 name2 value2
        assert variable_declaration_indices(["counter", "0", "flag", "false"]) == [0, 2]

    def test_name_without_value(self):
        # variable counter   (no initial value)
        assert variable_declaration_indices(["counter"]) == [0]

    def test_skips_substituted_names(self):
        assert variable_declaration_indices(["$dynamic", "0"]) == []


class TestUpvarLocalDeclarationIndices:
    def test_default_level(self):
        # upvar other local           -> pair (other, local); local at index 1
        assert upvar_local_declaration_indices("upvar", ["other", "local"]) == [1]

    def test_integer_level(self):
        # upvar 1 other local         -> pair (other, local); local at index 2
        assert upvar_local_declaration_indices("upvar", ["1", "other", "local"]) == [2]

    def test_hash_level_arbitrary_digits(self):
        """``upvar #3 other local`` — arbitrary #N levels are recognised."""
        assert upvar_local_declaration_indices("upvar", ["#3", "other", "local"]) == [2]

    def test_hash_zero_level(self):
        assert upvar_local_declaration_indices("upvar", ["#0", "g", "l"]) == [2]

    def test_multiple_pairs(self):
        # upvar 1 a A b B             -> locals A (idx 2) and B (idx 4)
        assert upvar_local_declaration_indices("upvar", ["1", "a", "A", "b", "B"]) == [2, 4]

    def test_skips_substituted_names(self):
        # Pair with a $-prefixed side is dropped.
        assert upvar_local_declaration_indices("upvar", ["1", "$dyn", "local"]) == []

    def test_namespace_upvar_lowered_form(self):
        # Lowered ``namespace upvar ns src dst`` form: command="namespace",
        # args[0] == "upvar", args[1] == namespace, args[2:] are pairs.
        assert upvar_local_declaration_indices("namespace", ["upvar", "mymod", "src", "dst"]) == [3]

    def test_namespace_upvar_direct_form(self):
        # command="namespace upvar", args[0] == namespace, args[1:] are pairs.
        assert upvar_local_declaration_indices("namespace upvar", ["mymod", "src", "dst"]) == [2]

    def test_empty_args(self):
        assert upvar_local_declaration_indices("upvar", []) == []
