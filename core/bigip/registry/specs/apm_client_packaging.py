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
            "apm_client_packaging",
            module="apm",
            object_types=("client-packaging",),
        ),
        header_types=(("apm", "client-packaging"),),
        properties=(),
    )
