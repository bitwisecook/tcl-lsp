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
            "apm_aaa_localdb",
            module="apm",
            object_types=("aaa localdb",),
        ),
        header_types=(("apm", "aaa localdb"),),
        properties=(),
    )
