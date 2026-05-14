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
            "apm_policy_policy_item",
            module="apm",
            object_types=("policy policy-item",),
        ),
        header_types=(("apm", "policy policy-item"),),
        properties=(),
    )
