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
            "ltm_profile_traffic_acceleration",
            module="ltm",
            object_types=("profile traffic-acceleration",),
        ),
        header_types=(("ltm", "profile traffic-acceleration"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_traffic_acceleration",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="tcp-handshake-timeout", value_type="integer"),
            BigipPropertySpec(name="time-wait-timeout", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
