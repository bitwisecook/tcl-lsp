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
            "ltm_policy_strategy",
            module="ltm",
            object_types=("policy-strategy",),
        ),
        header_types=(("ltm", "policy-strategy"),),
        properties=(),
    )
