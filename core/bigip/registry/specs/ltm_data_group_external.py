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
            "ltm_data_group_external",
            module="ltm",
            object_types=("data-group external",),
        ),
        header_types=(("ltm", "data-group external"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="external-file-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="separator", value_type="string"),
            BigipPropertySpec(name="source-path", value_type="string"),
            BigipPropertySpec(name="type", value_type="integer"),
        ),
    )
