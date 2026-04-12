# Enriched from F5 iRules reference documentation.
"""b64encode -- Returns a string that is base-64 encoded, or if an error occurs, an empty string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/b64encode.html"


@register
class B64encodeCommand(CommandDef):
    name = "b64encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="b64encode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a string that is base-64 encoded, or if an error occurs, an empty string.",
                synopsis=("b64encode ANY_CHARS",),
                snippet="Returns a string that is base-64 encoded, or if an error occurs, an empty string.",
                source=_SOURCE,
                examples=("when RULE_INIT {\n    set ::key [AES::key]\n}"),
                return_value="b64encode <string> Returns a string that is base-64 encoded, or if an error occurs, an empty string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="b64encode ANY_CHARS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
