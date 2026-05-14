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
            "apm_oauth_oauth_scope",
            module="apm",
            object_types=("oauth oauth-scope",),
        ),
        header_types=(("apm", "oauth oauth-scope"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="scope-description", value_type="string", allow_none=True),
            BigipPropertySpec(name="scope-name", value_type="string"),
            BigipPropertySpec(name="scope-value", value_type="string", allow_none=True),
        ),
    )
