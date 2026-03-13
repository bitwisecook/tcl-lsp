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
            "sys_icall_handler_triggered",
            module="sys",
            object_types=("icall handler triggered",),
        ),
        header_types=(("sys", "icall handler triggered"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="script", value_type="string"),
            BigipPropertySpec(name="status", value_type="enum", enum_values=("active", "inactive")),
            BigipPropertySpec(
                name="subscriptions",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="event-name", value_type="string", in_sections=("subscriptions",)
            ),
            BigipPropertySpec(
                name="filters",
                value_type="enum",
                in_sections=("subscriptions",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("filters",)),
            BigipPropertySpec(
                name="match-algorithm", value_type="string", in_sections=("filters",)
            ),
        ),
    )
