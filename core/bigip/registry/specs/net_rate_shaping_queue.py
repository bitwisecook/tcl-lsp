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
            "net_rate_shaping_queue",
            module="net",
            object_types=("rate-shaping queue",),
        ),
        header_types=(("net", "rate-shaping queue"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="pfifo-max-size", value_type="integer"),
            BigipPropertySpec(name="pfifo-min-size", value_type="integer"),
            BigipPropertySpec(name="sfq-bucket-count", value_type="integer"),
            BigipPropertySpec(name="sfq-bucket-size", value_type="integer"),
            BigipPropertySpec(name="sfq-perturbation", value_type="integer"),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("pfifo", "sfq")),
        ),
    )
