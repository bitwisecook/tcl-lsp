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
            "gtm_datacenter",
            module="gtm",
            object_types=("datacenter",),
        ),
        header_types=(("gtm", "datacenter"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="contact",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="location", value_type="unknown", default="none"),
            BigipPropertySpec(name="metadata", value_type="unknown"),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="prober-fallback",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "any-available",
                    "inside-datacenter",
                    "none",
                    "outside-datacenter",
                    "pool",
                ),
                default="any-available",
            ),
            BigipPropertySpec(
                name="prober-pool",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
                references=("gtm_prober_pool",),
            ),
            BigipPropertySpec(
                name="prober-preference",
                value_type="enum",
                enum_values=("inside-datacenter", "outside-datacenter", "pool"),
                default="inside-datacenter",
            ),
            BigipPropertySpec(name="value", value_type="string"),
        ),
    )
