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
            "sys_ecm_cloud_provider",
            module="sys",
            object_types=("ecm cloud-provider",),
        ),
        header_types=(("sys", "ecm cloud-provider"),),
        properties=(),
    )
