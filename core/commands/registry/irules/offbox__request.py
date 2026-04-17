# Enriched from F5 iRules reference documentation.
"""OFFBOX::request -- Performs a request with a payload to certain service."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/OFFBOX__request.html"


_av = make_av(_SOURCE)


@register
class OffboxRequestCommand(CommandDef):
    name = "OFFBOX::request"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="OFFBOX::request",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Performs a request with a payload to certain service.",
                synopsis=(
                    "OFFBOX::request SERVICE PAYLOAD (((cache KEY) blocking) | (blocking | (cache KEY)) | (blocking TIMEOUT))?",
                ),
                snippet="Performs a request with a payload to certain service.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '                 if { [HTTP::uri] eq "login.php" } {\n'
                    '                     OFFBOX::request "/Common/offbox::ip_reputation" [TCP::client_addr] cache [TCP::client_addr] blocking\n'
                    "                 }\n"
                    "             }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="OFFBOX::request SERVICE PAYLOAD (((cache KEY) blocking) | (blocking | (cache KEY)) | (blocking TIMEOUT))?",
                    arg_values={
                        0: (
                            _av(
                                "cache",
                                "OFFBOX::request cache",
                                "OFFBOX::request SERVICE PAYLOAD (((cache KEY) blocking) | (blocking | (cache KEY)) | (blocking TIMEOUT))?",
                            ),
                            _av(
                                "blocking",
                                "OFFBOX::request blocking",
                                "OFFBOX::request SERVICE PAYLOAD (((cache KEY) blocking) | (blocking | (cache KEY)) | (blocking TIMEOUT))?",
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
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
