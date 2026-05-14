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
            "apm_resource_client_rate_class",
            module="apm",
            object_types=("resource client-rate-class",),
        ),
        header_types=(("apm", "resource client-rate-class"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="burst", value_type="integer"),
            BigipPropertySpec(name="ceiling", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="dscp", value_type="integer"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("borrow", "discard", "shape"),
            ),
            BigipPropertySpec(name="rate", value_type="integer"),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("best-effort", "controlled-load", "guaranteed"),
            ),
        ),
    )
