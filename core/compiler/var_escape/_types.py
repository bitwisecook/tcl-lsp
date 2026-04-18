"""Types for the var-escape analysis."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Iterable


class EscapeTag(Enum):
    """Where a Tcl variable must live at runtime.

    ``LOCAL`` — only accessed through statically resolved positions; the
    WASM local slot is the single source of truth.

    ``FRAME`` — must live in the runtime frame so the interpreter (or an
    ``upvar`` alias) can read and write it by name.
    """

    LOCAL = "local"
    FRAME = "frame"


def join(a: EscapeTag, b: EscapeTag) -> EscapeTag:
    """Join operator on the lattice: FRAME dominates."""
    if a is EscapeTag.FRAME or b is EscapeTag.FRAME:
        return EscapeTag.FRAME
    return EscapeTag.LOCAL


@dataclass(frozen=True, slots=True)
class ProcEscapeSummary:
    """Per-procedure escape classification.

    ``tags`` maps variable name to its escape tag. Names not present
    default to ``EscapeTag.LOCAL`` — the caller may treat the dict as
    "what needs spilling".

    ``dynamic_barrier`` is set when the analysis encountered a
    construct whose name-reference set cannot be bounded (``eval
    $body``, ``uplevel 1``, ``info level``, ``{*}$dynamic`` in an
    unknown call, etc.). In that case every variable in the proc is
    effectively ``FRAME`` regardless of what ``tags`` contains, and
    the codegen must fall back to the sync-everything path.

    ``frame_needed`` is a convenience flag for codegen: True if the
    proc needs a runtime frame at all. Equivalent to
    ``dynamic_barrier or any(tag is FRAME for tag in tags.values())``.

    ``upvar_source_names`` is the set of literal variable names this
    proc (or any of its transitive callees once the interprocedural
    pass has run) names as the *source* of a caller-frame ``upvar``.
    A caller must treat any of its local vars whose names appear in a
    callee's ``upvar_source_names`` as ``FRAME`` — the callee aliases
    them by name from the frame. ``unbounded_upvar_source`` is True
    when the source set can't be enumerated (dynamic source name,
    pessimistic callee, …) — callers must spill every local.

    ``direct_callees`` is the set of qualified proc names this proc
    calls with statically known arguments, used by the interprocedural
    pass to drive the fixpoint.
    """

    tags: dict[str, EscapeTag] = field(default_factory=dict)
    dynamic_barrier: bool = False
    frame_needed: bool = False
    upvar_source_names: frozenset[str] = frozenset()
    unbounded_upvar_source: bool = False
    direct_callees: frozenset[str] = frozenset()
    has_fallback: bool = False
    # Per-SSA-version escape tags, populated by the flow-sensitive
    # CFG+SSA propagation.  Empty when the analysis was driven from
    # an IR-only source (no CompilationUnit).  ``ssa_tags`` is keyed
    # by ``(var_name, ssa_version)`` — see
    # ``core.compiler.ssa.SSAValueKey``.  The per-name ``tags``
    # field is the join over this dict and is what codegen consumes.
    ssa_tags: dict[tuple[str, int], EscapeTag] = field(default_factory=dict)

    def tag(self, name: str) -> EscapeTag:
        """Return the tag for ``name`` (defaults to ``LOCAL``)."""
        if self.dynamic_barrier:
            return EscapeTag.FRAME
        return self.tags.get(name, EscapeTag.LOCAL)

    def is_frame(self, name: str) -> bool:
        """Shorthand: does ``name`` need to live in the runtime frame?"""
        return self.tag(name) is EscapeTag.FRAME

    def with_escapes(
        self,
        extra_escaped: Iterable[str],
        *,
        pessimistic: bool = False,
    ) -> "ProcEscapeSummary":
        """Return a new summary with ``extra_escaped`` spilled to FRAME.

        Used by the interprocedural pass to fold callee-induced
        escapes (names a callee uses as ``upvar`` sources) into a
        caller's summary without mutating the originally computed
        structure.
        """
        new_tags = dict(self.tags)
        for name in extra_escaped:
            new_tags[name] = EscapeTag.FRAME
        new_pessimistic = self.dynamic_barrier or pessimistic
        new_frame_needed = new_pessimistic or any(
            tag is EscapeTag.FRAME for tag in new_tags.values()
        )
        return ProcEscapeSummary(
            tags=new_tags,
            dynamic_barrier=new_pessimistic,
            frame_needed=new_frame_needed,
            upvar_source_names=self.upvar_source_names,
            unbounded_upvar_source=self.unbounded_upvar_source,
            direct_callees=self.direct_callees,
            has_fallback=self.has_fallback,
            ssa_tags=dict(self.ssa_tags),
        )
