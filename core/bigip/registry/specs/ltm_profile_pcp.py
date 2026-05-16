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
            "ltm_profile_pcp",
            module="ltm",
            object_types=("profile pcp",),
        ),
        header_types=(("ltm", "profile pcp"),),
        properties=(
            BigipPropertySpec(
                name="announce-after-failover",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="announce-multicast",
                value_type="integer",
                default="10 re-sends",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_pcp",),
                default="pcp, a profile that is shipped in the software",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="map-filter-limit", value_type="integer", default="1"),
            BigipPropertySpec(
                name="map-limit-per-client",
                value_type="integer",
                default="65535 (unlimited)",
            ),
            BigipPropertySpec(
                name="map-recycle-delay",
                value_type="integer",
                default="60 (seconds)",
            ),
            BigipPropertySpec(
                name="max-mapping-lifetime",
                value_type="integer",
                default="86400 (seconds), or 1 day",
            ),
            BigipPropertySpec(
                name="min-mapping-lifetime",
                value_type="integer",
                default="600 (seconds), or 10 minutes",
            ),
            BigipPropertySpec(
                name="rule",
                value_type="reference",
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
            BigipPropertySpec(name="third-party-allowed-subnets", value_type="unknown"),
            BigipPropertySpec(
                name="third-party-option",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
