"""Profile-guided optimisation (PGO) — opt-in, off by default.

This package consumes execution-profile data and produces optimisation
*suggestions* from it.  It is deliberately decoupled from the standard
optimiser pipeline (:mod:`compiler.optimiser`): nothing here runs unless a
profile is explicitly supplied through these entry points, and the PGO
suggestion codes (``P``-series) are registered as their own
:class:`~shared.codes.CodeKind` so the optimisation profiles never enable
them.

Internal representation
-----------------------
:mod:`compiler.pgo.occurrence` is a normalised, lossless execution-event
model that covers **both** C Tcl execution traces (``trace add execution``,
``TCL_COMPILE_STATS`` opcode counts) and the F5 BIG-IP ``rule-profiler``
occurrence log (events, rule/command VM boundaries, bytecode, var-mods,
flow/VS context).  :class:`~compiler.pgo.profile_data.ProfileData` rolls a
stream up into the counts the analyses query.

Importers
---------
* :func:`compiler.pgo.trace_parser.parse_f5_log` — F5 rule-profiler logs.
* :func:`compiler.pgo.tclsh_capture.capture_profile` — live tclsh tracing.

Analyses
--------
* :func:`compiler.pgo.reorder.find_pgo_suggestions` — branch reordering.
"""

from __future__ import annotations

from .occurrence import FlowContext, Occurrence, OccurrenceKind
from .profile_data import ProfileData, merge
from .reorder import REORDER_CODE, find_pgo_suggestions
from .trace_parser import parse_f5_log

__all__ = [
    "FlowContext",
    "Occurrence",
    "OccurrenceKind",
    "ProfileData",
    "REORDER_CODE",
    "find_pgo_suggestions",
    "merge",
    "parse_f5_log",
]
