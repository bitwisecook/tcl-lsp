# Enriched from F5 iRules reference documentation.
"""HTTP::reject_reason -- Returns the reason HTTP is aborting"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__reject_reason.html"


_av = make_av(_SOURCE)


@register
class HttpRejectReasonCommand(CommandDef):
    name = "HTTP::reject_reason"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::reject_reason",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the reason HTTP is aborting",
                synopsis=("HTTP::reject_reason ('as_num')?",),
                snippet="This returns the reason HTTP aborted the connection, either as a string, or as a numeric id suitable for an error code.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REJECT {\n"
                    '    log local0. "HTTP Aborted:" [HTTP::reject_reason]\n'
                    '    log local0. "Error code:" [HTTP::reject_reason as_num]\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::reject_reason ('as_num')?",
                    arg_values={
                        0: (
                            _av(
                                "as_num",
                                "HTTP::reject_reason as_num",
                                "HTTP::reject_reason ('as_num')?",
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
