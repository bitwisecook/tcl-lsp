# Enriched from F5 iRules reference documentation.
"""ADAPT::enable -- Enables, disables or returns the enable state."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__enable.html"


@register
class AdaptEnableCommand(CommandDef):
    name = "ADAPT::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables, disables or returns the enable state.",
                synopsis=("ADAPT::enable (ADAPT_CTX)? (ADAPT_SIDE)? (BOOLEAN)?",),
                snippet=(
                    "The ADAPT::enable command enables, disables or returns the enable\n"
                    "state of the ADAPT filter on the current or specified side of the\n"
                    "virtual server connection for which the iRule is being executed."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "     ADAPT::enable true\n"
                    "     ADAPT::enable response false\n"
                    "}"
                ),
                return_value="Returns the current of modified enable state.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::enable (ADAPT_CTX)? (ADAPT_SIDE)? (BOOLEAN)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"HTTP", "REQUESTADAPT", "RESPONSEADAPT"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
