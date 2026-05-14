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
            "gtm_wideip_https",
            module="gtm",
            object_types=("wideip https",),
        ),
        header_types=(("gtm", "wideip https"),),
        properties=(
            BigipPropertySpec(
                name="aliases",
                value_type="reference",
                repeated=True,
                default="none",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="failure-rcode",
                value_type="enum",
                enum_values=("formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail"),
                default="noerror",
            ),
            BigipPropertySpec(
                name="failure-rcode-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="failure-rcode-ttl",
                value_type="integer",
                default="0, meaning no SOA is included (i",
            ),
            BigipPropertySpec(
                name="last-resort-pool",
                value_type="reference",
                references=("gtm_pool_https",),
                default="none",
            ),
            BigipPropertySpec(
                name="load-balancing-decision-log-verbosity",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ),
                default="none",
            ),
            BigipPropertySpec(name="metadata", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="minimal-response",
                value_type="enum",
                required=True,
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="persist-cidr-ipv4", value_type="integer"),
            BigipPropertySpec(name="persist-cidr-ipv6", value_type="integer"),
            BigipPropertySpec(
                name="persistence",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="pool-lb-mode",
                value_type="enum",
                enum_values=("ratio", "round-robin", "topology"),
                default="round-robin",
            ),
            BigipPropertySpec(
                name="pools",
                value_type="unknown",
                allow_none=True,
                references=("gtm_pool_https",),
                default="none",
            ),
            BigipPropertySpec(
                name="rules",
                value_type="list",
                allow_none=True,
                references=(
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ),
                default="none",
            ),
            BigipPropertySpec(
                name="topology-prefer-edns0-client-subnet",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="ttl-persistence", value_type="integer", default="3600"),
            BigipPropertySpec(name="value", value_type="string"),
        ),
    )
