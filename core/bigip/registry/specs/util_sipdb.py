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
            "util_sipdb",
            module="util",
            object_types=("sipdb",),
        ),
        header_types=(("util", "sipdb"),),
        properties=(),
    )
