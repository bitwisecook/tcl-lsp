# Enriched from F5 iRules reference documentation.
"""HTTP2::stream -- Gets or sets the stream attributes including id and priority."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__stream.html"


@register
class Http2StreamCommand(CommandDef):
    name = "HTTP2::stream"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::stream",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the stream attributes including id and priority.",
                synopsis=("HTTP2::stream (id | (priority (PRIORITY)?))?",),
                snippet=(
                    "This command can be used to determine the stream attributes including id and priority. This command can also be used to set the priority for a current active stream.\n"
                    "\n"
                    "HTTP2::stream\n"
                    "    Returns the stream id. Returns 0 if HTTP/2 is not active.\n"
                    "\n"
                    "HTTP2::stream id\n"
                    "    Returns the stream id. Returns 0 if HTTP/2 is not active.\n"
                    "\n"
                    "HTTP2::stream priority\n"
                    "    Returns the priority of the current stream.\n"
                    "\n"
                    "HTTP2::stream priority <priority>\n"
                    "    Sets the priority of the current active stream. The return is 0 is the priority was set. Return is an error if the priority is out of bounds (exceeding 8 bits)."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if {[HTTP2::version] != 0} {\n"
                    "        set new_pri [URI::query [HTTP::uri] pri]\n"
                    '        if { $new_pri != "" } {\n'
                    "             HTTP2::stream priority $new_pri\n"
                    "        }\n"
                    '        HTTP::header insert "X-HTTP2-Values stream/pritority " "[HTTP2::stream]/[HTTP2::stream priority]"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::stream (id | (priority (PRIORITY)?))?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP2"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
