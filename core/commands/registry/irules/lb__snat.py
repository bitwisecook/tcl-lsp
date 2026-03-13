# Enriched from F5 iRules reference documentation.
"""LB::snat -- Returns information on the SNAT configuration for the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__snat.html"


@register
class LbSnatCommand(CommandDef):
    name = "LB::snat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::snat",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns information on the SNAT configuration for the current connection.",
                synopsis=("LB::snat",),
                snippet=(
                    "This command returns information on the SNAT configuration for the current connection.\n"
                    "\n"
                    "Possible output values are those which can be set by the snat and snatpool commands."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Check if SNAT is enabled on the VIP\n"
                    '    if {[LB::snat] eq "none"}{\n'
                    '        log local0. "Snat disabled on [virtual name]"\n'
                    "    } else {\n"
                    '        log local0. "Snat enabled on [virtual name].  Currently set to [LB::snat]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="LB::snat",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::snat",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SNAT_SELECTION,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
