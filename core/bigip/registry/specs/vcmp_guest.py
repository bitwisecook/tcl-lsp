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
            "vcmp_guest",
            module="vcmp",
            object_types=("guest",),
        ),
        header_types=(("vcmp", "guest"),),
        properties=(
            BigipPropertySpec(name="allowed-slots", value_type="list"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="boot-priority", value_type="integer", default="65535"),
            BigipPropertySpec(
                name="capabilities",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="cores-per-slot", value_type="integer"),
            BigipPropertySpec(name="hostname", value_type="unknown"),
            BigipPropertySpec(name="initial-hotfix", value_type="unknown", required=True),
            BigipPropertySpec(name="initial-image", value_type="unknown", required=True),
            BigipPropertySpec(name="management-gw", value_type="unknown", required=True),
            BigipPropertySpec(name="management-ip", value_type="unknown", required=True),
            BigipPropertySpec(
                name="management-network",
                value_type="enum",
                enum_values=("bridged", "isolated"),
                default="Bridged",
            ),
            BigipPropertySpec(name="min-slots", value_type="integer", default="1"),
            BigipPropertySpec(name="slots", value_type="integer", default="1"),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                enum_values=("configured", "deployed", "provisioned"),
            ),
            BigipPropertySpec(name="traffic-profile", value_type="unknown"),
            BigipPropertySpec(name="virtual-disk", value_type="unknown"),
            BigipPropertySpec(
                name="vlans",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
