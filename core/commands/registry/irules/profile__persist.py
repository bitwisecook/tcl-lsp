# Enriched from F5 iRules reference documentation.
"""PROFILE::persist -- Returns the value of a persistence profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__persist.html"


_av = make_av(_SOURCE)


@register
class ProfilePersistCommand(CommandDef):
    name = "PROFILE::persist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::persist",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of a persistence profile setting.",
                synopsis=("PROFILE::persist ((instance PROFILE_PERSIST ATTR) | (mode MODE ATTR))",),
                snippet="Returns the current value of the specified setting in the assigned persistence profile.",
                source=_SOURCE,
                return_value="Returns the current value of the specified setting in the assigned persistence profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::persist ((instance PROFILE_PERSIST ATTR) | (mode MODE ATTR))",
                    arg_values={
                        0: (
                            _av(
                                "instance",
                                "PROFILE::persist instance",
                                "PROFILE::persist ((instance PROFILE_PERSIST ATTR) | (mode MODE ATTR))",
                            ),
                            _av(
                                "mode",
                                "PROFILE::persist mode",
                                "PROFILE::persist ((instance PROFILE_PERSIST ATTR) | (mode MODE ATTR))",
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
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
