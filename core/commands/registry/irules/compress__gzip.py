# Enriched from F5 iRules reference documentation.
"""COMPRESS::gzip -- Sets HTTP data compression criteria."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__gzip.html"


_av = make_av(_SOURCE)


@register
class CompressGzipCommand(CommandDef):
    name = "COMPRESS::gzip"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::gzip",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets HTTP data compression criteria.",
                synopsis=("COMPRESS::gzip (request | response)? (",),
                snippet=(
                    "Sets criteria for compressing HTTP responses.\n"
                    "\n"
                    "COMPRESS::gzip memory_level <level>\n"
                    "    Sets the gzip memory level.\n"
                    "\n"
                    "COMPRESS::gzip window_size <size>\n"
                    "    Sets the gzip window size.\n"
                    "\n"
                    "COMPRESS::gzip level <level>\n"
                    "    Specifies the amount and rate of compression."
                ),
                source=_SOURCE,
                examples=("when HTTP_RESPONSE {\n  COMPRESS::gzip level 9\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::gzip (request | response)? (",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "COMPRESS::gzip request",
                                "COMPRESS::gzip (request | response)? (",
                            ),
                            _av(
                                "response",
                                "COMPRESS::gzip response",
                                "COMPRESS::gzip (request | response)? (",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
