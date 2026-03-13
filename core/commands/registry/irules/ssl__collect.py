# Enriched from F5 iRules reference documentation.
"""SSL::collect -- Collect plaintext data after SSL offloading."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__collect.html"


@register
class SslCollectCommand(CommandDef):
    name = "SSL::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collect plaintext data after SSL offloading.",
                synopsis=("SSL::collect (LENGTH)?",),
                snippet="Starts the collection of plaintext data either indefinitely or for the specified amount of data. On successful collection, the corresponding data event is triggered. For clientside collection, the CLIENTSSL_DATA event is triggered. For serverside collection, the SERVERSSL_DATA event is triggered.",
                source=_SOURCE,
                examples=("when SERVERSSL_HANDSHAKE {\n    SSL::collect\n}"),
                return_value="SSL::collect [<length>] Starts the collection of plaintext data either indefinitely or for the specified amount of data. When is specified, the data event will not be triggered until that length has been collected.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::collect (LENGTH)?",
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
