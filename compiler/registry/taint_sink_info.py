"""Lightweight data container for taint sink classification results."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class TaintSinkInfo:
    """Result of a single-pass taint sink classification query.

    ``network_sink_args`` carries the argument positions that flow into
    the network-address slot (D5-T104):

    * ``None`` -> not a network sink at all.
    * empty tuple ``()`` -> whole-statement scan (any tainted arg is a
      network sink candidate -- used by iRules ``connect`` that takes
      the address via opaque options).
    * non-empty tuple -> the listed positional indexes are the network
      slots; T104 fires only when the tainted var sits in one of them.
    """

    is_code_sink: bool = False
    output_sink: str | None = None
    output_sink_is_subcommand_qualified: bool = False
    log_sink: str | None = None
    network_sink_args: tuple[int, ...] | None = None
    interp_eval_subcommands: frozenset[str] | None = None


_EMPTY_TAINT_SINK_INFO = TaintSinkInfo()
