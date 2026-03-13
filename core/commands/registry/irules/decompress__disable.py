# Enriched from F5 iRules reference documentation.
"""DECOMPRESS::disable -- Disable DECOMPRESS feature on current flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DECOMPRESS__disable.html"


_av = make_av(_SOURCE)


@register
class DecompressDisableCommand(CommandDef):
    name = "DECOMPRESS::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DECOMPRESS::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disable DECOMPRESS feature on current flow.",
                synopsis=("DECOMPRESS::disable (request | response)?",),
                snippet="Disable DECOMPRESS feature on current flow.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DECOMPRESS::disable (request | response)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "DECOMPRESS::disable request",
                                "DECOMPRESS::disable (request | response)?",
                            ),
                            _av(
                                "response",
                                "DECOMPRESS::disable response",
                                "DECOMPRESS::disable (request | response)?",
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
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
