# Enriched from F5 iRules reference documentation.
"""RADIUS::id -- This command returns the RADIUS message id"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RADIUS__id.html"


@register
class RadiusIdCommand(CommandDef):
    name = "RADIUS::id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RADIUS::id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns the RADIUS message id",
                synopsis=("RADIUS::id",),
                snippet="This command returns the RADIUS message id",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    let msg_id [RADIUS::id]\n"
                    '    log local0. "recieved radius message with id $msg_id"\n'
                    "}"
                ),
                return_value="This command returns the RADIUS message id",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RADIUS::id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset(
                    {
                        "CLIENT_ACCEPTED",
                        "CLIENT_CLOSED",
                        "CLIENT_DATA",
                        "SERVER_CLOSED",
                        "SERVER_CONNECTED",
                        "SERVER_DATA",
                    }
                )
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
