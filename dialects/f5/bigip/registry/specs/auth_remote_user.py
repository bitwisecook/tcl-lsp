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
            "auth_remote_user",
            module="auth",
            object_types=("remote-user",),
        ),
        header_types=(("auth", "remote-user"),),
        properties=(
            BigipPropertySpec(name="default-partition", value_type="reference", default="all"),
            BigipPropertySpec(
                name="default-role",
                value_type="enum",
                enum_values=(
                    "acceleration-policy-editor",
                    "admin",
                    "application-editor",
                    "auditor",
                    "firewall-manager",
                    "fraud-protection-manager",
                    "guest",
                    "irule-manager",
                    "log-manager",
                    "manager",
                    "no-access",
                    "operator",
                    "resource-admin",
                    "user-manager",
                    "web-application-security-administrator",
                    "web-application-security-editor",
                    "web-application-security-operations-administrator",
                ),
                default="no-access",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="remote-console-access",
                value_type="enum",
                enum_values=("disabled", "tmsh"),
                default="disabled",
            ),
        ),
    )
