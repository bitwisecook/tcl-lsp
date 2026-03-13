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
            "gtm_monitor_real_server",
            module="gtm",
            object_types=("monitor real-server",),
        ),
        header_types=(("gtm", "monitor real-server"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_real_server",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="metrics", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
