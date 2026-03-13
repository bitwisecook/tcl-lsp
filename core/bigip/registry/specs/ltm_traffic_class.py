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
            BigipPropertySpec(name="classification", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="destination-mask", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="destination-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="protocol", value_type="enum", enum_values=("any", "protocol")),
            BigipPropertySpec(
                name="source-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="source-mask", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="source-port", value_type="integer", min_value=0, max_value=65535
            ),
        ),
    )
