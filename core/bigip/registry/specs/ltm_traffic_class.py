from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_traffic_class",
            module="ltm",
            object_types=("traffic-class",),
        ),
        header_types=(("ltm", "traffic-class"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="classification", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination-address", value_type="string", allow_none=True),
            BigipPropertySpec(name="destination-mask", value_type="string", allow_none=True),
            BigipPropertySpec(name="destination-port", value_type="reference"),
            BigipPropertySpec(name="protocol", value_type="unknown"),
            BigipPropertySpec(name="source-address", value_type="string", allow_none=True),
            BigipPropertySpec(name="source-mask", value_type="string", allow_none=True),
            BigipPropertySpec(name="source-port", value_type="reference"),
        ),
    )
