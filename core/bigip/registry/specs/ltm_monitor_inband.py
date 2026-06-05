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
            "ltm_monitor_inband",
            module="ltm",
            object_types=("monitor inband",),
        ),
        header_types=(("ltm", "monitor inband"),),
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
                references=("ltm_monitor_inband",),
                default="inband",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="failure-interval", value_type="integer", default="30"),
            BigipPropertySpec(
                name="failures",
                value_type="integer",
                default="3, which means that the system marks the pool member unavailable at the fourth failure",
            ),
            BigipPropertySpec(name="response-time", value_type="integer"),
            BigipPropertySpec(name="retry-time", value_type="integer"),
        ),
    )
