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
                name="fallback",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "active-directory",
                    "apm-auth",
                    "cert-ldap",
                    "ldap",
                    "local",
                    "radius",
                    "tacacs",
                ),
                default="local",
            ),
        ),
    )
