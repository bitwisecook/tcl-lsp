# Enriched from F5 iRules reference documentation.
"""ADAPT::context_current -- Gets the current context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_current.html"


@register
class AdaptContextCurrentCommand(CommandDef):
    name = "ADAPT::context_current"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_current",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the current context.",
                synopsis=("ADAPT::context_current",),
                snippet=(
                    "Obtains a handle for the current context. The current context\n"
                    "is usually that in which the event occurred from which this\n"
                    "command was issued.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_current"
                ),
                source=_SOURCE,
                examples=(
                    "when ADAPT_REQUEST_RESULT {\n"
                    "    set ctx [ADAPT::context_current]\n"
                    "    if {$ctx == $req_ctx2 && $need_another_ctx} {\n"
                    "        set req_ctx3 [ADAPT::context_create my_req_ctx3]\n"
                    "        ADAPT::select $req_ctx3 ivs-icap-req3\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the handle of the current context.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_current",
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
