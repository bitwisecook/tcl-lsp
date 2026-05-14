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
            "pem_subscribers",
            module="pem",
            object_types=("subscribers",),
        ),
        header_types=(("pem", "subscribers"),),
        properties=(),
    )
