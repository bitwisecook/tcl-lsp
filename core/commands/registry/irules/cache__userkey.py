# Enriched from F5 iRules reference documentation.
"""CACHE::userkey -- Allows users to add user-defined values to the key used by the cache to reference the cached content."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__userkey.html"


@register
class CacheUserkeyCommand(CommandDef):
    name = "CACHE::userkey"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::userkey",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows users to add user-defined values to the key used by the cache to reference the cached content.",
                synopsis=("CACHE::userkey KEY",),
                snippet=(
                    "By default, cached content is stored with a unique key referring to both\n"
                    "the URI of the resource to be cached and the User-Agent for which it\n"
                    "was formatted. If multiple variations of the same content must be\n"
                    "cached under specific conditions (different client), you can use this\n"
                    "command to create a unique key, thus creating cached content specific\n"
                    "to that condition. This can be used to prevent one user or group's\n"
                    "cached data from being served to different users/groups."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if {[matchclass [IP::client_addr] equals $::InternalIPs]} {\n"
                    '    CACHE::userkey "Internal"\n'
                    "  } else {\n"
                    '    CACHE::userkey "External"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::userkey KEY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
