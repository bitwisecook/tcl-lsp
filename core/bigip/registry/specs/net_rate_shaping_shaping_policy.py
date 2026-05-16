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
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="ceiling-percentage", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="drop-policy",
                value_type="reference",
                allow_none=True,
                references=("net_rate_shaping_drop_policy",),
                default="none",
            ),
            BigipPropertySpec(name="max-burst", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="queue",
                value_type="reference",
                allow_none=True,
                references=(
                    "net_rate_shaping_queue",
                    "sys_nethsm_async_queue_stat",
                    "sys_nethsm_sync_queue_stat",
                ),
                default="none",
            ),
            BigipPropertySpec(name="rate-percentage", value_type="integer", default="0 (zero)"),
        ),
    )
