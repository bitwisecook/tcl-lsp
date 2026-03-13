# Enriched from F5 iRules reference documentation.
"""IPFIX::msg -- IPFIX::msg Provides the ability to create, delete and set values in an IPFIX message that can then be used to send IPFIX message based on processing in the iRule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IPFIX__msg.html"


_av = make_av(_SOURCE)


@register
class IpfixMsgCommand(CommandDef):
    name = "IPFIX::msg"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IPFIX::msg",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="IPFIX::msg Provides the ability to create, delete and set values in an IPFIX message that can then be used to send IPFIX message based on processing in the iRule.",
                synopsis=("IPFIX::msg ((create IPFIX_TEMPLATE) |",),
                snippet=(
                    "Provides the ability to create, delete and set data values in an IPFIX\n"
                    "message based on the provided IPFIX_TEMPLATE."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    '    set static::http_track_dest ""\n'
                    '    set static::http_track_tmplt ""\n'
                    "}"
                ),
                return_value="IPFIX::msg create returns an IPFIX_MESSAGE object that is used by the IPFIX::msg set|delete and IPFIX::destination send commands.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IPFIX::msg ((create IPFIX_TEMPLATE) |",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "IPFIX::msg create",
                                "IPFIX::msg ((create IPFIX_TEMPLATE) |",
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
