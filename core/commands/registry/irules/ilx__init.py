# Enriched from F5 iRules reference documentation.
"""ILX::init -- Creates a handle to a running ILX plugin extension."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ILX__init.html"


@register
class IlxInitCommand(CommandDef):
    name = "ILX::init"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ILX::init",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a handle to a running ILX plugin extension.",
                synopsis=("ILX::init (EXTENSION | (PLUGIN EXTENSION))",),
                snippet="Creates a handle for future use by ILX::call and ILX::notify.  This handle is a reference to a running ILX plugin extension.  The lifetime of this variable affects the behavior of the ILX target if controlled by BIG-IP.  Instances of the plugin extension will be held in draining mode as long as there are open references to the ILX handle in any event.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Get a handle to the running extension instance to call into.\n"
                    "    set RPC_HANDLE [ILX::init my_plugin my_extension]\n"
                    "    # Make the call and store the response in $rpc_response\n"
                    "    set rpc_response [ILX::call $RPC_HANDLE my_js_function arg1 arg2]\n"
                    "}"
                ),
                return_value="Returns a handle to the running extension to call into.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ILX::init (EXTENSION | (PLUGIN EXTENSION))",
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
