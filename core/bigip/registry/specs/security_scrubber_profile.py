from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "security_scrubber_profile",
            module="security",
            object_types=("scrubber profile",),
        ),
        header_types=(("security", "scrubber profile"),),
        properties=(
            BigipPropertySpec(name="advertisement-ttl", value_type="integer"),
            BigipPropertySpec(
                name="scrubber-categories",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="advertisement-method",
                value_type="enum",
                in_sections=("scrubber-categories",),
                allow_none=True,
                enum_values=(
                    "bgp-flowspec-method",
                    "bgp-method",
                    "none-method",
                    "silverline-method",
                ),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-advertisement-action",
                value_type="enum",
                in_sections=("scrubber-categories",),
                enum_values=("drop", "redirect", "rate-limit", "qos"),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-dscp-value",
                value_type="integer",
                in_sections=("scrubber-categories",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-rate-limit",
                value_type="integer",
                in_sections=("scrubber-categories",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-redirect-asn-community",
                value_type="string",
                in_sections=("scrubber-categories",),
            ),
            BigipPropertySpec(
                name="blacklist-category", value_type="string", in_sections=("scrubber-categories",)
            ),
            BigipPropertySpec(
                name="next-hop", value_type="string", in_sections=("scrubber-categories",)
            ),
            BigipPropertySpec(
                name="next-hop-v6", value_type="string", in_sections=("scrubber-categories",)
            ),
            BigipPropertySpec(
                name="route-domain-name", value_type="string", in_sections=("scrubber-categories",)
            ),
            BigipPropertySpec(
                name="scrubber-netflow-protected-server",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="advertisement-method",
                value_type="enum",
                in_sections=("scrubber-netflow-protected-server",),
                allow_none=True,
                enum_values=(
                    "bgp-flowspec-method",
                    "bgp-method",
                    "none-method",
                    "silverline-method",
                ),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-advertisement-action",
                value_type="enum",
                in_sections=("scrubber-netflow-protected-server",),
                enum_values=("drop", "redirect", "rate-limit", "qos"),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-dscp-value",
                value_type="integer",
                in_sections=("scrubber-netflow-protected-server",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-rate-limit",
                value_type="integer",
                in_sections=("scrubber-netflow-protected-server",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-redirect-asn-community",
                value_type="string",
                in_sections=("scrubber-netflow-protected-server",),
            ),
            BigipPropertySpec(
                name="blacklist-category",
                value_type="string",
                in_sections=("scrubber-netflow-protected-server",),
            ),
            BigipPropertySpec(
                name="next-hop",
                value_type="string",
                in_sections=("scrubber-netflow-protected-server",),
            ),
            BigipPropertySpec(
                name="next-hop-v6",
                value_type="string",
                in_sections=("scrubber-netflow-protected-server",),
            ),
            BigipPropertySpec(
                name="route-domain-name",
                value_type="string",
                in_sections=("scrubber-netflow-protected-server",),
            ),
            BigipPropertySpec(
                name="scrubber-rt-domain",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="absolute-threshold", value_type="integer", in_sections=("scrubber-rt-domain",)
            ),
            BigipPropertySpec(
                name="advertisement-method",
                value_type="enum",
                in_sections=("scrubber-rt-domain",),
                allow_none=True,
                enum_values=(
                    "bgp-flowspec-method",
                    "bgp-method",
                    "none-method",
                    "silverline-method",
                ),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-advertisement-action",
                value_type="enum",
                in_sections=("scrubber-rt-domain",),
                enum_values=("drop", "redirect", "rate-limit", "qos"),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-dscp-value",
                value_type="integer",
                in_sections=("scrubber-rt-domain",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-rate-limit",
                value_type="integer",
                in_sections=("scrubber-rt-domain",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-redirect-asn-community",
                value_type="string",
                in_sections=("scrubber-rt-domain",),
            ),
            BigipPropertySpec(
                name="next-hop", value_type="string", in_sections=("scrubber-rt-domain",)
            ),
            BigipPropertySpec(
                name="next-hop-v6", value_type="string", in_sections=("scrubber-rt-domain",)
            ),
            BigipPropertySpec(
                name="percentage-threshold",
                value_type="integer",
                in_sections=("scrubber-rt-domain",),
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                in_sections=("scrubber-rt-domain",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="scrubber-rd-network-prefix",
                value_type="enum",
                in_sections=("scrubber-rt-domain",),
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-advertisement-action",
                value_type="enum",
                in_sections=("scrubber-rd-network-prefix",),
                enum_values=("drop", "redirect", "rate-limit", "qos"),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-dscp-value",
                value_type="integer",
                in_sections=("scrubber-rd-network-prefix",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-rate-limit",
                value_type="integer",
                in_sections=("scrubber-rd-network-prefix",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-redirect-asn-community",
                value_type="string",
                in_sections=("scrubber-rd-network-prefix",),
            ),
            BigipPropertySpec(
                name="dst-ip", value_type="string", in_sections=("scrubber-rd-network-prefix",)
            ),
            BigipPropertySpec(
                name="mask", value_type="integer", in_sections=("scrubber-rd-network-prefix",)
            ),
            BigipPropertySpec(
                name="next-hop", value_type="string", in_sections=("scrubber-rd-network-prefix",)
            ),
            BigipPropertySpec(
                name="excluded-vlans",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="scrubber-virtual-server",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="absolute-threshold",
                value_type="integer",
                in_sections=("scrubber-virtual-server",),
            ),
            BigipPropertySpec(
                name="advertisement-method",
                value_type="enum",
                in_sections=("scrubber-virtual-server",),
                allow_none=True,
                enum_values=(
                    "bgp-flowspec-method",
                    "bgp-method",
                    "none-method",
                    "silverline-method",
                ),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-advertisement-action",
                value_type="enum",
                in_sections=("scrubber-virtual-server",),
                enum_values=("drop", "redirect", "rate-limit", "qos"),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-dscp-value",
                value_type="integer",
                in_sections=("scrubber-virtual-server",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-rate-limit",
                value_type="integer",
                in_sections=("scrubber-virtual-server",),
            ),
            BigipPropertySpec(
                name="bgp-flowspec-redirect-asn-community",
                value_type="string",
                in_sections=("scrubber-virtual-server",),
            ),
            BigipPropertySpec(
                name="next-hop", value_type="string", in_sections=("scrubber-virtual-server",)
            ),
            BigipPropertySpec(
                name="next-hop-v6", value_type="string", in_sections=("scrubber-virtual-server",)
            ),
            BigipPropertySpec(
                name="percentage-threshold",
                value_type="integer",
                in_sections=("scrubber-virtual-server",),
            ),
            BigipPropertySpec(
                name="vs-name", value_type="string", in_sections=("scrubber-virtual-server",)
            ),
            BigipPropertySpec(
                name="silverline", value_type="reference", repeated=True, references=("auth_user",)
            ),
        ),
    )
