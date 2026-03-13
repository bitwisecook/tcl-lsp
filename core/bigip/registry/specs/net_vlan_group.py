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
            "net_vlan_group",
            module="net",
            object_types=("vlan-group",),
        ),
        header_types=(("net", "vlan-group"),),
        properties=(
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="bridge-in-standby", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="bridge-multicast", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="bridge-traffic", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="members", value_type="enum", allow_none=True, enum_values=("default", "none")
            ),
            BigipPropertySpec(
                name="migration-keepalive", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("opaque", "translucent", "transparent", "virtual-wire"),
            ),
            BigipPropertySpec(
                name="proxy-excludes",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none"),
            ),
        ),
    )
