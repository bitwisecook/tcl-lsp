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
            "pem_protocol_diameter_avp",
            module="pem",
            object_types=("protocol diameter-avp",),
        ),
        header_types=(("pem", "protocol diameter-avp"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="avp-code", value_type="integer"),
            BigipPropertySpec(
                name="data-type",
                value_type="integer",
                enum_values=(
                    "enumerated",
                    "float32",
                    "float64",
                    "grouped",
                    "rat-type",
                    "time",
                    "unsigned32",
                    "unsigned64",
                ),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="length", value_type="integer"),
            BigipPropertySpec(
                name="parent-avp",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
                references=("pem_protocol_diameter_avp",),
            ),
            BigipPropertySpec(name="vendor-id", value_type="integer"),
        ),
    )
