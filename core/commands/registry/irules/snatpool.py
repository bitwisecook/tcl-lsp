# Enriched from F5 iRules reference documentation.
"""snatpool -- Assigns the specified SNAT pool or SNAT pool member to the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/snatpool.html"


_av = make_av(_SOURCE)


@register
class SnatpoolCommand(CommandDef):
    name = "snatpool"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="snatpool",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Assigns the specified SNAT pool or SNAT pool member to the current connection.",
                synopsis=("snatpool SNAT_POOL_OBJ (member IP_ADDR)?",),
                snippet=(
                    "Causes the pool of addresses identified by <snatpool_name> to be used\n"
                    "as translation addresses to create a SNAT."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [TCP::local_port] == 531 } {\n"
                    "     snatpool chat_snatpool\n"
                    "}\n"
                    "  elseif { [TCP::local_port] == 25 } {\n"
                    "     snatpool smtp_snatpool member 10.20.30.40\n"
                    " }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snatpool SNAT_POOL_OBJ (member IP_ADDR)?",
                    arg_values={
                        0: (
                            _av(
                                "member",
                                "snatpool member",
                                "snatpool SNAT_POOL_OBJ (member IP_ADDR)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SNAT_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
