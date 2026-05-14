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
            "ltm_node2",
            module="ltm",
            object_types=("node2",),
        ),
        header_types=(("ltm", "node2"),),
        properties=(),
    )
