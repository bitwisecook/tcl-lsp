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
            "security_firewall_schedule",
            module="security",
            object_types=("firewall schedule",),
        ),
        header_types=(("security", "firewall schedule"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference"),
            BigipPropertySpec(name="daily-hour-end", value_type="unknown"),
            BigipPropertySpec(name="daily-hour-start", value_type="unknown"),
            BigipPropertySpec(name="date-valid-end", value_type="integer"),
            BigipPropertySpec(name="date-valid-start", value_type="unknown"),
            BigipPropertySpec(
                name="days-of-week",
                value_type="enum",
                enum_values=(
                    "friday",
                    "monday",
                    "saturday",
                    "sunday",
                    "thursday",
                    "tuesday",
                    "wednesday",
                ),
            ),
            BigipPropertySpec(name="description", value_type="unknown"),
        ),
    )
