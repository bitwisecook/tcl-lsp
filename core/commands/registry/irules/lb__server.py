# Enriched from F5 iRules reference documentation.
"""LB::server -- Returns information about the currently selected server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__server.html"


_av = make_av(_SOURCE)


@register
class LbServerCommand(CommandDef):
    name = "LB::server"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::server",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns information about the currently selected server.",
                synopsis=("LB::server ('name' | 'pool' | 'route_domain' |",),
                snippet=(
                    "This command allows you to query for information about the member selected after a load balancing decision has been made.\n"
                    "\n"
                    'If no server was selected (all servers down), this command with either no arguments or the "name" argument will return the pool name only--useful for determining the default pool applied to a virtual server. If the node command is called prior to this command a null string is returned as the node command overrides any prior pool selection logic.\n'
                    "\n"
                    "LB::server [name | pool | route_domain | addr | port | priority | ratio | weight | ripeness]"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Save the name of the VIP's default pool\n"
                    "    set default_pool [LB::server pool]\n"
                    "}"
                ),
                return_value="LB::server returns a Tcl list with pool, pool member address and port. If no server was selected yet or all servers are down, returns default pool name only.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::server ('name' | 'pool' | 'route_domain' |",
                    arg_values={
                        0: (
                            _av(
                                "name",
                                "LB::server name",
                                "LB::server ('name' | 'pool' | 'route_domain' |",
                            ),
                            _av(
                                "pool",
                                "LB::server pool",
                                "LB::server ('name' | 'pool' | 'route_domain' |",
                            ),
                            _av(
                                "route_domain",
                                "LB::server route_domain",
                                "LB::server ('name' | 'pool' | 'route_domain' |",
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
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
