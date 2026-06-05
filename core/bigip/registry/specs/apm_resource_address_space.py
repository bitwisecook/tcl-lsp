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
            "apm_resource_address_space",
            module="apm",
            object_types=("resource address-space",),
        ),
        header_types=(("apm", "resource address-space"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cfg-uri", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="discovery-interval", value_type="unknown"),
            BigipPropertySpec(
                name="dns",
                value_type="list",
                repeated=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="ipv4",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="ipv6",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="last-discovery-time", value_type="unknown"),
            BigipPropertySpec(name="max-response-size", value_type="unknown", default="128 kB"),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="trusted-ca-bundle", value_type="reference"),
            BigipPropertySpec(name="type", value_type="unknown"),
        ),
    )
