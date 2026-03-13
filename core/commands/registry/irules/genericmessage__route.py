# Enriched from F5 iRules reference documentation.
"""GENERICMESSAGE::route -- Adds, deletes, or looks up message routes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__route.html"


_av = make_av(_SOURCE)


@register
class GenericmessageRouteCommand(CommandDef):
    name = "GENERICMESSAGE::route"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GENERICMESSAGE::route",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Adds, deletes, or looks up message routes.",
                synopsis=(
                    "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                ),
                snippet=(
                    "The GENERICMESSAGE::route command allows you to add, delete, or lookup\n"
                    "message routes."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    GENERICMESSAGE::route add dst "client-[IP::remote_addr]" host "[IP::remote_addr]:[TCP::remote_port]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "GENERICMESSAGE::route virtual",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                            ),
                            _av(
                                "add",
                                "GENERICMESSAGE::route add",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                            ),
                            _av(
                                "delete",
                                "GENERICMESSAGE::route delete",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                            ),
                            _av(
                                "lookup",
                                "GENERICMESSAGE::route lookup",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
