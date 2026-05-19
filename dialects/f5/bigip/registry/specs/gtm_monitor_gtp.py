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
            "gtm_monitor_gtp",
            module="gtm",
            object_types=("monitor gtp",),
        ),
        header_types=(("gtm", "monitor gtp"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_gtp",),
                default="gtp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string", shape_kind="endpoint"),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
            BigipPropertySpec(name="probe-attempts", value_type="integer", default="3"),
            BigipPropertySpec(name="probe-interval", value_type="integer", default="1"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="protocol-version", value_type="integer", default="version 1"),
            BigipPropertySpec(name="timeout", value_type="integer", default="120 seconds"),
        ),
    )
