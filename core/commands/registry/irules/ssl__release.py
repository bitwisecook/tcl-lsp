# Enriched from F5 iRules reference documentation.
"""SSL::release -- Releases the collected plaintext data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__release.html"


@register
class SslReleaseCommand(CommandDef):
    name = "SSL::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the collected plaintext data.",
                synopsis=("SSL::release (LENGTH)?",),
                snippet="Releases the collected plaintext data to the next layer/filter up.",
                source=_SOURCE,
                examples=(
                    "when SERVERSSL_DATA {\n"
                    "    # Do something with the decrypted data\n"
                    "    set payload [SSL::payload]\n"
                    "\n"
                    "    # Release the payload\n"
                    "    SSL::release\n"
                    "}"
                ),
                return_value="SSL::release [<length>] Releases the collected plaintext data to the next layer/filter up.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::release (LENGTH)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp", profiles=frozenset({"CLIENTSSL", "SERVERSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
