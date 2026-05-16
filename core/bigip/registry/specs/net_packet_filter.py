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
            "net_packet_filter",
            module="net",
            object_types=("packet-filter",),
        ),
        header_types=(("net", "packet-filter"),),
        properties=(
            BigipPropertySpec(
                name="action",
                value_type="enum",
                enum_values=("accept", "continue", "discard", "reject"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="logging",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="order", value_type="integer", required=True),
            BigipPropertySpec(name="rate-class", value_type="reference"),
            BigipPropertySpec(
                name="rule",
                value_type="unknown",
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
                name="vlan",
                value_type="reference",
                references=("net_vlan", "net_vlan_allowed", "net_vlan_group"),
            ),
        ),
    )
