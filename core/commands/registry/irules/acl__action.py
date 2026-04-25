# Enriched from F5 iRules reference documentation.
"""ACL::action -- Sets or retrieves the current ACL action."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACL__action.html"


@register
class AclActionCommand(CommandDef):
    name = "ACL::action"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACL::action",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or retrieves the current ACL action.",
                synopsis=("ACL::action (default |",),
                snippet=(
                    "The ACL::action command allows you to determine the ACL action in the\n"
                    "FLOW_INIT event. This command requires the Advanced Firewall\n"
                    "Manager module."
                ),
                source=_SOURCE,
                examples=(
                    "when FLOW_INIT {\n"
                    "  if { [IP::addr [IP::client_addr] equals 172.29.97.151] } {\n"
                    "    ACL::action allow\n"
                    "    virtual /Common/my_http_vs\n"
                    '    log "FLOW_INIT: ACL allow to /Common/my_http_vs"\n'
                    "  }\n"
                    "}"
                ),
                return_value="When no argument is provided, the command will return an integer value corresponding to an action that will be taken: + 0 is a drop + 1 is reset (or reject) + 2 is allow (or accept) + 3 is allow-final (or accept-decisively)",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACL::action (default |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
