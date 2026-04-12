# Enriched from F5 iRules reference documentation.
"""L7CHECK::protocol -- Set or get L7 protocol value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/L7CHECK__protocol.html"


@register
class L7checkProtocolCommand(CommandDef):
    name = "L7CHECK::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="L7CHECK::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set or get L7 protocol value.",
                synopsis=(
                    "L7CHECK::protocol set VALUE",
                    "L7CHECK::protocol get",
                ),
                snippet="The L7CHECK::protocol commands allow you to set or retrieve L7 protocol value.",
                source=_SOURCE,
                examples=(
                    "when L7CHECK_CLIENT_DATA {\n"
                    '    if { [L7CHECK::protocol get] == "https" } {\n'
                    "        pool clients_https\n"
                    "    } else {\n"
                    "        pool clients_non_https\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="L7CHECK::protocol set VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CONNECTOR", "L7CHECK"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
