# Enriched from F5 iRules reference documentation.
"""domain -- Parses the specified string as a dotted domain name and returns the last portions of the domain name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/domain.html"


@register
class DomainCommand(CommandDef):
    name = "domain"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="domain",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Parses the specified string as a dotted domain name and returns the last portions of the domain name.",
                synopsis=("domain DOMAIN COUNT",),
                snippet=(
                    "A custom iRule function which parses the specified string as a\n"
                    "dotted domain name and returns the last <count> portions of the domain\n"
                    "name."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST\n"
                    'if { [HTTP::uri] ends_with ".html" } {\n'
                    "      pool cache_pool\n"
                    "      set key [crc32 [concat [domain [HTTP::host] 2] [HTTP::uri]]]\n"
                    "}\n"
                    "...\n"
                    "\n"
                    "This code:\n"
                    "\n"
                    " log local0. [domain www.sub.my.domain.com 1]   ; # result: com\n"
                    " log local0. [domain www.sub.my.domain.com 2]   ; # result: domain.com\n"
                    " log local0. [domain www.sub.my.domain.com 3]   ; # result: my.domain.com\n"
                    " log local0. [domain www.sub.my.domain.com 4]   ; # result: sub.my.domain.com"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="domain DOMAIN COUNT",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
