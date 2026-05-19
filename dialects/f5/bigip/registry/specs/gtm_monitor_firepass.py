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
            "gtm_monitor_firepass",
            module="gtm",
            object_types=("monitor firepass",),
        ),
        header_types=(("gtm", "monitor firepass"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cipherlist", value_type="unknown", default="HIGH:!ADH"),
            BigipPropertySpec(name="concurrency-limit", value_type="integer", default="95"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_firepass",),
                default="firepass_gtm",
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
            BigipPropertySpec(name="max-load-average", value_type="unknown", default="12"),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="timeout", value_type="integer", default="90 seconds"),
            BigipPropertySpec(name="username", value_type="reference", default="gtmuser"),
        ),
    )
