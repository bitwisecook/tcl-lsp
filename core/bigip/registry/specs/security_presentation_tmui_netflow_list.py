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
            "security_presentation_tmui_netflow_list",
            module="security",
            object_types=("presentation tmui netflow-list",),
        ),
        header_types=(("security", "presentation tmui netflow-list"),),
    )
