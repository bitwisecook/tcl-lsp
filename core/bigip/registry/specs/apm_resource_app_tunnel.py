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
            "apm_resource_app_tunnel",
            module="apm",
            object_types=("resource app-tunnel",),
        ),
        header_types=(("apm", "resource app-tunnel"),),
        properties=(
            BigipPropertySpec(name="acl-order", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="application-launch-warning",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="apps",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="customization-group",
                value_type="list",
                required=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "app-tunnel",
                    "last",
                    "network-access",
                    "remote-desktop",
                    "web-application",
                ),
            ),
        ),
    )
