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
            "pem_reporting_format_script",
            module="pem",
            object_types=("reporting format-script",),
        ),
        header_types=(("pem", "reporting format-script"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="definition", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
