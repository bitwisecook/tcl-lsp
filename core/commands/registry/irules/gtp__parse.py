# Enriched from F5 iRules reference documentation.
"""GTP::parse -- Creates a new GTP message from a byte stream."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__parse.html"


@register
class GtpParseCommand(CommandDef):
    name = "GTP::parse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::parse",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new GTP message from a byte stream.",
                synopsis=("GTP::parse BYTE_STREAM",),
                snippet=(
                    "Creates a new GTP message from a byte stream.\n"
                    'Returns a TCL object of type "GTP-Message"'
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set payload [UDP::payload]\n"
                    "    set t2 [GTP::parse $payload]\n"
                    '    log local0. "GTP type [GTP::header type -message $t2]"\n'
                    '    log local0. "GTP teid [GTP::header teid -message $t2]"\n'
                    "}"
                ),
                return_value='Returns a TCL object of type "GTP-Message"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::parse BYTE_STREAM",
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
