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
            BigipPropertySpec(name="contact", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="location", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="prober-fallback",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "any-available",
                    "inside-datacenter",
                    "outside-datacenter",
                    "pool",
                    "none",
                ),
            ),
            BigipPropertySpec(
                name="prober-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="prober-preference",
                value_type="enum",
                enum_values=("inside-datacenter", "outside-datacenter", "pool"),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
