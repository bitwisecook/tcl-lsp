from __future__ import annotations

from dataclasses import dataclass, field

from ..semantic_model import AnalysisResult, Range, Scope


@dataclass
class AnalyserSnapshot:
    """Checkpoint of ``Analyser`` state at a chunk boundary.

    Captures the cumulative ``AnalysisResult`` and internal tracking
    state so that the analyser can resume from a checkpoint when only
    later chunks have changed.  The scope tree is deep-copied so that
    the snapshot is fully independent of live analyser state.
    """

    result: AnalysisResult
    last_comment: str
    const_strings: dict[int, dict[str, tuple[str, Range]]]
    regex_vars: set[tuple[int, str]]
    current_event: str | None
    conditional_depth: int = 0
    # Command aliases: alias_name -> (target_cmd, prepended_args).
    command_aliases: dict[str, tuple[str, tuple[str, ...]]] = field(default_factory=dict)
    # Map from old scope id → scope object for scope identity reconstruction.
    scope_id_map: dict[int, Scope] = field(default_factory=dict)
    # Variable-as-command sites: (var_name, method_name_or_None, token_range).
    var_command_sites: list[tuple[str, str | None, Range, bool]] = field(default_factory=list)
    # Command-substitution-as-command sites: (cmd_text, method_name_or_None, range, in_method).
    cmd_command_sites: list[tuple[str, str | None, Range, bool]] = field(default_factory=list)
    # Pending trace-callback registrations: tuples of candidate qualified
    # proc names for callbacks whose target may not yet be parsed.
    pending_trace_callbacks: list[tuple[str, ...]] = field(default_factory=list)
