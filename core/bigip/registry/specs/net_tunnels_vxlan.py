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
            "net_tunnels_vxlan",
            module="net",
            object_types=("tunnels vxlan",),
        ),
        header_types=(("net", "tunnels vxlan"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="flooding-type",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "multicast", "multipoint", "replicator"),
            ),
            BigipPropertySpec(
                name="encapsulation-type", value_type="enum", enum_values=("vxlan", "vxlan-gpe")
            ),
        ),
    )
