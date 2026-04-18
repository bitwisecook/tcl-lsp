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
            "net_rate_shaping_class",
            module="net",
            object_types=("rate-shaping class",),
        ),
        header_types=(("net", "rate-shaping class"),),
        properties=(
            BigipPropertySpec(name="ceiling", value_type="integer"),
            BigipPropertySpec(name="ceiling-percentage", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="direction", value_type="enum", enum_values=("any", "to-client", "to-server")
            ),
            BigipPropertySpec(name="drop-policy", value_type="string"),
            BigipPropertySpec(name="max-burst", value_type="integer"),
            BigipPropertySpec(name="parent", value_type="string"),
            BigipPropertySpec(name="queue", value_type="string"),
            BigipPropertySpec(name="rate", value_type="integer"),
            BigipPropertySpec(name="rate-percentage", value_type="integer"),
            BigipPropertySpec(name="shaping-policy", value_type="boolean", allow_none=True),
        ),
    )
