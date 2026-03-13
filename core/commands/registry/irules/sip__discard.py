# Enriched from F5 iRules reference documentation.
"""SIP::discard -- Discards the current SIP message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__discard.html"


@register
class SipDiscardCommand(CommandDef):
    name = "SIP::discard"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::discard",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Discards the current SIP message.",
                synopsis=("SIP::discard",),
                snippet=(
                    "Discards a SIP message\n"
                    "\n"
                    "SIP::discard\n"
                    "\n"
                    "     * Discards the current SIP message"
                ),
                source=_SOURCE,
                examples=("when SIP_RESPONSE {\n  SIP::discard\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::discard",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
