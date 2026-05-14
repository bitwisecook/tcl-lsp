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
            "net_f5optics",
            module="net",
            object_types=("f5optics",),
        ),
        header_types=(("net", "f5optics"),),
        properties=(),
    )
