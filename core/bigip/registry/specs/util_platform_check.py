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
            "util_platform_check",
            module="util",
            object_types=("platform check",),
        ),
        header_types=(("util", "platform check"),),
        properties=(),
    )
