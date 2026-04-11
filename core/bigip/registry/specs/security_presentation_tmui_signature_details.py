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
            "security_presentation_tmui_signature_details",
            module="security",
            object_types=("presentation tmui signature-details",),
        ),
        header_types=(("security", "presentation tmui signature-details"),),
    )
