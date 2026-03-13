# Enriched from F5 iRules reference documentation.
"""DHCPv6::option -- This command retrieves, sets or deletes the option by id."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__option.html"


_av = make_av(_SOURCE)


@register
class Dhcpv6OptionCommand(CommandDef):
    name = "DHCPv6::option"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::option",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command retrieves, sets or deletes the option by id.",
                synopsis=("DHCPv6::option (delete)? OPTION (VALUE)?",),
                snippet=(
                    "This command retrieves, sets or deletes the option by id\n"
                    "\n"
                    "Details (syntax);\n"
                    "DHCPv6::option <id>\n"
                    "DHCPv6::option <id> <value>\n"
                    "DHCPv6::option delete <id>"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_DATA {\n        log local0. "Option [DHCPv6::option 18]"\n    }'
                ),
                return_value="when retrieving, this command returns the value of the option via option id",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::option (delete)? OPTION (VALUE)?",
                    arg_values={
                        0: (
                            _av(
                                "delete",
                                "DHCPv6::option delete",
                                "DHCPv6::option (delete)? OPTION (VALUE)?",
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
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
