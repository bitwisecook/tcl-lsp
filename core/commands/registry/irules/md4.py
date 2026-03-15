# Generated from F5 iRules reference documentation -- do not edit manually.
"""md4 -- Returns the RSA MD4 Message Digest Algorithm message digest of the specified string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/md4.html"


@register
class Md4Command(CommandDef):
    name = "md4"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="md4",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the RSA MD4 Message Digest Algorithm message digest of the specified string.",
                synopsis=("md4",),
                snippet="Returns the RSA Data Security, Inc.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="md4",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
