# Curated community template proc.
"""xff_uniq_ordered_ip_list -- Order-preserved unique valid IPs from XFF."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/xff_uniq_ordered_ip_list.html"


@register
class XffUniqOrderedIpListProc(CommandDef):
    name = "xff_uniq_ordered_ip_list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="xff_uniq_ordered_ip_list",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary=(
                    "Return a deduplicated list of valid non-loopback IP "
                    "addresses from the X-Forwarded-For header, preserving "
                    "the original header order."
                ),
                synopsis=(
                    "call xff_uniq_ordered_ip_list",
                    'call xff_uniq_ordered_ip_list "X-Real-IP"',
                ),
                snippet=(
                    "Like `xff_uniq_sorted_ip_list` but preserves the original "
                    "header order instead of sorting.  Use this when the order "
                    "of IPs in the headers matters (e.g. tracing the proxy chain).  "
                    "It comes with a memory and performance penalty over the "
                    "sorted `xff_list` so should only be used when truly "
                    "necessary.\n"
                    "\n"
                    "  - Entries that are not IPv4 or IPv6 are removed\n"
                    "  - Both IPv4 and IPv6 addresses are collected and returned\n"
                    "  - The order of the request IPs is preserved\n"
                    "  - Duplicate IPs are collapsed\n"
                    "  - FQDNs are not valid IPs and are therefore removed\n"
                    "  - Loopback / zero addresses (`127.0.0.0/8`, "
                    "`0.0.0.0/32`, `::/127`) are filtered out"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST priority 350 {\n"
                    "    foreach ip [call xff_uniq_ordered_ip_list] {\n"
                    '        if {[class match -- $ip eq "blacklist-ips"]} {\n'
                    '            log local0. "Blocking bad XFF IP: $ip"\n'
                    "            reject\n"
                    "            return\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="A Tcl list of unique IP address strings in original header order.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="xff_uniq_ordered_ip_list ?xff_header_name?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
