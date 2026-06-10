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
            "ltm_persistence_persist_records",
            module="ltm",
            object_types=("persistence persist-records",),
        ),
        header_types=(("ltm", "persistence persist-records"),),
        properties=(),
    )
