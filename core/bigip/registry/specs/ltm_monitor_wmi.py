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
            "ltm_monitor_wmi",
            module="ltm",
            object_types=("monitor wmi",),
        ),
        header_types=(("ltm", "monitor wmi"),),
        properties=(
            BigipPropertySpec(name="agent", value_type="string"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="command", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_wmi",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="metrics", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="url", value_type="unknown"),
            BigipPropertySpec(name="username", value_type="reference", allow_none=True),
        ),
    )
