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
            "apm_policy_image_file",
            module="apm",
            object_types=("policy image-file",),
        ),
        header_types=(("apm", "policy image-file"),),
        properties=(),
    )
