# Enriched from F5 iRules reference documentation.
"""BIGPROTO::enable_fix_reset -- Enable or Disable Reset of FIX Protocol Connections"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BIGPROTO__enable_fix_reset.html"


@register
class BigprotoEnableFixResetCommand(CommandDef):
    name = "BIGPROTO::enable_fix_reset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BIGPROTO::enable_fix_reset",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable or Disable Reset of FIX Protocol Connections",
                synopsis=("BIGPROTO::enable_fix_reset BOOLEAN",),
                snippet="When set to disabled, TCP RST frame will not be sent when BIG-IP detects there is a hash collision on ePVA offloading of FIX flows. Instead, it will try to re-offload the connection.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    BIGPROTO::enable_fix_reset true\n"
                    "    BIGPROTO::enable_fix_reset false\n"
                    "            }"
                ),
                return_value="none",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BIGPROTO::enable_fix_reset BOOLEAN",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
