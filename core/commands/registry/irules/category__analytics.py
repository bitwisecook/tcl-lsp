# Enriched from F5 iRules reference documentation.
"""CATEGORY::analytics -- Controls response analytics engine."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CATEGORY__analytics.html"


@register
class CategoryAnalyticsCommand(CommandDef):
    name = "CATEGORY::analytics"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CATEGORY::analytics",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls response analytics engine.",
                synopsis=("CATEGORY::analytics BOOL_VALUE",),
                snippet="Enables or disables the analytics server on a per request basis (requires SWG license)",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    set this_uri http://[HTTP::host][HTTP::uri]\n"
                    "    set reply [CATEGORY::lookup $this_uri]\n"
                    '    log local0. "uri $this_uri returns category=$reply"\n'
                    '    if { $reply equals "Adult Material" } {\n'
                    "        CATEGORY::analytics enable\n"
                    "    }\n"
                    "}"
                ),
                return_value="No return value",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CATEGORY::analytics BOOL_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CATEGORY", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
