# Enriched from F5 iRules reference documentation.
"""ILX::notify -- Calls an ILX method asynchronously."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ILX__notify.html"


@register
class IlxNotifyCommand(CommandDef):
    name = "ILX::notify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ILX::notify",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Calls an ILX method asynchronously.",
                synopsis=("ILX::notify HANDLE METHOD (ARGS)*",),
                snippet='Make a call to the plugin extension defined by the handle but do not wait for a response before continuing to process the remainder of the iRule. The delivery of the call to the plugin extension is "best effort" and is not guaranteed.',
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Get a handle to the running extension instance to call into.\n"
                    "    set RPC_HANDLE [ILX::init my_plugin my_extension]\n"
                    "    # Make the asynchronous call\n"
                    "    ILX::notify $RPC_HANDLE my_js_function arg1 arg2\n"
                    "}"
                ),
                return_value="None",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ILX::notify HANDLE METHOD (ARGS)*",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
