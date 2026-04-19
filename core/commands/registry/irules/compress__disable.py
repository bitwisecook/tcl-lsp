# Enriched from F5 iRules reference documentation.
"""COMPRESS::disable -- Disables compression for the current HTTP response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__disable.html"


_av = make_av(_SOURCE)


@register
class CompressDisableCommand(CommandDef):
    name = "COMPRESS::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables compression for the current HTTP response.",
                synopsis=("COMPRESS::disable (request | response)?",),
                snippet=(
                    "Disables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective.\n"
                    "\n"
                    "COMPRESS::disable\n"
                    "    Disables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [TCP::mss] >= 1280 } {\n"
                    "    COMPRESS::disable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::disable (request | response)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "COMPRESS::disable request",
                                "COMPRESS::disable (request | response)?",
                            ),
                            _av(
                                "response",
                                "COMPRESS::disable response",
                                "COMPRESS::disable (request | response)?",
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
