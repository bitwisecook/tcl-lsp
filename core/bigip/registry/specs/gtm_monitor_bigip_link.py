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
                name="defaults-from", value_type="reference", references=("gtm_monitor_bigip_link",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
