# Enriched from F5 iRules reference documentation.
"""SSL::allow_dynamic_record_sizing -- Get or set dynamic record sizing."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__allow_dynamic_record_sizing.html"


@register
class SslAllowDynamicRecordSizingCommand(CommandDef):
    name = "SSL::allow_dynamic_record_sizing"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::allow_dynamic_record_sizing",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set dynamic record sizing.",
                synopsis=("SSL::allow_dynamic_record_sizing (ZERO_ONE)?",),
                snippet=(
                    "SSL::allow_dynamic_record_sizing\n"
                    "  Returns the currently set value for allowing dynamic record sizing\n"
                    "SSL::allow_dynamic_record_sizing ( 0 | 1 )\n"
                    "  0 disables dynamic record sizing, 1 enables it.\n"
                    "  Dynamic record sizing, when using protocols such as HTTP, can increase respnonsiveness of a website."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    SSL::allow_dynamic_record_sizing 1\n}"),
                return_value="SSL::allow_dynamic_record_sizing Returns the currently set dynamic record sizing value. SSL::allow_dynamic_record_sizing [0|1] There is no return value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::allow_dynamic_record_sizing (ZERO_ONE)?",
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
