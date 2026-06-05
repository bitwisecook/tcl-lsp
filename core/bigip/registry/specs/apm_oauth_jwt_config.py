from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "apm_oauth_jwt_config",
            module="apm",
            object_types=("oauth jwt-config",),
        ),
        header_types=(("apm", "oauth jwt-config"),),
        properties=(),
    )
