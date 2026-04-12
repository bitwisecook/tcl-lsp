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
            "auth_remote_role",
            module="auth",
            object_types=("remote-role",),
        ),
        header_types=(("auth", "remote-role"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="role-info",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="attribute", value_type="string", in_sections=("role-info",)),
            BigipPropertySpec(
                name="console",
                value_type="enum",
                in_sections=("role-info",),
                enum_values=("disabled", "tmsh"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("role-info",)),
            BigipPropertySpec(
                name="deny",
                value_type="enum",
                in_sections=("role-info",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="line-order", value_type="integer", in_sections=("role-info",)),
            BigipPropertySpec(name="role", value_type="string", in_sections=("role-info",)),
            BigipPropertySpec(
                name="application-editor", value_type="string", in_sections=("role-info",)
            ),
            BigipPropertySpec(
                name="firewall-manager",
                value_type="reference",
                in_sections=("role-info",),
                references=("ltm_rule",),
            ),
            BigipPropertySpec(
                name="no-access",
                value_type="reference",
                in_sections=("role-info",),
                references=("auth_user",),
            ),
            BigipPropertySpec(
                name="web-application-security-administrator",
                value_type="string",
                in_sections=("role-info",),
            ),
            BigipPropertySpec(
                name="web-application-security-editor",
                value_type="string",
                in_sections=("role-info",),
            ),
            BigipPropertySpec(
                name="user-partition", value_type="string", in_sections=("role-info",)
            ),
            BigipPropertySpec(name="remote-role", value_type="string"),
            BigipPropertySpec(name="role", value_type="string", in_sections=("remote-role",)),
            BigipPropertySpec(name="dc1", value_type="string", in_sections=("role",)),
            BigipPropertySpec(
                name="attribute",
                value_type="reference",
                in_sections=("dc1",),
                references=("auth_user",),
            ),
            BigipPropertySpec(
                name="console",
                value_type="reference",
                in_sections=("dc1",),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="line-order", value_type="string", in_sections=("dc1",)),
            BigipPropertySpec(
                name="role", value_type="reference", in_sections=("dc1",), references=("auth_user",)
            ),
            BigipPropertySpec(
                name="user-partition",
                value_type="reference",
                in_sections=("dc1",),
                references=("auth_partition", "auth_user"),
            ),
            BigipPropertySpec(name="dc2", value_type="string", in_sections=("role",)),
            BigipPropertySpec(
                name="attribute",
                value_type="reference",
                in_sections=("dc2",),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="line-order", value_type="string", in_sections=("dc2",)),
        ),
    )
