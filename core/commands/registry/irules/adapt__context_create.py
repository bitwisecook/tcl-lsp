# Enriched from F5 iRules reference documentation.
"""ADAPT::context_create -- Creates a new dynamic adaptation context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_create.html"


@register
class AdaptContextCreateCommand(CommandDef):
    name = "ADAPT::context_create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_create",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new dynamic adaptation context.",
                synopsis=("ADAPT::context_create (ADAPT_SIDE)? NAME",),
                snippet=(
                    "Creates a new dynamic adaptation context in the ADAPT filter on\n"
                    "the current or specified side of the virtual server connection\n"
                    "for which the iRule is being executed. Maybe called mulitple\n"
                    "times to dynamically create chains of adaptation contexts.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_create <name>\n"
                    "\n"
                    "    * Creates a dynamic context on the current side.\n"
                    "      This must be called from the request-adapt side, so has\n"
                    "      the same effect as ADAPT::context_create request <name>."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    # Configure a response context from the current (response) side.\n"
                    "    ADAPT::select $rsp_ctx2 ivs-icap-rsp2\n"
                    "    ADAPT::timeout $rsp_ctx2 2000\n"
                    "}"
                ),
                return_value="Returns the context handle.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_create (ADAPT_SIDE)? NAME",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP", "REQUESTADAPT"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
