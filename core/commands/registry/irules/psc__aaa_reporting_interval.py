# Enriched from F5 iRules reference documentation.
"""PSC::aaa_reporting_interval -- Get or set AAA reporting interval."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__aaa_reporting_interval.html"


@register
class PscAaaReportingIntervalCommand(CommandDef):
    name = "PSC::aaa_reporting_interval"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::aaa_reporting_interval",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set AAA reporting interval.",
                synopsis=("PSC::aaa_reporting_interval (INTERVAL)?",),
                snippet="The PSC::aaa_reporting_interval command gets the AAA reporting interval or sets the AAA reporting interval when the optional value is given.",
                source=_SOURCE,
                return_value="Return AAA reporting interval when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::aaa_reporting_interval (INTERVAL)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
