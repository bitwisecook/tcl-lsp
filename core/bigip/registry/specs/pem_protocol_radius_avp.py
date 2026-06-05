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
            "pem_protocol_radius_avp",
            module="pem",
            object_types=("protocol radius-avp",),
        ),
        header_types=(("pem", "protocol radius-avp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="data-type",
                value_type="integer",
                enum_values=(
                    "3gpp-rat-type",
                    "3gpp-user-location-info",
                    "ipaddr",
                    "ipv6addr",
                    "ipv6prefix",
                    "octet",
                    "time",
                ),
                default="string",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="max-length", value_type="integer", default="253"),
            BigipPropertySpec(name="min-length", value_type="integer", default="1"),
            BigipPropertySpec(name="type", value_type="integer"),
            BigipPropertySpec(name="vendor-id", value_type="integer"),
            BigipPropertySpec(name="vendor-type", value_type="integer"),
        ),
    )
