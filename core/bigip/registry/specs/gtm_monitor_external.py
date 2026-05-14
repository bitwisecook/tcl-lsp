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
            BigipPropertySpec(name="args", value_type="unknown", allow_none=True, default="none"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_external",),
                default="external",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="run", value_type="unknown", default="none"),
            BigipPropertySpec(name="timeout", value_type="integer", default="120 seconds"),
            BigipPropertySpec(name="user-defined", value_type="reference", allow_none=True),
        ),
    )
