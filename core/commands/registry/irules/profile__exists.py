# Enriched from F5 iRules reference documentation.
"""PROFILE::exists -- Determine if a profile is configured on a virtual server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__exists.html"


@register
class ProfileExistsCommand(CommandDef):
    name = "PROFILE::exists"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::exists",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Determine if a profile is configured on a virtual server.",
                synopsis=(
                    "PROFILE::exists TYPE (NAME)?",
                    "PROFILE::exists persist MODE (NAME)?",
                ),
                snippet=(
                    "Determine if a profile is configured on a virtual server.\n"
                    "\n"
                    'Note that the results of the PROFILE::exists "profile type" command is specific to the context of the event. For example, with a client SSL profile associated with the virtual server, PROFILE::exists clientssl will return 1 in clientside events and 0 in serverside events. Likewise, PROFILE::exists serverssl will return 0 in clientside events and 1 in serverside events.'
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "   if { [PROFILE::exists clientssl] == 1} {\n"
                    '      log local0. "client SSL profile enabled on virtual server"\n'
                    "   }\n"
                    "}"
                ),
                return_value="Returns 1 if the profile is configured on the current virtual server. Returns 0 if the profile is not configured on the current virtual server.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::exists TYPE (NAME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
