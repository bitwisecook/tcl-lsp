# Enriched from F5 iRules reference documentation.
"""GTP::clone -- Returns a cloned copy of the GTP message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__clone.html"


@register
class GtpCloneCommand(CommandDef):
    name = "GTP::clone"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::clone",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a cloned copy of the GTP message.",
                synopsis=("GTP::clone (MESSAGE_VAR)?",),
                snippet="Returns a cloned copy of the GTP message.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set payload [UDP::payload]\n"
                    "    set t2 [GTP::parse $payload]\n"
                    "    set t3 [GTP::clone $t2]\n"
                    '    log local0. "GTP type [GTP::header type -message $t3]"\n'
                    '    log local0. "GTP teid [GTP::header teid -message $t3]"\n'
                    "}"
                ),
                return_value="Returns a cloned copy of the GTP message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::clone (MESSAGE_VAR)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
