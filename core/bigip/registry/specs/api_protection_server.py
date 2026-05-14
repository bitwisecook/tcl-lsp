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
            "api_protection_server",
            module="api-protection",
            object_types=("server",),
        ),
        header_types=(("api-protection", "server"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="serverssl-profile", value_type="string", allow_none=True),
            BigipPropertySpec(name="url", value_type="string"),
        ),
    )
