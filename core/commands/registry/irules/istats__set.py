# Enriched from F5 iRules reference documentation.
"""ISTATS::set -- Sets the given key's value within iStats."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ISTATS__set.html"


@register
class IstatsSetCommand(CommandDef):
    name = "ISTATS::set"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ISTATS::set",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the given key's value within iStats.",
                synopsis=("ISTATS::set KEY VALUE",),
                snippet="Set the given key's value within iStats",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  # send request to /invalidate?policy=<policy>\n"
                    '  if { [HTTP::path] eq "/invalidate" } {\n'
                    "        set wa_policy [URI::query [HTTP::uri] policy]\n"
                    '        if { $wa_policy ne "" } {\n'
                    '          ISTATS::set "WA policy string $wa_policy" "invalidated"\n'
                    "        }\n"
                    '        HTTP::respond 200 content "<html><body>Cache Invalidated for Policy: $wa_policy</body></html>"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ISTATS::set KEY VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ISTATS,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
