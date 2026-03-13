# Enriched from F5 iRules reference documentation.
"""forward -- Sets the connection to forward IP packets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/forward.html"


@register
class ForwardCommand(CommandDef):
    name = "forward"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="forward",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the connection to forward IP packets.",
                synopsis=("forward",),
                snippet=(
                    "Sets the connection to forward IP packets. This is strict forwarding\n"
                    "and will bypass any pool configured on the virtual server.\n"
                    "The request will be forwarded out the appropriate interface according\n"
                    "to the routes in the LTM routing table. No destination address or port\n"
                    "translation is performed."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [class match [IP::client_addr] equals my_hosts_class]} {\n"
                    "    snat 192.168.100.12\n"
                    "  } else {\n"
                    "    forward\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="forward",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True, also_in=frozenset({"PERSIST_DOWN"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
