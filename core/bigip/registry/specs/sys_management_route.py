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
            "sys_management_route",
            module="sys",
            object_types=("management-route",),
        ),
        header_types=(("sys", "management-route"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="gateway",
                value_type="reference",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
                references=("net_self", "net_route", "ltm_virtual_address"),
            ),
            BigipPropertySpec(name="mtu", value_type="string"),
            BigipPropertySpec(name="network", value_type="string"),
            BigipPropertySpec(
                name="type", value_type="enum", enum_values=("interface", "blackhole")
            ),
        ),
    )
