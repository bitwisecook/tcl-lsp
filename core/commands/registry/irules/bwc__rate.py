# Enriched from F5 iRules reference documentation.
"""BWC::rate -- This command is used to modify max-user rate for dynamic policy."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__rate.html"


@register
class BwcRateCommand(CommandDef):
    name = "BWC::rate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::rate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to modify max-user rate for dynamic policy.",
                synopsis=(
                    "BWC::rate SESSION_ID BW_VALUE",
                    "BWC::rate SESSION_ID APPLICATION_NAME BW_VALUE",
                ),
                snippet="This command is used to modify max-user rate for dynamic policy after it is created. This irule can modify the rate for a session or category.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set mycookie [IP::remote_addr]:[TCP::remote_port]\n"
                    "    BWC::policy attach gold_user $mycookie\n"
                    "    BWC::color set gold_user p2p\n"
                    "    BWC::rate $mycookie p2p 1000000bps\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::rate SESSION_ID BW_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
