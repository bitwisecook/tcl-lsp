# Enriched from F5 iRules reference documentation.
"""WEBSSO::enable -- Causes APM to do the SSO processing on a request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WEBSSO__enable.html"


@register
class WebssoEnableCommand(CommandDef):
    name = "WEBSSO::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WEBSSO::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes APM to do the SSO processing on a request.",
                synopsis=("WEBSSO::enable",),
                snippet=(
                    "This command causes APM to do the SSO processing on the HTTP request.\n"
                    "This is to allow admin to re-enable WEBSSO processing for a request if\n"
                    "it was disabled before by doing WEBSSO::disable for the request. The\n"
                    "scope of this iRule command is per HTTP request. Admin needs to execute\n"
                    "it for each HTTP request."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WEBSSO::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ACCESS", "HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
