# Enriched from F5 iRules reference documentation.
"""COMPRESS::method -- Specifies the preferred compression algorithm."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__method.html"


_av = make_av(_SOURCE)


@register
class CompressMethodCommand(CommandDef):
    name = "COMPRESS::method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::method",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Specifies the preferred compression algorithm.",
                synopsis=("COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",),
                snippet=(
                    "Specifies the preferred compression algorithm.\n"
                    "\n"
                    "COMPRESS::method prefer [ ’gzip’ | ’deflate’ ]\n"
                    "    Specifies the preferred compression algorithm, either gzip or deflate."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST_SEND {\n COMPRESS::method prefer gzip\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                    arg_values={
                        0: (
                            _av(
                                "gzip",
                                "COMPRESS::method gzip",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                            ),
                            _av(
                                "deflate",
                                "COMPRESS::method deflate",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                            ),
                            _av(
                                "request",
                                "COMPRESS::method request",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                            ),
                            _av(
                                "response",
                                "COMPRESS::method response",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
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
