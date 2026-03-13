# Enriched from F5 iRules reference documentation.
"""FLOW::create_related -- Creates a related client side and server side flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOW__create_related.html"


@register
class FlowCreateRelatedCommand(CommandDef):
    name = "FLOW::create_related"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOW::create_related",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a related client side and server side flow.",
                synopsis=(
                    "FLOW::create_related (((-translation-loose) (-hairpin))#)? (FLOW_CREATE_RELATED_SUBCMDS)+",
                ),
                snippet=(
                    "Creates a related connection. Each related connection has two flows in it, a clientside flow and a serverside flow. The clientside flow is created using\n"
                    'the information provided in "clientflow" and serverside flow is created using the information provided in the "serverflow". Both these flows are linked\n'
                    "together and form a connection. BIGIP excepts that the the first packet always comes from the client side of the connection for all protocols except UDP.\n"
                    "The returned TCL handle points to the clientside flow. [FLOW::peer] command can be used to get a handle to the peer flow."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "            # LSN pool with prefix 4.4.4.0/30,port-range=2000-2005 and NAPT mode is configured. Parent connection is translated as follows\n"
                    "            # 10.10.0.1%1:60412 -> 10.20.0.1%1:9000 TO 4.4.4.1:1084  10.20.0.1:9000  tcp\n"
                    "            # Subscriber side: 10.10.0.1%1:60412 -> 10.20.0.1%1:9000\n"
                    "            # Internet side: 4.4.4.1:1084  10.20.0.1:9000\n"
                    "            # Below is an example of couple of related connections \n"
                    "            \n"
                    "            # Connection-1:"
                ),
                return_value="TCL handle for the client side flow. On error an exception is thrown with a message indicating the cause of failure. The string representation of the TCL handle can be used to retrieve the flow details.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOW::create_related (((-translation-loose) (-hairpin))#)? (FLOW_CREATE_RELATED_SUBCMDS)+",
                    options=(
                        OptionSpec(
                            name="-translation-loose",
                            detail="Option -translation-loose.",
                            takes_value=False,
                        ),
                        OptionSpec(name="-hairpin", detail="Option -hairpin.", takes_value=False),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_DATA", "SERVER_CONNECTED", "SERVER_DATA"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FLOW_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
