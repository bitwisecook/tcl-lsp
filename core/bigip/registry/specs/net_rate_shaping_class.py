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
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="ceiling", value_type="integer"),
            BigipPropertySpec(name="ceiling-percentage", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="direction",
                value_type="enum",
                enum_values=("any", "to-client", "to-server"),
            ),
            BigipPropertySpec(
                name="drop-policy",
                value_type="reference",
                references=("net_rate_shaping_drop_policy",),
            ),
            BigipPropertySpec(name="max-burst", value_type="integer"),
            BigipPropertySpec(name="parent", value_type="reference"),
            BigipPropertySpec(
                name="queue",
                value_type="enum",
                enum_values=("pfifp", "sfq"),
                references=(
                    "net_rate_shaping_queue",
                    "sys_nethsm_async_queue_stat",
                    "sys_nethsm_sync_queue_stat",
                ),
            ),
            BigipPropertySpec(name="rate", value_type="integer"),
            BigipPropertySpec(name="rate-percentage", value_type="integer"),
            BigipPropertySpec(
                name="shaping-policy",
                value_type="reference",
                allow_none=True,
                references=("net_rate_shaping_shaping_policy",),
            ),
        ),
    )
