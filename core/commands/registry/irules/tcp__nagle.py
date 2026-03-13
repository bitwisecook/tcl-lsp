# Enriched from F5 iRules reference documentation.
"""TCP::nagle -- Toggles the Nagle mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__nagle.html"


_av = make_av(_SOURCE)


@register
class TcpNagleCommand(CommandDef):
    name = "TCP::nagle"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::nagle",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles the Nagle mode.",
                synopsis=("TCP::nagle (enable | disable | auto)",),
                snippet=(
                    "Enables or disables the Nagle algorithm on the current TCP connection.\n"
                    "Nagle waits for additional data before sending undersized packets, see RFC896 for details.\n"
                    "The auto option enables or disables Nagle based on connection conditions."
                ),
                source=_SOURCE,
                examples=(
                    "# Change the TCP Nagle mode to auto.\n"
                    "when CLIENT_ACCEPTED {\n"
                    "    TCP::nagle auto\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::nagle (enable | disable | auto)",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "TCP::nagle enable",
                                "TCP::nagle (enable | disable | auto)",
                            ),
                            _av(
                                "disable",
                                "TCP::nagle disable",
                                "TCP::nagle (enable | disable | auto)",
                            ),
                            _av("auto", "TCP::nagle auto", "TCP::nagle (enable | disable | auto)"),
                        )
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
