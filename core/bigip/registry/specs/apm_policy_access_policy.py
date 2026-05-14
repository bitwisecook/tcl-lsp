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
            "apm_policy_access_policy",
            module="apm",
            object_types=("policy access-policy",),
        ),
        header_types=(("apm", "policy access-policy"),),
        properties=(),
    )
