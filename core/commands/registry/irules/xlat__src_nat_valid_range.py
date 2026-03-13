# Enriched from F5 iRules reference documentation.
"""XLAT::src_nat_valid_range -- Return a list of valid source-translation endpoint ranges."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__src_nat_valid_range.html"


@register
class XlatSrcNatValidRangeCommand(CommandDef):
    name = "XLAT::src_nat_valid_range"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::src_nat_valid_range",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return a list of valid source-translation endpoint ranges.",
                synopsis=("XLAT::src_nat_valid_range",),
                snippet="Returns a list of lists containing valid source-translation addresses and port-ranges (source-translation endpoints). This command must be called every time a new connection/listener needs to be created to retrieve the valid source translation information. This data must not be cached. Only the source-translation address used by the parent with a valid port-range is returned, not all of the endpoints in the source-translation object/pool. In PBA mode multiple source-translation addresses and port-ranges can be returned if the client has multiple active blocks.",
                source=_SOURCE,
                examples=(
                    "when SA_PICKED {\n"
                    "    set sa_list [XLAT::src_nat_valid_range]\n"
                    "\n"
                    "    foreach sa $sa_list {\n"
                    '        log local0. "address=[lindex $sa 0] port-start=[lindex $sa 1] port-end=[lindex $sa 2]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="A list of lists containing the valid source-translation endpoint ranges. For e.g. { {address-1 start-port end-port} {address-2 start-port end-port} ...}",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::src_nat_valid_range",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_DATA", "SA_PICKED", "SERVER_CONNECTED", "SERVER_DATA"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
