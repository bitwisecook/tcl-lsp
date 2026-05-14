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
            "gtm_wideip_mx",
            module="gtm",
            object_types=("wideip mx",),
        ),
        header_types=(("gtm", "wideip mx"),),
        properties=(
            BigipPropertySpec(name="aliases", value_type="reference"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="failure-rcode",
                value_type="enum",
                enum_values=("formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail"),
            ),
            BigipPropertySpec(
                name="failure-rcode-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="failure-rcode-ttl", value_type="integer"),
            BigipPropertySpec(
                name="last-resort-pool",
                value_type="reference",
                references=("gtm_pool_mx",),
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
            ),
            BigipPropertySpec(name="metadata", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="minimal-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="persist-cidr-ipv4", value_type="integer"),
            BigipPropertySpec(name="persist-cidr-ipv6", value_type="integer"),
            BigipPropertySpec(
                name="persistence",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="pool-lb-mode",
                value_type="enum",
                enum_values=("ratio", "round-robin", "topology"),
            ),
            BigipPropertySpec(
                name="pools",
                value_type="unknown",
                allow_none=True,
                references=("gtm_pool_mx",),
            ),
            BigipPropertySpec(name="pools-cname", value_type="unknown", allow_none=True),
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
            ),
            BigipPropertySpec(
                name="topology-prefer-edns0-client-subnet",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="ttl-persistence", value_type="integer"),
            BigipPropertySpec(name="value", value_type="string"),
        ),
    )
