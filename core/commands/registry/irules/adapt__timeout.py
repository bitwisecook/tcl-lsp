# Enriched from F5 iRules reference documentation.
"""ADAPT::timeout -- Sets or returns the timeout attribute."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__timeout.html"


@register
class AdaptTimeoutCommand(CommandDef):
    name = "ADAPT::timeout"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::timeout",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the timeout attribute.",
                synopsis=("ADAPT::timeout (ADAPT_CTX)? (ADAPT_SIDE)? (TIMEOUT_VALUE)?",),
                snippet=(
                    "The ADAPT::timeout command sets or returns the timeout attribute\n"
                    "of the ADAPT filter on the current or specified side of the\n"
                    "virtual server connection for which the iRule is being executed.\n"
                    "The timeout (in milliseconds) is how long ADAPT will wait for\n"
                    "a result from the internal virtual server before deciding the\n"
                    "service is down."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '     if { [HTTP::header "Content-Type"] contains "image" } {\n'
                    "        ADAPT::select ivs-icap-image\n"
                    "        ADAPT::timeout 500\n"
                    "     }\n"
                    '     if { [HTTP::header "Content-Type"] contains "video" } {\n'
                    "        ADAPT::select ivs-icap-video\n"
                    "        ADAPT::timeout 2000\n"
                    "     }\n"
                    " }"
                ),
                return_value="Returns the current or modified timeout in milliseconds.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::timeout (ADAPT_CTX)? (ADAPT_SIDE)? (TIMEOUT_VALUE)?",
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
