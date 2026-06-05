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
            "vcmp_traffic_profile",
            module="vcmp",
            object_types=("traffic-profile",),
        ),
        header_types=(("vcmp", "traffic-profile"),),
        properties=(BigipPropertySpec(name="color-policer", value_type="unknown"),),
    )
