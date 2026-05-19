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
            "gtm_distributed_app",
            module="gtm",
            object_types=("distributed-app",),
        ),
        header_types=(("gtm", "distributed-app"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="dependency-level",
                value_type="enum",
                allow_none=True,
                enum_values=("datacenter", "link", "none", "server", "wideip"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="disabled-contexts",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="persist-cidr-ipv4", value_type="integer"),
            BigipPropertySpec(name="persist-cidr-ipv6", value_type="integer"),
            BigipPropertySpec(
                name="persistence",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="ttl-persistence", value_type="integer", default="3600"),
            BigipPropertySpec(
                name="wideips",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none"),
                default="none",
            ),
        ),
    )
