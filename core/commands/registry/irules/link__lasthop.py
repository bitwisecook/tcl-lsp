# Enriched from F5 iRules reference documentation.
"""LINK::lasthop -- Returns the MAC address of the last hop."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LINK__lasthop.html"


_av = make_av(_SOURCE)


@register
class LinkLasthopCommand(CommandDef):
    name = "LINK::lasthop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LINK::lasthop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the MAC address of the last hop.",
                synopsis=("LINK::lasthop ('id' | 'type' | 'name')?",),
                snippet=(
                    "Returns the MAC address of the last hop.\n"
                    "Note:\n"
                    "  * In 11.4, you can extend LINK::lasthop with sub-commands to retrieve\n"
                    "    the lasthop id, type, name, respectively. Without sub-command,\n"
                    "    LINK::lasthop returns the MAC address as before."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  set lastmac [LINK::lasthop]\n"
                    "  session add uie [IP::client_addr] $lastmac 180\n"
                    "}"
                ),
                return_value="LINK::lasthop [id]",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LINK::lasthop ('id' | 'type' | 'name')?",
                    arg_values={
                        0: (
                            _av(
                                "id", "LINK::lasthop id", "LINK::lasthop ('id' | 'type' | 'name')?"
                            ),
                            _av(
                                "type",
                                "LINK::lasthop type",
                                "LINK::lasthop ('id' | 'type' | 'name')?",
                            ),
                            _av(
                                "name",
                                "LINK::lasthop name",
                                "LINK::lasthop ('id' | 'type' | 'name')?",
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
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
