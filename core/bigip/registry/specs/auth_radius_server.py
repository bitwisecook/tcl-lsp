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
            "auth_radius_server",
            module="auth",
            object_types=("radius-server",),
        ),
        header_types=(("auth", "radius-server"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
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
                default="1812",
            ),
            BigipPropertySpec(name="secret", value_type="unknown", required=True),
            BigipPropertySpec(
                name="server",
                value_type="string",
                required=True,
                allow_none=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="timeout", value_type="integer", default="3"),
        ),
    )
