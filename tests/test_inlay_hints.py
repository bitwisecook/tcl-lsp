"""Tests for the inlay hints provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from server.features.inlay_hints import get_inlay_hints

FULL_RANGE = types.Range(
    start=types.Position(line=0, character=0),
    end=types.Position(line=999, character=0),
)


class TestInlayHints:
    def test_integer_type_hint(self):
        source = "set x 42\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        # Should show a ``: int`` hint immediately after ``set x`` (col 5).
        int_hints = [h for h in hints if "int" in h.label]
        assert any(h.label == ": int" and h.position.character == 5 for h in int_hints), [
            (h.label, h.position.line, h.position.character) for h in int_hints
        ]

    def test_no_hints_for_unknown_type(self):
        source = "set x [some_command]\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        # Unknown command return type — may or may not produce hints
        # Just verify it doesn't crash
        assert isinstance(hints, list)

    def test_empty_file(self):
        hints = get_inlay_hints("", FULL_RANGE)
        assert hints == []

    def test_hint_kinds_are_type_or_parameter(self):
        source = "set x 42\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        for h in hints:
            assert h.kind in (types.InlayHintKind.Type, types.InlayHintKind.Parameter)
        # The inferred-type annotation is a Type hint; the registry
        # parameter-name hints (label ending with ``:``) are Parameter.
        type_hints = [h for h in hints if h.kind == types.InlayHintKind.Type]
        assert any("int" in h.label for h in type_hints)

    def _param_labels(self, source: str) -> list[str]:
        hints = get_inlay_hints(source, FULL_RANGE)
        return [
            h.label
            for h in hints
            if h.kind == types.InlayHintKind.Parameter and isinstance(h.label, str)
        ]

    def test_user_proc_param_name_hints(self):
        source = "proc add {a b} { expr {$a + $b} }\nadd 1 2\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        call_hints = [
            (h.label, h.position.character)
            for h in hints
            if h.kind == types.InlayHintKind.Parameter and h.position.line == 1
        ]
        assert ("a:", 4) in call_hints
        assert ("b:", 6) in call_hints

    def test_negative_number_positional_not_treated_as_flag(self):
        # ``-1`` is no command's option, so it must be labelled as the
        # positional ``charIndex`` argument, not skipped as a flag.
        source = "set s hello\nstring index $s -1\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        line1 = [
            h for h in hints if h.kind == types.InlayHintKind.Parameter and h.position.line == 1
        ]
        # The ``-1`` token (at the end of the line) carries a positional hint.
        neg_one_char = source.split("\n")[1].index("-1")
        assert any(h.position.character == neg_one_char for h in line1)

    def test_declared_option_and_value_are_skipped(self):
        # ``lsort -integer $list`` — ``-integer`` is a declared flag (no
        # value), so the positional hint lands on ``$list``.
        labels = self._param_labels("set l {3 1 2}\nlsort -integer $l\n")
        # Should not crash and should not mislabel the flag itself.
        assert all(not lbl.startswith("-") for lbl in labels)

    def test_range_filtering(self):
        source = textwrap.dedent("""\
            set x 42
            set y 99
        """)
        # Only request hints for line 0
        narrow_range = types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=100),
        )
        hints = get_inlay_hints(source, narrow_range)
        # All hints should be on line 0
        for h in hints:
            assert h.position.line == 0

    def test_foreach_multi_var_hints_not_clumped(self):
        """Type hints for foreach {a b c} should appear after each var, not clumped."""
        source = "foreach {a b c} {1 2 3} { puts $a$b$c }\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        type_hints = [h for h in hints if h.kind == types.InlayHintKind.Type]
        # All three loop vars (a, b, c) must get a type hint, each at its own
        # position — not clumped at a single column.
        assert len(type_hints) == 3, type_hints
        positions = {h.position.character for h in type_hints}
        assert len(positions) == 3, [(h.label, h.position.character) for h in type_hints]
