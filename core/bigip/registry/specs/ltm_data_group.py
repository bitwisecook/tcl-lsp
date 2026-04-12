from __future__ import annotations

from ..models import BigipObjectKindSpec, BigipObjectSpec
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "data_group", table_name="data_groups", resolver_name="resolve_data_group"
        ),
    )
