"""Lightweight data container for taint sink classification results."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class TaintSinkInfo:
    """Result of a single-pass taint sink classification query."""

    is_code_sink: bool = False
    output_sink: str | None = None
    output_sink_is_subcommand_qualified: bool = False
    log_sink: str | None = None
    is_network_sink: bool = False
    interp_eval_subcommands: frozenset[str] | None = None


_EMPTY_TAINT_SINK_INFO = TaintSinkInfo()
