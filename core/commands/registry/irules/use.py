# Enriched from F5 iRules reference documentation.
"""use -- A BIG-IP 4.X statement."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .virtual import VirtualCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/use.html"


_av = make_av(_SOURCE)


@register
class UseCommand(CommandDef):
    name = "use"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="use",
            deprecated_replacement=VirtualCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="A BIG-IP 4.X statement.",
                synopsis=(
                    "use clone pool POOL_OBJ (member IP_ADDR)?",
                    "use nexthop ((IP_ADDR) | ((VLAN_OBJ) (IP_ADDR | MAC_ADDR | transparent)?))",
                    "use node (IP_TUPLE | (IP_ADDR (PORT)?))",
                    "use pool POOL_OBJ (member (IP_TUPLE | (IP_ADDR (PORT)?)))?",
                ),
                snippet=(
                    "This is a BIG-IP 4.X statement, provided for backward-compatibility. The use statement must be paired with certain BIG-IP 9.X commands such as node, pool, rateclass, snat, and snatpool.\n"
                    "\n"
                    "The use command is not required on BIG-IP 9.X systems."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    if { [HTTP::uri] contains "aol" } {\n'
                    "        use pool aol_pool\n"
                    "    } else {\n"
                    "        use pool all_pool\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="use clone pool POOL_OBJ (member IP_ADDR)?",
                    arg_values={
                        0: (
                            _av(
                                "member", "use member", "use clone pool POOL_OBJ (member IP_ADDR)?"
                            ),
                            _av(
                                "transparent",
                                "use transparent",
                                "use nexthop ((IP_ADDR) | ((VLAN_OBJ) (IP_ADDR | MAC_ADDR | transparent)?))",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
