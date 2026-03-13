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
            "ltm_snatpool",
            module="ltm",
            object_types=("snatpool",),
        ),
        header_types=(("ltm", "snatpool"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="members", value_type="enum", allow_none=True, enum_values=("default", "none")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
