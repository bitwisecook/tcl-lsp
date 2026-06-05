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
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_traffic_acceleration",),
                default="traffic-acceleration",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="idle-timeout",
                value_type="integer",
                default="five minutes or 300000 milliseconds",
            ),
            BigipPropertySpec(
                name="tcp-handshake-timeout",
                value_type="integer",
                default="5000 milliseconds",
            ),
            BigipPropertySpec(
                name="time-wait-timeout",
                value_type="integer",
                default="2000 milliseconds",
            ),
        ),
    )
