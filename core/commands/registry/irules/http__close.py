# Enriched from F5 iRules reference documentation.
"""HTTP::close -- Closes the HTTP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__close.html"


@register
class HttpCloseCommand(CommandDef):
    name = "HTTP::close"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::close",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Closes the HTTP connection.",
                synopsis=("HTTP::close",),
                snippet="Closes the HTTP connection.",
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n  set method [HTTP::method]\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::close",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
