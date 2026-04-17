# Enriched from F5 iRules reference documentation.
"""SSL::maximum_record_size -- Get or set the maximum egress record size."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__maximum_record_size.html"


@register
class SslMaximumRecordSizeCommand(CommandDef):
    name = "SSL::maximum_record_size"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::maximum_record_size",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the maximum egress record size.",
                synopsis=("SSL::maximum_record_size (SSL_RECORD_SIZE)?",),
                snippet=(
                    "SSL::maximum_record_size\n"
                    "  Returns the currently set maximum egress record size.\n"
                    "SSL::maximum_record_size #####\n"
                    "  Set the maximum egress record size."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    SSL::maximum_record_size 1234\n}"),
                return_value="SSL::maximum_record_size Returns the currently set maximum egress record size. SSL::maximum_record_size ##### There is no return value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::maximum_record_size (SSL_RECORD_SIZE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
