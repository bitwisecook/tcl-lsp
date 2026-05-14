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
            "security_bot_defense_signature",
            module="security",
            object_types=("bot-defense signature",),
        ),
        header_types=(("security", "bot-defense signature"),),
        properties=(
            BigipPropertySpec(
                name="category",
                value_type="reference",
                references=(
                    "ltm_classification_category",
                    "ltm_classification_stats_url_category",
                    "ltm_classification_url_category",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_bot_defense_anomaly_category",
                    "security_bot_defense_signature_category",
                    "security_dos_bot_signature_category",
                    "security_firewall_ipi_category_info",
                    "security_ip_intelligence_blacklist_category",
                    "security_scrubber_dwbl_scrubber_category_stats",
                    "sys_url_db_url_category",
                ),
            ),
            BigipPropertySpec(
                name="domains",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="risk",
                value_type="enum",
                enum_values=("high", "low", "medium"),
            ),
            BigipPropertySpec(
                name="rule",
                value_type="string",
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
            BigipPropertySpec(name="signature-references", value_type="string"),
            BigipPropertySpec(name="url", value_type="unknown"),
            BigipPropertySpec(
                name="match-type",
                value_type="enum",
                in_sections=("url",),
                enum_values=("contains",),
            ),
            BigipPropertySpec(name="search-string", value_type="string", in_sections=("url",)),
            BigipPropertySpec(name="user-agent", value_type="unknown"),
            BigipPropertySpec(
                name="match-type",
                value_type="enum",
                in_sections=("user-agent",),
                enum_values=("contains",),
            ),
            BigipPropertySpec(
                name="search-string",
                value_type="string",
                in_sections=("user-agent",),
            ),
        ),
    )
