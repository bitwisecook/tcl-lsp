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
            "apm_oauth_jwk_config",
            module="apm",
            object_types=("oauth jwk-config",),
        ),
        header_types=(("apm", "oauth jwk-config"),),
        properties=(),
    )
