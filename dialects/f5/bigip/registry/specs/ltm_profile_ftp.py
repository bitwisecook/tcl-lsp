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
            "ltm_profile_ftp",
            module="ltm",
            object_types=("profile ftp",),
        ),
        header_types=(("ltm", "profile ftp"),),
        properties=(
            BigipPropertySpec(
                name="allow-active-mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="allow-ftps",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
                usage_flags=frozenset(("deprecated",)),
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
                references=("ltm_profile_ftp",),
                default="ftp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="enforce-tls-session-reuse",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="ftps-mode",
                value_type="enum",
                enum_values=("allow", "disallow", "require"),
                default="disallow",
            ),
            BigipPropertySpec(
                name="inherit-parent-profile",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="log-profile",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
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
            ),
            BigipPropertySpec(
                name="security",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="translate-extended",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
