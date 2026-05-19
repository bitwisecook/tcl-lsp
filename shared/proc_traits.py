"""Procedure-parameter usage trait enum.

`ProcArgTrait` describes how a proc parameter is used inside its body —
the same information drives shimmer analysis, taint propagation, the
unused-variable diagnostic, and the interprocedural summariser. The
enum is pulled into `shared/` so both the analyser (which derives the
traits) and the compiler optimiser/taint (which consume them through
interprocedural summaries) can name them without forcing one to
depend on the other.
"""

from __future__ import annotations

from enum import Enum, auto


class ProcArgTrait(Enum):
    """How a proc parameter is used inside the proc body.

    These traits drive optimisation, shimmer analysis, taint propagation,
    and diagnostics by telling downstream passes how a parameter value
    flows through the proc.
    """

    EVAL = auto()  # Argument is eval'd as a script (eval, uplevel, subst)
    BODY = auto()  # Argument is used as a loop/control body
    VAR_WRITE = auto()  # Argument names a variable that the proc writes (upvar + set)
    VAR_READ = auto()  # Argument names a variable that the proc reads (upvar read-only)
    EXPR = auto()  # Argument is evaluated as an expression
    LOOP_LIST = auto()  # Argument is used as the list in a foreach/lmap
