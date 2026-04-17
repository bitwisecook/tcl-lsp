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
            "ltm_profile_http3",
            module="ltm",
            object_types=("profile http3",),
        ),
        header_types=(("ltm", "profile http3"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_http3",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="header-table-size", value_type="integer"),
        ),
    )
