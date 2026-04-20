# Enriched from F5 iRules reference documentation.
"""WEBSSO::select -- Use specified SSO configuration object to do SSO for the HTTP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WEBSSO__select.html"


@register
class WebssoSelectCommand(CommandDef):
    name = "WEBSSO::select"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WEBSSO::select",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Use specified SSO configuration object to do SSO for the HTTP request.",
                synopsis=("WEBSSO::select WEBSSO_OBJECT",),
                snippet=(
                    "This command causes APM to use specified SSO configuration object to do\n"
                    "SSO for the HTTP request. Admin should make sure that the selected SSO\n"
                    "method works for the specified request (and is enabled on backend\n"
                    "server request is going to). The scope of this iRule command is per\n"
                    "HTTP request. Admin needs to execute it for each HTTP request."
                ),
                source=_SOURCE,
                examples=(
                    "when ACCESS_ACL_ALLOWED {\n"
                    "    set req_uri [HTTP::uri]\n"
                    '    if { $req_uri starts_with "/owa" } {\n'
                    '        if { $req_uri eq "/owa/auth/logon.aspx?url=https://mysite.com/owa/&reason=0" } {\n'
                    "            WEBSSO::select owa_form_base_sso\n"
                    '        } elseif { $req_uri eq "/owa/auth/logon.aspx?url=https://mysite.com/ecp/&reason=0" } {\n'
                    "            WEBSSO::select ecp_form_base_sso\n"
                    "        }\n"
                    "    }\n"
                    "    unset req_uri\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WEBSSO::select WEBSSO_OBJECT",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ACCESS", "HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
