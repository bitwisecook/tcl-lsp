# Enriched from F5 iRules reference documentation.
"""HTTP::passthrough_reason -- Returns the reason for the most recent switch to pass-through mode by the HTTP filter."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__passthrough_reason.html"


_av = make_av(_SOURCE)


@register
class HttpPassthroughReasonCommand(CommandDef):
    name = "HTTP::passthrough_reason"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::passthrough_reason",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the reason for the most recent switch to pass-through mode by the HTTP filter.",
                synopsis=("HTTP::passthrough_reason ('as_num')?",),
                snippet=(
                    "This command returns the reason for the most recent switch to\n"
                    "pass-through mode by the HTTP filter."
                ),
                source=_SOURCE,
                examples="when HTTP_DISABLED { log local2. [HTTP::passthrough_reason] }",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::passthrough_reason ('as_num')?",
                    arg_values={
                        0: (
                            _av(
                                "as_num",
                                "HTTP::passthrough_reason as_num",
                                "HTTP::passthrough_reason ('as_num')?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
