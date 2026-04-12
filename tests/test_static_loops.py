"""Tests for conservative static loop evaluation helpers."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.static_loops import summarise_static_for_loop


def test_static_loop_handles_if_branching() -> None:
    env = summarise_static_for_loop(
        [
            "set i 0",
            "$i < 3",
            "incr i",
            "if {$i == 1} {set x 10} else {set x 20}",
        ]
    )
    assert env is not None
    assert env.get("i") == 3
    assert env.get("x") == 20


def test_static_loop_handles_switch_dispatch() -> None:
    env = summarise_static_for_loop(
        [
            "set i 0; set mode a",
            "$i < 1",
            "incr i",
            "switch $mode { a {set v 1} default {set v 9} }",
        ]
    )
    assert env is not None
    assert env.get("v") == 1


def test_static_loop_switch_requires_resolvable_subject() -> None:
    env = summarise_static_for_loop(
        [
            "set i 0",
            "$i < 1",
            "incr i",
            "switch $mode { a {set v 1} default {set v 9} }",
        ]
    )
    assert env is None
