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
            "wom_verify_config",
            module="wom",
            object_types=("verify-config",),
        ),
        header_types=(("wom", "verify-config"),),
        properties=(),
    )
