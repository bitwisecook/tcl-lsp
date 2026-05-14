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
            "util_verify_encryption",
            module="util",
            object_types=("verify encryption",),
        ),
        header_types=(("util", "verify encryption"),),
        properties=(),
    )
