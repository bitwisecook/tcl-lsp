# Enriched from F5 iRules reference documentation.
"""fasthash -- Returns a hash for the specified string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/fasthash.html"


@register
class FasthashCommand(CommandDef):
    name = "fasthash"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fasthash",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a hash for the specified string.",
                synopsis=("fasthash DATA",),
                snippet=(
                    "fasthash is guaranteed to return a high quality hash of the input as quickly as practical. The hash value returned is between 0 and 2^63-1 inclusive (a positive integer).\n"
                    "\n"
                    "fasthash was added because there are many use cases (ie CARP) which need a hash of some value (ie URI) and which were using crc32 (which is a bad and slow hash function).\n"
                    "\n"
                    "Note: fasthash does not guarantee to provide the same hash value across different BIGIP versions and over BIGIP reboots. Do not use fasthash for long term and persistent storage."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    set str "hello world"\n'
                    '    log local0. "hash of $str is [fasthash $str]"\n'
                    "}"
                ),
                return_value="Returns the numeric hash for the specified string",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fasthash DATA",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
