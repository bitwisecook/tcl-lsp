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
            "ltm_auth_tacacs",
            module="ltm",
            object_types=("auth tacacs",),
        ),
        header_types=(("ltm", "auth tacacs"),),
        properties=(
            BigipPropertySpec(
                name="accounting",
                value_type="enum",
                enum_values=("send-to-all-servers", "send-to-first-server"),
            ),
            BigipPropertySpec(
                name="authentication",
                value_type="enum",
                required=True,
                enum_values=("use-all-servers", "use-first-server"),
            ),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="encryption",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="protocol", value_type="unknown"),
            BigipPropertySpec(name="secret", value_type="string", required=True),
            BigipPropertySpec(
                name="servers",
                value_type="string",
                required=True,
                shape_kind="endpoint",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="service",
                value_type="reference",
                required=True,
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
        ),
    )
