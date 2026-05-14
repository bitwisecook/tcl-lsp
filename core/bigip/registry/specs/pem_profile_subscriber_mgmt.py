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
            "pem_profile_subscriber_mgmt",
            module="pem",
            object_types=("profile subscriber-mgmt",),
        ),
        header_types=(("pem", "profile subscriber-mgmt"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("pem_profile_subscriber_mgmt",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dhcp-lease-query", value_type="unknown"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("dhcp-lease-query",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="vs-name",
                value_type="reference",
                in_sections=("dhcp-lease-query",),
            ),
            BigipPropertySpec(
                name="drop-unknown-traffic-from-server-side",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="sess-creation-from-server-side",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
