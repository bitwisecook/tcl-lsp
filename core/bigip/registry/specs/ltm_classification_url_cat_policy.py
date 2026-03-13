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
            "ltm_classification_url_cat_policy",
            module="ltm",
            object_types=("classification url-cat-policy",),
        ),
        header_types=(("ltm", "classification url-cat-policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="feed-lists", value_type="enum", repeated=True, enum_values=("add", "delete")
            ),
        ),
    )
