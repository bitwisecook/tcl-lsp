from __future__ import annotations

from ..models import BigipObjectKindSpec, BigipObjectSpec
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "profile", table_name="profiles", resolver_name="resolve_profile"
        ),
        header_types=(("ltm", "profile server-ssl"),),
    )
