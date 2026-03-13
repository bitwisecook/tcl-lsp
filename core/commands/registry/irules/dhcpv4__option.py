# Enriched from F5 iRules reference documentation.
"""DHCPv4::option -- This command retrieves,sets or deletes the option by id number."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv4__option.html"


_av = make_av(_SOURCE)


@register
class Dhcpv4OptionCommand(CommandDef):
    name = "DHCPv4::option"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv4::option",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command retrieves,sets or deletes the option by id number.",
                synopsis=("DHCPv4::option (delete)? OPTION (VALUE)?",),
                snippet=(
                    "This command retrieves,sets or deletes the option by id number\n"
                    "\n"
                    "Details (syntax);\n"
                    "DHCPv4::option <id>\n"
                    "DHCPv4::option <id> <value>\n"
                    "DHCPv4::option delete <id>"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_DATA {\n        log local0. "Option [DHCPv4::option 18]"\n    }'
                ),
                return_value="This command returns value by option id number when retrieving",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv4::option (delete)? OPTION (VALUE)?",
                    arg_values={
                        0: (
                            _av(
                                "delete",
                                "DHCPv4::option delete",
                                "DHCPv4::option (delete)? OPTION (VALUE)?",
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
