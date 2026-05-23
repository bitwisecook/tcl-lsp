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
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="categories",
                value_type="list",
                required=True,
                block=(
                    BigipPropertySpec(
                        name="ip-tos",
                        value_type="integer",
                        in_sections=("categories",),
                        enum_values=("pass-through",),
                        default="pass-through, which indicates, do not modify UDP packets",
                    ),
                    BigipPropertySpec(
                        name="link-qos",
                        value_type="enum",
                        in_sections=("categories",),
                        enum_values=("pass-through",),
                        default="pass-through, which indicates, do not modify UDP packets",
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
                        usage_flags=frozenset(("optional",)),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="ip-tos",
                value_type="integer",
                in_sections=("categories",),
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
            ),
            BigipPropertySpec(
                name="link-qos",
                value_type="enum",
                in_sections=("categories",),
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
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
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dynamic",
                value_type="unknown",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="ip-tos",
                value_type="integer",
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
            ),
            BigipPropertySpec(
                name="link-qos",
                value_type="enum",
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
            ),
            BigipPropertySpec(name="log-period", value_type="integer"),
            BigipPropertySpec(name="log-publisher", value_type="string", allow_none=True),
            BigipPropertySpec(name="max-rate", value_type="integer"),
            BigipPropertySpec(name="max-user-rate", value_type="integer"),
            BigipPropertySpec(
                name="max-user-rate-pps",
                value_type="integer",
                default="0 (not configured)",
            ),
            BigipPropertySpec(name="measure", value_type="unknown"),
            BigipPropertySpec(
                name="traffic-priority-map",
                value_type="string",
                usage_flags=frozenset(("optional",)),
            ),
        ),
    )
