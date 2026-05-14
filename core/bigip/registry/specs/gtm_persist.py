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
            "gtm_persist",
            module="gtm",
            object_types=("persist",),
        ),
        header_types=(("gtm", "persist"),),
        properties=(),
    )
