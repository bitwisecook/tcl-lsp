# Enriched from F5 iRules reference documentation.
"""HTTP2::enable -- Changes the HTTP2 filter from passthrough to full parsing mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__enable.html"


_av = make_av(_SOURCE)


@register
class Http2EnableCommand(CommandDef):
    name = "HTTP2::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the HTTP2 filter from passthrough to full parsing mode.",
                synopsis=("HTTP2::enable ('clientside')? ('serverside')?",),
                snippet=(
                    "Changes the HTTP2 filter from passthrough to full parsing mode. This\n"
                    "command is useful when it is unknown whether HTTP2 will be used at\n"
                    "configuration time, but known at runtime.  This command is quite tricky\n"
                    "to get right.  In general, it may be better to use HTTP2::disable."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    HTTP2::disable\n"
                    "    if { !([IP::addr [IP::client_addr] eq 10.0.0.0/8]) } {\n"
                    "        HTTP2::enable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::enable ('clientside')? ('serverside')?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "HTTP2::enable clientside",
                                "HTTP2::enable ('clientside')? ('serverside')?",
                            ),
                            _av(
                                "serverside",
                                "HTTP2::enable serverside",
                                "HTTP2::enable ('clientside')? ('serverside')?",
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
