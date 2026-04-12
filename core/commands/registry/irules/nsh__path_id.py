# Enriched from F5 iRules reference documentation.
"""NSH::path_id -- Set/Get the Path ID for NSH."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NSH__path_id.html"


@register
class NshPathIdCommand(CommandDef):
    name = "NSH::path_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NSH::path_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set/Get the Path ID for NSH.",
                synopsis=("NSH::path_id DIRECTION (NSH_PATH_ID)?",),
                snippet=(
                    "Set: Path ID for NSH.\n"
                    "            Get(DIRECTION as the only parameter): path id from NSH."
                ),
                source=_SOURCE,
                examples=(
                    "th ID for NSH.\n"
                    "            when CLIENT_ACCEPTED {\n"
                    "                NSH::path_id serverside_egress 10\n"
                    "                set mypath_id [NSH::path_id serverside_egress]\n"
                    "            }"
                ),
                return_value="None for set, value of path id for get.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NSH::path_id DIRECTION (NSH_PATH_ID)?",
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
