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
            "gtm_monitor_bigip",
            module="gtm",
            object_types=("monitor bigip",),
        ),
        header_types=(("gtm", "monitor bigip"),),
        properties=(
            BigipPropertySpec(
                name="aggregate-dynamic-ratios",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "average-members",
                    "average-nodes",
                    "none",
                    "sum-members",
                    "sum-nodes",
                ),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_bigip",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="non-default", value_type="unknown"),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
