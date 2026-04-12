# Generated from F5 iRules reference documentation -- do not edit manually.
"""COMPRESS::nodelay -- F5 iRules command `COMPRESS::nodelay`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__nodelay.html"


_av = make_av(_SOURCE)


@register
class CompressNodelayCommand(CommandDef):
    name = "COMPRESS::nodelay"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::nodelay",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `COMPRESS::nodelay`.",
                synopsis=("COMPRESS::nodelay (request | response)?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::nodelay (request | response)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "COMPRESS::nodelay request",
                                "COMPRESS::nodelay (request | response)?",
                            ),
                            _av(
                                "response",
                                "COMPRESS::nodelay response",
                                "COMPRESS::nodelay (request | response)?",
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
