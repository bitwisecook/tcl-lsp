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
            "ltm_data_group_internal",
            module="ltm",
            object_types=("data-group internal",),
        ),
        header_types=(("ltm", "data-group internal"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="records",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="data", value_type="string", in_sections=("records",)),
            BigipPropertySpec(name="type", value_type="integer"),
        ),
    )
