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
            BigipPropertySpec(name="address", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="arp",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="auto-delete",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="connection-limit",
                value_type="integer",
                default='0, meaning ""no limit',
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="icmp-echo",
                value_type="enum",
                enum_values=("all", "always", "any", "disabled", "enabled", "selective"),
                default="enabled",
            ),
            BigipPropertySpec(name="mask", value_type="unknown", required=True, default="255"),
            BigipPropertySpec(name="metadata", value_type="unknown"),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="route-advertisement",
                value_type="enum",
                enum_values=("all", "always", "any", "disabled", "enabled", "selective"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="server-scope",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "any", "none"),
                default="any",
            ),
            BigipPropertySpec(
                name="spanning",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
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
