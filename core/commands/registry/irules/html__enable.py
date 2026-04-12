# Enriched from F5 iRules reference documentation.
"""HTML::enable -- Enable the processing of HTML for this transaction."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTML__enable.html"


@register
class HtmlEnableCommand(CommandDef):
    name = "HTML::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTML::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable the processing of HTML for this transaction.",
                synopsis=("HTML::enable",),
                snippet="Enable the processing of HTML for this transaction.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '    if {$host == "www.f5.com"} {\n'
                    "        HTML::enable\n"
                    "    }\n"
                    '    log local0. "host: $host"\n'
                    "}"
                ),
                return_value="empty return code.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTML::enable",
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
