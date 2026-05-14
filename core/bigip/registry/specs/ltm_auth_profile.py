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
            "ltm_auth_profile",
            module="ltm",
            object_types=("auth profile",),
        ),
        header_types=(("ltm", "auth profile"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="configuration",
                value_type="reference",
                allow_none=True,
                references=("apm_aaa_f5_mfa_configuration", "apm_configuration_captcha"),
            ),
            BigipPropertySpec(name="cookie-key", value_type="string"),
            BigipPropertySpec(name="cookie-name", value_type="string"),
            BigipPropertySpec(name="credential-source", value_type="unknown"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_auth_profile",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="enabled", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="rule",
                value_type="reference",
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
