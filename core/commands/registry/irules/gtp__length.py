# Enriched from F5 iRules reference documentation.
"""GTP::length -- This value is returned as read from the message header."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__length.html"


@register
class GtpLengthCommand(CommandDef):
    name = "GTP::length"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::length",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This value is returned as read from the message header.",
                synopsis=("GTP::length ('-message' MESSAGE)?",),
                snippet="This value is returned as read from the message header.",
                source=_SOURCE,
                examples=(
                    'when GTP_SIGNALLING_INGRESS {\n    log local0. "GTP length [GTP::length]"\n}'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::length ('-message' MESSAGE)?",
                    options=(
                        OptionSpec(name="-message", detail="Option -message.", takes_value=True),
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
