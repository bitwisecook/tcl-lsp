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
            "apm_policy_agent_resource_assign",
            module="apm",
            object_types=("policy agent resource-assign",),
        ),
        header_types=(("apm", "policy agent resource-assign"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="rules",
                value_type="unknown",
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
                name="type",
                value_type="enum",
                enum_values=("acls", "general", "resources", "webtop-and-webtop-links"),
            ),
        ),
    )
