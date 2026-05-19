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
            "pem_global_settings_hsl_report",
            module="pem",
            object_types=("global-settings hsl-report",),
        ),
        header_types=(("pem", "global-settings hsl-report"),),
        properties=(BigipPropertySpec(name="format-version", value_type="unknown"),),
    )
