# Enriched from F5 iRules reference documentation.
"""XLAT::src_config -- Retrieve the source-translation configuration."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__src_config.html"


@register
class XlatSrcConfigCommand(CommandDef):
    name = "XLAT::src_config"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::src_config",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Retrieve the source-translation configuration.",
                synopsis=("XLAT::src_config",),
                snippet=(
                    "Return the source translation configuration as a list. With the values in the following order: type,source translation object/pool.\n"
                    "\n"
                    "type - The source translation type as a string. Possible values are: NONE, AUTOMAP, SNAT, LSN, SECURITY-DYNAMIC-PAT, SECURITY-DYNAMIC-NAT, SECURITY-STATIC-NAT, SECURITY-STATIC-PAT\n"
                    "pool - the source translation object/pool name. NA when not applicable(NONE and AUTOMAP types)."
                ),
                source=_SOURCE,
                examples=('when SA_PICKED {\n    log local0. "[XLAT::src_config]"\n}'),
                return_value="Return the source translation configuration as a list. On error an exception is thrown with a message indicating the cause of failure.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::src_config",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            excluded_events=("RULE_INIT",),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
