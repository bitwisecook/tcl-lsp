# Enriched from F5 iRules reference documentation.
"""ADAPT::preview_size -- Sets or returns the preview-size attribute."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__preview_size.html"


@register
class AdaptPreviewSizeCommand(CommandDef):
    name = "ADAPT::preview_size"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::preview_size",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the preview-size attribute.",
                synopsis=("ADAPT::preview_size (ADAPT_CTX)? (ADAPT_SIDE)? (SIZE)?",),
                snippet=(
                    "The ADAPT::preview_size command sets or returns the preview-size\n"
                    "attribute of the ADAPT filter on the current or specified side of\n"
                    "the virtual server connection for which the iRule is being executed."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '    if { [HTTP::header "Content-Type"] contains "image" } {\n'
                    "        ADAPT::select ivs-icap-image\n"
                    "        ADAPT::preview_size 10000\n"
                    "    }\n"
                    '    if { [HTTP::header "Content-Type"] contains "video" } {\n'
                    "       ADAPT::select ivs-icap-video\n"
                    "       ADAPT::preview_size 30000\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the current or modified preview size (bytes).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::preview_size (ADAPT_CTX)? (ADAPT_SIDE)? (SIZE)?",
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
