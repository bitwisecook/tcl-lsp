# Enriched from F5 iRules reference documentation.
"""SSL::profile -- Switch between different SSL profiles."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__profile.html"


@register
class SslProfileCommand(CommandDef):
    name = "SSL::profile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::profile",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Switch between different SSL profiles.",
                synopsis=("SSL::profile PROFILE_OBJ",),
                snippet=(
                    "This command allows you to switch between SSL profiles, both client and server. Note: This should be done before the SSL negotiation occurs, or your rule will require the use of the SSL::renegotiate command.\n"
                    "\n"
                    "In order to switch SSL profiles, a profile must be assigned to the virtual to begin with; switching the clientssl profile requires an existing clientssl profile, and similarly for serverssl profiles. You can also use SSL::disable to use SSL selectively."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    SSL::renegotiate\n}"),
                return_value="SSL::profile <profile_name> Switch to the defined SSL profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::profile PROFILE_OBJ",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
