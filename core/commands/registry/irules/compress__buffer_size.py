# Enriched from F5 iRules reference documentation.
"""COMPRESS::buffer_size -- Sets the compression buffer size."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__buffer_size.html"


_av = make_av(_SOURCE)


@register
class CompressBufferSizeCommand(CommandDef):
    name = "COMPRESS::buffer_size"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::buffer_size",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the compression buffer size.",
                synopsis=("COMPRESS::buffer_size (request | response)? NONNEGATIVE_INTEGER",),
                snippet=(
                    "COMPRESS::buffer_size <value>\n"
                    "    Sets the compression buffer size according to the value you specify in bytes."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '  if { [HTTP::header Content-Type] contains "text/html;charset=UTF-8"} {\n'
                    "    COMPRESS::buffer_size 10240\n"
                    "    COMPRESS::enable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::buffer_size (request | response)? NONNEGATIVE_INTEGER",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "COMPRESS::buffer_size request",
                                "COMPRESS::buffer_size (request | response)? NONNEGATIVE_INTEGER",
                            ),
                            _av(
                                "response",
                                "COMPRESS::buffer_size response",
                                "COMPRESS::buffer_size (request | response)? NONNEGATIVE_INTEGER",
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
