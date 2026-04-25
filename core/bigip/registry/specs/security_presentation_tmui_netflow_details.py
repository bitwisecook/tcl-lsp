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
            "security_presentation_tmui_netflow_details",
            module="security",
            object_types=("presentation tmui netflow-details",),
        ),
        header_types=(("security", "presentation tmui netflow-details"),),
    )
