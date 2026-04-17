from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_classification_auto_update_status",
            module="ltm",
            object_types=("classification auto-update status",),
        ),
        header_types=(("ltm", "classification auto-update status"),),
    )
