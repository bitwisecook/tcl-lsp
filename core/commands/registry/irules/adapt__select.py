# Enriched from F5 iRules reference documentation.
"""ADAPT::select -- Sets or returns the internal virtual server (IVS) selection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__select.html"


@register
class AdaptSelectCommand(CommandDef):
    name = "ADAPT::select"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::select",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the internal virtual server (IVS) selection.",
                synopsis=("ADAPT::select (ADAPT_CTX)? (ADAPT_SIDE)? (NAME)?",),
                snippet=(
                    "The ADAPT::select command returns or selects the name of\n"
                    "the internal virtual server (IVS) associated with the ADAPT\n"
                    "filter on the current or specified side of the virtual server\n"
                    "connection for which the iRule is being executed."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '     if { [HTTP::header "Content-Type"] contains "image" } {\n'
                    "        ADAPT::select ivs-icap-image\n"
                    "        ADAPT::preview_size 10000\n"
                    "        ADAPT::enable yes\n"
                    "     }\n"
                    '     if { [HTTP::header "Content-Type"] contains "video" } {\n'
                    "        ADAPT::select ivs-icap-video\n"
                    "        ADAPT::preview_size 30000\n"
                    "        ADAPT::enable yes\n"
                    "     }\n"
                    "}"
                ),
                return_value="Returns the current or new internal virtual server (IVS) name.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::select (ADAPT_CTX)? (ADAPT_SIDE)? (NAME)?",
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
