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
            "net_rate_shaping_shaping_policy",
            module="net",
            object_types=("rate-shaping shaping-policy",),
        ),
        header_types=(("net", "rate-shaping shaping-policy"),),
        properties=(
            BigipPropertySpec(name="ceiling-percentage", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="drop-policy", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="max-burst", value_type="integer"),
            BigipPropertySpec(name="queue", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="rate-percentage", value_type="integer"),
        ),
    )
