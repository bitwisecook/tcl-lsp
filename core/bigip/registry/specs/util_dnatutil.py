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
            "util_dnatutil",
            module="util",
            object_types=("dnatutil",),
        ),
        header_types=(("util", "dnatutil"),),
        properties=(),
    )
