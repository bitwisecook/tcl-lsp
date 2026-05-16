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
            "sys_management_dhcp",
            module="sys",
            object_types=("management-dhcp",),
        ),
        header_types=(("sys", "management-dhcp"),),
        properties=(
            BigipPropertySpec(name="client-id", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="hostname", value_type="string"),
            BigipPropertySpec(
                name="request-options",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="send-options",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="supersede-options", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="value", value_type="unknown"),
        ),
    )
