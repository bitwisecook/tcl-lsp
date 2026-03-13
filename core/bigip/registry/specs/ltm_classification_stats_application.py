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
            "ltm_classification_stats_application",
            module="ltm",
            object_types=("classification stats application",),
        ),
        header_types=(("ltm", "classification stats application"),),
    )
