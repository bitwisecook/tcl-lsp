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
            "ltm_nat",
            module="ltm",
            object_types=("nat",),
        ),
        header_types=(("ltm", "nat"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="arp", value_type="unknown"),
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "disabled", "enabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="originating-address",
                value_type="string",
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(
                name="translation-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="vlans",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="vlans-disabled", value_type="unknown"),
            BigipPropertySpec(name="vlans-enabled", value_type="unknown"),
        ),
    )
