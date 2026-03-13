# Enriched from F5 iRules reference documentation.
"""PROFILE::vdi -- Returns the value of a VDI profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__vdi.html"


@register
class ProfileVdiCommand(CommandDef):
    name = "PROFILE::vdi"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::vdi",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of a VDI profile setting.",
                synopsis=("PROFILE::vdi ATTR",),
                snippet="Returns the current value of the specified setting in the assigned VDI profile.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0. "\\[PROFILE::vdi msrdp_ntlm_auth_name\\]:    [PROFILE::vdi msrdp_ntlm_auth_name]"\n'
                    '    log local0. "\\[PROFILE::vdi citrix_storefront_replacement\\]:   [PROFILE::vdi citrix_storefront_replacement]"\n'
                    "}"
                ),
                return_value="Returns the current value of the specified setting in the assigned VDI profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::vdi ATTR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
