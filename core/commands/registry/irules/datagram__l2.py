# Enriched from F5 iRules reference documentation.
"""DATAGRAM::l2 -- Returns Layer 2 destination address."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DATAGRAM__l2.html"


@register
class DatagramL2Command(CommandDef):
    name = "DATAGRAM::l2"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DATAGRAM::l2",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns Layer 2 destination address.",
                synopsis=("DATAGRAM::l2 dest",),
                snippet=(
                    "This iRules command returns the L2 destination of an ingress frame.\n"
                    "\n"
                    "DATAGRAM::l2 dest"
                ),
                source=_SOURCE,
                examples=(
                    "when FLOW_INIT {\n"
                    "     set l2_dest [ DATAGRAM::l2 dest ]\n"
                    '     log local0. "L2 destination MAC $l2_dest"\n'
                    "}"
                ),
                return_value="Returns the L2 destination of an ingress frame.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DATAGRAM::l2 dest",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FLOW"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
