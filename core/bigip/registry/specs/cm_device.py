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
            "cm_device",
            module="cm",
            object_types=("device",),
        ),
        header_types=(("cm", "device"),),
        properties=(
            BigipPropertySpec(name="comment", value_type="string"),
            BigipPropertySpec(name="configsync-ip", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="contact", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ha-capacity", value_type="integer"),
            BigipPropertySpec(name="location", value_type="string"),
            BigipPropertySpec(
                name="mgmt-unicast-mode", value_type="enum", enum_values=("both", "ipv4", "ipv6")
            ),
            BigipPropertySpec(name="mirror-ip", value_type="string"),
            BigipPropertySpec(name="mirror-secondary-ip", value_type="string"),
            BigipPropertySpec(name="multicast-interface", value_type="string"),
            BigipPropertySpec(name="multicast-ip", value_type="string"),
            BigipPropertySpec(
                name="multicast-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(
                name="unicast-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="ip", value_type="string", in_sections=("unicast-address",)),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("unicast-address",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="effective-ip", value_type="string", in_sections=("unicast-address",)
            ),
            BigipPropertySpec(
                name="effective-port",
                value_type="integer",
                in_sections=("unicast-address",),
                min_value=0,
                max_value=65535,
            ),
        ),
    )
