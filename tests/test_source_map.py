"""Tests for SourceMap offset/position conversions."""

from __future__ import annotations

from core.common.source_map import SourceMap


def test_position_offset_round_trip() -> None:
    source = "set a 1\nputs $a\n"
    source_map = SourceMap(source)

    off = source_map.position_to_offset(1, 5)
    pos = source_map.offset_to_position(off)

    assert pos.line == 1
    assert pos.character == 5
    assert pos.offset == off


def test_position_to_offset_clamps_bounds() -> None:
    source = "abc\ndef"
    source_map = SourceMap(source)

    assert source_map.position_to_offset(-10, -10) == 0
    assert source_map.position_to_offset(999, 999) == len(source)


def test_offset_to_position_clamps_bounds() -> None:
    source = "abc\ndef"
    source_map = SourceMap(source)

    start = source_map.offset_to_position(-5)
    end = source_map.offset_to_position(999)
    assert (start.line, start.character, start.offset) == (0, 0, 0)
    assert (end.line, end.character, end.offset) == (1, 3, len(source))


def test_range_from_offsets_is_inclusive() -> None:
    source = "line0\nline1"
    source_map = SourceMap(source)
    rng = source_map.range_from_offsets(6, 10)  # "line1"

    assert (rng.start.line, rng.start.character, rng.start.offset) == (1, 0, 6)
    assert (rng.end.line, rng.end.character, rng.end.offset) == (1, 4, 10)


def test_range_from_offsets_clamps_and_orders_bounds() -> None:
    source = "abc"
    source_map = SourceMap(source)
    rng = source_map.range_from_offsets(99, -10)

    assert (rng.start.line, rng.start.character, rng.start.offset) == (0, 2, 2)
    assert (rng.end.line, rng.end.character, rng.end.offset) == (0, 2, 2)


def test_empty_source_behaviour() -> None:
    source_map = SourceMap("")

    assert source_map.position_to_offset(10, 10) == 0
    pos = source_map.offset_to_position(10)
    assert (pos.line, pos.character, pos.offset) == (0, 0, 0)

    rng = source_map.range_from_offsets(0, 0)
    assert (rng.start.line, rng.start.character, rng.start.offset) == (0, 0, 0)
    assert (rng.end.line, rng.end.character, rng.end.offset) == (0, 0, 0)
