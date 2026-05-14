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
            "net_rate_shaping_drop_policy",
            module="net",
            object_types=("rate-shaping drop-policy",),
        ),
        header_types=(("net", "rate-shaping drop-policy"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="average-packet-size", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="fred-max-active",
                value_type="integer",
                default="0 (zero),which disables active flow limitation",
            ),
            BigipPropertySpec(name="fred-max-drop", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="fred-min-drop", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="inverse-weight", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="max-probability", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="max-threshold", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="min-threshold", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="red-hard-limit", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("fred", "red", "tail"),
                default="tail",
            ),
        ),
    )
