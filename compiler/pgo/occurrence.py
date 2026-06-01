"""Normalised execution-occurrence model — the lossless internal form.

Profile-guided optimisation has to ingest two very different telemetry
sources and represent both *without losing information*:

* **C Tcl** — execution traces from ``trace add execution <cmd>
  {enter|leave|enterstep|leavestep}`` (the portable, ship-anywhere hook,
  backed by ``Tcl_CreateObjTrace``), plus, on a ``TCL_COMPILE_STATS``
  build, per-opcode counts (``ByteCodeStats.instructionCount``) and the
  ``evalstats`` dump.

* **F5 iRules** — the BIG-IP ``rule-profiler`` occurrence log, whose lines
  are ``timestamp,RP_<TYPE>,<vs>,<value>,<pid>,<flow>,<remote tuple>,
  <local tuple>[,<depth>]`` covering event / rule / command / VM /
  bytecode / variable-modification occurrences with flow context.

Both are fundamentally **timestamped streams of typed occurrences**.  This
module is that common denominator: :class:`OccurrenceKind` is the union of
both vocabularies, :class:`FlowContext` carries the BIG-IP connection
tuple (absent for plain Tcl), and :class:`Occurrence` is one sample.  The
importers (:mod:`compiler.pgo.trace_parser`,
:mod:`compiler.pgo.tclsh_capture`) translate into this form; the
aggregated :class:`~compiler.pgo.profile_data.ProfileData` rolls a stream
up into the counts the analyser queries.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class OccurrenceKind(Enum):
    """A normalised occurrence type spanning C Tcl traces and F5 rule-profiler.

    The ``EVENT_*`` and ``*_VM_*`` kinds are iRule-only (no C Tcl analog);
    ``CMD_ENTER`` / ``CMD_EXIT`` / ``VAR_MOD`` / ``BYTECODE`` map onto both
    worlds.  Keeping the full union here means neither source is lossy.
    """

    # Event boundaries (iRules ``when`` handlers — no C Tcl analog).
    EVENT_ENTRY = "event-entry"
    EVENT_EXIT = "event-exit"
    # Rule boundaries (iRules — a single ``ltm rule`` body).
    RULE_ENTRY = "rule-entry"
    RULE_EXIT = "rule-exit"
    # Bytecode-VM boundaries (iRules — entering/leaving compiled bytecode).
    RULE_VM_ENTRY = "rule-vm-entry"
    RULE_VM_EXIT = "rule-vm-exit"
    CMD_VM_ENTRY = "cmd-vm-entry"
    CMD_VM_EXIT = "cmd-vm-exit"
    # Command boundaries — F5 RP_CMD_ENTRY/EXIT, C Tcl trace enter/leave
    # (and enterstep/leavestep, which carry the precise source line).
    CMD_ENTER = "cmd-enter"
    CMD_EXIT = "cmd-exit"
    # A single executed bytecode instruction — F5 RP_CMD_BYTECODE, or a
    # C Tcl ``instructionCount`` opcode.  ``value`` is the opcode name.
    BYTECODE = "bytecode"
    # A variable write — F5 RP_VAR_MOD, or a C Tcl ``trace add variable
    # write``.  ``value`` is ``name`` or ``name=value``.
    VAR_MOD = "var-mod"


#: F5 rule-profiler occurrence-type string -> :class:`OccurrenceKind`.
RP_OCCURRENCE_KINDS: dict[str, OccurrenceKind] = {
    "RP_EVENT_ENTRY": OccurrenceKind.EVENT_ENTRY,
    "RP_EVENT_EXIT": OccurrenceKind.EVENT_EXIT,
    "RP_RULE_ENTRY": OccurrenceKind.RULE_ENTRY,
    "RP_RULE_EXIT": OccurrenceKind.RULE_EXIT,
    "RP_RULE_VM_ENTRY": OccurrenceKind.RULE_VM_ENTRY,
    "RP_RULE_VM_EXIT": OccurrenceKind.RULE_VM_EXIT,
    "RP_CMD_VM_ENTRY": OccurrenceKind.CMD_VM_ENTRY,
    "RP_CMD_VM_EXIT": OccurrenceKind.CMD_VM_EXIT,
    "RP_CMD_ENTRY": OccurrenceKind.CMD_ENTER,
    "RP_CMD_EXIT": OccurrenceKind.CMD_EXIT,
    "RP_CMD_BYTECODE": OccurrenceKind.BYTECODE,
    "RP_VAR_MOD": OccurrenceKind.VAR_MOD,
}

#: C Tcl ``trace add execution`` operation -> :class:`OccurrenceKind`.
#: ``enterstep`` / ``leavestep`` fire per command *inside* a traced body
#: (the granularity we use to recover precise per-line counts).
TCL_TRACE_OP_KINDS: dict[str, OccurrenceKind] = {
    "enter": OccurrenceKind.CMD_ENTER,
    "leave": OccurrenceKind.CMD_EXIT,
    "enterstep": OccurrenceKind.CMD_ENTER,
    "leavestep": OccurrenceKind.CMD_EXIT,
}

#: Inverse of :data:`RP_OCCURRENCE_KINDS` — :class:`OccurrenceKind` to the
#: canonical ``RP_*`` string, for emitting rule-profiler-format logs.
KIND_TO_RP: dict[OccurrenceKind, str] = {v: k for k, v in RP_OCCURRENCE_KINDS.items()}


@dataclass(frozen=True, slots=True)
class FlowContext:
    """BIG-IP connection context for an occurrence (all fields optional).

    Empty/``None`` for plain-Tcl captures, fully populated from an F5
    rule-profiler log (so per-virtual-server / per-flow slicing stays
    possible without re-parsing the raw log).
    """

    virtual_server: str | None = None
    process_id: int | None = None
    flow_id: str | None = None
    remote_addr: str | None = None
    remote_port: int | None = None
    remote_rd: int | None = None
    local_addr: str | None = None
    local_port: int | None = None
    local_rd: int | None = None


@dataclass(frozen=True, slots=True)
class Occurrence:
    """One normalised execution occurrence."""

    kind: OccurrenceKind
    #: Occurrence value — command/event/rule name, opcode, or ``var[=value]``.
    value: str
    #: TMM/wall-clock timestamp in microseconds, when known.
    timestamp_us: int | None = None
    #: 1-based source line, when known (tclsh ``enterstep`` capture).
    line: int | None = None
    #: Rule / bytecode nesting depth (F5 trailing field), when known.
    depth: int | None = None
    #: Connection context, when known.
    context: FlowContext | None = None

    @property
    def var_name(self) -> str:
        """For :attr:`OccurrenceKind.VAR_MOD`, the variable name (drops ``=value``)."""
        head, _, _ = self.value.partition("=")
        return head
