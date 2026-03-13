# Curated community template proc.
"""http_content_len_max -- Enforce a maximum Content-Length or reject."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/http_content_len_max.html"


@register
class HttpContentLenMaxProc(CommandDef):
    name = "http_content_len_max"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_content_len_max",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary=(
                    "Return the HTTP Content-Length up to a maximum size "
                    "(default 1024), or reject if the header is not a valid integer."
                ),
                synopsis=(
                    "call http_content_len_max",
                    "call http_content_len_max 8192",
                ),
                snippet=(
                    "Returns the `Content-Length` header value clamped to *size* "
                    "(default 1024).\n"
                    "\n"
                    "  - `Content-Length: 512` → returns `512`\n"
                    "  - `Content-Length: 2048` with default size → returns `1024`\n"
                    "  - `Content-Length: 2048` with size `8192` → returns `2048`\n"
                    "  - No `Content-Length` header → returns the empty string\n"
                    "  - Non-integer `Content-Length` (RFC 2616 Section 14.13 "
                    "violation) → calls `reject`"
                ),
                source=_SOURCE,
                examples=(
                    "# Collect up to the default 1024 bytes\n"
                    "when HTTP_REQUEST priority 500 {\n"
                    "    HTTP::collect [call http_content_len_max]\n"
                    "}\n"
                    "\n"
                    "# Collect up to 8192 bytes\n"
                    "when HTTP_REQUEST priority 500 {\n"
                    "    HTTP::collect [call http_content_len_max 8192]\n"
                    "}"
                ),
                return_value=(
                    "The effective Content-Length (clamped to *size*), "
                    "the empty string if no header is present, "
                    "or the connection is rejected."
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_content_len_max ?max_size?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_BODY,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
