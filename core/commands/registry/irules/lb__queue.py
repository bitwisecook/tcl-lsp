# Enriched from F5 iRules reference documentation.
"""LB::queue -- Returns queue information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__queue.html"


@register
class LbQueueCommand(CommandDef):
    name = "LB::queue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::queue",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns queue information.",
                synopsis=(
                    "LB::queue queued",
                    "LB::queue on",
                    "LB::queue limit",
                    "LB::queue depth",
                ),
                snippet=(
                    "Returns queue information. Connection queuing details:\n"
                    "\n"
                    "    * Operates at the TCP level\n"
                    "    * Only engages when the connection limit is hit\n"
                    "    * Queue is specified by length, time, or both (in the pool configuration)\n"
                    "    * Queues operate per-tmm, there is no state sharing\n"
                    "        * Length limit divided by tmm count\n"
                    "        * FIFO guarantees only per-tmm\n"
                    "    * Queued at the pool level for non-persistent connections\n"
                    "    * Queued at the pool member level for persistent connections."
                ),
                source=_SOURCE,
                examples=(
                    "when LB_QUEUED {\n"
                    '    log local0. "[IP::local_addr] was queued - [LB::queue depth one pool1] / [LB::queue limit depth pool1]"\n'
                    "}"
                ),
                return_value="LB::queue limit depth|time [<pool name>] Returns queue limit info (depth is per-tmm)",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::queue queued",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
