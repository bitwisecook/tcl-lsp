# Enriched from F5 iRules reference documentation.
"""NSH::service_index -- Sets/Get the Service Index for NSH."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NSH__service_index.html"


@register
class NshServiceIndexCommand(CommandDef):
    name = "NSH::service_index"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NSH::service_index",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets/Get the Service Index for NSH.",
                synopsis=("NSH::service_index DIRECTION (NSH_SERVICE_IDX)?",),
                snippet=(
                    "Set: Service index for NSH.\n"
                    "            Get(DIRECTION as the only parameter): Service index from NSH."
                ),
                source=_SOURCE,
                examples=(
                    "rvice index for NSH.\n"
                    "            when CLIENT_ACCEPTED {\n"
                    "                NSH::service_index serverside_egress 20\n"
                    "                set myservice_index [NSH::service_index serverside_egress]\n"
                    "            }"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NSH::service_index DIRECTION (NSH_SERVICE_IDX)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
