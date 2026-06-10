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
            "apm_aaa_kerberos",
            module="apm",
            object_types=("aaa kerberos",),
        ),
        header_types=(("apm", "aaa kerberos"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auth-realm",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="keytab-file-obj",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(
                name="service-name",
                value_type="string",
                required=True,
                allow_none=True,
            ),
        ),
    )
