# Enriched from F5 iRules reference documentation.
"""snat -- Assigns the specified SNAT translation address to the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/snat.html"


@register
class SnatCommand(CommandDef):
    name = "snat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="snat",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Assigns the specified SNAT translation address to the current connection.",
                synopsis=("snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))",),
                snippet=(
                    "Causes the system to assign the specified source address to the\n"
                    "serverside connection(s). The assignment is valid for the duration of\n"
                    "the clientside connection or until 'snat none' is called. The iRule\n"
                    "SNAT command overrides the SNAT configuration of the virtual server or\n"
                    "a SNAT pool. It does not override the 'Allow SNAT' setting of a pool."
                ),
                source=_SOURCE,
                examples=(
                    "# Apply SNAT autmap if the selected pool member IP address is 1.1.1.1\n"
                    "when LB_SELECTED {\n"
                    "If { [IP::addr [LB::server addr] equals 1.1.1.1] } {\n"
                    "     snat automap\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP", "MR"}),
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
            ),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SNAT_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
