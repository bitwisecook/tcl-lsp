# Enriched from F5 iRules reference documentation.
"""whereis -- Returns geographical information on an IP address."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/whereis.html"


_av = make_av(_SOURCE)


@register
class WhereisCommand(CommandDef):
    name = "whereis"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="whereis",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns geographical information on an IP address.",
                synopsis=("whereis (ldns | IP_ADDR)",),
                snippet=(
                    "Returns the geographic location of a specific IP address.\n"
                    "For more information on using whereis in LTM, you can check Jason\n"
                    "Rahm's article\n"
                    "\n"
                    "Legal usage notes\n"
                    "\n"
                    "   The data is purchased by F5 for use on BIG-IP systems and products for\n"
                    "   traffic management. The key to understanding EULA compliance is to\n"
                    "   figure out where the geolocation decision is being made."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="whereis (ldns | IP_ADDR)",
                    arg_values={0: (_av("ldns", "whereis ldns", "whereis (ldns | IP_ADDR)"),)},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
