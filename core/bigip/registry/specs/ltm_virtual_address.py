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
            "ltm_virtual_address",
            module="ltm",
            object_types=("virtual-address",),
        ),
        header_types=(("ltm", "virtual-address"),),
        properties=(
            BigipPropertySpec(name="address", value_type="string"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="arp", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="auto-delete", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="connection-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="enabled", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="icmp-echo",
                value_type="enum",
                enum_values=("all", "always", "any", "disabled", "enabled", "selective"),
            ),
            BigipPropertySpec(name="mask", value_type="unknown"),
            BigipPropertySpec(name="metadata", value_type="unknown"),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(
                name="route-advertisement",
                value_type="enum",
                enum_values=("all", "always", "any", "disabled", "enabled", "selective"),
            ),
            BigipPropertySpec(
                name="server-scope",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "any", "none"),
            ),
            BigipPropertySpec(
                name="spanning",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(name="value", value_type="string"),
        ),
    )
