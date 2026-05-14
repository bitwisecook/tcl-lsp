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
            "apm_policy_agent_aaa_oauth",
            module="apm",
            object_types=("policy agent aaa-oauth",),
        ),
        header_types=(("apm", "policy agent aaa-oauth"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="auth-redirect-request", value_type="reference"),
            BigipPropertySpec(
                name="grant-type",
                value_type="enum",
                enum_values=("authorization-code", "password"),
            ),
            BigipPropertySpec(name="redirection-uri", value_type="string"),
            BigipPropertySpec(
                name="response",
                value_type="reference",
                references=(
                    "api_protection_response",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "asm_response_code",
                    "ltm_profile_response_adapt",
                    "sys_crypto_cert_validation_response_ocsp",
                ),
            ),
            BigipPropertySpec(name="scope", value_type="string", allow_none=True),
            BigipPropertySpec(name="scope-data-request", value_type="reference"),
            BigipPropertySpec(
                name="server",
                value_type="reference",
                references=(
                    "api_protection_server",
                    "apm_aaa_oauth_server",
                    "apm_oauth_oauth_resource_server",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "auth_radius_server",
                    "gtm_listener_doh_server",
                    "gtm_monitor_real_server",
                    "gtm_server",
                    "ltm_auth_crldp_server",
                    "ltm_auth_radius_server",
                    "ltm_monitor_real_server",
                    "ltm_profile_doh_server",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "sys_crypto_server",
                    "sys_smtp_server",
                    "wom_server_discovery",
                ),
            ),
            BigipPropertySpec(name="token-refresh-request", value_type="reference"),
            BigipPropertySpec(name="token-request", value_type="reference"),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("client", "scope")),
            BigipPropertySpec(name="validation-scopes-request", value_type="reference"),
        ),
    )
