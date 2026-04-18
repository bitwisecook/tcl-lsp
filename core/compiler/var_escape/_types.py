"""Types for the var-escape analysis."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


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
    """

    tags: dict[str, EscapeTag] = field(default_factory=dict)
    dynamic_barrier: bool = False
    frame_needed: bool = False

    def tag(self, name: str) -> EscapeTag:
        """Return the tag for ``name`` (defaults to ``LOCAL``)."""
        if self.dynamic_barrier:
            return EscapeTag.FRAME
        return self.tags.get(name, EscapeTag.LOCAL)

    def is_frame(self, name: str) -> bool:
        """Shorthand: does ``name`` need to live in the runtime frame?"""
        return self.tag(name) is EscapeTag.FRAME
