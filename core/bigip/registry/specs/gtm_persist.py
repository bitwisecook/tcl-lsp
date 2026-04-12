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
            "gtm_persist",
            module="gtm",
            object_types=("persist",),
        ),
        header_types=(("gtm", "persist"),),
        properties=(
            BigipPropertySpec(name="destination", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="key", value_type="string"),
            BigipPropertySpec(
                name="level", value_type="enum", enum_values=("application", "wideip")
            ),
            BigipPropertySpec(name="max-results", value_type="integer"),
            BigipPropertySpec(name="target-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="target-type",
                value_type="enum",
                enum_values=("datacenter", "link", "pool-member", "server"),
            ),
        ),
    )
