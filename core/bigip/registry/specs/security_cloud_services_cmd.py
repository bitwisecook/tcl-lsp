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
            "security_cloud_services_cmd",
            module="security",
            object_types=("cloud-services cmd",),
        ),
        header_types=(("security", "cloud-services cmd"),),
    )
