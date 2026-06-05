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
            "ltm_persistence_source_addr",
            module="ltm",
            object_types=("persistence source-addr",),
        ),
        header_types=(("ltm", "persistence source-addr"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_persistence_source_addr",),
                default="source_addr, the system default cookie persistence profile",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="hash-algorithm",
                value_type="enum",
                enum_values=("carp", "default"),
                default="default (no hash persistence)",
            ),
            BigipPropertySpec(
                name="map-proxies",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="map-proxy-address",
                value_type="string",
                shape_kind="ip-address",
                default="any which results in the default behavior of using one of the IP addresses from the proxy data-group/class",
            ),
            BigipPropertySpec(
                name="map-proxy-class",
                value_type="reference",
                default="none which will result in map_proxies using the class defined by the DB variable Persist",
            ),
            BigipPropertySpec(
                name="mask",
                value_type="string",
                allow_none=True,
                shape_kind="ip-address",
                default="::",
            ),
            BigipPropertySpec(
                name="match-across-pools",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="match-across-services",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="match-across-virtuals",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="mirror",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="override-connection-limit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="timeout", value_type="integer", default="180 seconds"),
        ),
    )
