# Generated from F5 iRules reference documentation -- do not edit manually.
"""ECA::select -- Selects one of NTLM Authentication configuration name and Kerberos Authentication configuration name or both"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ECA__select.html"


@register
class EcaSelectCommand(CommandDef):
    name = "ECA::select"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ECA::select",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Selects one of NTLM Authentication configuration name and Kerberos Authentication configuration name or both",
                synopsis=("ECA::select",),
                snippet="ECA::select is to select one of NTLM Authentication configuration name and Kerberos Authentication configuration name or both used when enforcing NTLM authentication or Kerberos authentication or Kerberos NTLM fallback option respectively for this particular connection.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ECA::select",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
