# Enriched from F5 iRules reference documentation.
"""IPFIX::destination -- IPFIX::destination Provides the ability to manage IPFIX logging destinations and send IPFIX messages based on processing in the iRule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IPFIX__destination.html"


@register
class IpfixDestinationCommand(CommandDef):
    name = "IPFIX::destination"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IPFIX::destination",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="IPFIX::destination Provides the ability to manage IPFIX logging destinations and send IPFIX messages based on processing in the iRule.",
                synopsis=("IPFIX::destination ((open (-publisher LOG_PUBLISHER)) |",),
                snippet=(
                    "Provides the ability to open and close IPFIX logging destinations in\n"
                    "the context of an iRule, as well as the ability to send IPFIX messages\n"
                    "to the IPFIX logging destinations."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    '    set static::http_track_dest ""\n'
                    '    set static::http_track_tmplt ""\n'
                    "}"
                ),
                return_value="IPFIX::destination open returns an IPFIX_DESTINATION object that is used by the IPFIX::destination close or send command.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IPFIX::destination ((open (-publisher LOG_PUBLISHER)) |",
                    options=(
                        OptionSpec(
                            name="-publisher", detail="Option -publisher.", takes_value=True
                        ),
                    ),
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
