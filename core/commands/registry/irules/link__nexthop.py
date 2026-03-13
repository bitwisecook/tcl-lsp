# Enriched from F5 iRules reference documentation.
"""LINK::nexthop -- Returns the MAC address of the next hop."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LINK__nexthop.html"


_av = make_av(_SOURCE)


@register
class LinkNexthopCommand(CommandDef):
    name = "LINK::nexthop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LINK::nexthop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the MAC address of the next hop.",
                synopsis=("LINK::nexthop ('id' | 'type' | 'name')?",),
                snippet=(
                    "Returns the MAC address of the next hop. Returns the broadcast address\n"
                    "ff:ff:ff:ff:ff:ff when called before a serverside connection has been\n"
                    "established.\n"
                    "Note:\n"
                    "  * In 11.4, you can use LINK::nexthop with sub-commands to retrieve\n"
                    "    the id, type and name of the next hop, respectively. Without\n"
                    "    sub-commands, LINK::nexthop returns the MAC address as before."
                ),
                source=_SOURCE,
                examples=(
                    "# Logging example\n"
                    "when CLIENT_ACCEPTED {\n"
                    '        log local0. "\\[LINK::lasthop\\]: [LINK::lasthop], \\[LINK::nexhop\\]: [LINK::nexthop]"\n'
                    "}"
                ),
                return_value="LINK::nexthop [id]",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LINK::nexthop ('id' | 'type' | 'name')?",
                    arg_values={
                        0: (
                            _av(
                                "id", "LINK::nexthop id", "LINK::nexthop ('id' | 'type' | 'name')?"
                            ),
                            _av(
                                "type",
                                "LINK::nexthop type",
                                "LINK::nexthop ('id' | 'type' | 'name')?",
                            ),
                            _av(
                                "name",
                                "LINK::nexthop name",
                                "LINK::nexthop ('id' | 'type' | 'name')?",
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
