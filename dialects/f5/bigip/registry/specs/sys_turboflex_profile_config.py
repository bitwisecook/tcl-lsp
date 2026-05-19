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
            "sys_turboflex_profile_config",
            module="sys",
            object_types=("turboflex profile-config",),
        ),
        header_types=(("sys", "turboflex profile-config"),),
        properties=(
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "turbofelx-asym-security",
                    "turboflex-adc",
                    "turboflex-base",
                    "turboflex-dns",
                    "turboflex-highspeed-layer4",
                    "turboflex-low-latency",
                    "turboflex-private-cloud",
                    "turboflex-security",
                ),
            ),
        ),
    )
