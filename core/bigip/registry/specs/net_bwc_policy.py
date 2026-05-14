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
            "net_bwc_policy",
            module="net",
            object_types=("bwc policy",),
        ),
        header_types=(("net", "bwc policy"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="categories", value_type="list"),
            BigipPropertySpec(
                name="ip-tos",
                value_type="integer",
                in_sections=("categories",),
                enum_values=("pass-through",),
            ),
            BigipPropertySpec(
                name="link-qos",
                value_type="enum",
                in_sections=("categories",),
                enum_values=("pass-through",),
            ),
            BigipPropertySpec(
                name="max-cat-rate",
                value_type="integer",
                in_sections=("categories",),
            ),
            BigipPropertySpec(
                name="max-cat-rate-percentage",
                value_type="integer",
                in_sections=("categories",),
            ),
            BigipPropertySpec(
                name="traffic-priority-map",
                value_type="string",
                in_sections=("categories",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dynamic", value_type="unknown"),
            BigipPropertySpec(name="ip-tos", value_type="integer", enum_values=("pass-through",)),
            BigipPropertySpec(name="link-qos", value_type="enum", enum_values=("pass-through",)),
            BigipPropertySpec(name="log-period", value_type="integer"),
            BigipPropertySpec(name="log-publisher", value_type="string", allow_none=True),
            BigipPropertySpec(name="max-rate", value_type="integer"),
            BigipPropertySpec(name="max-user-rate", value_type="integer"),
            BigipPropertySpec(name="max-user-rate-pps", value_type="integer"),
            BigipPropertySpec(name="measure", value_type="unknown"),
            BigipPropertySpec(name="traffic-priority-map", value_type="string"),
        ),
    )
