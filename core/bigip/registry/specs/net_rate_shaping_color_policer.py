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
            "net_rate_shaping_color_policer",
            module="net",
            object_types=("rate-shaping color-policer",),
        ),
        header_types=(("net", "rate-shaping color-policer"),),
        properties=(
            BigipPropertySpec(name="action", value_type="unknown"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="committed-burst-size", value_type="integer"),
            BigipPropertySpec(name="committed-information-rate", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="excess-burst-size", value_type="integer"),
        ),
    )
