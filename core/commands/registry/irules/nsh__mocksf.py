# Enriched from F5 iRules reference documentation.
"""NSH::mocksf -- Set option to mock SF functionality for NSH."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NSH__mocksf.html"


@register
class NshMocksfCommand(CommandDef):
    name = "NSH::mocksf"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NSH::mocksf",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set option to mock SF functionality for NSH.",
                synopsis=("NSH::mocksf",),
                snippet="Set option to mock SF functionality for NSH.",
                source=_SOURCE,
                examples=(
                    "cksf option for NSH.\n"
                    "            when FLOW_INIT {\n"
                    "                NSH::mocksf\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NSH::mocksf",
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
