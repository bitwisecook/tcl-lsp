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
            "ltm_profile_doh_server",
            module="ltm",
            object_types=("profile doh-server",),
        ),
        header_types=(("ltm", "profile doh-server"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_doh_server",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
