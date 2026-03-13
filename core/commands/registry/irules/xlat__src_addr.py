# Enriched from F5 iRules reference documentation.
"""XLAT::src_addr -- Retrieve the source translation address."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__src_addr.html"


@register
class XlatSrcAddrCommand(CommandDef):
    name = "XLAT::src_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::src_addr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Retrieve the source translation address.",
                synopsis=("XLAT::src_addr",),
                snippet="Retrieve the source translation address.",
                source=_SOURCE,
                examples=('when SA_PICKED {\n    log local0. "[XLAT::src_addr]"\n}'),
                return_value="Return the string representation of the source translation address.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::src_addr",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(also_in=frozenset({"SA_PICKED"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
