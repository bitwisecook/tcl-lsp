# Enriched from F5 iRules reference documentation.
"""MESSAGE::field -- Various operations for a message's fields."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MESSAGE__field.html"


_av = make_av(_SOURCE)


@register
class MessageFieldCommand(CommandDef):
    name = "MESSAGE::field"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MESSAGE::field",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Various operations for a message's fields.",
                synopsis=("MESSAGE::field ( ('names') |",),
                snippet=(
                    "This command is used for below mentioned operations for a message's field.\n"
                    "This is valid for messages of the following protocols:\n"
                    "\n"
                    "    SIP"
                ),
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    "    switch ( [MESSAGE::proto] ) {\n"
                    '        "SIP" {\n'
                    '           if { [MESSAGE::type] eq "request" } {\n'
                    "              set uri [MESSAGE::field value ':uri']\n"
                    '              log local0. "Message\'s URI is : $uri"\n'
                    "           }\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns value depends on the subcommands. See description for more details.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MESSAGE::field ( ('names') |",
                    arg_values={
                        0: (_av("names", "MESSAGE::field names", "MESSAGE::field ( ('names') |"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
