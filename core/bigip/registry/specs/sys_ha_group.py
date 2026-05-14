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
            "sys_ha_group",
            module="sys",
            object_types=("ha-group",),
        ),
        header_types=(("sys", "ha-group"),),
        properties=(
            BigipPropertySpec(name="active-bonus", value_type="integer"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="clusters",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("clusters",),
                allow_none=True,
            ),
            BigipPropertySpec(name="attribute", value_type="unknown", in_sections=("clusters",)),
            BigipPropertySpec(
                name="minimum-threshold",
                value_type="integer",
                in_sections=("clusters",),
            ),
            BigipPropertySpec(
                name="sufficient",
                value_type="enum",
                in_sections=("clusters",),
                enum_values=("all",),
            ),
            BigipPropertySpec(name="threshold", value_type="integer", in_sections=("clusters",)),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("clusters",)),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="pools",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("pools",),
                allow_none=True,
            ),
            BigipPropertySpec(name="attribute", value_type="unknown", in_sections=("pools",)),
            BigipPropertySpec(
                name="minimum-threshold",
                value_type="integer",
                in_sections=("pools",),
            ),
            BigipPropertySpec(name="sufficient", value_type="integer", in_sections=("pools",)),
            BigipPropertySpec(name="threshold", value_type="integer", in_sections=("pools",)),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("pools",)),
            BigipPropertySpec(
                name="trunks",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("trunks",),
                allow_none=True,
            ),
            BigipPropertySpec(name="attribute", value_type="unknown", in_sections=("trunks",)),
            BigipPropertySpec(
                name="minimum-threshold",
                value_type="integer",
                in_sections=("trunks",),
            ),
            BigipPropertySpec(name="sufficient", value_type="integer", in_sections=("trunks",)),
            BigipPropertySpec(name="threshold", value_type="integer", in_sections=("trunks",)),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("trunks",)),
        ),
    )
