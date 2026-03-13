# Enriched from F5 iRules reference documentation.
"""ASM::threat_campaign -- Returns the list of threat campaigns."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__threat_campaign.html"


_av = make_av(_SOURCE)


@register
class AsmThreatCampaignCommand(CommandDef):
    name = "ASM::threat_campaign"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::threat_campaign",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the list of threat campaigns.",
                synopsis=("ASM::threat_campaign ( names | staged_names )",),
                snippet="Returns the list of threat campaigns.",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    '    log local0. "names=[ASM::threat_campaign names] staged_names=[ASM::threat_campaign staged_names]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::threat_campaign ( names | staged_names )",
                    arg_values={
                        0: (
                            _av(
                                "names",
                                "ASM::threat_campaign names",
                                "ASM::threat_campaign ( names | staged_names )",
                            ),
                            _av(
                                "staged_names",
                                "ASM::threat_campaign staged_names",
                                "ASM::threat_campaign ( names | staged_names )",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
