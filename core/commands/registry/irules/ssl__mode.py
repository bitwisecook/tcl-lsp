# Enriched from F5 iRules reference documentation.
"""SSL::mode -- Gets the enabled/disabled state of SSL."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__mode.html"


@register
class SslModeCommand(CommandDef):
    name = "SSL::mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the enabled/disabled state of SSL.",
                synopsis=("SSL::mode",),
                snippet="Gets the enabled/disabled state of SSL",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { [TCP::local_port] != 443 } {\n"
                    "        SSL::disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="SSL::mode Gets the enabled/disabled state of SSL. Returns 1 if it is enabled, and 0 if it is disabled.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::mode",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
