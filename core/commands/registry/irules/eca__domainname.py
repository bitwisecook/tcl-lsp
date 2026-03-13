# Generated from F5 iRules reference documentation -- do not edit manually.
"""ECA::domainname -- Returns NTLM authenticating user's domain name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ECA__domainname.html"


@register
class EcaDomainnameCommand(CommandDef):
    name = "ECA::domainname"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ECA::domainname",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns NTLM authenticating user's domain name.",
                synopsis=("ECA::domainname",),
                snippet="The ECA::domainname command returns NTLM returns authenticating user's domain name",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ECA::domainname",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
