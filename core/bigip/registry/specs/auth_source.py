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
            "auth_source",
            module="auth",
            object_types=("source",),
        ),
        header_types=(("auth", "source"),),
        properties=(
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "active-directory",
                    "ldap",
                    "local",
                    "radius",
                    "tacacs",
                    "cert-ldap",
                    "apm-auth",
                ),
            ),
            BigipPropertySpec(name="fallback", value_type="enum", enum_values=("true", "false")),
        ),
    )
