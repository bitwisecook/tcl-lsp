# Enriched from F5 iRules reference documentation.
"""TCP::option -- Retrieves or changes TCP header options."""

# Introduced: BIG-IP v10+ (core TCP iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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
                synopsis=(
                    "TCP::option get <kind>",
                    "TCP::option set <kind> <value> ?all?",
                    "TCP::option noset <kind>",
                ),
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
                    synopsis="TCP::option <get|set|noset> <kind> ?value? ?all?",
                    arg_values={
                        0: (
                            _av("get", "Get TCP option value.", "TCP::option get <kind>"),
                            _av(
                                "set",
                                "Set TCP option value.",
                                "TCP::option set <kind> <value> ?all?",
                            ),
                            _av(
                                "noset",
                                "Prevent option from being set.",
                                "TCP::option noset <kind>",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 4),
            ),
            event_requires=EventRequires(
                transport="tcp",
                also_in=frozenset({"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "get": SubCommand(
                    name="get",
                    arity=Arity(1, 1),
                    detail="Get TCP option value.",
                    synopsis="TCP::option get <kind>",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.TCP_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "set": SubCommand(
                    name="set",
                    arity=Arity(2, 3),
                    detail="Set TCP option value.",
                    synopsis="TCP::option set <kind> <value> ?all?",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.TCP_STATE,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "noset": SubCommand(
                    name="noset",
                    arity=Arity(1, 1),
                    detail="Prevent option from being set.",
                    synopsis="TCP::option noset <kind>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.TCP_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
