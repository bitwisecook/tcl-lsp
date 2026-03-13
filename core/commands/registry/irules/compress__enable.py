# Enriched from F5 iRules reference documentation.
"""COMPRESS::enable -- Enables compression for the current HTTP response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__enable.html"


_av = make_av(_SOURCE)


@register
class CompressEnableCommand(CommandDef):
    name = "COMPRESS::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables compression for the current HTTP response.",
                synopsis=("COMPRESS::enable (request | response)?",),
                snippet=(
                    "COMPRESS::enable\n"
                    "    Enables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective.\n"
                    "\n"
                    "COMPRESS::enable request\n"
                    '    Enables compression for the current HTTP request. As with the normal COMPRESS::enable, you must enable the HTTP compression profile setting Selective Compression (version 11.x). You must also enable an HTTP profile with "request-chunking selective" selected.'
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '  if { [HTTP::header Content-Type] contains "text/html;charset=UTF-8"} {\n'
                    "    COMPRESS::enable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::enable (request | response)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "COMPRESS::enable request",
                                "COMPRESS::enable (request | response)?",
                            ),
                            _av(
                                "response",
                                "COMPRESS::enable response",
                                "COMPRESS::enable (request | response)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
