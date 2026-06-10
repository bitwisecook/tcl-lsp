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
            "gtm_monitor_bigip_link",
            module="gtm",
            object_types=("monitor bigip-link",),
        ),
        header_types=(("gtm", "monitor bigip-link"),),
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
                references=("gtm_monitor_bigip_link",),
                default="bigip_link",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="ip-address",
                default="*",
            ),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="10 seconds"),
            BigipPropertySpec(name="timeout", value_type="integer", default="30 seconds"),
        ),
    )
