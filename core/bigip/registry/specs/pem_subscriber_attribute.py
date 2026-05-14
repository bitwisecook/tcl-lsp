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
            "pem_subscriber_attribute",
            module="pem",
            object_types=("subscriber-attribute",),
        ),
        header_types=(("pem", "subscriber-attribute"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="export",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="import",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="well-known-attr-id",
                value_type="enum",
                enum_values=(
                    "called-station-id",
                    "calling-station-id",
                    "imeisv",
                    "imsi",
                    "ipaddr",
                    "not-defined",
                    "subs-id",
                    "user-location-info",
                ),
            ),
        ),
    )
