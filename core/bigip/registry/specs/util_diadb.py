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
            "util_diadb",
            module="util",
            object_types=("diadb",),
        ),
        header_types=(("util", "diadb"),),
        properties=(),
    )
