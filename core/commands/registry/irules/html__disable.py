# Enriched from F5 iRules reference documentation.
"""HTML::disable -- Disable the processing of HTML for this transaction."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTML__disable.html"


@register
class HtmlDisableCommand(CommandDef):
    name = "HTML::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTML::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disable the processing of HTML for this transaction.",
                synopsis=("HTML::disable",),
                snippet="Disable the processing of HTML for this transaction.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '    if {$host == "www.f5.com"} {\n'
                    "        HTML::disable\n"
                    "    }\n"
                    '    log local0. "host: $host"\n'
                    "}"
                ),
                return_value="empty return code.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTML::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
