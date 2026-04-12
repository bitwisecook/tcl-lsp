# Enriched from F5 iRules reference documentation.
"""LB::detach -- Detaches the server-side connection from the client-side."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__detach.html"


_av = make_av(_SOURCE)


@register
class LbDetachCommand(CommandDef):
    name = "LB::detach"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::detach",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Detaches the server-side connection from the client-side.",
                synopsis=("LB::detach ('disable')?",),
                snippet=(
                    "This command detaches the server-side connection from the client-side.\n"
                    "\n"
                    "LB::detach\n"
                    "    Detaches the server side connection from the client-side connection.\n"
                    "    Use in conjunction with TCP::notify as best practice."
                ),
                source=_SOURCE,
                examples=("when USER_RESPONSE {\n    LB::detach\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::detach ('disable')?",
                    arg_values={
                        0: (_av("disable", "LB::detach disable", "LB::detach ('disable')?"),)
                    },
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
