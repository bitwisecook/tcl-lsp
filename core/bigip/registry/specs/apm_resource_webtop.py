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
            "apm_resource_webtop",
            module="apm",
            object_types=("resource webtop",),
        ),
        header_types=(("apm", "resource webtop"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="customization-group", value_type="string"),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="minimize-to-tray",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="portal-access-start-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="warn-when-closed",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="webtop-type",
                value_type="enum",
                enum_values=("full", "last", "network-access"),
            ),
        ),
    )
