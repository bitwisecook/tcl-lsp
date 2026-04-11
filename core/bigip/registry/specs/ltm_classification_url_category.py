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
            "ltm_classification_url_category",
            module="ltm",
            object_types=("classification url-category",),
        ),
        header_types=(("ltm", "classification url-category"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="url-category-id", value_type="integer"),
            BigipPropertySpec(
                name="irule-event", value_type="enum", enum_values=("enabled", "disabled")
            ),
        ),
    )
