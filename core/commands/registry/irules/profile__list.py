# Enriched from F5 iRules reference documentation.
"""PROFILE::list -- Returns all the names of the profiles of the class asked for that are attached to this virtual server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__list.html"


_av = make_av(_SOURCE)


@register
class ProfileListCommand(CommandDef):
    name = "PROFILE::list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::list",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns all the names of the profiles of the class asked for that are attached to this virtual server.",
                synopsis=("PROFILE::list 'auth'",),
                snippet="This command returns all the names of the profiles of the class asked for that are attached to this virtual server.",
                source=_SOURCE,
                return_value="Returns all the names of the profiles of the class asked for that are attached to this virtual server",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::list 'auth'",
                    arg_values={0: (_av("auth", "PROFILE::list auth", "PROFILE::list 'auth'"),)},
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
