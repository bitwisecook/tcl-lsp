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
            BigipPropertySpec(
                name="daily-hour-end",
                value_type="unknown",
                default="24:00 (midnight)",
            ),
            BigipPropertySpec(
                name="daily-hour-start",
                value_type="unknown",
                default="0:00 (midnight at the start of the day)",
            ),
            BigipPropertySpec(
                name="date-valid-end",
                value_type="integer",
                default="19:14 1/18/2038 (the latest date expressible with a 32-bit integer)",
            ),
            BigipPropertySpec(
                name="date-valid-start",
                value_type="unknown",
                default="midnight 1/1/1970 (Unix epoch)",
            ),
            BigipPropertySpec(
                name="days-of-week",
                value_type="enum",
                repeated=True,
                enum_values=(
                    "friday",
                    "monday",
                    "saturday",
                    "sunday",
                    "thursday",
                    "tuesday",
                    "wednesday",
                ),
                default="all seven days",
            ),
            BigipPropertySpec(name="description", value_type="unknown"),
        ),
    )
