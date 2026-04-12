# Enriched from F5 iRules reference documentation.
"""SSL::allow_nonssl -- Get or set Allow Non-SSL connections."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__allow_nonssl.html"


@register
class SslAllowNonsslCommand(CommandDef):
    name = "SSL::allow_nonssl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::allow_nonssl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set Allow Non-SSL connections.",
                synopsis=("SSL::allow_nonssl (ZERO_ONE)?",),
                snippet=(
                    "SSL::allow_nonssl\n"
                    "  Returns the currently set value for Allow Non-SSL connections\n"
                    "SSL::allow_nonssl ( 0 | 1 )\n"
                    "  0 disables Non-SSL Connections, 1 enables it.\n"
                    "  Allow Non-ssl connections, sets SSL to passthrough mode."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    SSL::allow_nonssl 1\n}"),
                return_value="SSL::allow_nonssl Returns the currently set Allow Non-SSL connections value. SSL::allow_nonssl [0|1] There is no return value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::allow_nonssl (ZERO_ONE)?",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
