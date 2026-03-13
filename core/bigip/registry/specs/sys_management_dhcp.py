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
            BigipPropertySpec(name="client-id", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="hostname",
                value_type="string",
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(
                name="request-options",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="send-options",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="supersede-options", value_type="boolean", allow_none=True),
        ),
    )
