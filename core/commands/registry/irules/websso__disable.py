# Enriched from F5 iRules reference documentation.
"""WEBSSO::disable -- Forwards a request without doing SSO processing on it."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WEBSSO__disable.html"


@register
class WebssoDisableCommand(CommandDef):
    name = "WEBSSO::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WEBSSO::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Forwards a request without doing SSO processing on it.",
                synopsis=("WEBSSO::disable",),
                snippet=(
                    "This command causes APM to forward a request without doing SSO\n"
                    "processing on it. If APM receives HTTP 401 response from server, 401\n"
                    "response is forwarded to the end user. The scope of this iRule command\n"
                    "is per HTTP request. Admin needs to execute it for each HTTP request."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WEBSSO::disable",
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
