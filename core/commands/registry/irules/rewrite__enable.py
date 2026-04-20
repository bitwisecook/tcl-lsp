# Enriched from F5 iRules reference documentation.
"""REWRITE::enable -- Changes the REWRITE plugin from passthrough to full patching mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/REWRITE__enable.html"


@register
class RewriteEnableCommand(CommandDef):
    name = "REWRITE::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="REWRITE::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the REWRITE plugin from passthrough to full patching mode.",
                synopsis=("REWRITE::enable",),
                snippet=(
                    "Changes the REWRITE plugin from passthrough to full patching mode. A\n"
                    "place where this might be helpful would be a POST request where REWRITE\n"
                    "would modify the post body unnecessarily, so we disable it. However, we\n"
                    "want REWRITE to modify the response, so we would enable it later in the\n"
                    "HTTP_RESPONSE. Use of this command can be extremely tricky to get\n"
                    "exactly right; its use is not recommended in the majority of cases"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="REWRITE::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ACCESS", "FASTHTTP", "REWRITE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
