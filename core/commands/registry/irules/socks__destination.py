# Enriched from F5 iRules reference documentation.
"""SOCKS::destination -- This command allows you to get or set the SOCKS destination host or port."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SOCKS__destination.html"


_av = make_av(_SOURCE)


@register
class SocksDestinationCommand(CommandDef):
    name = "SOCKS::destination"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SOCKS::destination",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to get or set the SOCKS destination host or port.",
                synopsis=(
                    "SOCKS::destination ('host')? (HOST_ADDRESS)?",
                    "SOCKS::destination 'port' (PORT)?",
                ),
                snippet=(
                    "This command allows you to get or set the SOCKS host or port, individually, or both at the same time.\n"
                    "\n"
                    "Details (Syntax):\n"
                    'SOCKS::destination "hostname:port"\n'
                    "    Sets the destination to the given hostname and port tuple.\n"
                    "\n"
                    "SOCKS::destination\n"
                    '    Gets the destination in the format "hostname:port".\n'
                    "\n"
                    'SOCKS::destination host "hostname"\n'
                    "    Sets the destination to the given hostname, doesn't change the port.\n"
                    "SOCKS::destination host\n"
                    "    Gets the destination hostname.  (Without appending the port.)\n"
                    "\n"
                    'SOCKS::destination port "port_number"\n'
                    "    Sets the destination port, doesn't change the hostname."
                ),
                source=_SOURCE,
                examples=("when SOCKS_REQUEST {\n    SOCKS::destination example.com:1234\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SOCKS::destination ('host')? (HOST_ADDRESS)?",
                    arg_values={
                        0: (
                            _av(
                                "host",
                                "SOCKS::destination host",
                                "SOCKS::destination ('host')? (HOST_ADDRESS)?",
                            ),
                            _av(
                                "port",
                                "SOCKS::destination port",
                                "SOCKS::destination 'port' (PORT)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SOCKS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
