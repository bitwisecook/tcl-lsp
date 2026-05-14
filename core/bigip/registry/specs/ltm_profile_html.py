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
            "ltm_profile_html",
            module="ltm",
            object_types=("profile html",),
        ),
        header_types=(("ltm", "profile html"),),
        properties=(
            BigipPropertySpec(
                name="content-detection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="content-selection", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_html",),
            ),
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
        ),
    )
