# Enriched from F5 iRules reference documentation.
"""MR::connect_back_port -- Gets or sets connect_back_port for the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__connect_back_port.html"


@register
class MrConnectBackPortCommand(CommandDef):
    name = "MR::connect_back_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::connect_back_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets connect_back_port for the current connection.",
                synopsis=("MR::connect_back_port (NONNEGATIVE_INTEGER)?",),
                snippet="The MR::connect_back_port command gets or sets connect_back_port for the current connection, which is used to enable bi-directional persistence if the client connected through an ephemeral port.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::connect_back_port 5678\n"
                    "            }"
                ),
                return_value="Returns current connect_back_port value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::connect_back_port (NONNEGATIVE_INTEGER)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
