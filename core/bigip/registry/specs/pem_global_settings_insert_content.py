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
            "pem_global_settings_insert_content",
            module="pem",
            object_types=("global-settings insert-content",),
        ),
        header_types=(("pem", "global-settings insert-content"),),
        properties=(BigipPropertySpec(name="max-duration", value_type="unknown"),),
    )
