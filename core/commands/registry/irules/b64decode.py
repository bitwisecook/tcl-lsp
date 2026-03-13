# Enriched from F5 iRules reference documentation.
"""b64decode -- Returns a string that is base-64 decoded."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/b64decode.html"


@register
class B64decodeCommand(CommandDef):
    name = "b64decode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="b64decode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a string that is base-64 decoded.",
                synopsis=("b64decode ANY_CHARS",),
                snippet="Returns a string that is base-64 decoded.",
                source=_SOURCE,
                examples=("when RULE_INIT {\n   set ::key [AES::key]\n}"),
                return_value="b64decode <string> Returns a string that is base-64 decoded",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="b64decode ANY_CHARS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(init_only=True),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
