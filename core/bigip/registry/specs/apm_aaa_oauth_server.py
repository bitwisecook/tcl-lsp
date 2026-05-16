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
            "apm_aaa_oauth_server",
            module="apm",
            object_types=("aaa oauth-server",),
        ),
        header_types=(("apm", "aaa oauth-server"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="client-id", value_type="string"),
            BigipPropertySpec(name="client-jwe-key", value_type="reference"),
            BigipPropertySpec(name="client-secret", value_type="string", allow_none=True),
            BigipPropertySpec(name="client-serverssl-profile-name", value_type="reference"),
            BigipPropertySpec(name="dns-resolver-name", value_type="reference"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("client", "client-rs", "rs"),
            ),
            BigipPropertySpec(name="provider-name", value_type="reference"),
            BigipPropertySpec(name="resource-server-id", value_type="string"),
            BigipPropertySpec(name="resource-server-secret", value_type="string", allow_none=True),
            BigipPropertySpec(name="resource-serverssl-profile-name", value_type="reference"),
            BigipPropertySpec(
                name="rules",
                value_type="string",
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
                name="token-validation-interval",
                value_type="integer",
                allow_none=True,
            ),
        ),
    )
