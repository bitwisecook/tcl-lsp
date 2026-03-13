# Enriched from F5 iRules reference documentation.
"""GTP::respond -- Sends the GTP message back to the remote node of this connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__respond.html"


@register
class GtpRespondCommand(CommandDef):
    name = "GTP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends the GTP message back to the remote node of this connection.",
                synopsis=("GTP::respond MESSAGE",),
                snippet=(
                    "Sends this GTP message back to the remote node of this connection.\n"
                    "If this is clientside flow, send it back to client that initiated the connection.\n"
                    "If this is serverside flow, send it to the remote node that is connected to."
                ),
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_EGRESS {\n"
                    "    set t2 [GTP::new 2 10]\n"
                    "    GTP::respond $t2\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::respond MESSAGE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
