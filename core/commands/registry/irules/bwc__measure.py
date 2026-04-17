# Enriched from F5 iRules reference documentation.
"""BWC::measure -- This command allows you to measure rate for a particular traffic flow or flows belonging to the bwc instance."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__measure.html"


_av = make_av(_SOURCE)


@register
class BwcMeasureCommand(CommandDef):
    name = "BWC::measure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::measure",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to measure rate for a particular traffic flow or flows belonging to the bwc instance.",
                synopsis=("BWC::measure ( ('start' | 'stop') |",),
                snippet="After a flow has been assigned a policy, user can start or stop measurement on a per policy basis or on a per flow basis. Once the measurement is started the measured bandwidth can be read by the user using 'BWC::measure get ..' iRules. Optionally users can direct the bandwidth measurement results to a 'log publisher' configured on the BIGIP system. Based on the log_publisher setting the measurement results will be logged to the log server indicated in the 'log_publisher'. It is usually an external high speed log server.",
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n        TCP::collect     set count 0\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::measure ( ('start' | 'stop') |",
                    arg_values={
                        0: (
                            _av(
                                "start", "BWC::measure start", "BWC::measure ( ('start' | 'stop') |"
                            ),
                            _av("stop", "BWC::measure stop", "BWC::measure ( ('start' | 'stop') |"),
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
