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
            BigipPropertySpec(name="daily-hour-end", value_type="string"),
            BigipPropertySpec(name="daily-hour-start", value_type="string"),
            BigipPropertySpec(name="date-valid-end", value_type="string"),
            BigipPropertySpec(name="date-valid-start", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="days-of-week",
                value_type="enum",
                repeated=True,
                enum_values=(
                    "monday",
                    "tuesday",
                    "wednesday",
                    "thursday",
                    "friday",
                    "saturday",
                    "sunday",
                ),
            ),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(
                name="daily-hour-end", value_type="string", in_sections=("security",)
            ),
            BigipPropertySpec(
                name="daily-hour-start", value_type="string", in_sections=("security",)
            ),
            BigipPropertySpec(
                name="date-valid-end", value_type="string", in_sections=("security",)
            ),
            BigipPropertySpec(
                name="date-valid-start", value_type="string", in_sections=("security",)
            ),
            BigipPropertySpec(
                name="days-of-week", value_type="list", in_sections=("security",), repeated=True
            ),
        ),
    )
