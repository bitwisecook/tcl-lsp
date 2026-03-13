# Enriched from F5 iRules reference documentation.
"""HTTP2::disable -- Changes the HTTP2 filter from full parsing to passthrough mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__disable.html"


_av = make_av(_SOURCE)


@register
class Http2DisableCommand(CommandDef):
    name = "HTTP2::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the HTTP2 filter from full parsing to passthrough mode.",
                synopsis=("HTTP2::disable ('clientside')? ('serverside')? ('discard')?",),
                snippet=(
                    "Changes the HTTP2 filter from full parsing to passthrough mode. This\n"
                    "command is useful when using an HTTP2 profile with an application that\n"
                    "proxies data over HTTP."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    if { [HTTP::uri] contains "http1_backend"} {\n'
                    "        HTTP2::disable serverside\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::disable ('clientside')? ('serverside')? ('discard')?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "HTTP2::disable clientside",
                                "HTTP2::disable ('clientside')? ('serverside')? ('discard')?",
                            ),
                            _av(
                                "serverside",
                                "HTTP2::disable serverside",
                                "HTTP2::disable ('clientside')? ('serverside')? ('discard')?",
                            ),
                            _av(
                                "discard",
                                "HTTP2::disable discard",
                                "HTTP2::disable ('clientside')? ('serverside')? ('discard')?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
