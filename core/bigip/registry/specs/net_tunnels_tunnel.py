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
            "net_tunnels_tunnel",
            module="net",
            object_types=("tunnels tunnel",),
        ),
        header_types=(("net", "tunnels tunnel"),),
        properties=(
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "enabled", "disabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="local-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="secondary-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="mode", value_type="enum", enum_values=("bidirectional", "inbound", "outbound")
            ),
            BigipPropertySpec(name="mtu", value_type="integer"),
            BigipPropertySpec(
                name="use-pmtu", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="profile", value_type="string"),
            BigipPropertySpec(
                name="remote-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(name="tos", value_type="integer"),
            BigipPropertySpec(
                name="transparent", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="key", value_type="integer"),
        ),
    )
