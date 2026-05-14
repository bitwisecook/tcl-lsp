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
            "gtm_monitor_external",
            module="gtm",
            object_types=("monitor external",),
        ),
        header_types=(("gtm", "monitor external"),),
        properties=(
            BigipPropertySpec(name="args", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_external",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="run", value_type="unknown"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="user-defined", value_type="reference", allow_none=True),
        ),
    )
