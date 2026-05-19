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
            "pem_irule",
            module="pem",
            object_types=("irule",),
        ),
        header_types=(("pem", "irule"),),
        properties=(),
    )
