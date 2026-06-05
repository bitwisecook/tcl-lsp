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
            "wam_ad_policy",
            module="wam",
            object_types=("ad-policy",),
        ),
        header_types=(("wam", "ad-policy"),),
        properties=(
            BigipPropertySpec(
                name="ad-insertion-order",
                value_type="enum",
                enum_values=("random", "sequential"),
            ),
            BigipPropertySpec(
                name="ads",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify")),
                block=(
                    BigipPropertySpec(
                        name="preroll",
                        value_type="enum",
                        in_sections=("ads",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(name="url", value_type="unknown", in_sections=("ads",)),
                ),
            ),
            BigipPropertySpec(
                name="preroll",
                value_type="enum",
                in_sections=("ads",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="url", value_type="unknown", in_sections=("ads",)),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
