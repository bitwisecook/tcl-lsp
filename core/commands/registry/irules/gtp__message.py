# Enriched from F5 iRules reference documentation.
"""GTP::message -- Returns the entire GTP message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__message.html"


@register
class GtpMessageCommand(CommandDef):
    name = "GTP::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the entire GTP message.",
                synopsis=("GTP::message ('-message' MESSAGE)?",),
                snippet="Returns the entire GTP message.",
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_INGRESS {\n"
                    "    set t1 [GTP::message]\n"
                    '    log local0. "GTP type [GTP::header type -message $t1]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::message ('-message' MESSAGE)?",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
