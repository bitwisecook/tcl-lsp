# Enriched from F5 iRules reference documentation.
"""listen -- Sets up a related ephemeral listener to allow an incoming related connection to be established."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/listen.html"


_av = make_av(_SOURCE)


@register
class ListenCommand(CommandDef):
    name = "listen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="listen",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets up a related ephemeral listener to allow an incoming related connection to be established.",
                synopsis=("listen (<'proto' UNSIGNED_SHORT> |",),
                snippet=(
                    "Sets up a related ephemeral listener to allow an incoming related\n"
                    "connection to be established. The source address and/or port of the\n"
                    "related connection is unknown but the destination address and port are\n"
                    "known."
                ),
                source=_SOURCE,
                examples=('when RULE_INIT {\n      set my_port ""\n   }'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="listen (<'proto' UNSIGNED_SHORT> |",
                    arg_values={
                        0: (_av("proto", "listen proto", "listen (<'proto' UNSIGNED_SHORT> |"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True, also_in=frozenset({"PERSIST_DOWN"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
