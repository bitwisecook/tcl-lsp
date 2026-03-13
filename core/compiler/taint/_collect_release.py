"""Collect/release pairing analysis."""

from __future__ import annotations

from ...analysis.semantic_model import Range
from ..ir import (
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ..value_shapes import parse_command_substitution
from ._types import CollectWithoutReleaseWarning, ReleaseWithoutCollectWarning


def _iter_ir_statements(script: IRScript):
    """Yield every IR statement, recursing into structured bodies."""
    for stmt in script.statements:
        yield stmt
        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                yield from _iter_ir_statements(clause.body)
            if stmt.else_body is not None:
                yield from _iter_ir_statements(stmt.else_body)
        elif isinstance(stmt, IRFor):
            yield from _iter_ir_statements(stmt.init)
            yield from _iter_ir_statements(stmt.body)
            yield from _iter_ir_statements(stmt.next)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    yield from _iter_ir_statements(arm.body)
            if stmt.default_body is not None:
                yield from _iter_ir_statements(stmt.default_body)
        elif isinstance(stmt, IRWhile):
            yield from _iter_ir_statements(stmt.body)
        elif isinstance(stmt, IRForeach):
            yield from _iter_ir_statements(stmt.body)
        elif isinstance(stmt, IRCatch):
            yield from _iter_ir_statements(stmt.body)
        elif isinstance(stmt, IRTry):
            yield from _iter_ir_statements(stmt.body)
            for handler in stmt.handlers:
                yield from _iter_ir_statements(handler.body)
            if stmt.finally_body is not None:
                yield from _iter_ir_statements(stmt.finally_body)


def _find_collect_without_release(
    ir_script: IRScript,
) -> list[CollectWithoutReleaseWarning]:
    """Find ``*::collect`` calls with no matching ``*::release``."""
    collects: list[tuple[str, Range]] = []  # (protocol, range)
    releases: set[str] = set()  # protocol prefixes seen

    for stmt in _iter_ir_statements(ir_script):
        cmd: str | None = None
        rng: Range | None = None

        if isinstance(stmt, (IRCall, IRBarrier)):
            cmd = stmt.command
            rng = stmt.range
        elif isinstance(stmt, IRAssignValue):
            parsed = parse_command_substitution(stmt.value)
            if parsed is not None:
                cmd = parsed[0]
                rng = stmt.range

        if cmd is None or rng is None:
            continue

        if cmd.endswith("::collect"):
            protocol = cmd[: -len("::collect")]
            collects.append((protocol, rng))
        elif cmd.endswith("::release"):
            protocol = cmd[: -len("::release")]
            releases.add(protocol)

    warnings: list[CollectWithoutReleaseWarning] = []
    for protocol, rng in collects:
        if protocol not in releases:
            cmd_name = f"{protocol}::collect"
            warnings.append(
                CollectWithoutReleaseWarning(
                    range=rng,
                    command=cmd_name,
                    code="T200",
                    message=(
                        f"{cmd_name} without matching "
                        f"{protocol}::release; collected data is never released"
                    ),
                )
            )
    return warnings


def _find_release_without_collect(
    ir_script: IRScript,
) -> list[ReleaseWithoutCollectWarning]:
    """Find ``*::release`` calls with no matching ``*::collect``."""
    releases: list[tuple[str, Range]] = []  # (protocol, range)
    collects: set[str] = set()  # protocol prefixes seen

    for stmt in _iter_ir_statements(ir_script):
        cmd: str | None = None
        rng: Range | None = None

        if isinstance(stmt, (IRCall, IRBarrier)):
            cmd = stmt.command
            rng = stmt.range
        elif isinstance(stmt, IRAssignValue):
            parsed = parse_command_substitution(stmt.value)
            if parsed is not None:
                cmd = parsed[0]
                rng = stmt.range

        if cmd is None or rng is None:
            continue

        if cmd.endswith("::release"):
            protocol = cmd[: -len("::release")]
            releases.append((protocol, rng))
        elif cmd.endswith("::collect"):
            protocol = cmd[: -len("::collect")]
            collects.add(protocol)

    warnings: list[ReleaseWithoutCollectWarning] = []
    for protocol, rng in releases:
        if protocol not in collects:
            cmd_name = f"{protocol}::release"
            warnings.append(
                ReleaseWithoutCollectWarning(
                    range=rng,
                    command=cmd_name,
                    code="T201",
                    message=(
                        f"{cmd_name} without matching {protocol}::collect; no data was collected"
                    ),
                )
            )
    return warnings
