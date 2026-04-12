# Enriched from F5 iRules reference documentation.
"""imid -- Returns an i-mode identifier string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/imid.html"


@register
class ImidCommand(CommandDef):
    name = "imid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="imid",
            deprecated_replacement="IMID::id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an i-mode identifier string.",
                synopsis=("imid",),
                snippet=(
                    "Parses the BIG-IP 4.X http_uri variable and the user-agent header field to return an i-mode identifier string that can be used for i-mode session persistence. This is a BIG-IP 4.X function, provided for backward compatibility.\n"
                    "\n"
                    "The imid function takes no arguments and simply returns the string representing the i-mode identifier or the empty string, if none is found."
                ),
                source=_SOURCE,
                return_value="Parses the BIG-IP 4.X http_uri variable and the '''user-agent* header field to return an i-mode identifier string that can be used for i-mode session persistence.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="imid",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
