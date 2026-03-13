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
            BigipPropertySpec(
                name="default-partition", value_type="reference", references=("auth_partition",)
            ),
            BigipPropertySpec(name="default-role", value_type="string"),
            BigipPropertySpec(name="fraud-protection-manager", value_type="string"),
            BigipPropertySpec(name="auditor", value_type="string"),
            BigipPropertySpec(name="irule-manager", value_type="boolean"),
            BigipPropertySpec(name="operator", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="web-application-security-administrator", value_type="string"),
            BigipPropertySpec(name="web-application-security-editor", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="remote-console-access", value_type="enum", enum_values=("disabled", "tmsh")
            ),
        ),
    )
