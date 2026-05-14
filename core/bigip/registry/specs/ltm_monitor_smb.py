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
            "ltm_monitor_smb",
            module="ltm",
            object_types=("monitor smb",),
        ),
        header_types=(("ltm", "monitor smb"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_smb",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="get", value_type="unknown"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(
                name="server",
                value_type="reference",
                allow_none=True,
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
            BigipPropertySpec(
                name="service",
                value_type="reference",
                allow_none=True,
                references=(
                    "analytics_ssl_orchestrator_service_virtual_report",
                    "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                    "apm_aaa_f5_service_connector",
                    "apm_saml_artifact_resolution_service",
                    "apm_saml_attribute_consuming_service",
                    "net_service_policy",
                    "pem_service_chain_endpoint",
                    "security_bot_defense_micro_service",
                    "security_protocol_inspection_service",
                    "sys_application_service",
                    "sys_service",
                ),
            ),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(name="username", value_type="reference", allow_none=True),
        ),
    )
