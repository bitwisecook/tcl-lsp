# Enriched from F5 iRules reference documentation.
"""TCP::option -- Retrieves or changes TCP header options."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__option.html"


_av = make_av(_SOURCE)


@register
class TcpOptionCommand(CommandDef):
    name = "TCP::option"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::option",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Retrieves or changes TCP header options.",
                synopsis=("TCP::option ((get TCP_OPTION) |",),
                snippet=(
                    "Gets or sets the value of the specified option of the TCP header.\n"
                    "The TCP::option get command is only functional when BIG-IP has been configured to collect options before the iRule is called. In v10, this is done with a db variable and is effective only on the clientside. When called in the serverside context it returns an error indicating that the specified option was not configured for collection.\n"
                    "\n"
                    "In v11, this is configured through the TCP profile and the command can be used in either the serverside context or the clientside, depending on the profile configuration."
                ),
                source=_SOURCE,
                examples=(
                    "#Insert the client REAL IP (mostly used when the client IP is SNATted).\n"
                    "when SERVER_CONNECTED {\n"
                    "    scan [IP::client_addr] {%d.%d.%d.%d} a b c d\n"
                    "    TCP::option set 29 [binary format cccc $a $b $c $d] all\n"
                    "}"
                ),
                return_value='With the "get" keyword, returns the specified TCP option kind value. If the requested option kind was not configured for collection, an error indicating so is returned instead. If the option kind was specified but has not yet been seen on the current connection, the command returns null.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::option ((get TCP_OPTION) |",
                    arg_values={
                        0: (_av("get", "TCP::option get", "TCP::option ((get TCP_OPTION) |"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
