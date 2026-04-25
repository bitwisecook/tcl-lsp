# Enriched from F5 iRules reference documentation.
"""DECOMPRESS::enable -- Enable DECOMPRESS feature on current flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DECOMPRESS__enable.html"


_av = make_av(_SOURCE)


@register
class DecompressEnableCommand(CommandDef):
    name = "DECOMPRESS::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DECOMPRESS::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable DECOMPRESS feature on current flow.",
                synopsis=("DECOMPRESS::enable (request | response)?",),
                snippet="Enable DECOMPRESS feature on current flow.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DECOMPRESS::enable (request | response)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "DECOMPRESS::enable request",
                                "DECOMPRESS::enable (request | response)?",
                            ),
                            _av(
                                "response",
                                "DECOMPRESS::enable response",
                                "DECOMPRESS::enable (request | response)?",
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
