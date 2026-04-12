# Enriched from F5 iRules reference documentation.
"""rateclass -- Selects the specified rate class to use when transmitting packets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/rateclass.html"


@register
class RateclassCommand(CommandDef):
    name = "rateclass"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="rateclass",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Selects the specified rate class to use when transmitting packets.",
                synopsis=("rateclass RATE_CLASS",),
                snippet=(
                    "Causes the system to select the specified rate class to use when\n"
                    "transmitting packets."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [IP::addr [IP::client_addr] equals xxx.xxx.xxx.xxx] } {\n"
                    '    log local0. "[IP::client_addr] being handled by rateclass class1"\n'
                    "    rateclass class1\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="rateclass RATE_CLASS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
