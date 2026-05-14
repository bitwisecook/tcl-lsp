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
            "ltm_auth_crldp_server",
            module="ltm",
            object_types=("auth crldp-server",),
        ),
        header_types=(("ltm", "auth crldp-server"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="base-dn",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="host",
                value_type="string",
                required=True,
                allow_none=True,
                shape_kind="ip-address",
                default="none",
            ),
            BigipPropertySpec(
                name="port",
                value_type="reference",
                references=(
                    "net_port_list",
                    "net_port_mirror",
                    "security_firewall_port_list",
                    "security_firewall_port_misuse_policy",
                    "sys_log_config_destination_management_port",
                ),
                default="389",
            ),
            BigipPropertySpec(
                name="reverse-dn",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
