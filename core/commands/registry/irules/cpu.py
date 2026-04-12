# Enriched from F5 iRules reference documentation.
"""cpu -- Returns the average TMM cpu load for the given interval."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/cpu.html"


@register
class CpuCommand(CommandDef):
    name = "cpu"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="cpu",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the average TMM cpu load for the given interval.",
                synopsis=("cpu usage (",),
                snippet=(
                    "The cpu usage command returns the average TMM cpu load for the given\n"
                    "interval. All averages are exponential weighted moving averages over\n"
                    "the interval."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if{ [cpu usage 5sec] <= 1} {\n"
                    "    pool1\n"
                    "  } else {\n"
                    '    HTTP::redirect "http://anotherpool.com"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cpu usage (",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
