# Enriched from F5 iRules reference documentation.
"""TCP::proxybuffer -- Sets proxy buffer low and high thresholds."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__proxybuffer.html"


_av = make_av(_SOURCE)


@register
class TcpProxybufferCommand(CommandDef):
    name = "TCP::proxybuffer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::proxybuffer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets proxy buffer low and high thresholds.",
                synopsis=("TCP::proxybuffer ('auto' | (LOW HIGH))",),
                snippet="Sets thresholds at which the proxy buffer accepts (low) and stops accepting (high) new data, in bytes.",
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n    TCP::proxybuffer 100000 500000\n}"),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::proxybuffer ('auto' | (LOW HIGH))",
                    arg_values={
                        0: (
                            _av(
                                "auto",
                                "TCP::proxybuffer auto",
                                "TCP::proxybuffer ('auto' | (LOW HIGH))",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(server_side=True, transport="tcp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
