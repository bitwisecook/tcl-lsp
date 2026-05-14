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
            "apm_oauth_token_details",
            module="apm",
            object_types=("oauth token-details",),
        ),
        header_types=(("apm", "oauth token-details"),),
        properties=(),
    )
