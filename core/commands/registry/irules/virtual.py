# Enriched from F5 iRules reference documentation.
"""virtual -- Returns the name of the associated virtual server or selects another virtual server and an optional IP address and port to connect to."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/virtual.html"


_av = make_av(_SOURCE)


@register
class VirtualCommand(CommandDef):
    name = "virtual"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="virtual",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name of the associated virtual server or selects another virtual server and an optional IP address and port to connect to.",
                synopsis=(
                    "virtual",
                    "virtual (name | VIRTUAL_SERVER_OBJ) (IP_TUPLE | (IP_ADDR (PORT)?))?",
                ),
                snippet=(
                    "Returns the name of the associated virtual server that the connection\n"
                    "is flowing through. In 9.4.0 and higher, it can be also used to route\n"
                    "the connection to another virtual server and an optional IP address\n"
                    "and port, without leaving the BIG-IP."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  log local0. "Current virtual server name: [virtual name]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="virtual",
                    arg_values={
                        0: (
                            _av(
                                "name",
                                "virtual name",
                                "virtual (name | VIRTUAL_SERVER_OBJ) (IP_TUPLE | (IP_ADDR (PORT)?))?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            cse_candidate=True,
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
