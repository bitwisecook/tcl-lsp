# Enriched from F5 iRules reference documentation.
"""WS::release -- Releases the data collected using WS::collect."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__release.html"


@register
class WsReleaseCommand(CommandDef):
    name = "WS::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the data collected using WS::collect.",
                synopsis=("WS::release",),
                snippet=(
                    "WS::release\n"
                    "    Releases the data collected via WS::collect. Unless a subsequent WS::collect command was issued, there is no need to use the WS::release command inside of the WS_CLIENT_DATA and WS_SERVER_DATA events, since (in these cases) the data is implicitly released."
                ),
                source=_SOURCE,
                examples=(
                    "when WS_CLIENT_FRAME {\n    WS::collect frame 1000\n    set clen 1000\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::release",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
