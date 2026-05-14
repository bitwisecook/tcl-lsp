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
            "apm_policy_customization_languages",
            module="apm",
            object_types=("policy customization-languages",),
        ),
        header_types=(("apm", "policy customization-languages"),),
        properties=(),
    )
