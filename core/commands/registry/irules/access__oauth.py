# Enriched from F5 iRules reference documentation.
"""ACCESS::oauth -- OAuth related ACCESS iRule"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__oauth.html"


@register
class AccessOauthCommand(CommandDef):
    name = "ACCESS::oauth"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::oauth",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="OAuth related ACCESS iRule",
                synopsis=("ACCESS::oauth sign ((-payload VALUE) (-key JWK_OBJECT)",),
                snippet=(
                    "OAuth related ACCESS iRule\n"
                    "\n"
                    "ACCESS::oauth sign [ -header <raw-data> ] -payload <raw-data> -key <JWK object>\n"
                    "                   [ -alg <signing algorithm> ] [ -ignore-cert-expiry ]\n"
                    "\n"
                    "     * Returns a JSON Web Signature token based on provided payload and signed\n"
                    "       with provided JWK object. When the specified JWK object does not specify\n"
                    "       a JWS signing algorithm, an additional signing algorithm is required\n"
                    "       and must be provided with the -alg option."
                ),
                source=_SOURCE,
                examples=("when ACCESS_SESSION_CLOSED {\n    call delete_jws_cache\n}"),
                return_value="JSON Web Signature string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::oauth sign ((-payload VALUE) (-key JWK_OBJECT)",
                    options=(
                        OptionSpec(name="-payload", detail="Option -payload.", takes_value=True),
                        OptionSpec(name="-key", detail="Option -key.", takes_value=True),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
