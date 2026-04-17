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
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(name="secret", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="server",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
