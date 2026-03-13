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
            "ltm_classification_stats_url_category",
            module="ltm",
            object_types=("classification stats url-category",),
        ),
        header_types=(("ltm", "classification stats url-category"),),
    )
