# Enriched from F5 iRules reference documentation.
"""IKE::san_dirname -- something"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IKE__san_dirname.html"


@register
class IkeSanDirnameCommand(CommandDef):
    name = "IKE::san_dirname"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IKE::san_dirname",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="something",
                synopsis=("IKE::san_dirname (ANY_CHARS)*",),
                snippet="something",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IKE::san_dirname (ANY_CHARS)*",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
