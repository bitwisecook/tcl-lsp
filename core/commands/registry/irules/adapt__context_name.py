# Enriched from F5 iRules reference documentation.
"""ADAPT::context_name -- Gets the name of a dynamic adaptation context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_name.html"


@register
class AdaptContextNameCommand(CommandDef):
    name = "ADAPT::context_name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_name",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the name of a dynamic adaptation context.",
                synopsis=("ADAPT::context_name ADAPT_CTX",),
                snippet=(
                    "Obtains the name of an adaptation context. The name of a\n"
                    "dynamic context was specified when it was created. The name\n"
                    "of a static (profile) context is that of the ADAPT profile\n"
                    "on the side of the virtual server where the context resides.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_name <context>"
                ),
                source=_SOURCE,
                examples=(
                    "when ADAPT_RESPONSE_RESULT {\n"
                    "   set ctx [ADAPT::context_current]\n"
                    "   set ctx_name [ADAPT::context_name $ctx]\n"
                    '   log local0. "ADAPT_RESPONSE_RESULT in context $ctx_name"\n'
                    "}"
                ),
                return_value="Returns the context name.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_name ADAPT_CTX",
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
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
