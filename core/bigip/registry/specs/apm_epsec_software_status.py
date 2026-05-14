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
            "apm_epsec_software_status",
            module="apm",
            object_types=("epsec software-status",),
        ),
        header_types=(("apm", "epsec software-status"),),
        properties=(),
    )
