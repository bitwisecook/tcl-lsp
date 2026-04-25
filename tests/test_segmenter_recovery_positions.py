"""Position-rebasing tests for the segmenter's error-recovery path.

When the segmenter recovers from an unclosed ``{`` / ``[`` / ``"`` it
re-segments the tail of the source. The synthesized recovery body-token
must carry the *absolute* line + character of the recovery offset, not
``(0, 0)``; otherwise every recovered token reports a position as if the
slice were a fresh file. LSP features that compute positions from
recovered ranges (definition, references, hover) silently misreport
locations on files mid-edit when this rebasing is wrong.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.signature_scan import extract_signatures


def _build_source(prelude_lines: int, *, delimiter: str) -> tuple[str, int]:
    """Build a source with an unclosed *delimiter* on line 0 and a
    recoverable ``proc late`` *prelude_lines* lines down.

    Returns ``(source, expected_late_line)``.
    """
    if delimiter == "{":
        head = "proc early {} {\n"  # unclosed brace in body
    elif delimiter == "[":
        head = "set x [bogus\n"  # unclosed bracket in command sub
    elif delimiter == '"':
        head = 'set x "open\n'  # unclosed double quote
    else:  # pragma: no cover - guard
        raise ValueError(f"unsupported delimiter {delimiter!r}")

    # Prelude: zero or more blank/comment lines before the recovery target.
    body_lines = ["    # broken\n"] + ["\n"] * (prelude_lines - 2)
    # ``head`` consumes line 0; body_lines fill lines 1..prelude_lines-1;
    # ``proc late`` lands on line ``prelude_lines``.
    src = head + "".join(body_lines) + "proc late {} {}\n"
    return src, prelude_lines


@pytest.mark.parametrize("delimiter", ["{", "[", '"'])
@pytest.mark.parametrize("prelude_lines", [3, 5, 10])
def test_recovered_proc_name_range_uses_absolute_line(delimiter: str, prelude_lines: int) -> None:
    src, expected_line = _build_source(prelude_lines, delimiter=delimiter)
    result = extract_signatures(src)

    assert "::late" in result.all_procs, (
        f"recovery failed for delimiter={delimiter!r}, prelude={prelude_lines}"
    )
    proc = result.all_procs["::late"]

    # The proc name's reported line must match the actual line the
    # ``late`` keyword sits on in the original source.
    assert proc.name_range.start.line == expected_line, (
        f"name_range.start.line should be {expected_line} "
        f"but is {proc.name_range.start.line} "
        f"(delimiter={delimiter!r}, prelude={prelude_lines})"
    )

    # Sanity: offset stays absolute too. The token text on this line
    # is ``proc late {} {}`` so ``late`` starts five characters past
    # the line start.
    line_start = sum(len(line) + 1 for line in src.split("\n")[:expected_line])
    assert proc.name_range.start.offset == line_start + len("proc ")


@pytest.mark.parametrize("delimiter", ["{", "[", '"'])
def test_recovered_proc_name_character_is_absolute(delimiter: str) -> None:
    src, expected_line = _build_source(5, delimiter=delimiter)
    result = extract_signatures(src)
    proc = result.all_procs["::late"]
    # ``proc late ...`` always starts at column 0, then ``late`` at 5.
    assert proc.name_range.start.character == len("proc ")
