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
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_persistence_source_addr",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="hash-algorithm",
                value_type="enum",
                enum_values=("carp", "default"),
            ),
            BigipPropertySpec(
                name="map-proxies",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="map-proxy-address", value_type="string"),
            BigipPropertySpec(name="map-proxy-class", value_type="reference"),
            BigipPropertySpec(name="mask", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="match-across-pools",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="match-across-services",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="match-across-virtuals",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="mirror",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="override-connection-limit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
