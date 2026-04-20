# Enriched from F5 iRules reference documentation.
"""DIAMETER::dynamic_route_insertion -- Set whether dynamic route insertion is enabled."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__dynamic_route_insertion.html"


@register
class DiameterDynamicRouteInsertionCommand(CommandDef):
    name = "DIAMETER::dynamic_route_insertion"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::dynamic_route_insertion",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set whether dynamic route insertion is enabled.",
                synopsis=("DIAMETER::dynamic_route_insertion ( BOOLEAN )?",),
                snippet=(
                    'If status is set to "enabled", a dynamic route will be created for this connection.\n'
                    "\n"
                    'This value, once set, remains for the life of the connection.  After the connection is closed, this route will be removed once "timeout" seconds have elapsed.  The default timeout is set by the configuration option "dynamic-route-timeout".\n'
                    "\n"
                    "The zero-argument form of this command returns whether the setting is enabled on the current connection."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '                if { ([IP::address] starts_with "192.168.") } {\n'
                    "                    DIAMETER::dynamic_route_insertion disabled\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::dynamic_route_insertion ( BOOLEAN )?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
