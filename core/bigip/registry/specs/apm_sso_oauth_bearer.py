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
            "apm_sso_oauth_bearer",
            module="apm",
            object_types=("sso oauth-bearer",),
        ),
        header_types=(("apm", "sso oauth-bearer"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="hname", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="hvalue", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="oauth-server", value_type="string"),
        ),
    )
