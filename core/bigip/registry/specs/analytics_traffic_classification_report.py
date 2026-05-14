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
            "analytics_traffic_classification_report",
            module="analytics",
            object_types=("traffic-classification report",),
        ),
        header_types=(("analytics", "traffic-classification report"),),
        properties=(),
    )
